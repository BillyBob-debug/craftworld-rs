# C2: first native texture preview — 2026-10-05

Historical BC1 contract; current BC3 support is documented in [bc3-preview.md](bc3-preview.md).
C2 is complete: lbp-texture independently decodes the top mip of plain 2D GTF BC1,
and the native explorer displays it. This is the first visual asset milestone.
Original inputs remain read-only; no original asset/image export was added.
C3 adds TEX/DDS BC1: see [current TEX contract](tex-preview.md).
BC3, swizzle, cubes, volumes and higher mips remain unsupported.

## Supported contract and separation

CPU-only lbp-texture depends on serde and flate2, not catalog/GUI/GPU libraries.
`inspect` validates a small header before full reads. `decode_reader` accepts an
owner-supplied bounded read callback; the viewer supplies only B2-approved candidate
ranges. Each source read is at most 4,096 bytes and checks stable inputs/cancellation.
`decode_checked` validates/expands the complete accepted envelope, decodes top-mip
BC1 blocks to row-major RGBA8 and retains metadata. It does not resolve paths/IDs.

Supported GTF header: raw format 0x86, dimension 2, depth 1, cube 0, remap 0xaae4,
pitch/offset/location zero, flags 0 or bit 0 only, positive mip count up to 13.
Flags/mip count remain raw. Bit 0 produces a non-color/bump notice. Compression
flag 0/1; count zero raw remainder, equal-size chunks stored, otherwise strict zlib.
Chunk ranges must account for the archived resource exactly; each compressed
chunk must produce its exact declared output, reach stream end, consume all input.
Bytes after the top mip are explicitly unparsed; they include later mips and
unknown layout data. Never infer higher offsets from their size.

Budgets: 16 MiB archived/resource, 32 MiB expanded/resource, 4,096 per dimension,
4,194,304 pixels / 16 MiB output RGBA, 32,767 chunks. Checked count/range arithmetic
and row/chunk/page cancellation checkpoints precede major work. Errors distinguish
blocked, unsupported, invalid, budget, cancellation and source I/O. Protected
wrappers are refused before parsing/inflation. Bounds are work/input limits, not a
hard process-wide allocator limit. Seven original synthetic tests cover palette,
transparency/equal endpoints, endpoint rounding, rectangular clipped blocks,
raw/stored/mixed/zlib chunks, opaque tails, every compressed-resource truncation,
bad lengths/trailing streams, unsupported headers, budgets and mid-read cancellation.

The viewer's Texture tab works for MAP candidates and independent archive entries.
It selects candidates explicitly, reads/decodes on a cancellable worker, and retains
one CPU image plus one GPU texture. Source/selection changes cancel/drop stale jobs
and caches; detachment also removes preview. Reload rechecks source; cached pixels
do not continually reread files. RGBA, R/G/B/A grayscale views, alpha checkerboard,
aspect-preserving Fit and 0.125–8x zoom with scroll are implemented. GPU upload and
channel display are UI work; original bytes and parsing stay outside the UI thread.
Channel choice survives navigation; candidate/source/image state does not.

## Provenance and rounding experiment

