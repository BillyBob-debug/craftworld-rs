# Deterministic MAP resource sorting

D36 completed 2026-10-06 (America/Chicago).

The MAP resources list offers **MAP row order**, **Path**, **GUID** and **SHA1**.
The CLI equivalent is `--sort map|path|guid|hash`; omitted sort defaults to MAP row
order, and invalid values fail before launching the GUI. Archive-entry ordering is
unchanged in this milestone.

Implementation: `crates/lbp-viewer/src/app/resource_list.rs`, integrated with the
existing list in `app.rs` and options in `main.rs`. This is presentation logic only:
no parser, schema, original file I/O, dependencies or snapshot records change.

## Ordering and lifecycle

Sorting reorders the filtered vector of catalog indices. Duplicate GUID/hash/path
aliases remain separate records. Paths compare by their exact case-sensitive UTF-8
byte order. Fixed-width validated hexadecimal identifiers compare after ASCII case
folding, which agrees with unsigned numeric order for GUIDs. Equal primary values
use original MAP row index, then catalog index. MAP order uses those tie fields alone.
No per-comparison string allocation or asset reading is needed.

The filter/sort cache is recomputed when its filter, sort or snapshot changes. Lists
remain virtualized through `show_rows`. Selection, resource candidates, raw metadata,
worker caches and navigation history use original catalog indices. Changing sort
requests a scroll jump to the selected row if it remains visible; filtering can hide
that row while keeping its detail selection. Navigation reveals the target using the
existing filter-clearing policy, without changing sort. Snapshot replacement clears
selection and invalidates both list caches while retaining the chosen sort.

Sorting runs on the UI thread using the already bounded catalog (at most one million
resource rows). The local 36,854-row catalog was checked; latency at the maximum
budget was not benchmarked. Sort direction is ascending only; no locale collation,
directory tree or persistent preferences are claimed.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All passed: 147 tests including 56 viewer tests. Two authored sorting cases cover
reversed input determinism, duplicate aliases and mixed-case GUID ties, changed MAP
row order, empty lists, filter/hide/reveal, selected index and scroll jump, navigation
Back and snapshot replacement. Existing minimum-size/tab/native-navigation checks
still pass. An initial synthetic fixture changed MAP row IDs without updating its
dependency links and was correctly rejected by catalog validation. The fixture links
were repaired; no validation was relaxed.

Five hidden, auto-exit native captures completed with exit 0 on RTX 3080/Vulkan,
without attaching the original game directory:

| Capture | Ordering/filter | Rows |
|---|---|---|
| d36-sort-map | MAP row order | 36,854 |
| d36-sort-path | Path | 36,854 |
| d36-sort-guid | GUID | 36,854 |
| d36-sort-hash | SHA1 | 36,854 |
| d36-sort-path-plan | Path, query `.plan`, selected GUID 0002a641 | 7,342 |

Private `local-analysis/d36-sort-*.stdout.log` QA summaries contain the first ten
sorted records, count and selected catalog index. All five match independently
computed Python ordering from the existing snapshot, including exact path/GUID/hash,
MAP row and index. The filtered selection retains GUID0002a641. Path and filtered
selection PNGs were inspected: the controls fit, order is visible, and the selected
resource is shown at its sorted location. No manual dropdown or scrollbar interaction
was performed. Native logs verify only the first ten rows; synthetic tests verify
whole-list membership/order invariants on authored data.

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --sort path --query .plan --select-guid 0002a641 --qa-capture local-analysis/new-sorted-plan-list.png
target/debug/lbp-viewer.exe --sort invalid
```

The second command returns exit 1 with `Sort needs map/path/guid/hash`.

SFO/MAP hashes remain unchanged, DATA.FARC remains 1,023,990,298 bytes/read-only,
reference checkout clean at `d6a2ef80b5a09999f08f23a803b516096055e5c9`. No root Git
initialization, commit or publication. Authored source/docs are allowed and all
native artifacts, game data and references remain ignored. No current blocker.

## Next D37

Add similarly deterministic archive-entry ordering (table order, offset, size, SHA1)
with explicit archive/entry tie breaks. Preserve orphan entries, MAP alias counts,
selected original archive/entry indices, filtered virtualized lists and navigation.
Keep source reads and parser changes out of sorting. Verify authored equal-key and
multi-archive cases, selection/reset/filter invariants, and native snapshot-only
captures against independently computed ordering. Define cross-archive ordering
clearly in the contract before implementation.
