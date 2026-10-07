# C1 texture census and oracle — 2026-10-05

C1 is complete for the prepared BCUS98245 01.00 clear FARC scope. This is a
diagnostic example and an external decode experiment. The product scanner still
magic-identifies textures. C2 now adds a limited Rust GTF BC1 decoder and GUI
preview; C3 adds TEX/DDS BC1, see [C3 contract](tex-preview.md) and
[C2 contract](texture-preview.md). C1 results below are historical.
No whole texture corpus was extracted or decoded. No protected resource was used.

## Local census

`CONFIRMED_FROM_LOCAL_DATA`: 13,624 texture locations, 704,746,214 archived bytes
in total. Census reads only headers/chunk tables and enough TEX chunks to inspect
the first 128 DDS bytes. It validates source attachment first. 9,817,800 requested
diagnostic bytes, plus 4,704,862 source-validation bytes; page protection-prefix
reads are additional and not included in the diagnostic byte counter.

| Wrapper | Pixel format | Locations |
|---|---|---:|
| GTF space | 0x88 DXT5 / BC3 | 6,950 |
| GTF space | 0x86 DXT1 / BC1 | 4,453 |
| GTF space | 0x85 A8R8G8B8 | 174 |
| GTF space | 0x81 B8 | 56 |
| TEX space -> DDS | DXT1 / BC1 | 1,203 |
| TEX space -> DDS | DXT5 / BC3 | 787 |
| TEX space -> DDS | uncompressed 32-bit | 1 |

All 13,624 wrapper/table probes succeeded; all 1,991 TEX DDS headers succeeded.
All use compression flag 1, positive chunk counts, no equal-sized stored chunks,
and exactly account for archived payload bytes. Only TEX chunks needed for the DDS
prefix were actually inflated. Unread later chunks are not verified zlib streams.

GTF raw formats match base formats except three 0xa5 textures (0x85 plus LN).
All GTF remaps are 0xaae4, offset 0. 11,630 pitch fields are zero; two are 1,024
and one is 64. Width range 2–4,096, mip count 1–13. 11,632 declare dimension 2 /
depth 1; one dimension 3 / depth 16. Three declare cubemap. Flags: 10,227 zero,
1,406 bit 0 set. There are 4,450 plain 2D non-cube GTF BC1 locations.
TEX DDS caps2 is zero throughout. 1,640 DDS headers declare mip count zero;
this raw value means no mip count supplied, not an empty texture. Preserve it.

Two complete censuses are byte-identical, 12,484,205 bytes, SHA-256
`d919ad40eafe18885829ad00f31582a65ad7c9e3c761c9a5ab571f184531198d`.
Original SFO/MAP/FARC remain read-only. Stable-input checks and table fingerprints
are not a whole-image authenticity claim.

## Established wrapper rules and provenance

All Toolkit links below pin **d6a2ef80b5a09999f08f23a803b516096055e5c9**,
with paths relative to `lib/cwlib/src/main/java/cwlib/`:

