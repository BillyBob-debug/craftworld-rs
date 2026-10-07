# First implementation milestones

## A1 — Directory asset scanner (complete 2026-10-05)

Four-crate Rust workspace from `architecture.md` is implemented and tested. The
local CLI uses stable Windows GNU Rust; evaluate MSVC before GUI work. Prepare MAP/FARC/SFO
inputs into ignored `game-data/`, or accept a user-provided accessible game directory;
preserve the original ZIP. No decryption or whole-disc extraction.

Implement SFO identity, local MAP 0x100 and FARC tables; retain aliases and raw counts;
join hashes, offsets and declared/actual sizes. CLI terminal summary plus versioned,
deterministic JSON. Local acceptance: 36,854 MAP rows, 31,534 distinct FARC entries,
32,232 MAP rows resolving in FARC, zero duplicate GUIDs/archive hashes. Synthetic
tests cover truncated headers, excessive counts, checked offset arithmetic, duplicates,
invalid file ranges, unsupported revisions, and output/input separation.

## A2 — Resource metadata and oracle regression (complete 2026-10-05)

Add supported clear binary headers, resource magic/type dispatch, revision/branch and
dependency table inspection; bounded reads only. Report extension-inferred separately
from header-confirmed types; encrypted resources/SDAT are blocked; unknown/unparsed
resources stay in inventory. External loose-path inventory reports missing candidates.

Use pinned Toolkit Java readers headlessly in read-only mode to compare local table
rows and header/dependency fields. Keep oracle data/results ignored. Validate representative
old/new resource revisions with synthetic recipes. Two identical scans produce identical
inventory JSON; unknown statistics explicitly include skipped/blocked/unsupported cases.
This completes **Milestone A: Asset Scanner**, not just the current reconnaissance probe.
Implemented bounded supported header/dependency layouts, catalog links and schema 2.
Unmodified Toolkit parity covers all local table fields and 24 clear binary samples;
19 Rust tests pass. Textures and variable static-mesh headers remain explicitly
unparsed/partial. Actual results and remaining scope are in resource-metadata.md.

## B — Basic Asset Explorer (B1 + B2 complete 2026-10-05)

eframe/egui with wgpu backend, virtualized archive/resource list, metadata pane, GUID/
hash lookup, dependency and reverse-reference panes, bounded hex view, raw header info.
Reuse parser/catalog libraries; background workers, cancellation and bounded cache.
Hex bytes and full JSON dumps remain local and excluded from publication. Validate
navigation against scanner counts and responsiveness on the approximately 30k resource set.
No 3D viewport needed to make B useful.

B1 implements snapshot-only native browsing, type/status filters, exact GUID/hash
lookup, metadata/raw diagnostic projection and forward/reverse navigation. Catalog
loading is cancellable and off the UI thread; large lists are virtualized. The local
119 MB snapshot loads with scanner count parity and rendered QA. B2 adds validated
source attachment, bounded original hex bytes and independent archive-entry browsing,
including synthetic orphan/conflict cases. Source and hex validation reuse existing
readers through shared lbp-io. Full visual previews and live scanning remain deferred.

## C — First visual resource: textures

C1 (complete 2026-10-05) establishes a bounded metadata census and pinned community decode oracle
for actual clear texture variants. C2 (complete 2026-10-05) implements a simple pixel
format plus preview and synthetic layout/channel regression tests. Do not conflate
magic identification with pixel compatibility; unsupported variants remain explicit.

Implement common 2D PS3 GTF plus TEX/DDS as supported by Toolkit. First identify the
actual distribution of formats; then choose a simple present format. Compare dimensions,
mips, format, remap/swizzle and canonical pixels against Toolkit, not just extension.
Add texture preview with channel controls and explicit unsupported indicators.
Higher-mip controls require independently corroborated offsets; C1 demonstrated
Toolkit's synthetic mip-1 failure and unknown extra GTF expanded bytes.
Synthetic patterns catch channel order and layout mistakes. This is preferable to
meshes first because it isolates archive/resolution/compression/UI without skeletal,
morph, material and geometry complications.

