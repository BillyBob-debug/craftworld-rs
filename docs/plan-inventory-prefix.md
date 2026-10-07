# D32 modern plan inventory scalar prefix

2026-10-06 America/Chicago. Authored external-reader experiment establishes a bounded
inventory metadata slice following opaque Thing bytes. No independent Rust reader
or native UI for this stage yet; existing D31 prefix/export remains unchanged.

## Pinned sources and field sequence

Toolkit commit `d6a2ef80b5a09999f08f23a803b516096055e5c9`, source-built ignored cwlib
jar and clean reference checkout. Local owned BCUS98245 01.00 DATA.FARC sample scope:
48plans,28modern heads919..951,branch0/sub0,flags7. Proposed diagnostic envelope
heads893..951/flags0or7 has synthetic coverage. Older branches/streaming unknown.

CONFIRMED_FROM_COMMUNITY_CODE:
- [RPlan::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RPlan.java): inventory follows bounded opaque
  thingData and is gated by PLAN_DETAILS/nonstreaming.
- [InventoryItemDetails::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/inventory/InventoryItemDetails.java),
  strict head>0x37c: dateAdded(s64), SlotID struct, highlightSound GUID, colour(i32),
  rawtypeflags(i32), subtype(i32), titlekey(u32), descriptionkey(u32), then first
  recursive CreationHistory reference. Experiment stops before that reference.
- [SlotID::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/slot/SlotID.java): enum32 slot type then u32 number.
  Actual Serializer.struct + SlotID used after bounds preflight; no object graph.
- [MemoryInputStream](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java): fixed BE32/BE64 without
  compactflag; ULEB when flag1 set. GUID uses u32 then zero->null. Our raw metadata
  retains zero/high bits, not fabricated ResourceDescriptor or external dependency.
- [InventoryObjectType::fromFlags](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/InventoryObjectType.java) converts flags
  to revision-conditioned enumset. Diagnostic retains complete raw flags instead
  of silently losing unknown bits. SlotID uses stream.enum32 (unknown=null); generic
  SlotType.fromValue fallback to DEVELOPER is not this read path. Preserve raw enum.

Sourcefield-order confidence high, but full InventoryItemDetails/RPlan deserialize
not invoked. Primitive agreement and SlotID class agreement do not prove recursive
inventory/body compatibility, title translation, timestamp units or slot semantics.

## Signed64 reader conflict

CONFIRMED_FROM_COMMUNITY_CODE / authored synthetic reproduction:
MemoryInputStream.s64 compact path uses arithmetic `v >> 1`, not logical shift for
unsigned ZigZag. MemoryOutputStream.s64 writes standard `(value<<1)^(value>>63)`.
For encoded highbit values, reader sign extension disagrees with standard inverse:
encoded ffffffffffffffff gives reader0 versus standard i64MIN; encoded fffffffffffffffe
gives reader-1 versus standard i64MAX. No game resource in this sample set exhibits
this conflict: all dates encoded0. Do not infer real corpus occurrence or emulate
unproven game behavior from a tool edge bug.

Future diagnostic IR will retain `date_added_encoded_u64_hex`: exact decoded unsigned
wire integer in16lowercasehex digits (fixedBE64 for flags0; ULEB64 for flags7). No
normalization to timestamp or lossy JSONnumber, no requirement to match buggy signed
reader interpretation. Oracle sidecars `reader_date_i64_decimal` and
`standard_date_i64_decimal` record comparison independently of raw metadata; date
status explicitly flags conflict. Other uint32 fields remain raw numbers.

## Bounds and metadata contract

Authored adapters `scripts/PlanInventoryOracle.java` and runner reuse PlanOracle
ordinarywrapper/input/dependency/identifier guards from [plan-prefix.md](plan-prefix.md).
2MiBstored/sample,16MiBexpanded,48samples/64MiBrun,60sdispatch deadline/Xmx256m.
FARC read mode only. Plan stage fully validated before second bounded wrapper
constructor; redundant constructor does not reread source. No constructor can reach
protected decoding or extra trailer. No resource resolution/full Thing classes used.

