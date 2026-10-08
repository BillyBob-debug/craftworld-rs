# U2: FNTb glyph decoding and atlas layout

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan (item
U2; plan in `docs/ui-research.md` once PR #14 merges). Extends D79
([d79-fntb-font.md](d79-fntb-font.md)). **Not verified against disc data**; the
Integrator assigns the D-number, runs the PC check and updates STATUS/MODLOG.

## Question
D79 reads the FNTb header and glyph table but never decodes glyph pixels. Can every
glyph be decoded under budgets into a renderer-independent bitmap and atlas layout, so a
later Bevy UI (U6) can draw the game's own fonts?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| Glyph data offsets are relative to the bytes after the compression flag | `resources/RFontFace.java` constructor (`data = rest of stream`), `getGlyphData` | CONFIRMED_FROM_COMMUNITY_CODE |
| Compressed: BE u16 size at `offset` (includes itself); zlib stream is `[offset+2, offset+size)`; inflates to `boxW*boxH` bytes | `getGlyphData`; `util/Compressor.java::inflateData` (java.util.zip Inflater, zlib header) | CONFIRMED_FROM_COMMUNITY_CODE |
| Compressed pixel = one byte drawn as grey (coverage) | `getGlyph` | CONFIRMED_FROM_COMMUNITY_CODE (meaning as coverage: STRONG_INFERENCE) |
| Uncompressed: `boxW*boxH*2` bytes, drawn as green = first byte, blue = second | `getGlyphData`, `getGlyph` | CONFIRMED_FROM_COMMUNITY_CODE; meaning UNKNOWN |
| Fonts without the flag (head <= 0x272) are uncompressed | constructor (`isCompressed` default false) | CONFIRMED_FROM_COMMUNITY_CODE |

## Implementation (`crates/lbp-resource/src/font.rs`)
- `decode_glyph(resource, font, glyph)` → `GlyphBitmap { width, height, pixels }`,
  `GlyphPixels::Coverage(Vec<u8>)` or `TwoChannel(Vec<[u8; 2]>)` (raw, uninterpreted).
- Strict where Toolkit is lenient: the zlib stream must end exactly at its declared size
  and inflate to exactly `w*h` bytes (Toolkit's Inflater zero-fills short output and
  ignores extra); otherwise `Malformed`. Empty boxes decode to no pixels without reading
  a stream (Toolkit would read the prefix anyway).
- `decode_glyphs` decodes all glyphs under a total-bytes budget (default 64 MiB),
  counting malformed glyphs instead of failing; summary: decoded, empty, malformed,
  pixels, max coverage byte.
- `atlas_layout(glyphs, max_width, padding)`: shelf packing in glyph order, positions only,
  no pixels or GPU types (AGENTS.md rule 5).
- Example `font_glyphs` (PC helper) writes `<prefix>.json` plus PGM glyph sheets under
  `local-analysis/` (one sheet per raw channel for uncompressed fonts).

## Tests (synthetic recipes only, `tests/font.rs`)
`u2_uncompressed_glyphs_keep_two_raw_bytes_per_pixel`,
`u2_compressed_glyphs_inflate_to_exact_coverage` (good, short, long, trailing, invalid,
truncated prefix, empty box, budget, wrong resource length),
`u2_atlas_layout_wraps_rows_with_padding`.
Cloud (Linux): `cargo fmt --all --check` ok; `cargo clippy -q -p lbp-resource
--all-targets -- -D warnings` clean; `cargo test -q -p lbp-resource` all pass.

## PC verification (Integrator)
For every FNTb on 01.33 (D79 knows `icon_patch_font.fnt`; ui-research PC check 5):
`cargo run -q -p lbp-resource --example font_glyphs -- <extracted.fnt> local-analysis/<new-prefix>`.
Report decoded/empty/malformed per font, the compressed flag, and look at the sheet once
to settle whether coverage is 0..255 alpha and, for uncompressed fonts, what the two
channels mean. Toolkit parity: Toolkit `RFontFace.export` writes a comparable sheet.

## Expected effect / not tested
- Viewable/Complete: +0 (fonts, not assets). New UI-track count: glyphs decoded per font.
- Not tested: real fonts, line metrics/kerning (UNKNOWN), rendering.
