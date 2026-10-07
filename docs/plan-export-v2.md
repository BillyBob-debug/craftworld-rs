# Plan diagnostic export version 2

D35 completed 2026-10-06 (America/Chicago). The native Plan exporter now writes
version 2 metadata-only reports using the existing shared private writer. The
version 1 schema remains valid for historical base-only reports. There is no new
format parsing, source read, dependency or original asset export in this milestone.

## Contract

Implementation: `crates/lbp-viewer/src/app/plan/export.rs`.
Schema: `schemas/plan-diagnostic-v2.schema.json`.
Historical schema: `schemas/plan-diagnostic.schema.json` (version 1).

`scope` remains `cached_plan_diagnostic_metadata_not_asset_export` and
`source_state` remains `validated_at_read_not_rechecked_for_export`. Location is
normalized source-relative path plus offset/size; host paths, timestamps and raw
error text are excluded. No source is revalidated or reread during export.

`inspection_mode` is `plan_prefix` or `plan_inventory_prefix`. Both `stages.plan`
and `stages.inventory` are always present, each with status, requested flag,
sanitized error kind and nullable metadata. The explicit projection includes nine
base-prefix fields plus flags/expanded length, and exactly 13 inventory scalar/extent
fields. Original Thing/inventory bytes, recursive records, images and translated or
normalized values are excluded. The date remains encoded unsigned bits in 16 lowercase
hex digits; raw uint32 values preserve unknown enum/flag bits.

| Situation | Request outcome | Plan stage | Inventory stage |
|---|---|---|---|
| Base-only success | complete | available | not_requested |
| Both requested stages succeed | complete | available | available |
| Inventory fails after valid base | partial | available | failed/unsupported/blocked/cancelled |
| Base fails with inventory requested | failed | failed/unsupported/blocked/cancelled | not_available |
| Base fails in base-only mode | failed | failed/unsupported/blocked/cancelled | not_requested |

A stage unavailable because the base failed has null metadata and null error kind:
it was not independently attempted. Partial request error kind identifies the
inventory failure; failed request error kind identifies the base failure. Successful
stages have null error kind. Only sanitized categories from the shared helper appear.

Busy, unselected, incomplete and inconsistent cache states are refused before the
writer runs. A requested inventory stage with a valid base but neither inventory
metadata nor an error is incomplete. Contradictory metadata/error caches, deeper
results without a base and inventory results in base-only mode are refused. Reset
continues cancelling jobs and discarding caches. Production polling suppresses
cancelled results; a synthetic stage cancellation test verifies report projection,
not publication of cancelled worker metadata.

The shared writer serializes within an 8 MiB budget before creating a new `.json`
inside an existing ordinary workspace `local-analysis` directory. It refuses overwrite,
missing parents and escaping paths. Existing limitations remain: ancestor replacement
races are not fully covered, and a failed write can leave a partial file that cannot
be overwritten by this writer. See [shared writer policy](plan-export.md).

## Provenance and verification

The disk rules, evidence categories, pin, revision applicability and known signed-64
conflict remain in [inventory prefix evidence](plan-inventory-prefix.md),
[CPU parser contract](plan-inventory-parser.md) and
[native inspector contract](plan-inventory-inspector.md). No format claims changed.
Pinned external oracle: Craftworld Toolkit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`.

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All passed: 145 tests including 54 viewer tests. Two new cases cover raw u64/u32
extremes, 13-field projection, deterministic encoding, requested/not-requested,
partial/cancel/unsupported outcomes, base preservation and contradictory/incomplete
cache refusal. Existing busy/reset/path/error-sanitization checks continue passing.
The initial test assumed InventoryPrefix implemented Clone; it does not. The test
was corrected to project its expected value before moving it into the cache, without
changing the CPU IR.

Five hidden, auto-exit native runs all returned exit 0 (RTX 3080/Vulkan), with private
artifacts `local-analysis/d35-plan-v2-{small,large,old,prefix,repeat}.*`:

- Small inventory selection GUID `00007c0b`: 1685-byte report.
- Large inventory selection GUID `0002a641`: 1687-byte report.
- Old revision 622 GUID `00012660`: 715-byte unsupported/unavailable report.
- Large base-only selection: 1165-byte report, inventory not requested.
- Repeated large inventory selection: byte-identical 1687-byte report.

Four supported base projections and three inventory projections exactly match
`d32-plan-inventory-oracle.json`. All five reports validate against the final Draft
2020-12 schema. Four synthetic partial outcomes validate; ten invalid mutations
(including bad date hex, uint32/extent overflow, payload extra field, wrong version,
outcome/mode/status inconsistencies and mismatched stage error kind) are rejected.
Historical D31 version1 report still validates. The large native PNG was inspected;
no manual export/button/cancel/scroll interaction or all-GPU/full-corpus claim.

Example (choose fresh names):

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 0002a641 --qa-tab plan-inventory --qa-capture local-analysis/new-plan-v2.png --qa-plan-json local-analysis/new-plan-v2.json
```

SFO/MAP hashes remain unchanged; DATA.FARC remains 1,023,990,298 bytes and read-only;
reference checkout clean at the pin. Default-deny ignore boundaries retained. No root
Git initialization, commit, publication or license decision. No current blocker.

## Next D36: practical resource browsing

Add deterministic MAP-resource sorting by path, GUID and SHA1, retaining an original
MAP-row-order option. Keep filtered results virtualized and preserve original catalog
indices, aliases, selected resources and navigation behavior. Tie-break equal values
by original row/index. Verify synthetic duplicate aliases, filter/sort/reload and
selection invariants plus native sorted-list capture. No original source reads, disk
format changes, GUI dependency changes or recursive Plan parsing are needed.
