# D90 corpus texture decode census (01.33) — 2026-10-07

## Question

On 01.33 the scanner only magic-identifies the 23,477 texture MAP rows. How many
actually decode with the existing Rust texture decoder, and why do the rest fail?

## Method

New read-only example `crates/lbp-preview/examples/corpus_texture_census.rs`
(no decoder change, no new format support; `crates/lbp-preview/src/linked.rs` untouched,
`lbp_preview::resolve` only called).

- Scope (`scope`: `corpus_all_texture_map_rows`): every snapshot MAP row whose
  magic-confirmed type is `texture`, grouped by content hash (BTreeMap, deterministic),
  resolved once with `lbp_preview::resolve(catalog, None, hash)`, read through the
  approved `AttachedSource::read_hex_checked` pages and decoded once with
  `lbp_texture::decode_checked` (top mip only). RGBA is dropped immediately; no pixels
  are written anywhere.
- Units: distinct texture hash and distinct catalog GUID (a hash outcome applies to
  every GUID naming it; aliases preserved; GUIDs with mixed outcomes are counted).
- Outcomes map one-to-one from the decoder/resolver: decoded, unsupported
  (`Error::Unsupported`), malformed (`Error::Invalid`), over_budget, missing_file
  (resolver "Unresolved"), ambiguous, blocked_protected, read_failed. Reasons keep the
  decoder's own text.
- Format labels come from an independent label-only probe: GTF raw format byte plus
  2d/cube/volume; TEX DDS FourCC or RGB bit count from the first 128 expanded bytes
  (strict bounded zlib, at most 16 chunks). The probe never decides an outcome. Layout
  rules: CONFIRMED_FROM_COMMUNITY_CODE (Toolkit pin d6a2ef80, see `docs/textures.md`),
  DDS_HEADER CONFIRMED_FROM_DOCUMENTATION.
- Buckets: archived (stored) bytes, and max(width, height) from the probe.
- Budgets: at most 50,000 hashes; total archived bytes read default 4 GiB
  (`--max-total-bytes`, at most 16 GiB, checked add before each read); per texture the
  decoder's 16 MiB input / 32 MiB expanded / 4 Mi pixels (oversized inputs are refused
  without reading); `--max-seconds` default 7200. Exceeding a global budget aborts.
- Output: versioned JSON (`schema_version` 1, `scope`, `units`, `budgets`), new file
  under ignored `local-analysis/` only; elapsed time printed to stdout only.

## Counts (01.33: disc + update 0133, snapshot d68-inventory-0133-verified.json)

Baseline: before D90 the decode totals were unknown (scanner identification only;
C1 measured 01.00 headers, not decodes).

| Outcome | Hashes | GUIDs |
|---|---:|---:|
| In scope | 23,247 | 23,477 |
| Decoded (top mip) | 22,973 | 23,195 |
| Unsupported | 274 | 282 |
| Malformed / over budget / missing file / ambiguous / blocked / read failed | 0 | 0 |

Unsupported reasons: "pixel format or layout flags" 270 hashes / 278 GUIDs;
"cube/volume/dimension" 4 / 4. GUIDs with mixed outcomes: 0.

| Format (probe label) | Decoded h / g | Unsupported h / g |
|---|---:|---:|
| GTF 0x88 (BC3) | 13,030 / 13,146 | 0 |
| GTF 0x86 (BC1) | 7,270 / 7,328 | 4 / 4 (cube) |
| TEX/DDS DXT5 | 1,411 / 1,446 | 0 |
| TEX/DDS DXT1 | 1,261 / 1,274 | 0 |
| TEX/DDS RGB32 flags 0x41 (BGRA8) | 1 / 1 | 0 |
| GTF 0x85 (A8R8G8B8) | 0 | 187 / 187 (186 2d, 1 volume) |
| GTF 0x81 (B8) | 0 | 80 / 88 |
| GTF 0xa5 (0x85 + LN) | 0 | 3 / 3 |

