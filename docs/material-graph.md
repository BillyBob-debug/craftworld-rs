# D7 material graph metadata evidence — 2026-10-05

Current result: D8 independent optional graph metadata reader now matches all32
canonical samples. D7 external evidence below remains the source/applicability
record; D8 implementation and verification are appended.

Bounded graph reconnaissance and an external oracle are complete. No Rust graph
reader, graph evaluation, shader execution or textured rendering was added. The
existing independent material prefix and Material UI are unchanged. Shader/code
bytes remain opaque and absent from output. All diagnostic samples stay ignored.

## Per-rule provenance and applicability

Reader pin: Toolkit `d6a2ef80b5a09999f08f23a803b516096055e5c9`. This is external
reader use and independent format evidence, not copied/adapted/translated product
source. Toolkit MIT and inherited notices remain in references; own-source license
is undecided. Guarded adapter source contains no recovered proprietary code.

| Claim | Evidence and source | Scope, conflict and coverage |
| --- | --- | --- |
| Boxes then wires follow prefix wrap arrays, each with integer count and inline structures | CONFIRMED_FROM_COMMUNITY_CODE: [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java), [Serializer::arraylist/struct](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java) | Ordinary branch-zero PS3 heads920–951 only. Graph counts guarded before allocation; no reference-pointer arrays or new branches guessed. |
| Box: type integer, declared parameter count, eight parameter integers, four fixed-width floats x/y/w/h, subtype integer, two embedded animations | CONFIRMED_FROM_COMMUNITY_CODE: [MaterialBox::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java) | All accepted heads are newer than0x2a4; legacy six-parameter/one-animation layouts excluded. Parameters preserve raw u32 bit patterns, not automatically floats. Position preserves IEEE bits. Toolkit ignores declared parameter count; guard accepts8only, other counts unsupported pending evidence. Actual139boxes all declare8. |
| Texture-sample type1 uses parameter5 as texture-slot index | CONFIRMED_FROM_COMMUNITY_CODE: MaterialBox::TEXTURE_SAMPLE_INDEX and constructors; [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java) | Actual55sample boxes reference slots0–4within eight prefix descriptors. This is a slot link, not a diffuse/normal designation. BoxType is explicitly incomplete; raw unknown kinds retained. Subtype1changes texture-transform interpretation; do not assume every sampler uses a simple scale/offset. No transform/evaluation performed. |
| Embedded animation: base vector4, component float-array count/data, byte-name count/data, component mask | CONFIRMED_FROM_COMMUNITY_CODE: [MaterialParameterAnimation::serialize/getNumberOfComponentsAnimated](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialParameterAnimation.java) | Float-array values stored component-major; derived key count=count/popcount(mask&15), only when count>0. Guard rejects nonzero count with zero components/nondivisible count before reader invocation. Raw name bytes remain local diagnostic hex, max64; actual all3bytes. Keys never exported, only counts/digest; no playback. |
| Wire: signed indices from/to, raw source/target port bytes, five swizzle bytes | CONFIRMED_FROM_COMMUNITY_CODE: [MaterialWire::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialWire.java) and [MemoryInputStream::s32/i32](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java) | Packed signed indices use zigzag; fixed mode signed BE32. Synthetic negative endpoint retained as out-of-range metadata, not automatically called corruption. All118actual wires fit their box arrays. Extreme signed oracle bug below. |
| Output port names0diffuse,1opacity,2specular,3bump,4self-illumination,6reflect | CONFIRMED_FROM_COMMUNITY_CODE: [BrdfPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BrdfPort.java) | Names are community semantic annotations, not local rendering proof. Apply to output-node port domain only; arbitrary other node ports are not globally BRDF ports. Raw values retained; shader meaning, blending and original visual correctness UNKNOWN. |
| Sound enum and parameter-animation array follow graph | CONFIRMED_FROM_COMMUNITY_CODE: RGfxMaterial::serialize / Revisions thresholds | Not parsed by D7; residual bytes retained as UNKNOWN locally. No animation resource/gameplay scope implied. |

## Local observations

`CONFIRMED_FROM_LOCAL_DATA`:32previously selected clear GMTs,232,687archived bytes,
same16heads920,928,931,932,933,935,936,937,940,941,944,945,946,947,949,950, flags7,
branch0, BCUS98245 APP_VER01.00.32/32accepted by guarded external stream/box/wire
readers. Every preceding prefix remains exactly equal to D4 evidence. This is
external-reader corroboration/consistency, **not independent Rust graph parity**.

