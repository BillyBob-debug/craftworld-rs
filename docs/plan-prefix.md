# D28 bounded external plan-prefix oracle

2026-10-06 America/Chicago. Authored experiment invokes unchanged pinned community
reader primitives and ordinary clear wrapper constructor; no translated source,
Thing traversal, full RPlan/RLevel decode, inventory edits, visual preview or export
of resource bytes. This supplies evidence for an independent CPU plan diagnostic.

## Provenance and applicability

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`, ignored clean local source-built
cwlib-0.1.jar. Confidence: high for observed prefix bounds/reader primitive agreement;
partial for complete RPlan serialization because full resource deserializer not run.
Applies to sampled BCUS98245 01.00 DATA.FARC plans, heads919..951/branch0/flags7;
proposed support envelope893..951/branch0/flags0or7 also has synthetic coverage.
No sample for every possible payload/layout at those revisions.

| Format rule | Evidence category / source | Test coverage and limits |
|---|---|---|
| RPlan starts optional streaming bool at subVersion>=0xcc; inner revision; bytearray | CONFIRMED_FROM_COMMUNITY_CODE: [RPlan::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RPlan.java), [Revisions.STREAMING_PLAN](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/Revisions.java). Branch0 selected so no streaming bool | 48 local prefixes; streaming/other branches unsupported; field ordering derived from source, full RPlan not invoked |
| Inner revision uses flag-controlled i32, interpreted raw unsigned for diagnostics | CONFIRMED_FROM_COMMUNITY_CODE: [MemoryInputStream::i32](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java); RPlan ignores stored value and uses outer revision | FixedBE32 and ULEB recipes, raw0xffffffff; 48local stored revisions equal outer, not a universal validity requirement |
| bytearray length uses i32 and then exactly size bytes | CONFIRMED_FROM_COMMUNITY_CODE: [Serializer::bytearray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java), MemoryInputStream::bytearray/bytes | Unchanged bytearray called only after strict count/extent guard; local data hashes/offsets, synthetic truncation/overflow/budget |
| Plan inventory details follow opaque thingData for head>=0x197/nonstreaming | CONFIRMED_FROM_COMMUNITY_CODE: RPlan::serialize; [InventoryItemDetails::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/inventory/InventoryItemDetails.java) | Tail length only, contents not read/decoded; no claimed inventory validity |
| Clear binary dependency table uses BE32 count, flags, optionalGUID/SHA1, BE32 type | CONFIRMED_FROM_COMMUNITY_CODE: [SerializedResource::processDependencies](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/types/SerializedResource.java); prior resource-metadata.md | Every extent checked; local count2..77; synthetic duplicateentries retain rawcount2 while reader HashSet reports1. Do not use deduplicated reader count as inventory count |
| Binary wrapper/decompression matches pinned getStream | CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE: SerializedResource.process and prior strict authored TextureOracle.clearPayload guard | All48 strict expanded buffers equal reader buffer; stored/raw/zerochunk synthetic wrappers; bounded zlib exercised by local corpus |

No conflicts in 48sample prefix outcomes. Scope difference: community stream accepts
arbitrary compression bit combinations and unbounded ULEB; experiment accepts only
flags0/7 and <=5byte unsigned32 with valid high nibble. This is a diagnostic bound,
not evidence that every other game layout is corrupt. Branch/streaming/older layouts
remain explicitly excluded. No strengthened inner-revision-equality requirement.

## Guard sequence and outputs

`scripts/PlanOracle.java` and `run-plan-oracle.ps1` are authored adapters. Runner
checks pin/clean source and compiles only authored adapters against ignored jar.
Original FARC opened RandomAccessFile r. Manifest48samples max, source path exact,
offset/size bounds, 2MiB/sample and64MiB/run, resource SHA1 checked before constructor.
PLNb only; modern revision893..951, branch/sub0, flags0/7, compressionbool0/1.
Dependency count<=4096, recognized flagbits and every optional field extent checked;
dependency table must end exactly at stored resource end. Trailing ALSR/other data is
outside this experiment. Constructor sees neither encryption nor unbounded counts.

Strict wrapper preflight sums expanded chunk sizes before inflation (16MiB), then
shared strict helper rejects short/stalled/overlong/trailing compressed chunks;
clear buffer must equal pinned reader. Guarded prefix integer reads compare offsets;
opaque bytearray is read by unchanged MemoryInputStream only after its length and
extent are bounded. SHA256 computed from that opaque data but bytes are never output.
No recursive object deserializer invoked. Xmx256m heap. 60second deadline and thread
interruption checked between sample dispatches; this is not a hard preemption of an
individual library call. All calls have bounded input/allocation; console termination
is process-level cancellation, not a UI cancellation guarantee.

Version1 private report includes outerhead/flags/expandedbytes, raw/readerunique
dependency counts; prefix raw inner revision, thingData bytes/start/hash, parsedprefix
bytes, unparsed bytes including whole blob+tail, inventorytail start/length. The term
inventorytail describes byte location, not parsed field content or valid inventory.
Unsupported/malformed experiments retain rejected status/reason, never inventedzero.
Exclusive new ignored JSON only; no absolute source paths or original byte dumps.

## Reproduction and results

```powershell
./scripts/run-plan-oracle.ps1 -Output local-analysis/new-plan-oracle.json
```

Three runs (d28-plan-oracle.json, -repeat.json, -final.json), each48samples /
313643archivedB, all parsed prefixes, zero rejected, report30321B byte-identical.
Sample48prefixes span28heads919..951. Inner revision equals outer in all48.
Expanded183..125771B; opaque Thing99..125680B; data start3..5; inventorytail51..193B;
raw/readerunique dependencies2..77 and equal for all local samples (no sample duplicate
collapse observed). These are sample ranges, not global format limits.

Authored Java selftests pass before each corpus run: flags0/7, raw/zerochunk wrappers,
raw0xffffffff inner value, exact array byte count/tail, duplicate dependency2->1,
every wrapper truncation, encrypted marker, branch, lower/upperhead boundaries,
unsupported flags, dependencycount/flag/trailer rejection, array budget, integer
high-nibble overflow and declared inflation budget rejected before decompression.
Existing helper covers strict zlib mechanisms; no full resource/body compatibility
claim. Initial selftest failed because default MemoryInputStream.seek is relative;
corrected adapter to SeekMode.Begin before rerunning. No original data involved in
that failed synthetic test. No Rust changes or new Rust-check claim this milestone.

SFO/MAP hashes unchanged, FARC1023990298B ReadOnly/Archive; referenceclean/pinunchanged;
private corpus outputs, classes/jar/manifests remain ignored. No Git init/commit/publish.
Default-deny explicitly allowlists only authored PlanOracle.java in addition to existing
source/scripts/docs. No proprietary fixtures or byte samples written.

## Next D29

Implement independent CPU `lbp-resource::plan` prefix diagnostics using existing
binary wrapper and checked integer decoder. Preserve raw inner revision, opaque
array start/length/SHA256, parsedprefix/unparsed/tail lengths; no returning blob bytes.
Head893..951/branch0/flags0or7; do not require inner revision==outer. Bound stored2MiB /
expanded16MiB, cancellation, arithmetic and actual array extent; unknown branches and
protected magic stop. Synthetic malformed/flags/revisions/unknownbits/truncation tests;
read-only parity example against48pinned oracle records and deterministic repeat.
Then native plan inspector/export as a separate reviewed milestone. Inventory fields
and recursive level Things remain UNKNOWN/unparsed until separately established.
