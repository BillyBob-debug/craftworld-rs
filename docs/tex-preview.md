# C3 TEX/DDS BC1 top-mip preview — 2026-10-05

Historical BC1 contract; current BC3 support is documented in [bc3-preview.md](bc3-preview.md).
C3 is complete. CPU-only lbp-texture supports ordinary clear TEX envelopes at byte
4, shared bounded inflation, a 128-byte DDS prefix and the existing independent
BC1 top-mip kernel. Native Texture tab exposes DDS metadata, Fit/zoom/channel and
alpha/checkerboard previews. Raw mip count zero is retained and shown as unspecified.
GTF-only flags/remap/format fields remain null for TEX.

## Provenance and limits

`CONFIRMED_FROM_DOCUMENTATION`: [DDS_HEADER](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header)
and [DDS_PIXELFORMAT](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-pixelformat)
define little-endian fields, structure sizes 124/32, dimensions, mip count,
flags/pitch/depth/FourCC/masks/caps. All these fields are retained in DdsHeader.

`CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, SerializedResource::process,
Compressor::decompressData and RTexture::process establish envelope/chunk rules
and DDS payload interpretation; [pinned source links](textures.md).
Independent implementation uses published BC1 semantics and the established C2
endpoint rounding; no source translation or swizzle implementation.

Protected TEXe/GTFe, NPD and SCE are blocked. DX10, non-DXT1, unknown pixel flags, cubes, volumes and extended caps are
explicitly unsupported. Checked lengths, cancellation and exact chunk input/output
consumption apply. Limits: archived 16 MiB, expanded 32 MiB, 4096 per dimension,
4 Mi pixels, 32767 chunks, raw mip count at most 13. Source reads use approved
read-only callbacks, pages at most 4096 bytes; TEX dimensions are checked after
bounded expansion of DDS. Tail bytes after top mip remain unparsed.

BUMP suffix detection follows the pinned reader but requires four bytes beyond
the top mip to avoid mistaking block indices for a footer. A synthetic collision
test covers this deliberate restriction. Complete footer semantics, higher mips,
other formats and original renderer fidelity are `UNKNOWN`.

## Verification

`CONFIRMED_FROM_LOCAL_DATA`: 15 selected TEX BC1 resources (63,693 archived bytes,
dimensions 16–512, including non-square and raw mip zero) match pinned Toolkit
canonical RGBA, dimensions, raw mips, no_srgb and every retained DDS field.
This is sample-scoped parity, not full-corpus or original-GPU equivalence.
16 broader GTF samples and refreshed C1's six supported BC1 samples also match,
retaining coverage of 19 distinct GTF resources. Old C1 output lacked the new DDS
fields; its initial three metadata mismatches were resolved by refreshing the
unchanged pinned oracle. Superseded reports remain ignored.

Four authored TEX tests cover raw/split DDS prefix, stored/zlib chunks, raw fields,
zero mips, partial blocks, suffix collision, truncated data, invalid DDS sizes,
unsupported variants, budgets, exact decompression, protection and cancellation.
Workspace 53 Rust tests, three Python report tests, workspace Clippy, fmt and
native viewer build pass. Native RGBA, alpha and unsupported DXT5 captures were
inspected on RTX 3080/Vulkan. No manual interaction or other backend tested.

Ignored evidence under local-analysis/:
- c3-tex-samples.json, c3-tex-oracle-metadata.json, c3-tex-parity-final.json.
- c3-gtf-regression.json, c3-c1-oracle-metadata.json, c3-c1-regression-final.json.
- c3-tex-rgba.png, c3-tex-alpha.png, c3-tex-unsupported.png and logs.

Reproduce after `. ./scripts/use-local-rust.ps1`:
```powershell
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/c3-tex-samples.json -Output local-analysis/NEW-oracle.json
cargo run -p lbp-catalog --example texture_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-oracle.json local-analysis/NEW-parity.json
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -m unittest discover -s tests -p test_texture_report.py -v
cargo build -p lbp-viewer
```
Select metadata samples with texture_report.py `select-tex-bc1`; manifest/oracle
paths are private evidence, not distributable fixtures. See [C2 usage](texture-preview.md)
for viewer controls and [roadmap](roadmap.md) for the next bounded BC3 task.