- 139 boxes, 2–11 per material; 118 wires, 1–14 per material.
- Raw box kinds:0=32,1=55,3=24,10=22,12=4,2=2. Community names: output,
  texture sample, color, multiply, mix, thing color respectively. Do not infer
  support for all other raw kinds from those six observed kinds.
- Subtypes: 138 zero, 1 one. 55 raw slot links:0=29,1=15,2=7,3=3,4=1.
  Slot-link totals and56prefix references are different measurements; duplicate
  sampling and unused descriptors must not be flattened or guessed from totals.
- 278 embedded animations: 276 zero-value arrays; 2 arrays of 241 floats, masks4/8,
  derived241keys each. All names3bytes. Oracle compares base IEEE bits, name,
  mask/key count and canonical component-array digest against actual reader keys.
- Wire port pairs(source:target):0:0=58,0:1=28,0:2=15,0:3=9,0:4=4,0:6=4.
  All sampled slot links and wire endpoints fit bounds. No graph traversal,
  acyclicity or topology semantics were inferred.
- 30 materials leave 2 bytes, 2 leave 989 bytes after graph. These tails remain unparsed;
  they are not empty and are not automatically corrupt.

## Oracle conflicts and guard budgets

Toolkit s32casts packed unsigned value to signed int then arithmetic-shifts right.
Synthetic zigzag0xffffffffreturns0, whereas unsigned-shift zigzag arithmetic gives
Integer.MIN_VALUE. The harness asserts/reports that known oracle conflict; no
reference source was patched. Future independent reader must use correct unsigned
shift/signed reconstruction and retain raw encoded value for diagnostics if useful.
Actual sampled endpoints0–10and synthetic-1fit the unaffected range. Do not generalize
oracle agreement to extreme signed values. Wire negative/sentinel meaning is UNKNOWN.

Parameter counts other than8are not answered by the reader's permissive ignored
count. The adapter rejects them before MaterialBox deserialization. A nonzero
animation array with zero component mask would cause division-by-zero in the reader;
the guard rejects it first. Padding at a short fixed-width read is rejected by final
offset checks. No unexpected disagreement/failure occurred on these32samples.

Adapter first bounds-walks each box/wire, then invokes the unmodified reader on an
isolated bounded structure. It compares type/subtype, every parameter, position bits,
animation metadata/digest, wire endpoints/ports/swizzle and complete byte consumption.
It never constructs/deserializes a whole RGfxMaterial or evaluates the graph.

Inherited prefix limits:48samples,1MiBstored each,16MiBtotal reads,8MiBaccepted
expanded,32MiBtemporary chunk guard,4096dependencies,JVM256MiB. Additional graph
limits:1024boxes,4096wires,4096component floats per animation,64name bytes, exactly
8parameters/2animations per modern box. All counts are bounded before allocation;
original FARC is opened read-only. External readers are not certified safe for
arbitrary hostile input. All protected/other branches remain excluded.

## Verification and reproduction

```powershell
./scripts/run-material-oracle.ps1 -Graph -Output local-analysis/NEW-material-graph-oracle.json
python scripts/material_graph_report.py local-analysis/NEW-material-graph-oracle.json local-analysis/d4-material-prefix-oracle-final.json local-analysis/NEW-material-graph-consistency.json
python -m unittest discover -s tests -p test_material_graph_report.py
./scripts/run-material-oracle.ps1 -Output local-analysis/NEW-prefix-regression-oracle.json
python scripts/material_prefix_report.py local-analysis/NEW-prefix-regression-oracle.json local-analysis/d3-material-header-oracle.json local-analysis/NEW-prefix-regression-consistency.json
```

Final guarded32graph samples pass; consistency139boxes/118wires, zero out-of-range
actual links. Two graph oracle runs are byte-identical. Prefix regression32samples/
56normalized references passes. Java authored synthetic cases: fixed/packed graph,
signed-1/out-of-range endpoint, all truncations, declared parameter mismatch, box
count budget, animation count/mask rejection and known extreme signed conflict.
Three Python cases cover raw unknown ports/masks/out-of-range slots/endpoints,
count/length/slot/animation inconsistencies, changed prefixes/pins and duplicates.

Ignored artifacts: d7-material-graph-oracle-guarded.json,
d7-material-graph-consistency-final.json, d7-prefix-regression-oracle.json and
d7-prefix-regression-consistency.json under local-analysis; earlier reports retained.
No new proprietary extraction, input writes, binary publication or Rust/UI change.
Prior74Rust tests/fmt/Clippy/native QA remain latest product validation; not rerun
for diagnostic-only changes. Existing reference pin/source cleanliness retained.

