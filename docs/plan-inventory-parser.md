# D33 independent optional inventory scalar-prefix reader

2026-10-06 America/Chicago. CPU-only `lbp-resource::plan::inspect_inventory` adds the
bounded metadata slice verified by [D32 external oracle](plan-inventory-prefix.md).
No new dependencies, native UI, payload export or recursive object parser.

## API and separation

Existing `plan::inspect` / PlanInspection / version1 diagnostic schema remain
unchanged. Private prefix helper factors existing logic, with no public expanded
buffer. Combined inspect_inventory validates/expands once, computes opaque digest
once and returns PlanInventoryInspection: valid base PlanInspection plus
`inventory: Result<InventoryPrefix, binary::Error>`. Base wrapper/prefix failure is
outer Error; deeper inventory failure is inner Error, retaining base diagnostic.
No inventedzero/empty metadata on deeper failure. Caller must honor its cancellation
token before publishing; cancellation while parsing deeper stage may return valid
base with innerCancelled. Existing prefix-only cancellation behavior unchanged.

Inventory IR has exact encoded date u64 as16hex digits, raw uint32slot/type/number/
soundGUID/colour/typeflags/subtype/title/descriptionkey, inventorystart, consumed
prefixbytes/end and remainingunparsedcount. Zero/highbits/unknownenums retained;
no date normalization, buggy-reader emulation, EnumSet loss or translation lookup.
No signed64 JSONprecision risk. Stop before first CreationHistory reference; no
recursive tags, icon, user/photo/creator or Thing fields read. Bytecounts do not
establish a valid inventory/object graph.

Head893..951/branch0/sub0,flags0/7 enforced by existing planwrapper/prefix gate.
Stored2MiB/expanded16MiB and existing checked integer32/payload bounds unchanged.
Private date reader fixedBE64 or<=10ULEB64bytes/finalbyte<=1; checked offsets and
strict truncation/overflow. Field-by-field cancellation and before success publish.
No new counts/allocations from inventory data; only fixed13fieldmetadata projection.
The CPU library has no filesystem/UI/GPU dependencies. Opaque arrays never escape.

## Provenance and conflicts

Full source URLs/pin/classmethods/evidence/build applicability and limitations:
[plan-inventory-prefix.md](plan-inventory-prefix.md), near implementation comments in
`plan/inventory.rs`. Independent authored implementation, no reader source copied,
adapted or translated. Community field order from InventoryItemDetails::serialize;
primitives and actual SlotID externally tested, not full InventoryItemDetails.

Known compact s64 arithmetic-shift conflict is intentionally avoided by reporting
unsigned encoded bits; no decodedtimestamp field. All48local dates/slots/sounds zero;
nonzero/highbit cases only synthetic. Unknown legacy/streaming/ref/object semantics
still unimplemented. Optional reader failure does not discredit already parsed prefix.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-resource --test plan
cargo run -p lbp-catalog --example plan_inventory_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d32-plan-inventory-oracle.json local-analysis/new-inventory-parity.json
cargo run -p lbp-catalog --example plan_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d28-plan-oracle.json local-analysis/new-prefix-regression.json
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All pass:140workspace/49viewer tests; eightplan tests include fournew inventorycases.
Authored tests cover modernrevisionedges, flags0/7, raw/zlib/zerochunk/storedchunk,
encoded0/1/2/u64max/bit63/extremes, unknownraw32/zeroGUID/key, untouched invalidrecursive
tag, everyinventoryprefix truncation, missingtail, retainedbase, u64/u32overflow,
outerprotection/branch/cancel and deeperstagecancel distinct from base failure.
No proprietary fixture bytes.

Initial synthetic zlib recipe happened to have compressedbytes==expandedbytes for
one new case. Format treats equal sizes as stored chunks, so test recipe was invalid
for its intended zlib path. Corrected authored encoder to choose a distinct valid
stream when equality occurs; parser/inflate rule unchanged. Retried narrowtest passes.
No original data involved in failed test.

Read-only parity example: validatedsource,4096pages,48samples/64MiBrun, exactpin/scope,
uniquehash-location requirement, sourcestability check and privateexclusiveJSON.
48inventory and48combinedbase records allfieldequal D32oracle;313643resourceB/run,
zero mismatch/failure. d33-plan-inventory-parity.json/repeat byte-identical61628B.
D33prefixregression all48matches and report byte-identical to originalD29planparity,
confirming privatehelper refactor preserves existing output. APIstagefailures tested
synthetically; localcohort includes no deeperfailure/protectedresource experiment.
No new nativeorvisualverification claimed.

SFO/MAP hashes unchanged, FARC1023990298B ReadOnly/Archive, pinreferenceclean,
privategame/reports ignored, authoredRust/docs allowlisted and rootGit absent.
No original writes, publishing or license decision.

## Next D34

Native optional Plan inventory-prefix mode, boundedexisting sourceworker and combined
inspection; preserve/cache/display base when deeperstage fails. Separate outer and
inventoryerrors, requested/notrequested outcomes, reset/cancel/candidates and raw
lossless labels. QAalias for inventorymode and cachedoracle field comparisons;
synthetic lifecycle/layout/errorcases and native captures. Existingversion1 export
continues prefixonly; version2 stage-aware export/schema follows separately so old
JSONcontract is not silently redefined. No recursive refs/date/title semantics.
