# D23 animation prefix evidence

2026-10-06 America/Chicago. Snapshot-only census and external-reader oracle, no
product animation parser/UI/playback yet. Original sources remain read-only.

CONFIRMED_FROM_LOCAL_DATA:854 .anim MAP aliases/748unique hashes; separately748
confirmed ANMb locations,2,785,763 stored bytes, largest41,320. All are branch0,
integer flags7, zero dependencies. Prepared DATA.FARC scope, not whole-disc coverage.

| Revision | Locations |
|---|---:|
| 844 | 5 |
| 890 | 1 |
| 891 | 637 |
| 892 | 1 |
| 895 | 27 |
| 902 | 16 |
| 903 | 3 |
| 905 | 4 |
| 906 | 1 |
| 907 | 1 |
| 911 | 1 |
| 913 | 18 |
| 914 | 9 |
| 920 | 2 |
| 926 | 7 |
| 927 | 6 |
| 928 | 1 |
| 929 | 3 |
| 935 | 2 |
| 940 | 1 |
| 946 | 2 |

## Rules, source and limits

Community links pin Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, clean ignored
MIT checkout. Reader invoked externally; no animation implementation translated.

CONFIRMED_FROM_COMMUNITY_CODE: [RAnimation::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RAnimation.java).
Below0x378(888), bones precede32-bit frame/fps/morph fields and track/vector arrays
use an older layout; old locator code has TODO. Five local head844 resources;
the two selected old samples remain explicitly unsupported by this modern oracle.

For head>=888, the prefix is three16-bit words(frame/fps/loop), morph byte, five
unsigned byte counts(bones, rotation/position/scale/morph tracks),16-bit locator-key
count. Head>0x38b(907) adds position offset/scale Vector4; head>0x3b1(945) adds fat
byte. Then serialized animation-byte-array length, opaque bytes and locator array.
Raw16/8-bit values and IEEE32 bits are preserved; absent optional fields null.
Community frame/fps/loop are signed shorts and morph signed byte: do not invent
semantic signedness/ranges from a raw diagnostic. Fixed BE16/byte/float and packed
length behavior: [MemoryInputStream](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java).
Wrapper evidence remains [resource metadata](resource-metadata.md).

Locator-key count differs from locator-array count. [Locator::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/animation/Locator.java)
contains position/name/loop/type, discovery only. No locator records/strings parsed.
Fat meaning UNKNOWN: source comment guesses and read path ignores alternate widths;
two sampled head946 resources contain raw1. Playback, keyframes, bone hierarchy/hash
semantics, coordinate/scaling rules and mesh attachment remain unverified/UNKNOWN.

## Oracle and checks

Authored animation_report.py selects minimum/maximum stored size per header cohort
from existing snapshot, never assets.35samples cover21cohorts. AnimationOracle.java
uses pinned stream readers and checks wrapper expansion against SerializedResource.
Caps48samples,2MiB stored/sample,64MiB stored/run,16MiB expanded/sample,256MiB JVM,
60s dispatch deadline. SHA1 verified before library entry; clear ANMb/branch0/heads
888..951/flags0or7/zero dependencies only. FARC opened r, output exclusive/private.

Each run reads205,629 archived bytes:33modern prefixes across20revisions accepted,
two head844 samples rejected. Observed frames5..2000/fps15or30 are sample facts,
not universal bounds. No keyframe/asset arrays exported. Full unmodified RAnimation
invoked only after allocation/packed-minimum-byte preflight and empty locator array:
23samples exactly corroborate scalar/track counts, optional position bits and fat.
Ten prefixes have1..6 locators and full reader is not invoked. No reader failures.
Successful invocation is not animation correctness or full-payload compatibility.

```powershell
python -m unittest discover -s tests -p test_animation_report.py -v
python scripts/animation_report.py local-analysis/a2-inventory-verified.json local-analysis/NEW-animation-census.json local-analysis/NEW-animation-samples.json
./scripts/run-animation-oracle.ps1 -SampleManifest local-analysis/NEW-animation-samples.json -Output local-analysis/NEW-animation-oracle.json
```

Use bundled Python if absent from PATH. Three Python tests pass: alias/magic
distinction, deterministic selection/order, budget accounting/caps/private output.
Java synthetics pass heads888/907/908/945/946/951, integer flags0/7, all truncated
prefixes and actual full-reader checks. First sandbox canonical resolution failed
WinError5; scoped authorized execution resolved it without weakening guards.
Census/manifest/guarded oracle repeats byte-identical; optional-field corroboration
adds no mismatches. Product Rust/UI unchanged:115Rust tests last runD22, not rerun
here. No native QA needed. Original hashes/FARC ReadOnly/size unchanged, pin clean,
default-deny audit intact, no root Git or publishing. No approval rejection.

Private evidence: d23-animation-census/samples/repeat; animation-oracle/final/fields
JSON under local-analysis. Authored adapters, recipes, tests and docs trackable;
assets, reports and generated classes remain ignored.

Next D24: CPU-only modern prefix metadata under lbp-resource using existing checked
wrapper/decompression/integer helpers. Match all33prefixes, preserve raw fields,
optional bits/nulls, opaque byte length/start/unparsed counts. Head844 unsupported;
no keyframe/locator allocation. Synthetic revision/truncation/budget/cancel checks
and bounded local parity before a later native Animation inspector.

D24 now implements this modern prefix independently and matches all33samples;
old layouts remain explicit Unsupported. [Parser contract](animation-parser.md).
