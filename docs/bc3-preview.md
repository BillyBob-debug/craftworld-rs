# C4 bounded BC3 top-mip preview — 2026-10-05

C4 is complete: independent CPU BC3/DXT5 decoder for ordinary clear plain 2D GTF
raw format 0x88 and TEX/DDS DXT5. Existing bounds, cancellation, source validation,
wrapper inflation, raw metadata and preview cache/channel controls are retained.
Header now records BlockFormat (BC1 or BC3); UI labels the actual decoded format.
BC3 top-mip block size is 16 bytes; following mip/tail bytes remain unparsed.
No swizzle, cubes/volumes, DX10, uncompressed formats or protected decoding added.

## Rule provenance and conflicts

`CONFIRMED_FROM_DOCUMENTATION`: [Microsoft BC3](https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression)
defines two alpha endpoints and packed three-bit indices with two interpolation
modes; [Textures with Alpha Channels](https://learn.microsoft.com/en-us/windows/win32/direct3d9/textures-with-alpha-channels)
defines byte ordering and four RGB colors regardless of endpoint order.
Implementation independently derives palette arithmetic and packed indices.
CPU endpoint rounding remains the C2 nearest normalized convention; interpolation
uses integer floor. D3D9 examples add rounding bias whereas D3D10 formulas do not;
this is an explicitly tested CPU convention, not a claim of universal GPU fidelity.

`CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, CellGcmEnumForGtf and RTexture::parseGTF
map 0x88 to DXT5; external DDSReader::getDXT5Alpha matches floor interpolation.
[Source links and shared wrapper evidence](textures.md).

The initial synthetic Java guard failed for reversed RGB endpoints: pinned
DDSReader::decodeDXT5 calls getDXTColor, which incorrectly applies BC1's
three-color/transparency rule. Authored green/red endpoints give Toolkit pixel 2
ARGB da7f7f00 and pixel 3 zero; specification-correct Rust gives da55aa00 and
b6aa5500. This failure was investigated, not hidden or imitated. The guard now
asserts and reports that known conflict while separately checking alpha modes with
ordered colors. Pillow 12.3.0 corroborates reversed/equal color and alpha rules on
synthetic blocks via scripts/check-bc3-oracles.py. Pillow low RGB565 endpoint
expansion differs (0x18e3 -> 24,28,24 versus our 25,28,25), so it is not used as
an interchangeable local pixel oracle. No third-party decoder copied or altered.

`CONFIRMED_FROM_LOCAL_DATA`: 35 selected BC3 resources, 28 GTF and 7 TEX,
2,096,689 archived bytes. All match Toolkit exact RGBA hashes, dimensions, raw
mip counts, no_srgb and retained DDS fields. Selection is median per observed
wrapper/mips/flags/square/partial-block group. These samples do not establish that
all local BC3 blocks avoid the known Toolkit color bug. Full corpus not decoded.
15 TEX BC1 and 16 GTF BC1 regression samples still match exactly.

## Checks and reproduction

Four authored BC3 tests cover both alpha modes/equality/packed 48-bit indices,
reversed and equal colors, interpolation rounding, partial/rectangular blocks,
orientation, raw/zlib GTF/TEX, truncation prefixes, strict zlib lengths/trailing
input, unsupported flags, protection, budgets and block-row cancellation.
Workspace 57 Rust tests pass. Final workspace Clippy/fmt and native build pass;
Clippy's constant chunks_exact lint was corrected. Four Python report tests pass,
and the synthetic Pillow corroboration passes. Native GTF BC3 RGBA and TEX BC3
alpha captures inspected on RTX 3080/Vulkan; raw DDS mip zero displays correctly.
No manual interaction, other GPU/backend, higher mips or rendering fidelity tested.

```powershell
. ./scripts/use-local-rust.ps1
python scripts/texture_report.py select-bc3 local-analysis/c1-texture-census-dds.json local-analysis/NEW-bc3-samples.json
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/NEW-bc3-samples.json -Output local-analysis/NEW-bc3-oracle.json
cargo run -p lbp-catalog --example texture_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-bc3-oracle.json local-analysis/NEW-bc3-parity.json
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -m unittest discover -s tests -p test_texture_report.py -v
python scripts/check-bc3-oracles.py
cargo build -p lbp-viewer
```

Ignored evidence: c4-bc3-samples/oracle/parity.json, c4-bc1-tex-regression.json,
c4-bc1-gtf-regression.json, c4-gtf-rgba.png/c4-tex-alpha.png and logs. Native local
QA GUIDs are 00017440 and 0000c9cd. No assets exported or trackable fixtures added.
Next: D1 bounded mesh prefix/census and pinned reader oracle before topology/viewer.
