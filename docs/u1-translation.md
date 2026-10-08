# U1: translation tables (`.trans`) and the LAMS key hash

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U1). **Not verified against disc data**; the
Integrator assigns the D-number, runs the PC checks below and updates STATUS/MODLOG.

## Question
Every name the Popit, slot badges, pins and menus show is a u32 LAMS key into a
translation table. What is the `.trans` layout and the key hash, and can a bounded
reader resolve keys and tags to strings?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| TRANSLATION is type 30 with no magic; detected by the `.trans` extension | `enums/ResourceType.java`, `types/data/ResourceInfo.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Data starting with BE U+FEFF is the legacy text form: UTF-16 lines, tag then value, empty tag lines skipped, key = LAMS hash of the tag | `resources/RTranslationTable.java` byte[] constructor | CONFIRMED_FROM_COMMUNITY_CODE |
| Binary form: BE i32 count, count × {u32 key, i32 offset}; offsets relative to `4 + 8*count` | same | CONFIRMED_FROM_COMMUNITY_CODE |
| A string is read from its offset; the end is the next BE U+FEFF unit after the first unit, or the end of data; decoded as UTF-16 with mark detection | same | CONFIRMED_FROM_COMMUNITY_CODE |
| Repeated key: last row wins (HashMap put) | same | CONFIRMED_FROM_COMMUNITY_CODE |
| `makeLamsKeyID`: base-27 fold of UTF-16 units 1..32 (space padded, from the 32nd down), high part seeded 0xC8509800 or folded from units 33..64 when longer than 32; `(low + high*0xDEADBEEF) & 0xFFFFFFFF` | `RTranslationTable::makeLamsKeyID` | CONFIRMED_FROM_COMMUNITY_CODE; test vectors computed by running the pinned function |
| Rows sorted by unsigned key (Toolkit `build()` writes them sorted) | `RTranslationTable::build` | STRONG_INFERENCE for game files; counted, not enforced |
| English table g54800, patch table g69910 | `tools/unlockall/RestlessPopulator.java` | CONFIRMED_FROM_COMMUNITY_CODE (paths/languages UNKNOWN) |

## Implementation (`crates/lbp-resource/src/translation.rs`)
- `lams_key(tag)`, `parse_translation_table(bytes, TranslationLimits)` returning
  `TranslationTable` (form, rows in file order, `get(key)`, `translate(tag)`, summary).
- Untrusted input: negative count/offset, row table past the data, string start past
  `len - 2` and an odd tail without a mark are `Malformed` (where Toolkit would throw);
  entry count (default 1 Mi) and total decoded code units (default 64 Mi, offsets may
  share strings) are `LimitExceeded`. Odd-length legacy text is `Malformed` (Toolkit
  would decode a replacement character; strict here).
- Counted, not fatal: repeated keys, out-of-order keys, shared offsets, strings without a
  mark, little-endian marked strings, invalid UTF-16 (replaced with U+FFFD as Java does).
- Not wired into `inspect`/scanner or either viewer yet (U3 does the browser).

## Tests (synthetic recipes only, `tests/translation.rs`)
`u1_lams_key_matches_toolkit_vectors`, `u1_binary_strings_run_to_next_mark_or_end`,
`u1_empty_string_and_end_of_data`, `u1_repeated_keys_keep_all_rows_and_last_wins`,
`u1_unmarked_little_endian_and_invalid_strings_are_counted`,
`u1_malformed_offsets_counts_and_limits`, `u1_legacy_text_form_alternates_tag_and_value`.

Cloud (Linux): `cargo fmt --all --check` ok; `cargo clippy -q -p lbp-resource
--all-targets -- -D warnings` clean; `cargo test -q -p lbp-resource` all pass (7 new).

## PC verification (Integrator)
1. Find every `.trans` on 01.33 (MAP paths, sizes, binary vs text form); confirm g54800
   and g69910.
2. For each, extract the file to `local-analysis/` and run
   `scripts/run-translation-oracle.ps1 -Trans <file> -Output local-analysis/<new>.json`
   (Rust `translation_dump` example, then `TranslationOracle.java` against the pinned
   cwlib jar: every distinct key's text equal, tag hashes equal).
3. Report rows, distinct keys and the check counters per file.

## Expected effect / not tested
- Viewable/Complete: +0 (strings only, no assets). New count for the UI track: strings
  resolved per table.
- Not tested: real `.trans` files, the Java oracle (cwlib could not be built in the cloud
  container: jitpack dependencies blocked), languages other than English.