Wrapper rules: `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit
**d6a2ef80b5a09999f08f23a803b516096055e5c9**, SerializedResource::process,
CellGcmTexture constructor, Compressor::decompressData and RTexture::parseGTF.
[C1 field map](textures.md) links exact files and records BCUS98245 01.00 scope.
No community decompressor/pixel/swizzle implementation was translated or vendored.

BC1 structure/interpolation: `CONFIRMED_FROM_DOCUMENTATION`,
[Microsoft block-compression specification](https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression).
4x4/8-byte blocks, little-endian RGB565 endpoints, two-bit indices, endpoint-order
palette mode, transparent fourth entry in the three-color mode. CPU orientation
and output are corroborated by synthetic quadrant patterns and local oracle.

Endpoint quantization is a scoped CPU convention, not original GPU equivalence:
expand a channel `v` with maximum `m` as `(255*v + m/2)/m`, integer division;
floor weighted interpolation. Initial bit replication passed three simple C1
samples but failed **15 of 16** broader samples. Investigating pinned MIT
DDSReader's BIT5/BIT6 values revealed nearest normalized endpoint expansion.
An authored DDS block with endpoint 0x18e3 gives `(25,28,25,255)` in Toolkit,
versus `(24,28,24,255)` with bit replication. Independent normalized arithmetic
and an explicit regression test resolve the difference; no lookup table copied.
This rule is `CONFIRMED_FROM_COMMUNITY_CODE` (DDSReader endpoint tables and
getDXTColor methods) and `CONFIRMED_FROM_LOCAL_DATA` (authored synthetic execution
and 19 game samples). Exact GPU/renderer color-space behavior remains `UNKNOWN`.
The GPL-cited uncompressed swizzle helper is not involved.

## Local verification

Selected 16 samples by median archived size per present BC1 mip/flags/square group,
using C1 metadata only. Pinned unmodified Toolkit reads 2,463,926 archived bytes,
verifies sample payload SHA-1 and emits metadata/pixel hashes only. Rust independently
reads approved candidates. **All 16 top-mip RGBA digests, dimensions, declared mip
counts and non-color flags agree**; zero failures. Dimensions 4–2,048, actual
sample shapes include 4x32, 64x256, 2,048x1,024 and 2,048x2,048; declared mips 5–12.
These compare top mip only despite the declared mip counts. All 16 have flag zero.
Bit-0 handling is synthetic/header-derived, not a local BC1 bump-pixel claim.

All three originally supported C1 samples also agree after the correction,
including a fully transparent 16x16 sample. 19 distinct local samples total, not
all 4,450 eligible local GTF BC1 resources. No full-corpus pixel compatibility claim.
Both final reports stay ignored under local-analysis:

- c2-bc1-parity-final.json: 9,047 bytes, SHA-256
  `53bd50988bc2f3505af9e984278ef9ec7837ebaa0ea5b3e119eefc634a343f8e`.
- c2-c1-parity-final.json: 4,924 bytes, SHA-256
  `1eb357e693f75578500a191b70b7b7f2dadee8a0aeb6ae8422c18acb333ae1de`.

Full workspace/all-target tests pass (49 Rust tests including C1 example tests).
Final narrow texture/viewer tests pass after QA-channel persistence changes;
workspace Clippy passes, final affected-target Clippy and fmt pass, native build
passes. Three Python report tests pass. Initial Clippy range-loop/test-initializer
findings were corrected. Failed initial rounding report retained as evidence;
use only final reports to claim parity.

Native screenshots inspected on NVIDIA RTX 3080/Vulkan: 512x512 RGBA, alpha channel,
transparent checkerboard and explicit unsupported DXT5. Source validation uses
4,704,862 metadata bytes before reads. Screenshots/logs are ignored c2-* artifacts.
No manual mouse/keyboard delivery, release performance, other GPU/platform, original
game rendering, all textures, higher mips or general fuzzing was verified. Existing
scanner snapshots still say identified/unparsed: lazy preview does not rewrite them.

## Usage and reproduction

Activate the local toolchain where needed; filenames below must be new:

```powershell
. ./scripts/use-local-rust.ps1
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
```

Select a resource, then **Texture**. Archive entries also offer Hex/Texture.
Detach discards previews; unsupported layouts give a reason rather than guessed pixels.

```powershell
python scripts/texture_report.py select-bc1 local-analysis/c1-texture-census-dds.json local-analysis/c2-bc1-samples-new.json
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/c2-bc1-samples-new.json -Output local-analysis/c2-bc1-oracle-new.json
cargo run -p lbp-catalog --example texture_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/c2-bc1-oracle-new.json local-analysis/c2-bc1-parity-new.json
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -m unittest discover -s tests -p test_texture_report.py -v
```

Native QA-only flags: `--qa-tab texture`, `--qa-channel rgba|r|g|b|a`,
`--qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS`; capture exits automatically. Existing
QA GUIDs: 000156bb RGBA/alpha, 00001632 checkerboard, 00000577 unsupported DXT5.
These refer only to this private local build, not portable synthetic fixtures.
No original image bytes or resource captures are trackable fixtures.

## Next bounded task

C3 is complete: [TEX/DDS contract](tex-preview.md). C4 adds bounded plain 2D
BC3 top mips, after documented block rules and pinned oracle tests. Higher mips,
swizzle and meshes remain separate; see [exact task](roadmap.md).
