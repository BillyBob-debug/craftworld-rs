# D79: FNTb font face header and glyph-table metadata

Date 2026-10-07 (America/Chicago). Cloud implementer; **not verified against disc data**.

## Question
After D77 the 01.33 scan leaves one `unsupported` resource: `icon_patch_font.fnt` (FNTb,
head 985 = 0x3d9), refused because the word at +8 does not lead to a dependency table.
What is the FNTb layout, and can its header and glyph table be read without decoding
glyph bitmaps?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| FONTFACE is magic `FNT`, type 15; only binary (`b`) serialization is handled | `enums/ResourceType.java`, `types/data/ResourceInfo.java` (FONTFACE branch) | CONFIRMED_FROM_COMMUNITY_CODE |
| Not a `SerializedResource`: reads magic 0x464E5462, i32 revision, i32 "data offset" (ignored), i32 "Padding presumably", with `USE_NO_COMPRESSION` (no dependency table) | `resources/RFontFace.java` constructor | CONFIRMED_FROM_COMMUNITY_CODE |
| Then `shortarray` glyph index, `intarray` glyph-page-used, `array<GlyphInfo>`; each is big-endian u32 count + elements (Toolkit defaults 128 / 16) | `RFontFace` ctor; `io/streams/MemoryInputStream.java::shortarray/intarray`; `io/serializer/Serializer.java::array` | CONFIRMED_FROM_COMMUNITY_CODE |
| GlyphInfo = i16 character, i16 advance, i8 boxLeft, i8 boxTop, i8 boxW, i8 boxH, i32 offset, i16 nextGlyph (14 bytes) | `structs/font/GlyphInfo.java::serialize` | CONFIRMED_FROM_COMMUNITY_CODE |
| One bool "isCompressed" only when revision > 0x272; Toolkit comments "Probably not even close to the right revision" | `RFontFace` ctor | CONFIRMED_FROM_COMMUNITY_CODE (gate itself STRONG_INFERENCE) |
| Remaining bytes are glyph data. Compressed: at `offset` a BE u16 size including itself, zlib stream to `offset+size`, inflates to w*h bytes. Uncompressed: w*h*2 bytes | `RFontFace::getGlyphData`, `getGlyph` | CONFIRMED_FROM_COMMUNITY_CODE |
| The +8 word is not a dependency offset (implausible count) | local 01.33 sample, D68 scan | CONFIRMED_FROM_LOCAL_DATA (one sample) |
| What +8 actually points at, glyph-index / nextGlyph semantics, real glyph spans | not read in this task | UNKNOWN (PC verify) |

## Implementation (`crates/lbp-resource`)
- `font.rs`: `parse_font_face(bytes, FontLimits)` and crate-internal staged
  `read_font_face(fetch, size, limits)`. Untrusted counts capped (index 65536, page
  words 4096, glyphs 65536), checked against remaining bytes before allocation. The
  compression byte must be 0/1 (Toolkit accepts any non-zero; strict here, `Unsupported`).
  Glyph spans are bounds-checked only (`GlyphDataCheck`: in/out of bounds, empty boxes,
  non-negative index entries >= glyph count); an out-of-range span is counted, not fatal.
  Nothing is inflated or rendered. Words at +8/+12 are recorded raw.
- `metadata.rs::inspect`: FNTb (head <= 0x3f8) now reads the glyph tables under the
  normal metadata budget: header scope `font_face_header`, `compressed` and
  `payload_offset` (= glyph data start) set, status `metadata_parsed`,
  `dependency_status` `not_applicable_to_font_face`, new `font` summary field (omitted
  from JSON for every other resource). Failures keep the header and report
  malformed/limit/unsupported as before.

## Tests (synthetic recipes only, `tests/font.rs`)
`d79_uncompressed_layout_fields_and_spans`, `d79_compression_bool_only_after_0x272`,
`d79_compressed_spans_use_inclusive_u16_prefix_and_are_not_inflated`,
`d79_index_entries_past_glyph_count_are_counted_not_fatal`,
`d79_truncation_counts_and_magic` (every truncation through the flag malformed, count
limits, wrong marker), `d79_inspect_reports_font_summary_without_dependency_table`
(summary, budget refusal, no `font` key on other resources).

## Expected effect / not tested
- 01.33 scan: `unsupported` 1 -> 0, `metadata_parsed` +1, the FNT row carries a `font`
  summary. If the real file disagrees, the status/reason says where (malformed/limit).
- Viewable/Complete: +0 (no rendering by scope).
- Not tested: the real font bytes, Toolkit oracle parity on glyph counts, glyph
  decompression, the 0x272 gate on pre-LBP2 fonts, any rendering.
