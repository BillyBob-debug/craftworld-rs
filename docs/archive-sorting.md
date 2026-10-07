# Archive-entry ordering

D37 completed 2026-10-06 (America/Chicago).

The Archive entries list now offers **Table order**, **Offset**, **Size** and **SHA1**.
CLI: `--entries --entry-sort table|offset|size|hash`. The entry sort option does not
change MAP sorting or automatically switch modes; omitted entry sort means table
order. Invalid values fail before launching the GUI.

## Contract

UI-only implementation: `app/entry_order.rs`, integrated with `app/entries.rs`,
`app.rs` and viewer options. Sorting changes only a vector of original
`(archive_index, entry_position)` pairs. No parser, source read, snapshot record,
resource resolution, alias count or dependency changes.

Offset, size and SHA1 are ascending **global** keys across listed archives. Offsets
compare numerically within their respective files; equal offsets do not identify the
same data. SHA1 compares fixed-width validated hexadecimal text after ASCII case
folding, without per-comparison allocation. Ties use snapshot archive order,
original `entry_index`, then original vector position. Table order uses just those
tie fields. Archive path remains visible in the selected details and row hover text;
the sort control explains the cross-archive rule in its tooltip.

Every table entry remains eligible, including orphan hashes and duplicate records.
Existing query/archive filtering still runs before sorting; MAP type/status filters
remain irrelevant to this list. Filter/sort results are cached, virtualized and
invalidated by snapshot replacement. Changing sort requests a jump to the selected
entry if visible. Filtering can hide an entry without changing its detail selection.
Navigation Back restores its original archive/entry pair; snapshot replacement
clears selection while retaining the chosen sort. MAP sorting is independent.

Sort runs on the UI thread over the existing bounded catalog. The local 31,534-entry
archive was checked; maximum-budget latency (one million entries) was not benchmarked.
There is no descending order, persistent preference or filesystem tree in this change.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All passed:149 tests including58 viewer tests. Two new authored cases cover two
archives, non-vector table entry numbering, global primary order, equal numeric
keys and mixed-case SHA1 ties, orphan and duplicate entries, existing MAP aliases,
empty lists, query/archive filters, selected original indices and jump, Back,
mode transitions and snapshot replacement. Expected complete orders are explicit.
Existing orphan navigation and minimum-size all-tab tests continue passing.

A helper initially read the help source using Windows legacy encoding and failed
on a UTF-8 character. The help edit was repeated with explicit UTF-8, and affected
display punctuation in the entries edit was restored. Rust tests were already passing;
final complete format/test/Clippy/build checks followed the correction.

Five hidden, auto-exit native snapshot-only checks all returned exit0 on RTX3080/
Vulkan, with no game directory attached:

| Private artifact stem | Ordering | Entries |
|---|---|---|
| d37-entry-sort-table | table |31534|
| d37-entry-sort-offset | offset |31534|
| d37-entry-sort-size | size |31534|
| d37-entry-sort-hash | SHA1 |31534|
| d37-entry-sort-hash-selected | SHA1, exact hash query, entry0:10625 |1|

The first10 records/count/selection/alias counts in every `QA archive list` log match
independent Python ordering of the existing snapshot. The selected record remains
archive0/vector-position10625, table entry10625, SHA1
`560ba92ca1e3492158a5bcd4423f8f8338cb0051`, offset220196959, size41510, one MAP alias.
Size and selected-hash PNGs were inspected. Native verification is one local archive
and the first ten records per order; multi-archive/equal-key behavior is covered by
authored synthetic tests, not local multi-archive assets. No manual dropdown/scroll
interaction or maximum-budget performance claim.

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --entries --entry-sort size --qa-capture local-analysis/new-entry-size.png
target/debug/lbp-viewer.exe --entry-sort invalid
```

Invalid sort returns exit1 with `Entry sort needs table/offset/size/hash`.

Original SFO/MAP hashes unchanged, DATA.FARC1023990298B/read-only, reference clean
at `d6a2ef80b5a09999f08f23a803b516096055e5c9`. Authored files allowed, native artifacts/
game data/references ignored. No root Git, commit/publication/license decision or
current blocker.

## Next D38

Add bounded MAP-folder navigation using literal resource path prefixes: immediate
child folders/counts, parent/root breadcrumbs and a prefix filter combined with
existing query/type/archive filters and resource sorting. Retain aliases and original
catalog indices. Path grouping is metadata only, never filesystem traversal or path
normalization. Define unusual-path handling explicitly. Verify synthetic prefix
boundaries, duplicate aliases, filtering/reset/navigation invariants and native
snapshot-only folder counts/selection. Do not expand recursive Plan parsing; higher
texture mips still require independent offset evidence because of the known Toolkit
mip bug documented in textures.md.
