# D98 last textures: GCM remap 0x02ff and the volume texture — 2026-10-07

## Question

After D95, 3 of 23,247 texture hashes on 01.33 did not decode: 2 B8 4096x4096 masks with
GCM remap 0x02ff (583a0cff, GUID 00059f4f; eddd7021, GUID 0005d11c; patch_0127 and
patch_0128) and 1 A8R8G8B8 16x16x16 volume (6998ccc7, GUID 000079d8, DATA.FARC). What
does the remap word mean bit by bit, and how are a volume's slices laid out?

## Sources (read for findings; nothing copied or translated)

| Source | Licence | Pin | Used for |
|---|---|---|---|
| PSL1GHT `ppu/include/rsx/gcm_sys.h` (open PS3 SDK) | MIT (repo licence; no header in file) | f649a08fd536a9e27c08c7db2d93a2d7ee4c3bbe (master on 2026-10-07) | remap field shifts and mode values; LN flag 0x20; 128-byte swizzled cube face alignment |
| Toolkit `cwlib/enums/CellGcm.java`, `structs/texture/CellGcmTexture.java` | MIT | d6a2ef80b5a09999f08f23a803b516096055e5c9 | REMAP_FROM_A..B = 0..3; REMAP_ARGB 0xaae4, REMAP_BBBB 0xa9ff |
| RPCS3 `rpcs3/Emu/RSX/gcm_enums.h`, `RSX/VK/VKFormats.cpp`, `RSX/Utils/image_utils.hpp`, `RSX/Common/TextureUtils.cpp` | GPL-2.0 (read only, findings recorded in own words) | 8ffc512beb8c9605355c949e6c5a3f755df4d87e | remap enum values; B8 channel content before the remap; 3D swizzle order; swizzled vs LN decision |

Other local references (`references/`: lbp_bgm, cwlib-rs, farctool2, lbptoolspy-lite,
lbp-documentation-wiki) contain no remap or volume rules beyond Toolkit's constants.

## Rules (written before implementing)

| Rule | Evidence | Source |
|---|---|---|
| Remap word bits 0..7: four 2-bit source selects for outputs A (bits 0-1), R (2-3), G (4-5), B (6-7); select values A=0, R=1, G=2, B=3 | `CONFIRMED_FROM_COMMUNITY_CODE` | PSL1GHT `GCM_TEXTURE_REMAP_COLOR_{A,R,G,B}_SHIFT` 0/2/4/6 and `..._COLOR_*` 0..3; Toolkit `CellGcm.REMAP_FROM_*`; RPCS3 `CELL_GCM_TEXTURE_REMAP_FROM_*` |
| Remap word bits 8..15: four 2-bit output modes for A (8-9), R (10-11), G (12-13), B (14-15); 0 zero, 1 one, 2 remap (take the selected source) | `CONFIRMED_FROM_COMMUNITY_CODE` | PSL1GHT `..._TYPE_{A,R,G,B}_SHIFT` 8/10/12/14 and `..._TYPE_{ZERO,ONE,REMAP}` 0/1/2; RPCS3 `CELL_GCM_TEXTURE_REMAP_{ZERO,ONE,REMAP}` |
| Bit 16 is a remap order flag (XYXY/XXXY) for two-channel formats | `CONFIRMED_FROM_COMMUNITY_CODE` | PSL1GHT `GCM_TEXTURE_REMAP_ORDER_*`, RPCS3 RSXTexture remap override comments; refused here (0 users) |
| A B8 texel reads as A = one, R = G = B = the byte before the remap | `CONFIRMED_FROM_COMMUNITY_CODE` | RPCS3 VKFormats B8 component mapping (ONE, R, R, R in A, R, G, B order) |
| Swizzled (no LN 0x20) volume: texel (x, y, z) at the index whose bits are taken in turn from x, y, z starting at bit 0, an axis leaving the rotation once its log2 extent is used up; depth 1 reduces to the 2D order Toolkit uses | `CONFIRMED_FROM_COMMUNITY_CODE` (emulator only) | RPCS3 `image_utils.hpp` `calculate_z_index` / `convert_linear_swizzle_3d`, `TextureUtils.cpp` swizzle decision |
| A volume mip is one block of all its slices (slices back to back for LN; one 3D swizzle block when swizzled) | `CONFIRMED_FROM_COMMUNITY_CODE` | RPCS3 `TextureUtils.cpp` subresource layout (pitch x height x depth per mip) |