## Exact next task — D8 independent graph metadata reader

Extend the bounded independent Rust material reader with an optional diagnostic
graph method, preserving the existing prefix API/schema and UI. Parse raw box
parameters/position/subtype and bounded animation metadata/digests, wire signed
indices/raw ports/swizzle, explicit slot/endpoint range status and tail counts.
Use unsigned zigzag reconstruction, retain unknown kinds/ports/out-of-range links,
and reject unsupported parameter counts before indexing. Do not assume a DAG or
traverse recursively. Compare all32canonical graph samples and synthetic hostile
variants/budgets/cancellation before any graph UI. No graph evaluation, animation
playback, guessed texture roles, shader execution, textured renderer or game scope.

## D8 independent reader complete — 2026-10-05

`lbp-resource::material::inspect_graph` now returns GraphInspection with the
unchanged MaterialInspection flattened alongside GraphMetadata. The existing
`inspect` method and serialized prefix fields remain unchanged. Prefix and graph
share one bounded expansion and cursor; shader/blob bytes still do not escape.
This optional API is not called by the native Material tab yet.

Raw box type/declared count/eight parameters, IEEE x/y/w/h bits, subtype and two
embedded animation summaries are retained. Summaries contain base IEEE bits,
bounded component count/digest, raw name hex/mask and derived key count; original
key values are not returned. Texture sample type1links raw parameter5with an explicit
in-range flag; this does not imply that its descriptor is non-null or resolved.
Raw unknown kinds/subtypes/masks/ports, out-of-range slots/indices and cycles remain
diagnostic data. No traversal, DAG assumption, graph evaluation or role inference.

Wire signed decoding uses unsigned zigzag shift/reconstruction in packed mode and
signed BE32 in fixed mode. Synthetic i32::MIN/i32::MAX/-1round-trip correctly in
both encodings. Known Toolkit extreme behavior is deliberately not adopted; actual
sample endpoints are within its unaffected range and match exactly.

Bounds match D7 contract:1024boxes,4096wires,4096component values per embedded
animation,64name bytes and exactly8parameters per modern box, in addition to
prefix1MiBstored/8MiBexpanded/ordinary branch0heads920–951limits. Counts are checked
before allocation; array byte lengths use checked multiplication and checked slices.
Cancellation is checked during each box/wire/animation plus shared wrapper work.
Nonzero component count with zero/incompatible lower-four-bit mask is invalid;
empty component data preserves raw mask and yields zero keys. Unexpected declared
parameter count is unsupported, not silently ignored. No original shader/tail data
is exported; tail bytes remain explicitly unparsed.

`CONFIRMED_FROM_LOCAL_DATA`:32/32canonical graph outputs match the guarded pinned
external reader, including139boxes/118wires,278animation records/digests and55slot
links. Preceding prefix/expanded-size/compression fields also compare exactly.
Prefix-only regression32/32passes. This is scoped independent graph **metadata**
parity, not shader/material rendering, animation playback, all5,163GMTcoverage,
other-build compatibility or unknown graph semantics.

Five new authored integration cases cover raw/packed/compressed graph structures;
raw unknown kinds/ports/masks; NaN/signed-zero bits; component-major data digest and
embedded zero byte in name; empty graphs; cycles; out-of-range slot/negative and
extreme signed endpoints; every short graph inside rebuilt valid envelopes; opaque
tail preservation; unsupported counts/box-wire-animation-name budgets; zero and
nondivisible masks; cancellation during boxes/wires and protected markers. Prefix
tests also still cover envelope chunks/ULEB/revision limits. No original bytes are
committed fixtures. Initial test compilation found ambiguous literal integer type
for to_be_bytes; specified u32. No local sample/parser mismatch occurred.

Exact verification (local Rust activated):

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-resource
cargo test --workspace --all-targets
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d7-material-graph-oracle-guarded.json local-analysis/d8-material-graph-parity-final.json --graph
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d4-material-prefix-oracle-final.json local-analysis/d8-prefix-regression.json
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

79workspace tests, fmt/Clippy/native build pass; graph32/32and prefix32/32pass.
Final graph/prefix reports are ignored under local-analysis. Initial graph report
used old prefix row labels; corrected report labels/scope and repeated final graph
comparison. Metadata equality was already exact. Earlier report remains local
evidence. No new GUI integration/native capture/manual interaction or full material
corpus/other GPU/platform tested. Original fingerprints/size/read-only attributes
and ignored source/binary boundaries unchanged; no Git initialization or publishing.

