# Optional native Plan inventory inspector

D34 completed 2026-10-06 (America/Chicago).

The Plan tab now offers **Inspect inventory scalar prefix** for supported modern
PLNb resources. It uses the independently verified CPU reader from
[plan-inventory-parser.md](plan-inventory-parser.md). Parsing remains independent
of the UI. No new disk-format claims are introduced.

## Contract and provenance

The source rules, revision gates, pinned community class/method references,
signed-64 conflict and local evidence are recorded in
[plan-inventory-prefix.md](plan-inventory-prefix.md). The external oracle uses
Craftworld Toolkit commit `d6a2ef80b5a09999f08f23a803b516096055e5c9`.

The optional worker reads each source resource once, then calls the combined CPU
inspection. Existing early header preflight, 2 MiB input / 16 MiB expansion budgets,
4096-byte source paging and cancellation checks remain in effect. The cache stores
metadata only. Inventory errors remain separate from wrapper/base-prefix errors:
a valid base prefix remains available if the deeper scalar prefix fails.
Cancelled jobs cannot publish either cache. Reload, route/source/snapshot changes
clear both caches and errors; the chosen inspection mode survives reset.

The UI shows encoded date bits as 16 hexadecimal digits, eight raw uint32 fields
(slot type/number, sound GUID, colour, type flags, subtype, title and description
keys), and four prefix extent counts. Decimal/hex views preserve unknown bits.
Dates, enums, colours, localization keys and references are not normalized or
resolved. Inspection stops before recursive CreationHistory. The base prefix is
available in a collapsible section below the inventory details.

Version 1 JSON export remains explicitly base-prefix-only. Optional inventory
metadata and errors require a separately versioned, stage-aware envelope (D35).

## Verification

Commands run successfully:

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
cargo test -p lbp-viewer plan
cargo clippy -p lbp-viewer --all-targets -- -D warnings
```

The full suite passed 143 tests, including 52 viewer tests. Three added viewer cases
cover single-read optional dispatch, deeper failure preserving the base cache,
cancellation, reload/reset and raw-field layout. Existing route/snapshot assertions
also cover inventory cache clearing and mode retention. After those final test-only
assertions, nine focused Plan tests and viewer Clippy passed. A formatting difference
was corrected with `cargo fmt`; the subsequent format check passed.

Five hidden, auto-exit native runs completed with exit code 0 on RTX 3080 / Vulkan.
Four inventory-mode PNGs were inspected; the base-mode run was checked through its
cache log and export. Private artifacts are under `local-analysis/d34-plan-inventory-*`:

- Small MAP resource: GUID `00007c0b`, SHA1
  `1bfb2c52fa35e66c1c30a1f7e93464ae82105900`.
- Large MAP resource: GUID `0002a641`, SHA1
  `560ba92ca1e3492158a5bcd4423f8f8338cb0051`.
- Archive 0, entry 10625: same large resource.
- Old MAP resource: GUID `00012660`, revision 622, explicit unsupported result
  with neither metadata cache published.
- Base-only mode: large MAP resource, no inventory cache line.

All three supported inventory cache objects match all 13 scalar/extent fields in
`d32-plan-inventory-oracle.json`; their base cache objects also match. Base-only
mode matches the same base oracle. Both exported reports (inventory mode and base
mode) validate against `schemas/plan-diagnostic.schema.json` and are byte-identical
to the earlier 979-byte `d31-plan-large.json`.

Example native QA command (use a fresh output name):

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 0002a641 --qa-tab plan-inventory --qa-capture local-analysis/new-plan-inventory.png
```

No manual checkbox, scrolling, collapse, cancel or source-detach interaction was
performed. Synthetic lifecycle checks and automated cache captures do not prove
full corpus support, Thing compatibility, visual asset correctness or all GPU
compatibility. No recursive inventory interpretation or game assets were exported.

The SFO/MAP hashes are unchanged, DATA.FARC remains 1,023,990,298 bytes and read-only,
and the reference checkout remains clean at the pinned commit. No root Git repo,
commit, publication, license decision or current blocker.

## Exact next task: D35

Add version 2 cached Plan diagnostic JSON with explicit base/inventory stages,
requested mode, separate sanitized outcomes and metadata-only projections. Preserve
base metadata on a deeper failure; distinguish not-requested, unavailable and failed
inventory stages. Retain the version 1 schema for historical reports. Use the shared
private writer without rereading inputs; verify synthetic failure/cancel/busy/reset
cases, schema rejection, native oracle parity and deterministic repeat output.
Do not parse recursive references or introduce date/title semantics.
