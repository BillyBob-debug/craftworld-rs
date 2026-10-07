# D31 cached Plan diagnostic JSON

2026-10-06 America/Chicago. Native Plan inspector now saves completed cached
metadata to a new `.json` under ignored `local-analysis/`. UI Save and QA capture
invoke the same private production writer. No source rereads, parser invocation,
Thing bytes, inventory payload, edits or semantic interpretation.

## Contract and provenance

Version1 envelope in `schemas/plan-diagnostic.schema.json`. Explicit projection in
`app/plan/export.rs`: compressionflags/expandedbytes and all existing PlanPrefix
raw innerrevision, opaqueThingarray hash/bounds, parsedprefix/unparsed/tail counts.
Existing layout evidence is unchanged: [plan-prefix.md](plan-prefix.md) lists pinned
source URLs/classes/rules/conflicts/local applicability, [plan-parser.md](plan-parser.md)
defines independent CPU IR, [plan-inspector.md](plan-inspector.md) defines lifecycle.
No external implementation was copied/adapted/translated.

Envelope reports source-relative normalized location and stable outcome/error
categories. `source_state=validated_at_read_not_rechecked_for_export` explicitly
identifies cached results rather than a fresh source-integrity verification. No
host paths, raw error messages, timings or generationtimestamp in deterministicbody.
Unsupported/blocked/cancelled/failed completed inspections have null metadata.
Busy/unselected/incomplete/reset inspections refused. Reload clears messages;
route/source/snapshot reset clears export cache/state. Digest is opaque bytes only,
not a hash of inventory fields, resource wrapper or entire original image.

Existing shared `app/diagnostics.rs` enforces ordinary workspace local-analysis,
contained existing parent, JSON extension, new exclusive file and <=8MiB serialized
budget before opening destination. No filesystem reads of original source during
projection/save. Partial output on I/O failure may remain and cannot be overwritten;
ancestor races are not fully covered. No new writer/security/format behavior.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-viewer plan
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 0002a641 --qa-tab plan --qa-capture local-analysis/new-plan.png --qa-plan-json local-analysis/new-plan.json
```

All pass:136workspace/49viewer tests. Two new authored tests cover explicitprojection,
rawu32max/zeroopaquebytes/unparsedtail/hash, excluded payloadfields, no source handle,
unrelated candidate/QA state invariance, normalizedlocation, busy/incomplete/reset
refusals/cancellation and sanitized stable failure categories. Existing shared writer
private-path/new-only/budget tests pass. No proprietary fixture data committed.

Four hidden auto-exit native runs, exit0, RTX3080/Vulkan:
- d31-plan-small, GUID00007c0b/head926: report964B.
- d31-plan-large, GUID0002a641/head950: report979B.
- d31-plan-old, GUID00012660/head622: unsupported/null, report520B.
- d31-plan-large-repeat: byte-identical979B to first large report.

Three supported complete metadata objects equal D28 pinned oracle on every field.
All four validate with Draft202012Validator via ignored jsonschema4.23.0; schema
self-check passes. Five invalid mutations rejected: extra Thing payload, u32overflow,
invaliddigest, unsupportedstatus with metadata, version2. Inspected largePNG: export
section and saved confirmation visible. Native captures exercise production saver,
not manual editing/clicking/scrolling/filedialog; no new payload or visualcompatibility
claim. Every JSON/PNG/log remains ignored local diagnostic output.

SFO/MAP fingerprints unchanged; FARC1023990298B retains ReadOnly/Archive; pinned
reference clean, rootGit absent. Authored code/schema/docs allowed, privateoutputs/
originals/references ignored. No new dependencies, original writes or publishing.

## Next bounded task

D32: study modern InventoryItemDetails scalar prefix and SlotID/serialized integer
rules in pinned community reader, then bounded external oracle over established plan
samples. Candidate slice: raw date/slot/soundGUID/colour/type/subtype/title/description
keys, stopping before first recursive CreationHistory reference. Establish signed64/
compact gates and raw enum/type preservation before implementing. Do not interpret
names/title text, load references, recurse Things or guess inventory fields. Prefer
actual bounded reader classes/primitives and authored recipes; document unsupported
variants and exact coverage. Then independent optional metadata reader if evidence
supports it, preserving the existing prefix on deeper inspection failure.


## Superseded producer: D35 version 2

The native producer now writes [version2 stage-aware reports](plan-export-v2.md).
This document and `schemas/plan-diagnostic.schema.json` retain the historical
version1 base-only contract; existing reports remain valid. No silent field additions
to version1. Optional inventory metadata is included only under schema_version2.