C1 measured 13,624 clear textures and decoded 22 representative top mips through
the pinned external Toolkit library, with 191 matching fields. Plain 2D GTF BC1
is the C2 starting format (4,450 present locations); use its top mip only.
Details, excluded variants and synthetic recipes: [textures.md](textures.md).

## D — First mesh and richer inspection

RMesh-compatible geometry and material references -> plain mesh IR -> neutral viewer
shader, compared to Toolkit OBJ/glTF metadata and output. Add bones/animations and
materials progressively. Static meshes and Thing-level inspection are separate follow-ups.
Level rendering, simulation, physics and multiplayer remain outside these first milestones.

## C2 result / C3 scope

C2 implements plain GTF BC1 top mip: 19 distinct samples match Toolkit pixels,
dimensions/mips/flags; native RGBA/alpha/checkerboard/unsupported views inspected.
Contract: [texture-preview.md](texture-preview.md). C3 adds clear TEX/DDS BC1 with
the same pixels. BC3 and higher mips remain separate tasks.

## Completed implementation task — C4

C3 is complete: [verified TEX contract](tex-preview.md).

> Read AGENTS.md, STATUS.md, docs/tex-preview.md, docs/texture-preview.md and the
> pinned texture provenance. Add bounded plain 2D BC3/DXT5 top-mip decoding for
> existing clear GTF/TEX envelopes. Use published block specifications and the
> pinned Toolkit external oracle; do not translate reference code. Independently
> test both alpha endpoint modes, color interpolation, partial blocks, truncation,
> budgets and cancellation with authored synthetic structures. Compare bounded
> representative local BC3 RGBA and metadata before enabling native preview.
> Keep parser/UI separation, approved read-only access and explicit unsupported
> or protected outcomes. Run relevant tests/fmt/Clippy and native RGBA/alpha QA;
> update provenance, STATUS and MODLOG. No higher mips, swizzle, meshes, gameplay,
> bulk extraction, asset export, protected decoding, commits or publication.
> Stop at reviewable BC3 top-mip support.

## C4 result / active D1 scope

C4 complete: 35 BC3 samples and BC1 regressions match; synthetic Toolkit reversed
color conflict documented in bc3-preview.md. Continue D1 with existing RMesh reader
knowledge, bounded clear mesh census and external oracle. Establish prefix counts,
stream layouts/offsets and position/index metadata before topology or 3D preview.
No speculative geometry, static-mesh payload, skinning, morph animation or materials.

## D1 result / active next task — D2

D1 complete: 36 MSH prefixes compare exactly with canonical bits. See mesh-prefix.md.
Read pinned Primitive/Bone/AnimBone/MeshShapeVertex/MeshShapeInfo serializers,
resource descriptor and matrix encodings, plus RMesh::getTriangles. Establish
explicit primitiveType and bounded per-primitive index ranges before geometry
IR; compare list/strip/restart and degenerate handling using external helpers.
Only then add a neutral mesh viewport. No skinning, animation, materials, gameplay,
static-mesh guessing, asset export or protected decoding in this step.

## D2 result / completed bounded task — D3

D2 complete: 36 geometry samples match, native neutral wireframe verified.

> Read AGENTS/STATUS, docs/mesh-geometry.md and pinned Toolkit RGfxMaterial source.
> Add mesh primitive material-reference navigation using existing catalog resolution,
> preserving GUID/hash/both candidates and unresolved/duplicate outcomes. Establish
> bounded material metadata census and external pinned-reader oracle before writing
> a material parser or shader. Keep original inputs read-only, cancellation/budgets,
> independent format/IR/UI layers and synthetic fixtures. Record format provenance,
> exact tests and failed experiments. No full renderer, recovered-source translation,
> bulk extraction, protected decoding, skinning/animation, physics or networking.