**Consistency with the known words.** 0xaae4: modes 2,2,2,2, selects A<-A, R<-R, G<-G,
B<-B (identity). 0xa9ff: A mode 1 (one), R/G/B mode 2 from B; with B = byte this is
Toolkit's opaque gray L8 (and RPCS3's B8 mapping gives the same for 0xaae4, matching the
25 Toolkit-compared 0xaae4 masks). **0x02ff**: A mode 2 from B (the byte), R/G/B mode 0:
RGBA = (0, 0, 0, byte), an alpha-only mask. D95's STRONG_INFERENCE is now
`CONFIRMED_FROM_COMMUNITY_CODE` (the zero mode is a documented value, not inferred).

**Conflicts.** Toolkit ignores the remap word when exporting (both 0x02ff masks would
export as opaque gray) and has no volume layout (it 2D-unswizzles the first w x h
texels). Both are Toolkit limitations, not competing rules; there is no oracle for D98.

## Local experiment (failed to discriminate)

`CONFIRMED_FROM_LOCAL_DATA` for the header only: 6998ccc7 is GTF 0x85 (swizzled), mips 1,
dim 3, remap 0xaae4, 16x16x16, expanded 16,384 bytes = 16^3 x 4 exactly. A scratch script
summed neighbour texel differences under three layouts (3D interleave, 2D swizzle per
slice, linear slices): 3,923,808 / 3,869,346 / 3,827,757, i.e. within 3 %; alpha is 255
everywhere and 3,882 of 4,096 texels are distinct. The volume is noise-like, so local
smoothness cannot choose a layout; the slice order rests on RPCS3 alone.

## Implementation (`crates/lbp-texture/src/lib.rs`)

- `remap_channels(remap)`: rejects bits above 15 and mode 3; returns R, G, B, A outputs as
  zero / one / source index. Uncompressed GTF (A8R8G8B8, B8, swizzled and LN) accept any
  word that passes; BC1/BC3 still require 0xaae4 (BC remap untested, 0 users). Source
  channels: A8R8G8B8 big-endian A, R, G, B; B8 (255, b, b, b).
- Volumes (dimension 3, cube byte 0, depth > 1): `inspect`/`decode`/`decode_checked`/
  `decode_reader` refuse as `volume texture (2D decode)`. The layered entry points accept
  swizzled A8R8G8B8/B8 with power-of-two w, h, depth; BC or LN volumes are refused as
  `compressed or linear volume texture`. Budgets: w*h*depth (checked) under the per-format
  pixel budget, top-mip bytes under `MAX_EXPANDED`, RGBA of all slices under `MAX_RGBA`.
  `LayeredTexture`: `texture` = slice 0, `extra_faces` = slices 1..depth, layout
  "volume", `faces_decoded` = depth, `face_stride` None. Not rendered (slice 0 only would
  reach a preview, and no mesh route reaches it).
- `swizzle_index_3d` is written from the rule above; a unit test checks it equals the D91
  2D `swizzle_index` at depth 1.
- `corpus_texture_census`: decoded rows record `remap`; totals add
  `decoded_volume_hashes`; `decoded_remap_words_hashes` counts decoded GTF remap words.
  Its unit fixture for "unsupported" now uses GTF 0x83 (0x85 with remap 0 is now a valid
  all-zero remap and fails as truncated instead).
- `linked.rs` untouched; viewers unchanged (D95's 4 Mi GPU cap still refuses the 4096^2
  masks at upload).

Synthetic tests (`crates/lbp-texture/tests/d98_last.rs`, hand-built bytes): B8 under
0xaae4/0xa9ff gray, 0x02ff alpha mask (swizzled and LN); A8R8G8B8 identity, reversed
selects 0xaa1b, all-one 0x55e4, all-zero 0x00e4, 0xa9ff; mode 3 and bit 16 refused; BC1
non-identity refused; 4x2x8 volume with each texel tagged by (x, y, z) at an independently
hand-written bit order (x0 y0 z0 x1 z1 z2) decodes all slices through both layered entry
points; 2x2x2 B8 0x02ff volume; cancellation; 2D refusal, BC/LN volume, depth 3, cube
byte on a volume, one byte short, 8 Mi-pixel and 4096x4096x32768 budgets. `gtf_plain.rs`
and `d95_rest.rs` expectations updated (0x02ff/0xa9ff-on-ARGB no longer refused; volumes
refused by the 2D path with the new reason and decoded by the layered path).

