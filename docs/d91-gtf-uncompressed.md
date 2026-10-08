# D91 uncompressed GTF decode (0x85, 0x81, 0xa5) — 2026-10-07

## Question

D90 found 274 of 23,247 texture hashes on 01.33 that `lbp_texture::decode_checked`
refuses: GTF 0x85 A8R8G8B8 187, 0x81 B8 80, 0xa5 3, BC1 cubes 4. Can the top mip of
the 2D uncompressed ones decode with oracle pixel parity?

## Findings (written before implementing; no Toolkit code copied or translated)

Evidence: Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9,
`lib/cwlib/src/main/java/cwlib/`. Label: `CONFIRMED_FROM_COMMUNITY_CODE`, corroborated
`CONFIRMED_FROM_LOCAL_DATA` by the oracle parity below.

| Rule | Source (file / method) |
|---|---|
| Raw format byte = base format OR 0x20 (LN) OR 0x40 (UN); base 0x81 = B8, 0x85 = A8R8G8B8 | `enums/CellGcmEnumForGtf.java` values and `fromValue` (masks `CellGcm.LN`/`UN`), `enums/CellGcm.java` |
| Bytes per texel: B8 1, A8R8G8B8 4; top-mip size = width * bpp * height | `CellGcmEnumForGtf.getDepth` / `getImageSize` / `getPitch` |
| Every non-DXT GTF is unswizzled before DDS conversion; top mip starts at payload offset 0 | `resources/RTexture.parseGTF`, `util/DDS.convertSwizzleGtf` |
| Swizzle order: the low log2(min(w,h)) bits of x and y interleave, x on even and y on odd bit positions; remaining high bits of the longer axis sit above them | `DDS.convertSwizzleGtf` (masked-increment walk; also `DDS.getMortonNumber`) |
| A8R8G8B8 texels are stored big-endian: bytes A, R, G, B | `DDS.convertSwizzleGtf` byte reversal for bpp 4 + `DDSPF_A8R8G8B8` masks |
| B8 is presented as L8: R = G = B = byte, A = 255 | `DDS.getDDSHeader` (B8 -> `DDSPF_L8`), `external/DDSReader.readL8` |
| B8 remap 0xa9ff is Toolkit's `REMAP_BBBB`; ARGB identity is 0xaae4 | `structs/texture/CellGcmTexture` constants |

Local 01.33 header probe of the 274 refused hashes (scratch script, header bytes only):
0x85 2D 185 (+1 with no-sRGB flag 1), 0x85 volume 1; 0x81 remap 0xaae4 75, 0xa9ff 3,
0x02ff 2; 0xa5 3, all height 1 with pitch = width * 4. All are power-of-two; every
swizzled one has pitch 0, offset 0, location 0.

Implementation rules (`crates/lbp-texture/src/lib.rs`, `inspect` / `decode_checked`):

- Accepted: 0x85, 0x81 (swizzled) and 0xa5, 0xa1 (LN, linear rows). UN (0x40) and LN on
  BC formats stay "pixel format or layout flags".
- Remap: 0xaae4 for all; 0xa9ff additionally for B8. Others (0x02ff, an alpha-only
  remap) refused. B8 under 0xaae4 is decoded with the Toolkit L8 convention; what RSX
  hardware returns for B8 under the identity remap is UNKNOWN (sampler-time behaviour).