## D3 result / future bounded material extension

D3 navigation/header census complete:32Toolkit samples and candidate controls
verified, final67Rusttests/5Pythonchecks. See material-navigation.md.

> Before a material payload parser, establish pinned RGfxMaterial revision/flag
> applicability and a bounded external metadata oracle for scalar fields, shader
> blob offset/count boundaries, texture descriptors and wrap modes. Keep shader
> code bytes uninterpreted and absent from diagnostic output. Test synthetic
> variants/bounds and compare actual bounded samples before implementation.
> Textured preview needs independent slot/color-space/UV evidence. Do not infer
> diffuse/normal roles from dependency order or execute original shader bytecode.
> Preserve source/IR/UI separation, read-only inputs and all protection/publication
> boundaries. No full levels, skinning/animation/gameplay/physics/network expansion.

## D4 evidence complete / exact next bounded implementation

32material prefixes and56normalized slot references corroborated by pinned external
reader/header consistency; see material-prefix.md. No product material codec yet.
Next: independently implement its narrow PS3 GMT prefix, factoring shared bounded
binary inflation into lbp-resource first. Canonical metadata parity on32samples,
synthetic strict bounds/flags/descriptors/revisions and cancellation required before
UI integration. Opaque shader/tail bytes stay uninterpreted and unexported. No graph
evaluation, texture-role guesses, original shader execution or expanded game scope.

## D5 complete / exact next bounded task — D6

D5 narrow Rust material prefix matches32/32external samples; shared mesh binary
decoder regression36/36.71workspace tests/fmt/Clippy/build pass. See material-prefix.md.
Next: native Material metadata tab using bounded/cancellable validated source workers,
with raw scalar/IEEE bits, shader interval metadata, all eight texture slots and
wrap arrays. Resolve GUID/hash candidates separately and preserve aliases, conflicts,
nulls/unknowns; reuse navigation history. Synthetic tab/cancellation tests and one
bounded native sample QA required. No opaque byte export, graph evaluation, original
shader execution, texture-role inference, textured shading or expanded game scope.

## D6 complete / exact next bounded task — D7

Native Material metadata/slot navigation verified with74workspace tests and real
GMT/unsupported captures. Next: pinned MaterialBox/MaterialWire/BoxType/BrdfPort
layout applicability and guarded external graph metadata oracle, then bounded
sample comparisons. Document raw kinds/ports/slot links and conflicts/unknowns;
no independent graph parser before corroboration. No original shader execution,
guessed texture roles, graph evaluation, textured shading or expanded game scope.
Exact handoff: material-viewer.md.

## D7 complete / next bounded task — D8

Guarded graph oracle corroborates139boxes/118wires across32materials with unchanged
prefix evidence. Next: independent optional graph metadata reader preserving
prefix API/UI; raw parameters/position/subtype, embedded animation counts/digests,
correct signed zigzag indices, raw ports/swizzle, range statuses and remaining tail.
Unknown types/ports and out-of-range links preserved; no DAG assumption/evaluation.
32canonical sample parity plus synthetic bounds/budgets/cancellation before UI.
Exact contract/provenance/oracle conflicts: material-graph.md.

## D8 complete / next bounded task — D9

Independent graph32/32and prefix32/32parity,79workspace tests/fmt/Clippy/build pass.
Next: optional Material graph metadata tables using validated cancellable worker;
preserve successful prefix if graph fails. Virtualize boxes/wires, retain raw/range
states/unknowns, expose bounded animation summaries and texture-slot candidate
navigation without guessing roles. Synthetic state/layout/error/reset tests and
native real sample QA. No evaluation, shader execution, playback, textured renderer
or expanded game scope. Exact contract: material-graph.md.

## D9 complete / next bounded task — D10

