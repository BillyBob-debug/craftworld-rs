# D71: container bounds and mutation tests

Tests only; no reader behavior changed. All input bytes are authored inside the
tests (no game data, no `references/` or `local-analysis/` files).

## Formats covered

- MAP 0x100 (`lbp_archive::read_map`, `overlay_map`)
- FARC footer/index (`lbp_archive::read_farc`)
- PARAM.SFO identity (`lbp_io::sfo::read_identity`)
- Shared checked cursor (`lbp_core::ByteReader`)

## Tests

`crates/lbp-archive/tests/bounds.rs` (14):
map_empty_input_and_header_only, map_huge_count_is_rejected_without_allocation,
map_path_length_past_end_and_wrapping_are_rejected, map_zero_length_path_is_preserved,
map_non_utf8_path_is_malformed, map_duplicate_aliases_are_all_preserved_in_order,
overlay_keeps_duplicate_base_aliases_as_shadowed_and_patch_duplicates,
map_mutations_never_panic, farc_empty_and_short_inputs_are_rejected,
farc_count_larger_than_file_is_rejected_without_overflow,
farc_ranges_past_table_and_wrapping_are_rejected,
farc_zero_length_and_boundary_entries_are_accepted,
farc_duplicate_and_overlapping_entries_are_preserved, farc_mutations_never_panic.

`crates/lbp-io/tests/sfo_bounds.rs` (12): synthetic_sfo_reads_identity,
empty_and_truncated_header_are_malformed, zero_entries_is_accepted,
entry_count_overflow_and_budget, table_offsets_out_of_range_are_malformed,
key_and_value_offsets_out_of_range_are_malformed, key_without_terminator_is_malformed,
non_utf8_key_is_malformed, duplicate_key_is_malformed, zero_length_values,
identity_text_errors, sfo_mutations_never_panic.

`crates/lbp-core/tests/byte_reader.rs` (2): empty_input_reads_fail_without_advancing,
length_overflow_and_past_end_are_malformed.

Mutation tests: fixed-seed xorshift64 (seeds `0x0d71_0000_5eed_000{1,2,3}`), 4,000
iterations per format of 1-4 operations each (bit flip, random byte, boundary u32
overwrite, truncation, append). They assert no panic; MAP and FARC also check
invariants on accepted output (row indices, entry ranges below the table, table end
equal to file size). MAP/FARC alternate default limits and unlimited limits.

## Commands and results (2026-10-07, worktree)

- `cargo fmt --all --check`: ok.
- `cargo test -q -p lbp-core|lbp-archive|lbp-io`: all pass (2 + 14 + 12 new).
- `cargo clippy -q -p <same> --all-targets -- -D warnings`: clean for all three.

## Bugs found

None. No panic, overflow or unbounded allocation found. Observed behavior worth
knowing: duplicate SFO keys are rejected as Malformed (by design); FARC table
allocation is bounded by `max_table_bytes` and by the reported file size, so a
`Read + Seek` source that lies about its size under unlimited limits could request a
large allocation (not reachable with real files or default limits; not tested).

## Not tested

Real game files; 32-bit `usize` targets; `lbp_io::source` directory access;
coverage-guided fuzzing; I/O errors mid-read from the FARC source.