Date preflight<=10ULEBbytes, finalpayloadbyte<=1; fixed64 exactly8bytes. Every remaining
uint32 has existing strict<=5byte guard. Community u64/s64/i32 calls must consume
exact guarded offsets. SlotID actual class reads only two already bounded integers;
raw unknownenum retained even if classenum null. Tail extent from D28 prefix used;
no substring scanning or guessed offsets. Stop before CreationHistory tag, including
when authored tail tag is deliberately unknown. No future content is peeked/parsed.
Cancellation/interruption checked between sample dispatches; deadline not hard
preemption of individual library calls. Existing bounded-call limits remain.

Inventory metadata fields: rawdateencodedhex, slottype/number, soundGUID, colour,
typeflags/subtype, title/descriptionkey; inventorystart, consumedprefixbytes, end,
remainingunparsedbytes. Wholeplanprefix remains separate in oracle output. Byte
extent is not an objectcount. No opaque Thing/inventorybytes written to report.

## Verification and local observations

```powershell
./scripts/run-plan-inventory-oracle.ps1 -Output local-analysis/new-plan-inventory-oracle.json
```

Both runs pass authored synthetic guards and48local probes,313643originalresourceB
read/run, zero rejected. Two70846B reports byte-identical. Every retained planprefix
object equals corresponding earlierD28 report; optional experiment does not change
known metadata. Source fingerprints unchanged, FARCReadOnly, referenceclean/pin,
rootGitabsent and private output/classes ignored. No new Rust changes/check claim;
D31's136tests/fmt/Clippy/build remain last verified Rust state.

CONFIRMED_FROM_LOCAL_DATA (sample observations only): all48dateencoded0, slottype0,
slotnumber0, soundGUID0. All SlotID classfields agree and dateinterpretation agrees.
Typeflags:1/2/8/16/32/256/512/16384/32768; subtype0/4096/0x80000000. Colour/title/
descriptionkeys vary but no translation/categorization experiment performed.
Consumed inventoryprefix13..24bytes; remainingtail27..180bytes. Those remaining
bytes still include recursive metadata/other scalar records and are unparsed.

Synthetic cases: flags0/7; encodeddate0/1/2/longmax/highbit/u64max-1/u64max; raw
unknownslottype, u32maxslotnumber/typeflags, highbitscolour/subtype, zeroGUID/key,
actualSlotID behavior, strictdate/prefixoffsets, everyprefixtruncation and ULEB64
finalbyteoverflow. Unknown trailing recursive tag deliberately untouched. All pass,
including explicit knownsigned64conflict. No malformed input accepted to preserve
oracle agreement. No failed attempt or new blocker in this milestone.

UNKNOWN: nonzero local date/slot/sound values and their units/relationships, legacy
inventory variants, recursivecreationhistory/icon/user/photo/creator records,
validbody semantics, translatedtitle and complete parser fidelity. Source rules and
synthetic values support boundedrawfield extraction, not those meanings.

## Next D33

Add independent optional CPU planinventory-prefix stage, preserving current PlanPrefix
API/schema. Decode boundedrawULEB64/BE64 to losslesshex, existinguint32 decoder,
rawslot/sound/colour/type/subtype/keys and prefixend/unparsedcounts. No signeddate
normalization/enumloss/reference traversal. Reuse private expandedpayload/prefix
helper so standalone combined inspection need not inflate/hash twice; do not make
opaque bytes public. Preserve prefixdiagnostics when deeper stage fails.

Meaningful authored gates/truncation/overflow/cancel tests and48exactlocal oracle
matches for the new metadata, existing48planprefix regression/repeat, fmt/tests/Clippy.
Nativeoptionalinventory mode and version2cachedplanexport follow in separate milestone
only afterCPU parity. Avoid overwriting existingversion1 report contract silently.