Optional native Material graph metadata tables, independent prefix fallback and
slot candidate navigation complete.81workspace tests/fmt/Clippy/native build pass;
real head935capture inspected. Next investigate remaining clear GMTb tail with the
pinned isolated existing reader: sound enum and parameter-animation array are leads,
not specifications.32sample bounded diagnostics plus synthetic edge structures,
provenance/conflicts/unknown outcomes before independent parsing. No evaluation,
shader execution, asset/key export, protected decoding or game-scope expansion.
See material-graph.md for the exact contract.

## D10 complete / next bounded task — D11

Tail oracle32/32parsed with all prior prefix/graph metadata unchanged; repeated
output byte-identical. All22Python tests and Java synthetic guards pass. Next:
independent optional material::inspect_tail sharing one bounded expansion/walk,
raw sound and parameter-animation summaries/residual counts; stable prefix/graph
APIs, strict counts/bounds/cancellation/unsupported states.32canonical parity and
synthetic fixed/compact/wrapper/truncation/budget/cancellation checks plus Rust
fmt/tests/Clippy before any GUI change. No key export, playback, sound simulation,
shader evaluation, texture-role guesses or runtime work. Contract: material-tail.md.

## D11 complete / next bounded task — D12

Optional independent tail reader complete with one bounded shared expansion/walk;
tail32/graph32/prefix32canonical comparisons exact.85Rust tests/fmt/Clippy/native
build pass. Next: explicit native tail inspection with raw sound and virtualized
parameter-animation metadata/JSON, preserving prefix/graph when tail fails. Reuse
validated source/cancellation/cache; synthetic lifecycle/error/layout and native
empty/nonempty-array QA. No sound labels/binding/playback/key export/shader
execution/textured rendering/runtime expansion. Contract: material-tail.md.

## D12 complete / next bounded task — D13

Optional native tail inspection and independent earlier-stage fallback complete;
87Rust tests/fmt/Clippy/build pass, empty and both populated native arrays inspected.
Next bounded material coverage census: existing snapshot/validated source, unique
accessible clear locations, deterministic stage success/error counts/distributions,
protected/missing/unsupported/budget/unknown tails kept distinct. Per-resource and
aggregate limits/cancellation, ignored diagnostic JSON, synthetic validation. Select
new shapes for small pinned-oracle follow-up before parser changes; no extraction,
semantics/playback/shader evaluation/textured rendering/runtime. See material-tail.md.

## D13 complete / next bounded task — D14

Coverage census5,163available unique material locations, all current stages parsed
with zero residuals,44,060,339readbytes;91Rust/24Python tests/fmt/Clippy/build pass.
40representative follow-up samples selected for newly observed revision/rawsound/
arraycount features andgraph diversity. Next run pinned isolated tail oracle,
independent Rust metadata parity and deterministic repeat; preserve mismatches/
rejections before adjusting any parser assumption. No semantics/sound labels,
playback/shader execution/rendering/game scope. Contract: material-census.md.

## D14 complete / next bounded task — D15

Pinned oracle40/40and complete independent metadata parity match; deterministic
repeat. Combined72unique samples cover all18revision/28rawsound/7arraycount cohorts,
29of76graphshapes. Next select47remaining count shapes plus maximum1143component
array resource, at most48samples, enforce unchanged oracle byte/count guards and
compare complete metadata. No source changes required for successful D14; no
semantics/rendering or full-corpus claim. Contract: material-census.md.


## D15 complete / next bounded task — D16

48 additional complete oracle matches; combined 120 samples cover all 76 observed
count pairs and maximum 1,143-component array. Next native diagnostic graph diagram
using parsed indices/endpoints, raw labels, bounded deterministic layout and linked
metadata selection. Preserve invalid endpoints/fallback; synthetic layout checks
and native small/larger graph captures. No new parser, shader/sound/animation
semantics or runtime. Exact contract: material-census.md.


## D16 complete / next bounded task - D17