## Exact next bounded task — D9 native graph metadata inspection

Extend Material tab with optional bounded graph inspection using the same source
worker/cancellation/cache. Preserve prefix availability even if graph unsupported
or malformed. Show virtualized box/wire tables, raw fields/range flags, texture-slot
links and bounded embedded-animation summaries; graph JSON stays diagnostic-only.
Keep box/port labels clearly community annotations and unknown values raw. Navigate
slot descriptors through existing separate GUID/hash candidates; do not collapse
aliases/conflicts/nulls. Verify synthetic error/reset/cancellation/layout cases and
one real native sample. No node evaluation, shader execution, playback, textured
rendering, guessed sampler roles or expanded game scope.

## D9 native graph diagnostics complete — 2026-10-05

Material tab defaults to prefix inspection. Explicit graph inspection adds bounded
virtualized box/wire lists and selected-row raw parameters, subtype, position IEEE
bits, animation names/masks/counts/digests, endpoint/slot range statuses and graph
JSON. Only kinds 0 (OUTPUT) and 1 (TEXTURE_SAMPLE) have community annotations, from
the same pinned BoxType source above; other kinds/ports remain numeric. This is
metadata presentation, not a visual shader graph or interpretation of its behavior.

One validated read-only input worker, shared cancellation and source/selection
reset. Prefix and graph parsing expand sequentially under existing budgets. A
graph failure returns the successful prefix independently; cached prefix survives
graph-only cancellation. A source/row change discards both caches. User graph
preference survives reset. Reload clears current metadata. Separate GUID/hash
menus reuse existing candidate groups, preserving aliases/conflicts/nulls and
navigating through normal metadata/history handling. No automatic candidate choice.

Verification commands (project-local toolchain):

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-viewer
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

15 viewer tests / 81 workspace tests pass; fmt, Clippy and native build pass.
New authored cases cover independent graph failure, prefix retention, cancellation,
preference/reset, unknown kinds, extreme signed links, invalid slots, empty graph
lists, and headless layout. Existing candidate-group checks cover aliases and
conflicting identifiers. No new format rules or parser changes; D8 parity remains
the parser evidence, not a newly repeated oracle run.

Native QA (hidden, auto-exiting process; all artifacts ignored):

```powershell
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','000134dd','--qa-tab','material-graph','--qa-capture','local-analysis/d9-material-graph-native.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d9-material-graph-native.stdout.log -RedirectStandardError local-analysis/d9-material-graph-native.stderr.log
$qaProcess.WaitForExit(30000)
```

Real garage_car.gmat: head935, 8 slots, 5 boxes, 4 wires, 417 parsed graph bytes,
2 unknown tail bytes. RTX3080/Vulkan capture inspected: all box/wire rows visible,
raw parameters wrap within detail pane; further details remain scrollable.
No manual mouse navigation/menu selection, expanded animation panel, other GPU,
or complete material corpus tested. SFO/MAP hashes unchanged; FARC size/read-only
attributes and default-deny/private artifact exclusions verified. Reference source
clean, root still not Git, no commits/publishing or approval rejection.

## Exact next bounded task — D10 material tail oracle

Use the pinned existing reader to investigate the remaining ordinary clear GMTb
tail (sound enum and parameter-animation array are leads, not established layouts).
Extend only the isolated external diagnostic oracle, with count/byte/revision and
cancellation limits, retaining failure and unknown-tail outcomes. Compare the same
32 bounded samples and authored edge structures; record field provenance/conflicts
before proposing any independent parser extension. Do not execute shaders, export
animation keys/assets, decode protected inputs, or expand into gameplay/rendering.

Final D9 lifecycle refinement: a graph request during the initial prefix worker
is queued once after prefix success. Cancellation/failure never triggers an
automatic retry. Final fmt/workspace tests/Clippy/build repeated after this change;
all pass, complete ignored test log local-analysis/d9-workspace-tests.log.
Native capture preceded this lifecycle-only refinement; no manual queued-request
interaction tested. Diagnostic layout/parser behavior unchanged.

## D43 bounded consumer — 2026-10-06

Graph metadata parsing is unchanged. A shared preview consumer now selects an
unambiguous identity-UV0 direct texture diffuse route even with other branches,
labels ignored nodes/wires and preserves unsupported group geometry. Signed zero
is accepted for zero offsets. Strict policy, helper evidence and rendering limits:
[Diffuse projection](diffuse-projection.md). No full graph/shader evaluation.
