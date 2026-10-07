# D82 legacy CLAMP (5) and mirror-once (6-8) wrap modes — 2026-10-07

## Rule

- Raw wrap 5 (`CLAMP`) is accepted as its own `TextureAddress::Clamp` variant. It keeps
  raw value 5 in the IR (`raw()`, census `display_wrap_uv_raw`), is never merged with
  raw 3, and is displayed with edge clamping (Bevy `ImageAddressMode::ClampToEdge`,
  legacy eframe `TextureOptions::LINEAR` when both axes match). Label:
  "Clamp (legacy 5, shown edge-clamped)". Legacy linear-filter edge blending toward a
  border color is not modelled; this is a display approximation, not a fidelity claim.
- Raw 6, 7, 8 (`MIRROR_ONCE_CLAMP_TO_EDGE`, `MIRROR_ONCE_BORDER`, `MIRROR_ONCE_CLAMP`)
  stay explicitly unsupported with a dedicated "mirror-once" refusal. Raw 0, 4 (outside
  the D58 interior-only BORDER path), 9+ and absent axes stay refused as before.
- No disk-layout change: `lbp-resource` already retains both raw 8-slot arrays. Change
  is limited to `lbp-preview` addressing plus the two viewer sampler adapters; parsing
  crates stay independent of rendering.

## Evidence

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (read only, nothing copied):

- `CONFIRMED_FROM_COMMUNITY_CODE` (value-to-name): `lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java`
  enum values 1 WRAP, 2 MIRROR, 3 CLAMP_TO_EDGE, 4 BORDER, 5 CLAMP,
  6 MIRROR_ONCE_CLAMP_TO_EDGE, 7 MIRROR_ONCE_BORDER, 8 MIRROR_ONCE_CLAMP; read per
  slot by `RGfxMaterial::serialize` (`wrapS`/`wrapT` enum arrays).
- `CONFIRMED_FROM_COMMUNITY_CODE` (display mapping for 5): `lib/cwlib-gl/.../exporter/SceneExporter.java`
  (glTF sampler selection) and `lib/cwlib-gl/.../jobs/MultiMaterialBaker.java`
  (`MapTextureWrapParameter`) both treat CLAMP like CLAMP_TO_EDGE.
  Conflict: `lib/cwlib-gl/.../jobs/MaterialBaker.java` `MapTextureWrapParameter` has no
  CLAMP case and falls through to its REPEAT default. We follow the two explicit
  mappings; the default fall-through is not treated as a rule.
- `UNKNOWN` (6-8): no Toolkit file maps any mirror-once value to sampling behavior
  (all three bakers/exporters fall to their REPEAT default). The names suggest
  "mirror once around 0, then the named clamp", but that is name inference only, and
  pinned Bevy 0.19.1 `ImageAddressMode` has no mirror-once mode; emulation would need
  UV folding, which D54 policy excludes. Therefore refused.
- Applicability: GMAT revisions in the D68 01.33 census (920..1016); no revision gating
  for the enum observed in Toolkit.

## Counts (BCUS98245 01.00 + update 01.33 overlay)

Source `local-analysis/d68-material-census-0133b.json` (8,386 MAP rows, 8,159 clear
material locations, 0 unavailable); summary `local-analysis/d82-wrap-mode-counts.json`.
Unit: material locations whose S or T array holds the value in any of the 8 slots
(over-counts slots not bound to a sampler).

| raw | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| locations | 8,159 | 5 | 74 | 2 | 0 | 0 | 0 | 0 |

Modes 5-8: 0 locations before and 0 after; the change cannot alter route selection on
01.33. **+0 newly viewable / +0 newly Complete** (unit distinct catalog GUID; scope
01.33 material census). Corpus asset totals unchanged/unknown as in STATUS-CURRENT.

## Tests

Synthetic only: `wrapping.rs` covers all 16 pairs of {1,2,3,5}, raw-5 retention,
3/5 non-uniform distinctness, mirror-once refusal text for 6-8, and refusal of
0/4/6/7/8/9/255/absent; `border_domain.rs` BORDER+mirror-once refusal; Bevy adapter
sampler test adds [5,1], [2,5], [5,5].

## Commands (worktree, project-local toolchain)

- `cargo fmt --all --check`: pass.
- `cargo test -q -p lbp-preview`: pass (23 unit + others).
- `cargo test -q -p lbp-bevy-viewer -p lbp-viewer`: bevy pass; lbp-viewer 63 pass,
  2 fail in `app::diagnostics` (`private_root()` canonicalizes `<workspace>/local-analysis`,
  absent in a fresh worktree; environment-only, unrelated to wrap code).
- `cargo clippy -q -p lbp-preview -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings`: pass.

## Not tested

Native GPU result for raw 5 (no corpus user exists), original-game edge/filter
behavior of legacy CLAMP, any mirror-once rendering, Vita/LBP1/LBP3 data, viewer
capture (none needed: zero affected assets). No decryption or disassembly used.