Native material diagram and raw linked selection complete; 95 workspace tests and
final 21 viewer tests/fmt/Clippy/build pass, two native captures inspected. Next
cached material diagnostic JSON export: explicit private new-file output, versioned
metadata/stage outcomes, relative location, no extra asset reads. Preserve fallback
and containment/overwrite limits. Exact contract: material-diagram.md.


## D17 complete / next bounded task - D18

Cached material metadata export complete; 99workspace tests/fmt/Clippy/build, schema
validation and repeated native oracle-equal JSON pass. Next inspect existing texture
preview cache and add private metadata-only JSON export, factoring shared output
safeguards for the second consumer. No image pixels/asset export or new codec.
Versioned stage outcomes, synthetic refusal/budget checks, native texture oracle
comparison. Exact contract: material-export.md.


## D18 complete / next bounded task - D19

Cached texture metadata export and shared writer complete;101workspace tests/fmt/
Clippy/build, native two-wrapper metadata equality, channel-independent repeat and
Material regression pass. Next inspect mesh cache and add metadata/summary export,
excluding vertices/indices/bones/payload and additional source reads. Shared output
guards, synthetic lifecycle/projection checks and native prior-oracle comparison.
Exact contract: texture-export.md.


## D19 complete / next bounded task - D20

Cached mesh summaries complete;103workspace tests/fmt/Clippy/build, native list/strip
canonical oracle equality, explicit unsupported static export and deterministic
repeat pass. Next bounded Back/Forward navigation across MAP aliases, archive entries
andOverview, stored tab, validated indices and centralized cache/job invalidation.
Avoid stale detail rendering after transitions; state/toolbar event tests and native
captures. No format/dependency/source writes. Exact contract: mesh-export.md.

## D20 complete / next bounded task - D21

Bounded route/tab Back/Forward complete;108workspace tests/fmt/Clippy/build and
native synthetic roundtrip/local mesh regression pass. Next keyboard Back/Forward
using the existing checked route transition, visible shortcut help and consumed
events. Test disabled/repeated actions and typing behavior. No new format or UI
dependency; exact completed contract: navigation.md.

## D21 complete / next bounded task - D22

Keyboard history access complete;111workspace tests/fmt/Clippy/build, authored
keyboard/editor events and native help rendering pass. Return to format coverage:
inspect existing C1 uncompressed TEX oracle and documented DDS masks/pitch, then
implement bounded standard32-bit top-mip decode with synthetic distinctive channel,
row pitch, cancellation/truncation/budget checks and native pixel-oracle equality.
One local clear256x256 sample present. Keep higher mips/GTF swizzle excluded; no
GPL-origin helper translation. Exact selection/limits: navigation-shortcuts.md.

## D22 complete / next bounded task - D23

Standard-mask BGRA8 TEX preview complete;115workspace tests/fmt/Clippy/build,
fresh sample oracle and67sample regression, native/schema/repeat pass. Next address
animation inspection: read pinned RAnimation and enumerate accessible wrapper/
revision cohorts from existing snapshot before bounded scalar/count metadata oracle
and independent reader work.854 .anim MAP rows identified by extension only.
No playback/posing/runtime or speculative parser. Completed contract: bgra-preview.md.


## D23 complete / next bounded task - D24

Animation reconnaissance/oracle complete:748locations/21cohorts,33modern prefixes
and23full-reader field matches. Next modern CPU prefix metadata under lbp-resource:
existing wrapper/integer guards, synthetic revision/truncation/budget/cancel checks
and33exact local oracle matches. Preserve absent/null/raw fields and opaque byte
counts. Native inspector after parity; no keyframes/posing/playback. animation-prefix.md.


## D24 complete / next bounded task - D25

Modern CPU animation prefix complete;119workspace tests/fmt/Clippy/build,33exact
oracle matches, explicit old layouts, deterministic repeat and32material regression
pass. Next native Animation metadata tab using bounded validated-source worker/cache,
raw scalars/counts/bits/unparsed bytes, route/detach/snapshot reset and explicit
unsupported/protected states. Lifecycle/layout tests and native selected-oracle field
verification. No keyframe/locator decoding or playback. animation-parser.md.


