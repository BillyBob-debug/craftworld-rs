# D95 remaining textures: B8 budget, BC1 cubes, volume, remap 0x02ff — 2026-10-07

## Question

After D91, 32 of 23,247 texture hashes on 01.33 did not decode: 25 B8 over the 4 Mi
pixel budget, 4 BC1 cube maps, 1 A8R8G8B8 volume, 2 B8 with remap 0x02ff. Which of
these can decode with Toolkit evidence, within explicit memory bounds?

## Local measurements (scratch script, FARC tables + 32-byte GTF headers + chunk tables)

Read-only probe of the 32 hashes (`d91-texture-census-0133.json` rows), label
`CONFIRMED_FROM_LOCAL_DATA`:

| Group | Header | Expanded payload | Archive |
|---|---|---|---|
| 25 B8 over budget | 0x81, 4096x4096, 13 mips, remap 0xaae4, pitch 0 | 22,369,664 = 13-mip chain 22,369,621 rounded up to 128 | 20 DATA.FARC, 5 patches |
| 2 B8 remap 0x02ff | same, remap 0x02ff | same | patch_0127, patch_0128 |
| 4 BC1 cubes | 0x86, dim 2, cube 1, square 128/256/256/1024, full mips | exactly 6 x (BC1 mip chain rounded up to 128) on all 4 | 3 DATA.FARC (one also in patch_0111), 1 patch_0128 |
| 1 volume | 0x85, dim 3, 16x16, depth 16, 1 mip | 16,384 = 16 x 16 x 16 x 4 | DATA.FARC |

Archived sizes of the B8 masks: 6.0 to 14.6 MB, all under the 16 MiB input budget.

## Rules (written before implementing; no Toolkit code copied or translated)

Evidence: Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, `lib/cwlib/src/main/java/cwlib/`.

| Rule | Evidence | Source |
|---|---|---|
| (a) B8 is 1 byte per texel; a 4096x4096 top mip is 16 MiB | `CONFIRMED_FROM_COMMUNITY_CODE` | `enums/CellGcmEnumForGtf.getDepth`/`getImageSize` |
| (b) cube byte (header byte 7) == 1 marks a cube map; dimension > 2 marks a volume | `CONFIRMED_FROM_COMMUNITY_CODE` | `structs/texture/CellGcmTexture` constructor, `isCubemap`, `isVolumeTexture` |
| (b) cube face order +X, -X, +Y, -Y, +Z, -Z, face-major (each face its own mip chain); DXT data is passed through unswizzled | `CONFIRMED_FROM_COMMUNITY_CODE` | `util/DDS.getDDSHeader` (cube caps2 bits), `resources/RTexture.parseGTF` (DXT not unswizzled) |
| (b) each face's mip chain is padded to a 128-byte multiple | `CONFIRMED_FROM_LOCAL_DATA` (exact size on all 4 cubes) | probe above |
| (b) Toolkit's image of a cube is face 0's top mip | `CONFIRMED_FROM_COMMUNITY_CODE` | `util/Images.fromDDS` -> `DDSReader.read(dds, ARGB, 0)` |
| (b) volume slice layout | `UNKNOWN` | Toolkit writes no volume caps/depth and unswizzles only w x h (2D) per mip in `DDS.convertSwizzleGtf`, i.e. it has no volume layout |
| (c) remap 0x02ff | `UNKNOWN` in Toolkit; bit meaning `STRONG_INFERENCE` | see below |

**Conflict (cubes).** Toolkit prepends a DDS cube header to the payload without the
128-byte face padding, so its face k would start 72 x k bytes early (all four local
cubes have 72 bytes of padding per face). Face 0 starts
at offset 0 under both readings; faces 1..5 here use the locally measured stride and are
not oracle-compared.