## Counts (01.33, disc + update 0133, snapshot d68-inventory-0133-verified.json)

Unit: distinct texture hash (and distinct catalog GUID), scope all 23,247 texture MAP
hashes, top mip only (all faces/slices for layered textures). Baseline D95
(`b96-texture-census-0133.json`).

| Outcome | D95 hashes / GUIDs | D98 hashes / GUIDs |
|---|---:|---:|
| Decoded | 23,244 / 23,474 | 23,247 / 23,477 |
| Unsupported | 3 / 3 | 0 |

Newly decoded: +3 hashes / +3 GUIDs (B8 remap 0x02ff 2, A8R8G8B8 volume 1); no other hash
changed outcome or expanded size. Decoded remap words: 0xaae4 20,569, 0xa9ff 3, 0x02ff 2
(DDS-wrapped textures carry none). Texture decode is not mesh viewability: +0 newly
viewable / +0 newly Complete meshes claimed (no mesh census rerun; the masks exceed the
GPU upload cap and the volume is not reachable from mesh routes).

Output `local-analysis/d98-texture-census-0133.json` (13,528,691 bytes, SHA-256
3faef4232d227ce1039008aa231bb88079974bf25e81219b6faae622bd6490b7), 1,089,159,991
archived bytes read, 293 s release.

## Oracle parity (pinned Toolkit, DATA.FARC only)

`local-analysis/d98-texture-oracle-samples.json`: the volume (offset 915,376,966, 12,527
bytes). Toolkit (unchanged `scripts/run-texture-oracle.ps1`, pin clean, self-test pass)
excludes it: `non-simple 2D GTF excluded`; Toolkit has no volume decode to compare, so
`texture_parity` reports it as 1 unmatched sample (no Toolkit digest), not a pixel
conflict. The two 0x02ff masks are patch-only (not in DATA.FARC) and Toolkit ignores the
remap word, so they are not oracle-compared either. Regressions with the D98 decoder: D95
oracle file 7 compared, 0 mismatches; D91 oracle file 20 compared, 0 mismatches.

## Commands

Worktree build (`CARGO_TARGET_DIR` inside the worktree); game-data runs from the main
checkout with the worktree's release examples; outputs under its ignored local-analysis/.

```
cargo fmt --all --check                                       -> clean
cargo clippy -q -p lbp-texture -p lbp-preview -p lbp-catalog --all-targets -- -D warnings -> clean
cargo clippy -q -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings -> clean
cargo test -q -p lbp-texture                                  -> all pass (5 new in d98_last.rs, 1 unit)
cargo test -q -p lbp-preview -p lbp-catalog --all-targets     -> all pass (after the 0x83 fixture fix)
cargo test -q -p lbp-bevy-viewer                              -> pass
cargo build -q -p lbp-bevy-viewer -p lbp-viewer               -> ok
corpus_texture_census local-analysis\d68-inventory-0133-verified.json "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d98-texture-census-0133.json -> decoded 23247/23477, exit 0
scripts\run-texture-oracle.ps1 -Output local-analysis/d98-texture-oracle.json -SampleManifest local-analysis/d98-texture-oracle-samples.json -> 1 sample, excluded by Toolkit
texture_parity SNAPSHOT GAME-DIRS local-analysis\d98-texture-oracle.json local-analysis\d98-texture-parity.json -> 1 sample, 1 unmatched (no Toolkit digest)
texture_parity ... local-analysis\d95-texture-oracle.json local-analysis\d98-texture-parity-d95-samples.json -> 7 compared, 0 mismatches
texture_parity ... local-analysis\d91-texture-oracle.json local-analysis\d98-texture-parity-d91-samples.json -> 20 compared, 0 mismatches
```

## Not tested

- No oracle for either rule: the remap meaning rests on PSL1GHT + Toolkit constants +
  RPCS3; the volume order on RPCS3 alone (local data could not discriminate). Hardware
  behaviour, filtering, higher volume mips and how the game samples the volume are
  unclaimed.
- No visual or viewer check (no native windows), no mesh census rerun; lbp-viewer tests
  not rerun (D95 noted its diagnostics tests need `local-analysis/` in the cwd).