## D25 complete / next bounded task - D26

Native Animation inspector complete;124workspace tests/fmt/Clippy/build and four
selected native captures pass, supported cached fields match oracle. Next explicit
cached Animation JSON export through shared private writer, versioned schema/stage
outcomes, synthetic reset/refusal tests and native oracle/repeat/schema checks.
No blob/keyframe bytes, extra source reads or parser changes. animation-inspector.md.


## D26 complete / next bounded task - D27

Cached Animation metadata export verified with126Rust tests and native oracle/schema/
repeat checks. Next census level/plan headers using existing snapshot and pinned
RLevel/RPlan sources, identify a safe metadata-only prefix and external-oracle route.
Do not invoke unbounded full Thing deserialization or implement before evidence.
No full levels, rendering or gameplay. See animation-export.md.


## D27 complete / next bounded task - D28

Level/plan census and pinned reader study complete. D28: bounded external plan-prefix
oracle over48selectedmodern samples/28cohorts; preserve nonempty dependency tables,
strict input/inflation/array budgets and explicit unsupported outcomes. Prefix/hash/
tail metadata only, no Thing traversal. Then independent CPU plan prefix after
oracle agreement; native plan metadata later. Full levels deferred based on recursive
layout evidence, not assumed absent format knowledge. level-plan-recon.md.


## D28 complete / next bounded task - D29

48pinned external plan-prefix samples pass, syntheticguardtests and deterministic
repeat verified. D29 independent CPU planprefix using existing wrapper/integer guards,
rawinner and opaquearraylength/start/hash/tail only; authored bounds/cancel tests and
48oracle-record parity. Nativeplanmetadata/export follow after parity. No recursive
Things or levelrendering. plan-prefix.md.


## D29 complete / next bounded task - D30

IndependentCPUplanprefix verified with130Rusttests and48exactoraclematches/repeat.
Next nativePlan metadata tab using bounded validatedsource worker/cache, strictearly
preflight, reset/cancel/navigation/candidates, rawhash/bounds/tail diagnostics.
Synthetic lifecycle/layout plus native supported/unsupported oracle comparison.
No fullThing/inventory parser or edits. plan-parser.md.


## D30 complete / next bounded task - D31

NativePlanprefix inspector verified with134tests and foursupported/oldcaptures;
supportedcachedfields matchoracle. Next cached Plan diagnosticJSON via sharedprivate
writer, explicitmetadata projection/versionedschema/outcomes/refusals and native
oracle/schema/deterministicrepeat. No payloadarrays/readback/Thingsemantics.
plan-inspector.md.


## D31 complete / next bounded task - D32

CachedPlanexport verified with136tests/nativeoracle/schema/repeat checks. Next study
modernInventoryItemDetails scalarprefix and SlotID/integer64 gates, boundedexternal
oracle before CreationHistory reference. Rawtype/keys/slot metadata could improve
planinspection; require evidence before optional CPU stage/UI. Preserve prefix on
deeperfailure, no Thing/body/text lookup semantics or edits. plan-export.md.


## D32 complete / next bounded task - D33

Externalmoderninventory scalarprefix established on48plans, actualSlotID agreement,
repeat and highbit s64conflict documented. D33optional independentCPUmetadata with
losslessencodeddatehex, raw32fields andextentcounts, noenum/date/reference semantics.
48oraclematches + prefixregression/syntheticguards; nativeoption/version2export later.
plan-inventory-prefix.md.


## D33 complete / next bounded task - D34

OptionalCPUinventoryprefix verified with140Rusttests,48exactinventoryoraclematches and
unchanged48base regression/repeat. Next optionalnativePlaninventory mode, separate
errors/requestedstate, basepreservation, cancellation/reset andnativefieldchecks.
Existingv1exportprefix-only; stage-awarev2afterUI. No recursive/date/title semantics.
plan-inventory-parser.md.


