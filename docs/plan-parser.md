# D29 independent CPU plan-prefix reader

2026-10-06 America/Chicago. `lbp-resource::plan::inspect` independently implements
the bounded diagnostic slice established by [D28 external oracle](plan-prefix.md).
No external reader source copied/translated, no new dependency, GUI or Thing engine.
Exact pinned URLs/classes/evidence/build applicability/conflicts remain documented
there and in the module comment adjacent to implementation.

## Contract

Input immutable ordinary PLNb resource bytes; output `PlanInspection` includes flags,
expanded byte count and `PlanPrefix`: revision, raw unsigned inner revision, opaque
Thing array length/start/SHA256, parsedprefix length, unparsed bytes including blob
and inventorytail, inventorytail start/length. No original array bytes escape API.
No proof of valid inventory, Thing count, object parts, shape or complete compatibility.
Empty opaque array/zero tail are byte extents, not empty or valid game objects.

Head893..951/branch0/sub0; shared ordinary binary wrapper restricts flags0/7 and
validates metadata/dependency tables without treating nonempty dependencies as
unsupported. Stored2MiB, expanded16MiB. Protected markers explicit Blocked; unknown
magic/revision/branch/flags Unsupported, malformed extents Invalid, budget/cancel
explicit. Existing shared BE32/ULEB32 integer decoder reused unchanged. Inner
revision may differ from outer; raw0xffffffff retained in tests. Streaming and older
plan variants remain unsupported rather than guessed.

After checked integer length and checked start+length, API hashes bounded opaque
slice in <=4096byte chunks with cancellation between chunks and before publish.
Digest describes opaque bytes only, not inventorytail/header. Parsedprefix ends after
array length; unparsed count includes all array bytes even though digest was computed.
Inventorytail not consumed/validated. Payload wrapper validates dependency metadata;
this prefix does not return a competing dependency inventory. Disk/IR/UI separation
preserved: CPU module has no filesystem, GUI or rendering dependencies.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-resource --test plan
cargo run -p lbp-catalog --example plan_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d28-plan-oracle.json local-analysis/new-plan-parity.json
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All pass: four authored plan tests,130workspace/43viewer tests, fmt/Clippy/build.
Synthetic recipes cover heads893/919/951, flags0/7, raw/zlib/zerochunk/storedchunk,
rawinner mismatch/u32max, empty/nonempty opaque arrays and tails, expectedhash,
immutableinput, duplicate dependency entries, every prefix/blob/wrapper truncation,
old/new boundaries, branch/sub/unsupportedflags, protectedshort/long, ULEBoverflow,
array/stored/expandedbudget and cancellation before processing/during hash.
No proprietary fixture bytes used. No new shared integer/wrapper changes.

`lbp-catalog/examples/plan_parity.rs` uses validated read-only source attachment,
4096byte pages,48samples/64MiB total, exact pin/scope/schema and uniquehash-location
requirement. Absent/ambiguous candidates rejected, not silently selected. Private
exclusive JSON only. Source stability checked after run. This metadata comparison
is not a whole-image authenticity check or full payload compatibility proof.

Local48prefix records across28modern heads exactly match D28 oracle on all fields,
including opaque SHA256 and tail extents;313643resource bytes read/run. Zero mismatch
or failure. `d29-plan-parity.json` and repeat byte-identical30744B. SFO/MAP hashes
unchanged, FARC1023990298B ReadOnly/Archive, pinned reference clean, no rootGit or
publication. Authored Rust/docs allowlisted; reports and game/reference data ignored.
No nativeplan UI or visual check claimed this milestone.

## Next D30

Native Plan prefix tab for MAP/entry candidates through validated source, bounded
cancellable worker/cache. Preflight known header/revision/branch before body reads;
raw inner revision, opaque array hash/bounds/tail/unparsed diagnostics. Preserve
alias/conflictingcandidates, route/source/snapshot reset and explicit error states.
Synthetic lifecycle/layout tests plus native supported/unsupported comparisons with
pinned cached oracle. No Thing traversal, inventory semantics or plan edits.