**Remap 0x02ff.** Toolkit names only `REMAP_ARGB = 0xaae4` and `REMAP_BBBB = 0xa9ff`
(`CellGcmTexture` constants) and never reads the remap when decoding (`parseGTF`,
`getDDSHeader`), so a Toolkit export shows these two as ordinary gray L8. Reading the
field as four 2-bit output modes (high byte) over four 2-bit channel selects (low byte)
is consistent with both named constants (0xaae4: all "remap", identity selects; 0xa9ff:
R/G/B from B, alpha "one" = Toolkit's opaque gray). Under that reading 0x02ff means
alpha = B8 byte, RGB = 0: an alpha-only mask. This is `STRONG_INFERENCE` only (the
"zero" mode is not corroborated by any Toolkit constant), so 0x02ff stays refused rather
than decoded as gray (Toolkit's output, likely wrong) or as alpha (uncorroborated).

## Implementation (`crates/lbp-texture/src/lib.rs`)

- (a) `MAX_PIXELS_ONE_BYTE = 16 Mi` applies to B8 only; every other format keeps
  `MAX_PIXELS = 4 Mi`. Byte budgets are unchanged (16 MiB input, 32 MiB expanded; the
  real 13-mip B8 payload is 22.4 MB). New `MAX_RGBA = 64 MiB` caps the decoded RGBA per
  texture (all cube faces together); 4096x4096 B8 is exactly 64 MiB. Sides stay <= 4096.
- Memory downstream: GPU upload paths keep the 4 Mi pixel cap. Bevy `adapter::image_axes`
  now refuses larger images as `Unsupported: GPU texture budget (4 Mi pixels)` (was
  `Invalid: texture extent/budget` for the same bound); the legacy viewer's texture tab
  shows the decoded metadata with the same message instead of uploading. Preview
  composition (`add`, `alpha_mix`, `blend*`, `tint`, `product`) still refuses RGBA above
  `4 * MAX_PIXELS`. The D66 per-mesh 640 MiB retained budget counts `rgba.len()`, so a
  64 MiB mask counts in full there (<= 10 such textures per mesh before refusal).
  `linked.rs` untouched.
- (b) Cubes decode only through the new `decode_layered_checked`, `decode_reader_layered`
  and `inspect_layered`; `decode`, `decode_checked`, `decode_reader` and `inspect` refuse
  them as `cube map (2D decode)`, so mesh previews never sample a cube face as a 2D
  texture. Accepted: BC1/BC3, dimension 2, depth 1, square; uncompressed cubes, other cube
  byte values and non-square cubes are refused. The expanded payload must equal
  6 x align128(BC mip chain) exactly (checked arithmetic over <= 13 mips), else
  `cube payload is not six 128-byte-aligned mip chains`. Result `LayeredTexture`:
  `texture` = face 0 (+X) top mip, `extra_faces` = faces 1..5 RGBA, `layout` "cube",
  `faces_decoded` 6, `face_stride`. Decoded as data, not rendered.
- Volume: refused as `volume texture: slice layout UNKNOWN` (dimension 3, depth > 1).
- Remap 0x02ff: still `remap/pitch/offset/location/flags`.
- `corpus_texture_census` uses `decode_layered_checked` and records `layout`,
  `faces_decoded`, `decoded_cube_hashes` and the new budgets. `texture_parity` uses
  `decode_reader_layered` and compares face 0. `scripts/TextureOracle.java` admits B8 up
  to 16 Mi pixels and BC1/BC3 GTF cube maps (Toolkit image = face 0).

Synthetic tests (`crates/lbp-texture/tests/d95_rest.rs`, hand-built bytes): 4096x4096 B8
in 512 zlib chunks decodes (swizzle positions checked, RGBA = 64 MiB), its raw form
exceeds the input budget, 4096x2048 A8R8G8B8/BC1 and 8192-wide B8 refused; 8x8 two-mip
BC1 cube with six colored faces and 0xee padding decodes all faces through both layered
entry points and is refused by the 2D ones; cancellation; payload one byte short/long,
non-square, uncompressed and cube byte 2 refused; 2048 cube over `MAX_RGBA`; 2D through
the layered path equals `decode_checked`; volume and 0x02ff refused. `gtf_plain.rs`
expectations updated; Bevy adapter test for the GPU refusal.

## Counts (01.33, disc + update 0133, snapshot d68-inventory-0133-verified.json)

Unit: distinct texture hash (and distinct catalog GUID), scope all 23,247 texture MAP
hashes, top mip only (face 0 plus faces 1..5 for cubes). Baseline D91.

| Outcome | D91 hashes / GUIDs | D95 hashes / GUIDs |
|---|---:|---:|
| Decoded | 23,215 / 23,445 | 23,244 / 23,474 |
| Unsupported | 7 / 7 | 3 / 3 |
| Over budget | 25 / 25 | 0 |

Newly decoded: +29 hashes / +29 GUIDs (B8 4096x4096 25, BC1 cube 4); no other hash
changed outcome. Left: 2 B8 remap 0x02ff, 1 A8R8G8B8 volume. Texture decode is not mesh
viewability: +0 newly viewable / +0 newly Complete meshes claimed (no mesh census rerun;
the B8 masks exceed the GPU upload cap and cubes are not reachable from mesh routes).

Output `local-analysis/d95-texture-census-0133.json` (12,973,093 bytes, SHA-256
fd3d43618fa456d172bba99b69c22d11908744ea9b65b1d3985e33415666b810), 1,089,159,991
archived bytes read, 314 s release; `decoded_cube_hashes` 4.

## Oracle parity (pinned Toolkit, DATA.FARC only)

`local-analysis/d95-texture-oracle-samples.json`: 7 samples = the 2 smallest and 2
largest DATA.FARC 4096x4096 B8 masks by archived size, and all 3 DATA.FARC BC1 cubes
(128, 256, 256). Toolkit decoded 7/7 (the worktree's `TextureOracle.java`, compiled to
`local-analysis/oracle/d95-classes`, run from the main checkout with the same pin/clean
checks as `run-texture-oracle.ps1`). `texture_parity`: 7 compared, 0 pixel/metadata
mismatches. Cubes compare face 0 only. Regression: the D91 oracle file compares 20, 0
mismatches; the D90 oracle file now compares 24 with 2 "mismatches" that are the 933a/9b08
cubes the old oracle excluded (no Toolkit digest in that file); both match in the D95 file.

## Commands

Worktree build (`CARGO_TARGET_DIR` inside the worktree); game-data runs from the main
checkout with the worktree's release examples; outputs under its ignored local-analysis/.

```
cargo fmt --all --check                                       -> clean
cargo clippy -q -p lbp-texture -p lbp-preview -p lbp-catalog --all-targets -- -D warnings -> clean
cargo clippy -q -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings -> clean
cargo test -q -p lbp-texture -p lbp-catalog                   -> all pass (4 new in d95_rest.rs)
cargo test -q -p lbp-preview --all-targets                    -> all pass
cargo test -q -p lbp-bevy-viewer -p lbp-viewer                -> bevy pass; lbp-viewer 63 pass, 2 diagnostics tests fail (no local-analysis/ in worktree)
cargo build -q -p lbp-bevy-viewer -p lbp-viewer               -> ok
corpus_texture_census local-analysis\d68-inventory-0133-verified.json "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d95-texture-census-0133.json -> decoded 23244/23474, unsupported 3/3, exit 0
javac/java TextureOracle (worktree source; pin d6a2ef80 clean) --self-test -> pass
java TextureOracle game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC local-analysis/d95-texture-oracle-samples.json local-analysis/d95-texture-oracle.json -> 7 decoded
texture_parity SNAPSHOT GAME-DIRS local-analysis\d95-texture-oracle.json local-analysis\d95-texture-parity.json -> 7 compared, 0 mismatches
texture_parity ... local-analysis\d91-texture-oracle.json local-analysis\d95-texture-parity-d91-samples.json -> 20 compared, 0 mismatches
texture_parity ... local-analysis\d90-texture-oracle.json local-analysis\d95-texture-parity-d90-samples.json -> 24 compared, 2 = cubes the D90 oracle excluded
```

## Not tested

- Cube faces 1..5 (stride is local-data only; Toolkit conflicts) and the patch-only
  cube (ac27, 1024) and 5 patch-only B8 masks: Rust-decoded, not oracle-compared.
- No visual or viewer check (no native windows), no mesh census rerun; color space,
  higher mips and cube sampling unclaimed. lbp-viewer `app::diagnostics` tests need
  `local-analysis/` in the cwd and fail in a bare worktree (unrelated to D95).
