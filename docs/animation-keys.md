# ANMb keyframes and locators (runtime plan item Rc)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Third step of
`docs/runtime-research.md` phase R1, after D110 (bones) and Rb (skin streams). CPU only;
independent Rust; no playback UI.

## What changed

- `lbp_resource::animation::decode(bytes)` → `Animation`: the D24 prefix plus the whole
  `animData` blob (bone hash/links, raw locator-key records, the four animated-track index
  lists, rotation/position/scale/morph key tables) and the `Locator` array after it.
  Diagnostics: `clamped_rotations`, `trailing_bytes` (blob bytes after the morph keys),
  `unparsed_after_locators`.
- `Animation::pose(frame)` builds the full per-bone TRS and morph weights for an integer
  frame; `Animation::looped_frame(frame, looped)` maps a running frame number.
- `inspect` is unchanged in behaviour (it now shares `inspect_data` with `decode`).
- `lbp-preview` example `corpus_animation_census` for the PC check below.

## Rules and provenance

CONFIRMED_FROM_COMMUNITY_CODE, Toolkit
[d6a2ef80 RAnimation.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RAnimation.java)
(`serialize` read branch for version >= 0x378, `getLoopedFrame`, `getAnimDataForFrame`,
`unpack`), `structs/animation/Locator.java`, `io/streams/MemoryInputStream.java`.

| Rule | Detail |
|---|---|
| Blob order | bones × {u32 animHash, i8 parent, i8 firstChild, i8 nextSibling, pad}; locatorKeys × 4 raw bytes; rot, pos, scale, morph index lists (u8 each); pad to even offset; key tables |
| Table sizes | rotations/positions/scales: `bones + animated × (frames − 1)`; morphs: `morphCount + morphAnimated × (frames − 1)` |
| Rotation | 3 × BE i16. x has its LSB cleared before use; LSB set = negative w. Each / 32767; w = √(1 − x² − y² − z²) |
| Position, scale, morph | BE IEEE binary16 |
| Frame lookup | frame 0 = first `bones` (or `morphCount`) entries; frame f ≥ 1 track k at `base + (f − 1) × animated + k`, applied to index `animated[k]` |
| Loop | looped: `loopStart + (f − loopStart) mod (frames − loopStart)`; otherwise clamp to [0, frames − 1] |
| Locators | after the byte array: array count (payload integer form), then {v3 f32, str (s32 length: zigzag when integers are compressed), i8 looping, i8 type} |

Conflicts and our choices (labelled; not format facts):
- **f16:** Toolkit's `MemoryInputStream.f16` sets 0x3ff low mantissa bits on exact powers
  of two, and its subnormal loop condition (`!= 0`) does not normalise. We decode exact
  IEEE binary16. Values differ from Toolkit by under 1.3e-4 relative on powers of two, and
  on subnormals. An oracle comparison must use a tolerance.
- **Morph frame 0:** Toolkit's `getAnimDataForFrame` applies morph tracks at frame 0 with
  offset `morphCount − morphAnimated`, unlike the bone channels. We apply morph tracks only
  for frame ≥ 1.
- **w:** a negative radicand (Java NaN) is clamped to w = 0 and counted.
- **Track indices** past the bone (or morph) count are Invalid: no pose can be built.
- **Zero frames** are accepted only with no animated tracks.
- **Unknown, kept raw:** locator-key records (Toolkit skips them), `fat` (Toolkit's comment
  guesses 4-byte vs 2-byte keys), and `posOffset`/`posScale` (read, never applied by Toolkit).
  The census reports them so a local check can decide (runtime checks S5, S6).

Budgets: inherited prefix limits (2 MiB input, 16 MiB expanded), ≤ 255 bones and tracks
(u8 counts), morph count ≤ 127, 4,096 locators, 1,024-byte locator names; cancellation
every 1,024 keys and per locator.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0): fmt clean; `cargo test -p lbp-resource`
passes (binary16 unit test; `tests/animation_keys.rs`: tables, locators, poses at
frames 0-2, looping, both integer forms, bad track index, truncation, cancel); existing
D24 prefix tests unchanged; Clippy `-D warnings` clean on the CI crates.
Not tested here: lbp-bevy-viewer (CI), game data, Toolkit value parity.

## PC check (Integrator)

```powershell
cargo run -q --release -p lbp-preview --example corpus_animation_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/NEW-animation-census.json
```

Expected: every animation hash decodes, or fails with a named reason. Then:
- `trailing_bytes_histogram`: values only in 0..15 means 16-bit keys everywhere (answers
  the `fat` question together with `fat_histogram`); larger values are a finding.
- `bone_sets_top20` should contain the 43-hash Sackboy set. Compare it with the D110
  skeleton census signature (runtime check S4).
- `locator_names` lists the event names (runtime check S6).

Optional deeper check: extend the D23 Toolkit oracle to dump `getFrameData` values for a
few files and compare them with `pose()` within the f16 tolerance above.

Lead for later: the type table also names `animation_bank` (ABK), `animation_set` (AST) and
`animation_map` (AMP) resources. These are candidates for the state→animation mapping the
runtime plan lists as unknown.

Counts: +0 newly viewable, +0 newly Complete (no rendering change).

## Next

Rd: Bevy skinned preview using D110 bones, Rb skin streams and these poses.