## D34 complete / next bounded task - D35

Native optional Plan inventory diagnostics verified with 143 Rust tests and pinned
oracle cache comparisons. Next implement version2 metadata-only Plan export with
explicit requested/base/inventory stages, separate outcomes and partial failure
preserving the base. Retain version1 schema; shared writer, synthetic lifecycle,
schema/native parity/repeat checks. No recursive inventory or date/title semantics.
[Native contract](plan-inventory-inspector.md).


## D35 complete / next bounded task - D36

Stage-aware Plan export complete:145Rusttests/nativeoracle/schema/repeat pass.
Prioritize practical browsing next: stable MAPresource sorting by path/GUID/SHA1
and original row order, preserving aliases, selected indices, virtualized filtering
and navigation. Synthetic duplicate/selection/filter tests and native sortedcapture.
No deeper Plan/Thing parsing or new community-format research needed for this task.
[Export contract](plan-export-v2.md).


## D36 complete / next bounded task - D37

MAP sorting complete:147tests/fmt/Clippy/build and five native snapshot-only checks
pass. Next archive-entry table/offset/size/SHA1 order with documented crossarchive
and tie rules, preserving orphan entries, aliases and original selection/navigation.
Synthetic multiarchive/duplicate-key/filter/reset cases and native expectedsort checks.
No format research or new source reads needed. [Contract](resource-sorting.md).


## D37 complete / next bounded task - D38

Archive sorting verified:149tests/fmt/Clippy/build and five native snapshot-only checks.
Next literal MAP-folder prefix navigation with immediatechildren/counts/breadcrumbs,
combinedfilters/sorting, alias/index preservation and unusualpath rules. Authored
boundary/lifecycle tests plus native count/selection checks. No filesystem traversal,
new format parser or speculativehighermip offsets. [Contract](archive-sorting.md).


## D38 complete / next bounded task - D39

Folder-prefix browsing verified with152tests/fmt/Clippy/build and four native metadata
checks. Next optional neutral solidmesh display alongside wireframe, consuming already
verified positions/topology only. Evaluate bounded projected triangles, generatedface
shading/occlusion limits, authoredextreme/degenerate tests, meshmetadata regression and
native supported/unsupported visual checks. No large renderer/new format semantics.
[Folder contract](folder-browsing.md).


## D39 complete / next bounded task - D40

Optionalneutral solidmesh verified with154tests/fmt/Clippy/build and four native runs;
metadataexport unchanged. Next focus-preview and orthographicaxis controls to enlarge
canvas and inspectflat/edge-ongeometry. Syntheticaxis/lifecycle checks and native
focusedboard/default/unsupported captures. Preserve parser/export/source boundaries;
no new renderer dependencies or material/bone format work. [Contract](mesh-surface.md).

## D40 complete / next substantive task — D41

User steering superseded camera polish with a real material-linked mesh slice.
Direct diffuse now works for one primitive with identity UV0 and a single sample
connected to diffuse output;36 UV samples and exact floor texture pixels corroborate
the implementation. Next: multi-primitive meshes using the same established subset.
Preserve triangle→primitive ownership, verify partitions with the pinned reader,
resolve/bind each eligible group with budgets and ambiguity handling, and retain
diagnostic geometry for unsupported groups. No general shader graph evaluator or
level renderer. [D40 contract](textured-mesh.md).

User steering revises D41 to a Bevy preview frontend for the existing supported
floor asset: shared CPU loader, depth-tested perspective camera, orbit/pan/zoom,
unlit direct diffuse, explicit unsupported states and native verification. Keep
eframe diagnostics during migration; add egui inspector integration afterward.
Multiple-primitive binding work above becomes D42. [Acceptance target](bevy-transition.md).