| Rule | Evidence | Source and applicability | Corroboration / limits |
|---|---|---|---|
| `TEX ` starts chunk envelope at byte 4; `GTF ` has a 24-byte CellGcmTexture followed by envelope at byte 28 | CONFIRMED_FROM_COMMUNITY_CODE | [SerializedResource::process](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/types/SerializedResource.java), COMPRESSED_TEXTURE | Both clear wrappers present locally. These are separate layouts from ordinary binary resource headers/revisions/dependencies. `s`, `S`, `e` excluded. |
| GTF fields: u8 format/mips/dimension/cube; BE u32 remap; BE u16 width/height/depth; u8 location/flags; BE u32 pitch/offset | CONFIRMED_FROM_COMMUNITY_CODE | [CellGcmTexture constructor](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/texture/CellGcmTexture.java), MemoryInputStream default BE | Raw census keeps unsigned values; Toolkit width getters use signed shorts. Local dimensions fit both. Public dimension/mip/format getters compared on samples; remap/pitch/offset are census only. |
| Base format masks LN 0x20 and UN 0x40; BC1=0x86, BC2=0x87, BC3=0x88 | CONFIRMED_FROM_COMMUNITY_CODE | [CellGcmEnumForGtf::fromValue/isDXT](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/CellGcmEnumForGtf.java), enums/CellGcm.java | Preserve raw flags. No BC2 here; this census does not establish arbitrary flag behavior. |
| BE u16 flag/count, then count pairs of BE u16 stored/expanded sizes; equal sizes are raw; unequal zlib; count zero raw remainder | CONFIRMED_FROM_COMMUNITY_CODE | [Compressor::decompressData](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/Compressor.java) | Count limited to 32,767 for pinned signed-short reader. Synthetic tests cover equal/zero cases absent locally. Strict decompressor also requires exact output, stream end and no trailing input. Toolkit's inflater omits these checks. |
| TEX expanded payload starts DDS; ordinary GTF DXT payload becomes DDS without Morton conversion | CONFIRMED_FROM_COMMUNITY_CODE | [RTexture::process/parseGTF](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RTexture.java), util/DDS.java | Sampled top mip decoded by Toolkit. [Refresh GtfDecoder::ProcessPixels](https://github.com/LittleBigRefresh/Refresh/blob/25f03e40416ebe73016b9e9972e0fa7b11a7f0b0/Refresh.Core/Importing/Gtf/GtfDecoder.cs) independently dispatches GTF DXT1 to BC1. Refresh not executed. |
| DDS header is 124 bytes after magic; little-endian dimensions/mip count, pixel-format structure and caps | CONFIRMED_FROM_DOCUMENTATION | [Microsoft DDS_HEADER](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header), [DDS programming guide](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dx-graphics-dds-pguide) | Census verifies 124/32 structure sizes, keeps raw fields and masks. DX10/extensions remain unsupported. |
| BC1 uses 4x4 blocks, 8 bytes/block, RGB565 endpoints, indices and conditional transparency | CONFIRMED_FROM_DOCUMENTATION | [Microsoft block compression](https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression) | C2 specification basis; C1 does not implement these pixels. |

Format implementations should retain this block near their source, including build
scope and tests. The diagnostic example is independently authored from the layout
rules; it does not translate Toolkit's decoder or swizzle algorithms.

## Pinned external oracle

`CONFIRMED_FROM_LOCAL_DATA`: 22 deterministic clear, simple 2D samples selected
by present raw format family: smallest, largest supported mip count, rectangular
and bump variants where present. 524,137 archived bytes read. Each full sample's
SHA-1 matches its FARC identifier. Strict independent chunk inflation equals
Toolkit's expanded buffer before RTexture is constructed. All 22 decoded their
top mip. **191 field comparisons agree**, zero oracle failures/mismatches.

Oracle outputs payload/DDS SHA-256, DDS metadata, dimensions, no-sRGB indication
and canonical top-mip RGBA SHA-256. Canonical order is row-major top-to-bottom,
R/G/B/A bytes from BufferedImage.getRGB. No payload/image bytes written. Two oracle
outputs are identical: 17,181 bytes, SHA-256
`1dd1645535a051f5e258e798ed9ee5baa8e2f9b019b66002b4ed4402afae37a5`.
These digests are future expectations, not Rust pixel parity or visual QA.
No game-renderer color-space, remap, bump interpretation or GPU equivalence claim.

Limits: 48 samples, 16 MiB archived/sample, 128 MiB total archived reads, 32 MiB
expanded/sample, at most 4,096 per dimension and 4,194,304 top-mip pixels. Cubes,
volumes, unknown/protected wrappers are excluded before community constructors.
Outputs require a new filename under ignored local-analysis. Toolkit source pin
and clean tracked files are checked by the wrapper; jar must be source-built.

## Conflicts and unknown mip layout

`CONFIRMED_FROM_LOCAL_DATA`: all 11,400 simple 2D GTF DXT resources have declared
expanded bytes exceeding the standard tightly packed mip-chain size. Differences:
16 (5,837), 72 (3,681), 48 (934), 24 (576), 112 (155), 56 (124), 120 (69),
80 (24) bytes. This is a metadata length comparison, not a full inflation result.
**UNKNOWN**: whether extra bytes are padding, footer or other layout detail, and
where they occur. Do not reject every local texture as corrupt or guess offsets.
Do not expose higher mips until their actual layout has independent corroboration.

`CONFIRMED_FROM_COMMUNITY_CODE`: [DDSReader::read](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/external/DDSReader.java)
calculates BC1 mip skips as `8*(width+3)/4*(height+3)/4` with left-associative Java
integer arithmetic. This differs from `8*ceil(width/4)*ceil(height/4)`.
`CONFIRMED_FROM_LOCAL_DATA` (authored synthetic experiment, not game data): our
8x8 red / 4x4 green / 2x2 blue / 1x1 blue standard DDS recipe gives correct red
at mip 0, but mip 1 throws ArrayIndexOutOfBoundsException in pinned Toolkit.
Toolkit is therefore an oracle for sampled top mips here, not higher-mip offsets.
No reference code patched. DDS mip count zero must remain separately represented.

`UNKNOWN`: full non-DXT swizzle behavior, LN/UN combinations, GTFs/GTF S variants,
unusual remap/pitch/offset, cube face ordering, volumes, texture footer semantics,
all pixel formats and exact cross-decoder rounding. BC1 endpoint expansion and
interpolation need explicit synthetic rounding tests before comparing digests.

## Licensing boundary

Toolkit's top-level license is MIT; its external DDSReader carries a per-file
MIT notice, Copyright (c) 2015 Kenji Sasaki. Toolkit DDS::convertSwizzleGtf cites
[RPCS3 rsx_utils.h at 3d49976b3c0f2d2fe5fbd9dba0419c13b389c6ba](https://github.com/RPCS3/rpcs3/blob/3d49976b3c0f2d2fe5fbd9dba0419c13b389c6ba/rpcs3/Emu/RSX/rsx_utils.h).
That revision's [root LICENSE](https://github.com/RPCS3/rpcs3/blob/3d49976b3c0f2d2fe5fbd9dba0419c13b389c6ba/LICENSE)
is GPL version 2. Exact contribution/relicensing history remains unresolved; do
not infer that the cited routine can be translated under Toolkit's MIT license.
All of it stays an external oracle. Refresh remains AGPL-3.0 reference only.
C2 starts BC1 using the public block specification and excludes that swizzle path.

## Reproduction

Run from the project root; use new filenames because overwrite is refused:

```powershell
. ./scripts/use-local-rust.ps1
cargo run -p lbp-catalog --example texture_census -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/c1-texture-census-new.json
python scripts/texture_report.py select local-analysis/c1-texture-census-new.json local-analysis/c1-texture-samples-new.json
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/c1-texture-samples-new.json -Output local-analysis/c1-texture-oracle-new.json
python scripts/texture_report.py verify local-analysis/c1-texture-census-new.json local-analysis/c1-texture-verification-new.json --manifest local-analysis/c1-texture-samples-new.json --oracle local-analysis/c1-texture-oracle-new.json
cargo test -p lbp-catalog --all-targets
cargo clippy -p lbp-catalog --all-targets -- -D warnings
cargo fmt --all --check
python -m unittest discover -s tests -p test_texture_report.py -v
```

Rust: ten existing catalog tests plus four diagnostic tests passed. Python: three
new parity-report tests passed. Java wrapper runs synthetic clear/stored/raw/zlib
guards and top-mip/mip-offset experiment before local samples. Initial Clippy
constant-chunk lint corrected with as_chunks; final checks pass. GUI unchanged.
An initial header-only census is superseded by c1-texture-census-dds.json/repeat.

## C2 scope and synthetic recipes

Recommended first supported variant: `GTF ` raw format 0x86, dimension 2,
depth 1, cube 0, remap 0xaae4, pitch/offset 0; top mip only, bounded CPU RGBA IR.
Retain bump flag metadata; mark non-color preview appropriately. Unknown tail bytes
stay unparsed. Add TEX/DDS BC1 only after the same core decoder is verified.

Authored fixtures should generate bytes in memory from these recipes:

- 4x4 BC1 RGB565 red/green endpoints, indices cycling 0/1/2/3: channel order,
  little-endian endpoint/index fields, interpolation rounding.
- Endpoint ordering reversed/equal with index 3: transparent vs opaque modes.
- Four distinct 4x4 blocks in an 8x8 image: block order, row order, no Morton step.
- 7x5 image and 1x1 image: partial blocks, ceil counts, clipping and checked sizes.
- Low-valued 5/6-bit channels: distinguish bit replication from alternative rounding.
- GTF and TEX wrappers with raw, stored, zlib and mixed chunks; split DDS prefix
  across chunks; truncated/incorrect lengths, missing end marker, trailing data.
- Cube/volume, unknown raw format/remap/pitch/offset, encrypted markers, excess
  pixels/decompression/read budgets: explicit unsupported or blocked outcomes.

Compare selected BC1 top-mip metadata and RGBA digests with the recorded oracle.
Any mismatch must be investigated with specification/synthetic patterns, never
silently rounded away. Commit recipes/expected synthetic metadata only. GUI preview
then uses cancellable source reads/decode, bounded cache, aspect-preserving zoom,
checkerboard alpha and channel views; UI owns the GPU image, parser owns CPU IR.
Higher mips, BC3, uncompressed formats, cubes and volume previews are later tasks.