Decoded pixel formats (hashes): BC3 14,441, BC1 8,531, BGRA8 1. Decoded with bytes
after the top mip (unparsed, see C1 mip-layout UNKNOWN): 20,651; no-sRGB flag 2,725.

By archived size (decoded / unsupported hashes): <4 KiB 6,139 / 35; 4-16 KiB
6,993 / 75; 16-64 KiB 7,169 / 53; 64-256 KiB 2,332 / 26; 256 KiB-1 MiB 330 / 15;
1-4 MiB 10 / 38; 4-16 MiB 0 / 32. By max dimension: <=16 84 / 4; 17-64 3,127 / 66;
65-256 15,987 / 102; 257-1024 3,759 / 21; 1025-4096 16 / 81.

Excluded: 4 MAP rows with a `.tex` extension hint whose magic is JPEG (ffd8ffe0,
unconfirmed type); 244 texture-typed archive hashes without a MAP row.

Run: 1,089,159,991 archived bytes read, 1,649,536,388 expanded bytes decoded, 478 s
(release). Output `local-analysis/d90-texture-census-0133.json` (11,716,820 bytes,
SHA-256 c8ad49a695fadfa1ecf31a4db435e9e32e752754eda23354509b095c05db1c9d). A repeat run
(`d90-texture-census-0133-repeat.json`, 294 s) is byte-identical.

These are newly *measured* totals, not a code change: +0 newly viewable / +0 newly
Complete from D90. Unsupported work left: uncompressed GTF 0x85/0xa5 (190 hashes,
mostly large) and B8 0x81 (80), plus 4 BC1 cubes and 1 volume.

## Oracle cross-check (pinned Toolkit d6a2ef80)

25 deterministic DATA.FARC samples (2-4 per outcome/format/layout group, smallest and
largest) from the census, `scripts/run-texture-oracle.ps1` (disc DATA.FARC only; patch
archives not sampled). Toolkit decoded 22, excluded 3 (2 GTF BC1 cubes, 1 0x85 volume;
its runner refuses non-simple 2D). `texture_parity` against the oracle: 16 Rust-decoded
samples (4 each GTF BC1/BC3, TEX DXT1/DXT5) compared, 0 pixel/metadata mismatches,
0 failures. The 6 samples Toolkit decodes but Rust refuses (2 each GTF 0x85, 0x81,
0xa5) confirm those are decoder gaps, not malformed data. The single BGRA8 TEX was not
in the sample (not in DATA.FARC).

## Commands (from the main checkout; binaries built in the D90 worktree)

```
cargo build -q --release -p lbp-preview --example corpus_texture_census
corpus_texture_census SNAPSHOT "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d90-texture-census-0133.json  -> 22973/274 hashes, exit 0, 478 s
corpus_texture_census ... local-analysis\d90-texture-census-0133-repeat.json  -> byte-identical (same SHA-256)
python -I d90_select.py (scratchpad, not tracked) -> local-analysis\d90-texture-oracle-samples.json (25)
.\scripts\run-texture-oracle.ps1 -SampleManifest local-analysis/d90-texture-oracle-samples.json -Output local-analysis/d90-texture-oracle.json  -> 22 decoded, 3 excluded
texture_parity SNAPSHOT GAME-DIRS local-analysis\d90-texture-oracle.json local-analysis\d90-texture-parity.json  -> 16 compared, 0 mismatches
cargo fmt --all --check  -> clean
cargo test -q -p lbp-preview --all-targets  -> 42 + 5 new example tests passed
cargo clippy -q -p lbp-preview --all-targets -- -D warnings  -> clean
```

## Not tested

- Decodability only: no visual check, higher mips not decoded, no color-space claim.
- Oracle sample covers DATA.FARC only (13,220 of 23,477 rows); patch FARC textures
  were decoded by Rust but not cross-checked. BGRA8 TEX not oracle-compared here.
- 01.00-only corpus not rerun. The census is not shown in the app yet.
