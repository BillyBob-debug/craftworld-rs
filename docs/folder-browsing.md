# Literal MAP folder browsing

D38 completed 2026-10-06 (America/Chicago).

MAP resources now have child-folder links with row counts, Root/Up breadcrumbs and
the current prefix. CLI: `--folder MAP_PREFIX`. A trailing slash is appended when
absent; case and all other characters remain literal. Folder grouping is snapshot
metadata only and never filesystem traversal, source reads or path resolution.

Implementation: `app/folders.rs`, integrated with cached resource sorting, navigation
and viewer options. No catalog/disk format/schema/dependency changes.

## Contract

Counts follow query/type/status/archive filters. Each folder count includes all
matching descendants, counting duplicate MAP aliases separately. The resource list
also includes descendants. Immediate children sort by exact case-sensitive prefix;
resource order still follows the chosen MAP/path/GUID/SHA1 sort.

A prefix ends at a slash boundary, so `a/` cannot select `ab/`. Folder grouping
accepts nonempty relative slash-separated components excluding empty, dot/dot-dot,
backslash, colon and control characters. Unicode is retained. Paths outside this
shape remain in the root resource list as ungrouped rows; no normalization or
rewriting of original path metadata occurs. A non-root prefix selects only ordinary
paths with that exact prefix. A syntactically valid nonexistent prefix shows an
empty list; it is not an I/O or corruption error.

Each link/prefix is limited to 4096 UTF-8 bytes and32 components. Checked candidate
length guards avoid oversized allocations before creating links. Resources beyond
those limits remain visible at the nearest reachable prefix as ungrouped rows.
Only one child level is built, using existing filtered indices. Children and resource
lists are virtualized; the child scroller has110px maximum height. No recursive tree
or per-row filesystem access exists. Counts satisfy child subtree totals + direct
rows + ungrouped rows = visible resource rows.

Root/Up/child links preserve other filters, sorting, detail selection and its caches.
A hidden detail selection stays selected. Resource navigation/Back clears a hiding
folder prefix along with filters to reveal its target, using the existing resource
history policy. Folder clicks themselves are not resource-history entries. Switching
to archive overview hides folder controls; MAP browsing can retain that prefix.
Clear filters clears the prefix. Successful snapshot replacement resets to root,
except the initial CLI prefix applied once at startup. Failed snapshot loading keeps
the existing catalog/view. `--folder` is rejected with `--check-snapshot`, whose count
output does not implement folder browsing.

Grouping/filtering runs on the UI thread over the bounded existing catalog. Maximum
one-million-row latency was not benchmarked. This milestone does not provide complete
folder-history restoration or persistent preferences.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
cargo test -p lbp-viewer folders
cargo clippy -p lbp-viewer --all-targets -- -D warnings
```

All passed:152 tests including61 viewer tests. Three authored cases cover duplicate
aliases/counts, slash boundaries, unusual/root paths, Unicode/case, invalid components,
byte/depth budgets and preservation of resources at exhausted budgets, parent links,
combined query/type/status/archive filters with sorting, hidden selected resources,
navigation reveal and snapshot reset. After final test-only budget/filter assertions,
the three focused folder tests, fmt and viewer Clippy passed. Production code was
unchanged after the full build/native verification. Existing minimum-size all-tab,
sorting, archive and navigation checks remain green. No failed experiments this turn.

Four hidden, auto-exit native snapshot-only checks exit0 (RTX3080/Vulkan):

| Private artifact stem | Prefix/filter | Rows | Child folders | Direct rows |
|---|---|---:|---:|---:|
| d38-folders-root | root |36854|1|2|
| d38-folders-gamedata | gamedata/ |36852|21|0|
| d38-folders-plan-selected | main_03_sharp_shooting/, query .plan |13|0|13|
| d38-folders-absent | nonexistent |0|0|0|

All summaries match independent Python calculations from the existing snapshot:
visible count, child count and first10 child names/counts, direct/ungrouped totals,
first10 sorted resource rows and selected original index. None has unusual local
paths; synthetic cases cover that rule. The selected folder preserves GUID0002a641.
The gamedata and selected-folder PNGs were inspected. Root/Up controls, counts,
virtualized child area and selected detail fit. No manual folder-click, Up, Root or
scroll interaction was performed. QA summaries intentionally include only first10
child/resource records, not an unbounded diagnostic log.

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --folder gamedata --sort path --qa-capture local-analysis/new-folder.png
target/debug/lbp-viewer.exe --folder ../game-data
```

Invalid prefix and incompatible snapshot-check combination were rejected before
launch, exit1. No original source was attached during native checks. SFO/MAP hashes
unchanged, DATA.FARC1023990298B/read-only, reference clean at
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Authored files allowed; diagnostics,
references and game data ignored. No root Git/commit/publication or current blocker.

## Next D39

Improve the already understood mesh preview with an optional neutral solid surface
view alongside wireframe. Use existing verified positions/topology only; generate
face shading from geometry, without claiming parsed normals, UVs, material fidelity,
bones or morphs. Keep current budgets/selection/source lifecycle and parser unchanged.
Evaluate a bounded UI-side projected-triangle approach and document its occlusion
limits; retain wireframe fallback. Verify authored geometry/extremes/degenerates,
existing mesh metadata regression and native supported/unsupported captures before
claiming the visual result. No large renderer or speculative format work.