- Swizzled: pitch must be 0 and both dimensions power of two (NPOT swizzled refused;
  Toolkit's floor-log2 handling for NPOT is unverified). Top mip = w * h * bpp.
- LN: pitch from the header (offset 20), must be >= width * bpp else Invalid; top-mip
  extent = pitch * (h - 1) + row, checked arithmetic, refused above the 32 MiB budget
  before any indexing. For height 1 (all three local 0xa5 files) linear and swizzled
  orders coincide, so Toolkit (which ignores LN) and this reader agree; LN with h > 1
  is covered only synthetically. Conflict noted: Toolkit swizzles LN data too.
- Existing budgets unchanged (16 MiB input, 32 MiB expanded, 4 Mi pixels, 4096 sides).
- Cubes and volumes stay "cube/volume/dimension". Higher mips not decoded.

Synthetic tests: `crates/lbp-texture/tests/gtf_plain.rs` (hand-built headers/texels):
4x4, 4x2, 2x4, 8x1 swizzle orders written out by hand, A8R8G8B8 byte order, LN pitch
padding, B8 gray under both remaps, zlib and stored payloads, UN/LN-BC/A4R4G4B4/0x02ff/
mismatched remap/pitch-on-swizzled/NPOT/volume/cube refusals, short pitch, u32::MAX
pitch budget, truncated payloads, B8 pixel budget.

## Counts (01.33, disc + update 0133, snapshot d68-inventory-0133-verified.json)

Unit: distinct texture hash (and distinct catalog GUID), scope all 23,247 texture MAP
hashes, top mip only. Baseline D90: 22,973 / 23,195 decoded, 274 / 282 unsupported.

| Outcome | D90 hashes / GUIDs | D91 hashes / GUIDs |
|---|---:|---:|
| Decoded | 22,973 / 23,195 | 23,215 / 23,445 |
| Unsupported | 274 / 282 | 7 / 7 |
| Over budget | 0 | 25 / 25 |

Newly decoded: +242 hashes / +250 GUIDs (A8R8G8B8 189 = 0x85 186 + 0xa5 3; B8 53).
Left: 25 B8 over the 4 Mi pixel budget (4096-wide masks), 2 B8 with remap 0x02ff, 4
BC1 cubes, 1 A8R8G8B8 volume. Texture decode is not mesh viewability: +0 newly viewable
/ +0 newly Complete meshes are claimed here (no mesh census rerun).

Output `local-analysis/d91-texture-census-0133.json` (11,759,960 bytes, SHA-256
c66f9dc5a5ad6f7706dc0341b094bdf5fb25396dba0dbe5fa47dfc366890d951), 1,089,159,991
archived bytes read, 354 s release.

## Oracle parity (pinned Toolkit, DATA.FARC only)

`local-analysis/d91-texture-oracle-samples.json`: 20 samples, smallest 2 and largest 2
per (format, remap, square/rect, flags) group within the oracle pixel budget: 0x85 9
(incl. the no-sRGB one), 0x81 8, 0xa5 3. Toolkit decoded 20/20; `texture_parity`
compared 20, 0 pixel/metadata mismatches, 0 failures. Regression: the D90 oracle file
now compares 22 (16 BC + 6 formerly refused), 0 mismatches.

## Commands (outputs under the main checkout's ignored local-analysis/)

```
cargo test -q -p lbp-texture                       -> all pass (5 new in gtf_plain.rs)
cargo clippy -q -p lbp-texture --all-targets -- -D warnings  -> clean
cargo test -q -p lbp-preview --all-targets         -> 47 + 5 pass
cargo test -q -p lbp-catalog                       -> pass
cargo fmt --all --check                            -> clean
cargo build -q -p lbp-bevy-viewer -p lbp-viewer    -> ok
corpus_texture_census local-analysis\d68-inventory-0133-verified.json "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d91-texture-census-0133.json  -> 23215 / 25 / 7, exit 0
.\scripts\run-texture-oracle.ps1 -SampleManifest local-analysis/d91-texture-oracle-samples.json -Output local-analysis/d91-texture-oracle.json  -> 20 decoded
texture_parity SNAPSHOT GAME-DIRS local-analysis\d91-texture-oracle.json local-analysis\d91-texture-parity.json  -> 20 compared, 0 mismatches
texture_parity SNAPSHOT GAME-DIRS local-analysis\d90-texture-oracle.json local-analysis\d91-texture-parity-d90-samples.json  -> 22 compared, 0 mismatches
```

## Not tested

- B8 remap 0xa9ff (3 hashes, patch FARCs only) and all patch-archive textures: Rust
  decoded, not oracle-compared. LN with height > 1: synthetic only.
- No visual/viewer check, no mesh census rerun, color space and higher mips unclaimed.
- Over-budget B8 masks would need a larger pixel budget; not raised.
