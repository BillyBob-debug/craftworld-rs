# D22 standard-mask TEX/DDS BGRA8 top mip

Independent implementation, 2026-10-05 America/Chicago. CPU pixel-format IR now
uses PixelFormat rather than BlockFormat and adds BGRA8. Existing BC1/BC3 serialized
names/metadata remain unchanged. GUI uses the shared preview/channel/cache path;
no rendering dependency, original-file write or community decoder translation.

## Field rules and provenance

| Rule | Evidence and source | Applicability / coverage |
|---|---|---|
| Uncompressed RGB+alpha flags0x41,32bits and masks R/G/B/A=0x00ff0000/0x0000ff00/0x000000ff/0xff000000 | CONFIRMED_FROM_DOCUMENTATION: [DDS_PIXELFORMAT, flags and masks](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-pixelformat) | Ordinary TEX/DDS, FourCC zero,2D only. Other masks/flags/bit depths unsupported. Distinctive synthetic channels/alpha and local oracle. |
| Little-endian standard-mask pixels yield B,G,R,A bytes; independently rearrange to CPU RGBA | CONFIRMED_FROM_DOCUMENTATION: [DDS layout guide](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dx-graphics-dds-pguide); CONFIRMED_FROM_LOCAL_DATA: exact pinned sample pixel digest | No lookup/interpolation/swizzle helper copied. Synthetic alpha0/1/64/127/128/255, two rows and raw/zlib wrappers. |
| DDSD_PITCH0x8 declares scanline bytes; missing pitch flag uses computed width*4 | CONFIRMED_FROM_DOCUMENTATION: [DDS_HEADER, pitch member](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header) and layout guide | Pitched rows must be at least width*4; skip explicit padding, require full pitch*height bytes. Padded rows only synthetic-tested. |
| Local uncompressed sample uses DDSD_LINEARSIZE0x80000 rather than PITCH, value equals width*height*4 | CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE: external pinned RTexture::process and DDSReader::read | Header convention conflicts with documented uncompressed pitch usage. Strict compatibility accepts this exact-size convention; conflicting pitch+linear flags and differing linear size unsupported. Raw flags/value retained. |
| Bounds, wrapper inflation, BUMP suffix/no_srgb and raw mip count follow existing TEX reader | Existing [TEX provenance](tex-preview.md) | Row checks and cancellation added; tail bytes remain opaque. No higher-mip offsets inferred. |

Community oracle is Toolkit
[d6a2ef80b5a09999f08f23a803b516096055e5c9 RTexture](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RTexture.java)
and [external DDSReader](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/external/DDSReader.java),
kept as an external source-built oracle. Public specifications guide the authored
implementation. No inherited GPL-origin GTF swizzle routine is copied/translated.
Confidence: high for exact standard-mask top mip and measured build sample;
unverified for other variants/GPU fidelity.

## Local evidence and contract

BCUS98245 01.00: GUID00000824, gamedata/data/noise.tex, SHA1
57ea05cff5becc8d5044c71bb5e382edf6efa17a. One clear326,413-byte archive resource,
256x256,9 raw mips. Expanded DDS349,652 bytes; top pixels262,144 bytes;
87,380 bytes after top mip stay unparsed. RGBA SHA256
3af6f39cb3211fdafd1e9481ab5bdd455be5807b0707427fa605d4374d1dfc2c.
Fresh pinned oracle equals the existing C1 pixel digest and additionally records
all DDS fields, which exactly match the independent parser. Single-sample scope.

Input16MiB, expanded32MiB and pixel4Mi budgets unchanged. Pitch*height is checked
and capped before allocation/slicing. Cancellation checked per decoded row. Bounds
and source-validation safeguards remain shared with existing paths.

Diagnostic JSON schema1 gains BGRA8 in the pixel-format enum. Legacy field
top_mip_block_bytes remains for compatibility and means stored expanded top-mip
bytes, including row padding for BGRA8. No pixels exported. Native metadata equals
the oracle-validated parser projection. A repeated R-channel export is byte-identical
(1,361 bytes); schema validates and four malformed mutations are rejected.

## Verification and reproduction

After `. ./scripts/use-local-rust.ps1`:

```powershell
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/d22-bgra-samples.json -Output local-analysis/NEW-bgra-oracle.json
cargo run -p lbp-catalog --example texture_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-bgra-oracle.json local-analysis/NEW-bgra-parity.json
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00000824 --qa-tab texture --qa-texture-json local-analysis/NEW-bgra-native.json --qa-capture local-analysis/NEW-bgra-native.png
```

The private manifest selects the one hash from existing c1-texture-samples.json.
Fresh Java oracle guard/self-tests pass, reads326,413 asset bytes. Four new authored
tests cover channels, alpha, row padding, raw/zlib, legacy flags, tail, truncation,
unsupported masks/layouts, budgets, cancellation and bounded callbacks. Initial
ambiguous integer offset compilation error fixed by an explicit usize annotation.
115 workspace tests (37viewer), fmt, Clippy and build pass.

Regression oracle combines, deduplicates and preserves pinned sample records from
c2-bc1-oracle-confirmed, c3-tex-oracle-metadata, c4-bc3-oracle and d22-bgra-oracle.
67unique samples decoded:0 pixel/metadata mismatches,0 failures. Native RGBA and
R-channel captures both exit zero and inspected on RTX3080/Vulkan. Existing known
BC3 reversed-color Toolkit conflict remains documented, not imitated.

SFO/MAP hashes unchanged, DATA.FARC ReadOnly/size unchanged; pinned Toolkit clean,
no root Git. All manifests/reports/captures ignored. No manual interaction, other
GPU, mip-chain, color-space fidelity or uncompressed GTF compatibility claim.

Next D23: animation metadata reconnaissance with pinned RAnimation before any
payload parser. Snapshot contains854 .anim MAP rows; extension is not confirmed
magic. Determine accessible wrapper/revision cohorts, useful supported scalar/count
fields, reader limits and bounded external-oracle comparisons. No animation
playback, posing, executable work or speculative payload parser.
