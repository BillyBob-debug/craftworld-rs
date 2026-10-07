# D30 native Plan prefix inspector

2026-10-06 America/Chicago. Plan tab available on MAP resources and archive entries.
Uses [D29 verified CPU reader](plan-parser.md), no new format rules or dependencies.

## Behavior

Attach/validate source, select Plan and choose resource candidate. Every location
remains listed; no silent alias/conflict resolution. Worker preflight reads<=32B,
checks clear PLNb, header18B, head893..951, branch/sub0, flags0/7, compressionbool0/1,
protectedmarkers and stored2MiBbudget before body reads. Body pages<=4096B through
AttachedSource::read_hex_checked. Short reads fail. Existing parser expands<=16MiB
and checks array bounds/hash cancellation. No protected decode or Thing traversal.

Single worker/cache per view. Reload drops/cancels earlier job and clears old result;
Cancel suppresses queued success. Worker failure/disconnect yields expliciterror.
Sourcepointer change/detach, route transition and successful snapshot replacement
reset candidate/job/key/cache. Plan tab retained for supported entry routes and
history. Read-only sourceattachment and navigation architecture otherwise unchanged.

Scrollable raw metadata: outer/innerrevision, flags, expandedbytes, opaqueThing
length/start/SHA256, parsedprefix and unparsed counts, inventorytail start/length.
No title/icon/Thingcount/part semantics or empty/validinventory assertion. Inner/outer
mismatch does not establish corruption. Digest wraps when narrower than64glyphs.
No plan JSON export in this milestone; private capture logs optionally project cached
metadata as one QA line without another source read. Ordinary logs only head/rawinner/
opaque byte count. No new parser behavior or external source translation.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-viewer plan
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00007c0b --qa-tab plan --qa-capture local-analysis/new-plan.png
```

All pass:134workspace/47viewer tests. Four authored viewer cases cover earlypreflight,
4096paging/cancel/shortreads, cancelled/failed/disconnectedresults, job cancellation/
cache/candidate reset on route/snapshot replacement, Planentrytabretention, rawu32max/
zeroThing/unknowninventorytail and detachedlayout. Existing minimum-size all-tabs
layout includes Plan and passes. Source-detach reset follows same pointer-reset path;
no manual detach/candidate/Reload/Cancel or allGPU testing claimed.

Four hidden autoexit native runs, all exit0, RTX3080/Vulkan. All four PNGs inspected:
- `d30-plan-small`: GUID00007c0b, SHA1bfb2c52fa35e66c1c30a1f7e93464ae82105900;
  head926, expanded183B, opaque99B, tail81B.
- `d30-plan-large`: GUID0002a641, SHA560ba92ca1e3492158a5bcd4423f8f8338cb0051;
  head950, expanded125771B, opaque125680B, tail86B.
- `d30-plan-entry`: archive0:entry10625, same large resource, stored41510B
  offset220196959; same complete cached metadata.
- `d30-plan-old`: GUID00012660, head622; visibleUnsupported/no cachedmetadata.

Three supported QA JSONlogobjects exactly equal D28 oracle compressionflags,
expandedbytecount and everyprefixfield/hash. Oldcapture has no QA cachedprefixline.
All output PNG/logs ignored. Sourcebinding validated31534entries/4704862metadataB.
This verifies rendering of diagnostic metadata, not Thing/planvisual correctness,
manual scrolling/buttons or complete archive/corpus support.

SFO/MAP hashes unchanged, FARC1023990298B ReadOnly/Archive; pinnedreferenceclean and
no rootGit/commit/publication. Authored Rust/docs allowed, privateassets/artifacts
ignored. No current blocker.

## Next D31

Cached Plan metadata-only JSON exporter via shared privatewriter: explicitprojection,
version1schema/outcomes, busy/incomplete/resetrefusals, original bytesexcluded and
no newsource reads. Native supported/unsupported oracle/schema/repeatverification,
authored tests. Inventory/Thing/body parsing remains deferred.
