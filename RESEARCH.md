# Research ledger — 2026-10-06

## D60 MIRROR and independent products

Five Mirror materials: full raw arrays and diffuse routes match pinned oracle,
0mismatches. Candlestick00002487 gains1278triangle group,3/4 to4/4diffuse.
40GUID cohort:40viewable/0Complete unchanged; full-diffuse35 to36. One native
second-monitor/image check. Geometry/all textures unverified. [Evidence](docs/mirror-preview.md).

## D59 interior-only BORDER preview

Game border color remainsunknown. Controlled polaroid UV0 probe finds referenced
range0.0004995..0.9995005;whole selected UV0 array passes conservative1/4096margin.
Nearest same-samplealpha MIX can use in-range texels without invoking bordercolor.
Guard preserves source[4,4] separately from display[3,3],rejects near/outsideborder,
and scans whole channel for safe material reuse. No general BORDER/linear path.
Same39GUID cohort39viewable/0Complete unchanged,+0newly viewable/+0Complete;
all-diffuse34→35,polaroid gains32triangle group1/2→2/2. Native second-monitor
check/one image confirms texture and labels;game/geometry correctness unverified.
[Sources,inference,bounds,coverage](docs/interior-border-preview.md).

## D58 same-sample alpha MIX

Bouncepad surface00012fd3 is direct ThingColor, not a wired sampler; standalone
input remains unavailable. Polaroid00006a16 uses same-sample alpha on MIX factor
port2. Bounded independent selector and nearest-only texel bake implemented;
factor overrides stored weight,raw bits retained. Target1record0mismatches;
existing32diffuse28eligible versus27,48linked46unchanged. Local polaroid uses
BORDER4/4,stillunsupported;48structural samples yield no other established case.
Same39GUID beforecode baseline39viewable/34all-diffuse/0Complete unchanged,
+0newly viewable/+0Complete/0improved assets. Corpus totals unknown.
[Sources,conflicts,bounds,checks](docs/alpha-mix-preview.md).

## D57 constant COLOR/single-texture MIX

Remaining ThingColor requires unavailable instance input;no default inferred.
Existing pinned sample identifies bounce-pad casing00012fd2 with black COLOR/
identity UV0 texture MIX at0.5. Independent bounded constant blending extends
D55 to one sampler,keeping raw bits/UV/wrap and original pixels. One targeted
record0mismatches;existing32diffuse27eligible versus26,48linked46unchanged.
Beforecode cohort38GUIDs (prior37plus bouncepad):38viewable/34all-diffuse/0Complete
unchanged,+0newly viewable/+0Complete;bouncepad gains232triangle group0/2→1/2.
Other95triangle surface wired sampler remains unsupported. Headless checks only.
[Sources,scope,bounds,coverage](docs/single-mix-preview.md).

## D56 local animation base snapshots

Two glow materials have named b1/c1 XYZ row curves with201keys and identity local
base rows. Pinned metadata confirms row binding but cannot establish time/key rate,
interpolation or equivalence to global animation uniforms. The timed-preview
hypothesis is deferred; an explicitly opt-in local base snapshot uses existing
bounded UV transforms. Static default remains unchanged and snapshots cannot be
confirmed Complete. Two targeted records0mismatches; existing48/32sets0mismatches.
Same37GUIDs:37viewable/0Complete unchanged,static all-diffuse34,opt-in35;geometry
boy0001da30 gains2groups/223triangles4/6→6/6. Corpus totals unknown.
[Pinned sources,conflict,bounds,coverage](docs/animation-base-preview.md).

## D55 static COLOR/product MIX

Initial targetneon0000be15 depends on ThingColor for diffuse and sampler input;
1pinned metadata record0mismatches0eligible,static color cannot be inferred.
Five other remaining material records reveal one bounded static lamp MIX route,
two named201-key matrix samplers and LCD instance/missing-diffuse routes. Lamp
0001dc46 MIX5 combines COLOR1/product4 at stored weight;two integer Repeat UV0
samplers permit existing bounded grid. Independent selection and linear RGB/alpha
mix before encoding;no general graph/shader/instance support. Target1record and
existing48/32sets0mismatches,46/26eligible versus45/26. Same37GUIDs:37viewable
unchanged,all-diffuse33→34,0Complete;lamp0001e458 gains190triangle group3/4→4/4.
Headless89tests,0native/images;GPU/game/corpus fidelity unknown.
[Sources,pins,bounds,coverage,conflicts](docs/static-mix-preview.md).

## D54 independent U/V wrapping

Question: why does shared stitching material00001d92 fail wrapping across three
sampled assets? Target pinned headless metadata confirms static UV1 slot0 with
S=ClampToEdge(3),T=Repeat(1). Prior matching-axis policy lost this valid pairing.
Typed independent axes now reach Bevy's sampler descriptor; legacy eframe retains
geometry/reports mixed addressing unsupported. No shader/UV folding/source changes.
Target1record and existing48/32sets compare raw arrays/routes with0mismatches;
existing route eligibility45/26unchanged. Same37GUIDs:37viewable unchanged,
all-diffuse31→33,0Complete. Three assets gain3groups/81triangles. Headless CPU/
GPU-descriptor tests;no native launch/image. Original game/GPU visual/corpus fidelity
unknown. [Sources, bounds, conflicts, tests](docs/axis-wrap-preview.md).

## D53 four-texture diffuse chain

Question: can the handkerchief's final group use four samples without additional
coordinates/general graph traversal? Existing pinned material record has three
static multiplies, four leaves and exactly matched UV1/UV0/UV0/UV1 transforms.
Fixed chain selection, exact-bit pair matching and optional fourth GPU texture
reuse primary coordinates. No reader shader copied. Same48/32metadata sets:
0mismatches,45/26eligible versus44/26. Same37GUID baseline:37viewable unchanged,
all-diffuse30→31,0Complete. Handkerchief00007a6d gains98triangle group,2/3→3/3.
No corpus/game pixel/pose/filter certification. [Sources/bounds/tests](docs/four-product-preview.md).

Native second-monitor capture shows the newly textured center motif and prior
borders,3/3supported,1ready for review,1viewable/0complete with confirmations unset.
One image inspected; original game texture/geometry fidelity remains unverified.

## D52 three-texture nested diffuse products

Question: which remaining nested routes can reuse the existing two coordinate
arrays without general graph evaluation? Two pinned48material records contain
three static textures, with the secondary/third channel and transform exactly
matching. Two static multiplies and three distinct leaves are structurally bounded;
authored Bevy extension samples the third image using the secondary coordinates.
No recursive traversal, third coordinate array or reader shader copied. Existing
48/32oracle sets:0mismatches,44/26eligible versus42/26. Same37GUID baseline:
37viewable/30all-diffuse unchanged,0Complete; handkerchief00007a6d gains two
groups/160triangles,0/3→2/3. One second-monitor capture resolves shader pattern
display; third group remains neutral and manual Complete unavailable. Corpus/game
pixel/pose/filter fidelity unknown. [Sources/bounds/coverage](docs/nested-preview.md).

## D51 independent channel-pair products

Question: can blocked UV1×UV0 diffuse routes use the existing two-sample pipeline?
Pinned GfxAssembler sampler selection/MULTIPLY behavior and two existing48material
records confirm independent channel requests. Loader now selects each explicitly;
grid eligibility requires matching original channels. GPU attribute regression
preserves both arrays/triangle order. Same48/32oracle sets0mismatches,42/26eligible
versus40/26. Same37hash/GUID cohort:37→37viewable,26→30all-diffuse,0Complete.
Four assets gain4groups/1577triangles. Native circuit board00013051 shows892triangle
UV1×UV0 pattern and4/4supported with manual completeness unset. One image inspected
on second monitor. Legacy eframe fallback unchanged; corpus/game pixel/pose/filter
fidelity unknown. Rule sources/pin/methods/coverage/scope:docs/channel-pair-preview.md.
No new disk layout, shader changes, input writes, copied/recovered code or publication.

## D50 independent matrix-texture products

Question: can the tutorial floor's remaining matrix product use a periodic grid
without discarding matrix cross terms? Targeted pinned reader metadata for material
0001a547 (8732bytes) confirms static cross terms/offsets and identity second sampler;
grid unsuitable. Independently transformed UVs plus an authored two-sample Bevy
extension retain the actual coefficients. Pinned GfxAssembler static sampler/
diffuse MULTIPLY behavior reused; no shader output copied/translated/executed.
Toolkit source pin remains clean. Targeted1sample0mismatches; existing48/32records
also compared. Same37hash/GUID sample:37→37viewable,25→26all-diffuse,0Complete.
Tutorial floor4/5→5/5groups,2000triangle group improved; geometry counts unchanged.
One native second-monitor capture shows final pattern and1ready for review, with
manual completeness unset. Legacy eframe explicitly unsupported for independent
sampling. API/memory/source/rule labels/coverage/conflicts: docs/dual-preview.md.
Original-game pose/filter/GPU/pixel fidelity unknown; no input writes/decryption/
recovered code/proprietary product bundling. Initial Rust test helper shadowing
E0618 resolved by qualified call; no runtime shader error observed.

## D49 static two-texture diffuse multiply

Question: can common blocked diffuse MULTIPLY routes display two textures without
a general graph evaluator? Pinned Toolkit GfxAssembler::resolve non-BUMP MULTIPLY
uses component-wise inputs0/1 product; static samplers in local records often use
integer repeat scales on one UV channel. Independently implemented shared periodic
grid with checked LCM, linearRGB/linearalpha product and explicit nearest filtering.
Linear-filtered products do not commute with precomposition, so nearest is authored
preview policy; original filter/game shader fidelity unknown. Restrict positive
integer scales1..16/zerooffsets/staticunwired/subtype0/samechannel/Repeat. Matrices,
other channels, nested/animated/flagged routes remain unsupported. Pin/source/rule
labels/bounds/tests/applicability/conflicts: docs/product-preview.md. No reader
shader copied/translated/executed, recovered-code/decryption needed or input writes.
Same37hash/37GUID baseline:37→37viewable,23→25all-diffuse,0verifiedComplete;
7assets gain13groups/5733triangles. Tutorial floor1/5→4/5, remaining matrix product
neutral. Reused48/32metadata records:0mismatches,40/26eligible versus36/24.
One second-monitor native image resolves patterned product display; original
geometry/pose/game pixels not certified. Detailed private evidence in local-analysis.

## D40 material-linked textured mesh

Full-channel interleaved BE float UVs corroborated by36 pinned Toolkit samples,
including exact raw-bit digests and negative-zero synthetic guard. Pinned partial
UV getter has a byte-start conflict; independent whole-channel parser avoids it.
MaterialBox/BoxType/BrdfPort/TextureWrap and GfxAssembler establish the narrow direct
diffuse identity-UV0 predicate, padded-empty animation names and unchanged swizzle.
Floor mesh→material→texture verified locally; 512×256 BC1 pixels match the oracle.
Independent predicate/orchestration/display; no shader code translated. Game visual
equivalence and broader graphs remain UNKNOWN. [Provenance](docs/textured-mesh.md).
Next meaningful compatibility work: primitive ownership and per-primitive bindings.

## C4 BC3 completed

Public Microsoft BC3/alpha layouts plus pinned Toolkit floor-alpha convention;
35 actual samples match exact pixels/metadata. Synthetic reversed-color test
exposed Toolkit BC1 branch misuse for BC3; spec-correct Rust retained and Pillow
12.3.0 corroborates synthetic rules. Source links, applicability/conflicts and
checks: docs/bc3-preview.md. D1 starts with pinned RMesh stream-prefix research.

## C3 completed after resumption — 2026-10-05

TEX/DDS BC1 top-mip parity now compares all retained DDS fields as well as exact
pixels. 15 TEX samples, 16 GTF regression samples and six supported refreshed C1
samples pass. Native RGBA, alpha and unsupported TEX captures inspected; 53 Rust
and three Python tests, fmt/Clippy/native build pass. Older C1 oracle missing DDS
fields was refreshed using the same pin; no new format rule inferred from that
schema mismatch. See docs/tex-preview.md. Next: bounded BC3 top mips only.

## C3 partial / stopped by user

Microsoft DDS_HEADER and DDS_PIXELFORMAT checked; C1 pinned RTexture::process
re-read for TEX envelope/DDS and BUMP suffix. Shared strict chunk inflation and
the verified independent BC1 pixels now support clear TEX. Raw DDS flags/caps/masks
and mip count zero retained. 15 selected TEX top mips match the pinned oracle;
synthetic recipes cover split DDS prefixes, bounds, unsupported DX10/cubes/volumes,
raw fields and suffix-vs-block ambiguity. No new community source translated.
Native TEX QA and existing GTF oracle regression remain pending. See STATUS.

## C2 continuation

Independent plain GTF BC1 top mip uses C1 layout knowledge and the public block
specification. Broader Toolkit oracle exposed endpoint-rounding conflict: authored
0x18e3 block and pinned DDSReader confirm nearest normalized RGB565 expansion.
Independent arithmetic, no copied table, now gives 19 matching local sample hashes.
CPU convention is separate from unknown original GPU/color-space fidelity. Native
RGBA/alpha/checkerboard/unsupported views inspected. No higher mips, BC3, TEX or
swizzle implementation. Contract and next task: docs/texture-preview.md.
C1 absence claims below are historical.

## C1 continuation

Read pinned Toolkit SerializedResource, CellGcmTexture, CellGcmEnumForGtf,
Compressor, RTexture, DDS, Images and external DDSReader. Completed accessible
texture census using B2 validated source attachment. 13,624 clear wrappers,
established all TEX DDS prefixes; selected 22 present 2D variants. Unmodified
Toolkit agrees on 191 fields and decodes all top mips. Digests remain ignored;
Rust pixel decoder still absent. Exact per-field provenance and tests: docs/textures.md.

Official Microsoft DDS/block-compression specifications are the first BC1 basis.
Refresh 25f03e40416ebe73016b9e9972e0fa7b11a7f0b0 GtfDecoder::ProcessPixels
independently dispatches GTF DXT1 to BC1; read as AGPL reference, not executed/ported.
Investigated conflicts: standard mip-chain size does not account for all GTF bytes;
Toolkit DDSReader's mip arithmetic fails our synthetic mip-1 test. Preserve those
unknowns and support top mip first. Toolkit swizzle cites RPCS3 3d49976b whose
root LICENSE is GPLv2; contribution/relicensing history unresolved. No swizzle code
copied. Web cache misses resolved via ignored read-only pinned source downloads.
No executable investigation, protected decode, bulk extraction or game-source reuse.

## B2 continuation

Shared source/SFO code moved to lbp-io; catalog attachment reuses existing A1
readers/fingerprints and A2 protection-prefix rule. Project-authored validation
compares the source's SFO identity/hash, MAP hash/all rows, and FARC table/all entries
before bounded hex access. No new proprietary format knowledge was required.
Local source binds with 4,704,862 metadata bytes; zero local FARC orphan hashes.
No extraction or protected decoding. Source safety/limits and tests are in
docs/source-attachment.md; first texture oracle experiments become C1.

## B1 continuation

Metadata viewer now implemented using eframe/egui 0.36.2 and its wgpu backend,
pinned through Cargo.lock. Official crate docs/installed source guided hosting,
virtualized ScrollArea rows and viewport screenshot events. The local GNU build
renders on RTX 3080/Vulkan. No new original-format claims or community code ports
were needed: lbp-catalog projects the project's existing A2 wire metadata. Counts
and verification/limits are recorded in docs/viewer.md and STATUS.md.

## Decision

Start with an extracted-directory asset scanner. Use Craftworld Toolkit/cwlib as the
primary format implementation reference, with pinned external oracle runs. Existing
community work already covers archives, identifiers, resource wrappers, dependencies,
textures, meshes, materials, animations, and significant level serialization. Do not
rediscover these formats from the executable.

## Local guides reviewed

`references/ai-game-modding-guides-main/` is a ZIP snapshot, not a Git checkout;
its exact upstream revision is unknown. Read guides 00, 03, 04, 05, 06, 12 and both
AGENTS/STATUS templates. Origins: [guide repository](https://github.com/trevaintdead/ai-game-modding-guides).

- Search existing format readers before decompilation; own-copy runtime inputs.
- One concrete milestone at a time; explicitly list verified and missing features.
- Default-deny tracking, retained licenses, provenance per inherited component.
- Persistent rules, design documents, evidence logs, exact reproduction commands,
  and a fresh-session handoff; diagnose before changing code.
- Compiler/headless/visual/release checks establish different kinds of confidence.
- Synthetic parser checks plus user visual review, rather than judging gameplay from a build result.
- Guide 12 specifically warns that an upstream binary or catalog with unknown origin
  is not safe merely because its containing repository declares a license.

Worked example: [mw2-rust-rust-rewrite](https://github.com/Dj-Shortcut/mw2-rust-rust-rewrite)
was studied through guide 12, not built or audited here. Its main lesson is separate
inherited-engine provenance from authored project code and report limits honestly.
Linked examples checked at current public revisions:

| Example | Revision inspected | Useful practice |
|---|---|---|
| [hl2-rs](https://github.com/kvalls/hl2-rs) | b4b1731530a9ca146f9d45532f00ef7f9fc69e7d | README/AGENTS separate source-assets, common contracts, runtime; owned install read-only; packaged executable validation; incomplete campaign stated |
| [gang-beasts-rust](https://github.com/muffinmxn/gang-beasts-rust) | 815ffbcb4d68a080feb975cf214f14c9a41f1f51 | Default-deny ignore policy, user-side extractor/harness tooling |
| [benilla](https://github.com/samwhosung/benilla) | ffdc206ac2e87e3164e31caf26862c43a5180152 | Separate format crate and many focused inspection examples; architecture reference only |

No game-engine implementation was imported from these examples. Their README claims
were not independently validated by building/playing them.

## Evidence collected

1. Workspace listing, Git status: no root Git repository; existing `.gitignore`,
   scripts, ignored analysis outputs, and ignored references were preserved.
2. Reran disc probe with a 40 MiB prefix: identical JSON to preexisting inventory.
3. Independently implemented documented Toolkit MAP/FARC table layouts; one forward
   ZIP stream through ISO byte 1,059,897,882, retaining bounded buffers; no asset extraction.
4. Sampled 19 distinct resources from nine extension families. All sampled SHA-1
   values matched FARC entries; parsed only clear headers/dependency tables and GTF
   metadata. Resource revisions vary; disc APP_VER is not resource serialization revision.
5. Queried GitHub repository metadata, default-branch commits, selected source trees,
   Toolkit fork comparisons, and FarcTool forks. Results remain ignored locally.
6. Broad GitHub search for LittleBigPlanet plus web searches for format readers,
   Workbench, FARC documentation, Rust libraries, model/texture tools, and servers.
7. Consulted official egui/eframe and Bevy documentation for the viewer choice.

Search is broad reconnaissance, not a claim that every private Discord note or
repository has been found. No access to private Discord material was attempted.

## Source pins and epistemic limits

Primary: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9; paths and methods in
`docs/formats.md`. Local checkout HEAD matched the public API commit. FarcTool,
cwlib-rs and Asset Studio checkouts existed before this pass; their metadata is
recorded in `docs/community-projects.md`. API metadata is a date-specific snapshot,
not a support guarantee. A `pushed_at` timestamp is not proof of a new parser commit.

No license found means reuse permission is unresolved, not that factual format
observations cannot inform an independent implementation. Decomp-derived source is
excluded as implementation input even when a repository's top-level license exists.
MIT Toolkit source rules may be adapted with notices and attribution; asset/template
provenance and embedded third-party licenses still require separate review.

Conflict: Imagisphere FARC prose says magic precedes count while its diagram says
count precedes magic. Local footer and Toolkit agree with the diagram. Record the
local result; do not implement the contradictory prose.

## Oracle policy

Specification -> pinned existing parser -> controlled local sample -> targeted
executable investigation only if needed. Specifications can contain mistakes;
resolve disagreements explicitly with source and local data, as above.

- Toolkit Java reader comparison should dump rows/header/dependencies from its
  read-only constructors, never invoke `save`, import, install-mod, or decrypt paths.
- Align identity `(archive hash, GUID aliases, resource revision)` and normalize
  byte order/unsigned fields; compare counts, table offsets, type, flags, dependencies.
- Keep raw table order as diagnostic data and use sorted order for public comparison.
- Report differences before deciding which tool is right; Toolkit drops duplicate
  GUIDs and uses sets for dependencies, which can hide raw multiplicity.
- For textures compare wrapper, dimensions, mip count, pixel format, and canonical
  decoded RGBA mip hashes. For meshes compare streams, index bounds, primitives,
  material refs, skeleton/morph counts; render comparison is a later separate check.
- Commit only authored synthetic fixtures/recipes and small expected metadata.
  Full inventories, resource paths, decoded images, and captured game bytes stay local.

Actual unmodified pinned Toolkit oracle now agrees with all local MAP/FARC fields
and 24 clear binary header/dependency samples (123 normalized references). Java and
Maven were installed locally and cwlib built from source. Raw descriptor multiplicity
and null/unknown fields remain in Rust despite Toolkit's deduplication/filtering.
See docs/resource-metadata.md for exact comparisons and limits. Pixel/mesh oracle
agreement remains untested. The narrower synthetic Python TEX oracle is not GTF support.

## Research artifacts

A1 continuation: independent Rust SFO/MAP/FARC readers now implement the pinned
Toolkit layouts. Source-level provenance is retained; complete local field parity
with the Python probes was verified. This is not an external community oracle.
Deterministic generated inventories remain ignored under local-analysis/; see
STATUS and docs/scanner.md. No new payload formats were inferred during A1.

A2 continuation: census reached clear revision 951 (not only the original sample's
940); Toolkit corroborated samples for every newly observed head 941–951. Bounded
Rust readers now inspect canonical magic, ordinary headers and raw dependency
tables, with explicit partial static-mesh scope. 116,739 dependency references;
46 unresolved in selected MAP. Two schema-2 inventories are identical. No payload
semantics/decryption/GUI added; protected SDAT/EBOOT remain untouched. Authored oracle
harness, external source build and local comparison reports retain provenance.

Ignored: `local-analysis/disc-inventory-verified.json`, `lbp-tables.json`,
`resource-samples-verified.json`, GitHub metadata/trees/comparison responses and
selected public source downloads. Preexisting `iso-prefix.bin` is ignored and was
not modified. Trackable deliverables contain summaries, specifications, and code only.

## D1 mesh prefix evidence

Pinned RMesh field sequence and MemoryInputStream primitive encodings independently
implemented in lbp-mesh. 36 selected local prefixes match external stream helpers
and isolated RMesh::getVertices; no full mesh oracle invoked. Exact provenance,
cohort scope, float JSON mismatch and limits: docs/mesh-prefix.md. D2 source leads:
Primitive/Bone/AnimBone/MeshShapeVertex/MeshShapeInfo and RMesh::getTriangles.

## D2 mesh topology and native wireframe

Pinned Primitive/Bone/AnimBone/shape layouts, Serializer resource descriptors and
matrix encoding establish the explicit topology field. Independent triangulation
uses public list/strip/cut semantics; 36 bounded samples match per-primitive triangle
hashes and metadata. Bones are traversed, not simulated/transformed. Native neutral
wireframes inspected; exact sources/conflicts/commands: docs/mesh-geometry.md.

## D3 graphics-material reference/census evidence

5,163GMTlocations/13,426rawtexturedeps in current snapshot;32selectedheaders match
pinned Toolkit metadata. Native material menus retain separate GUID/hash candidates,
aliases and null/unresolved references. Read RGfxMaterial::serialize as next format
lead only; sampler roles, shader graph and payload compatibility remain unknown.
Reproduction/source/failed experiments: docs/material-navigation.md.

## D4 guarded material-prefix evidence

Pinned RGfxMaterial count methods and stream helpers corroborate32bounded PS3 GMT
prefixes;56slot identities agree with prior header/dependency oracle. No full graph
deserialize, shader interpretation or independent Rust material parity. Synthetic
truncation exposed Toolkit padded fixed-width reads; harness guard rejects them.
Exact per-field pin, branches, sample scope, budgets and reproduction:
docs/material-prefix.md. Next Rust parser remains separate authorized bounded work.

D5 now independently implements the corroborated prefix in lbp-resource::material;
32canonical metadata fields match completely,36mesh regressions corroborate shared
binary-decoder factoring. No opaque/shader bytes exported. Exact synthetic scope,
unknown graph/tails, limits/tests and UI next task: docs/material-prefix.md.

D6 now exposes corroborated prefix metadata and raw texture-slot candidate groups
in the native viewer. No new format inference. Native realGMT/unsupported rendering
corroborated;74workspace tests pass. UI bounds/lifecycle/untested manual interactions
and next existing-reader graph metadata investigation: docs/material-viewer.md.

D7 establishes graph box/wire/embedded-animation metadata via pinned isolated
readers on32samples.139boxes/118wires; prefix evidence unchanged. Extreme signed
zigzag oracle bug and permissive declared parameter count documented, without
patching reference source. No independent Rust graph parity/evaluation claimed.
Per-rule source URLs/limits/conflicts/tests and next contract: docs/material-graph.md.

D8 independent optional graph reader now matches32canonical guarded samples;
prefix32regression passes,79workspace tests/fmt/Clippy/build pass. Correct signed
extreme behavior is independently synthetic-tested rather than following Toolkit's
bug. No graph semantics/evaluation/playback/render fidelity claim. Bounds, exact
commands, sample scope and next UI task: docs/material-graph.md.

D9 native optional graph diagnostics complete: virtualized raw box/wire metadata,
animation summaries, separate slot identifier candidates and independent prefix/
graph errors.81workspace tests/fmt/Clippy/build pass and one real native head935
capture inspected. No additional parser inference or oracle rerun; pinned D8
knowledge still applies. Boundaries, test limits and next D10 tail-oracle task:
docs/material-graph.md.

D10 establishes material tail layout through pinned RGfxMaterial field sequence,
Serializer::array and isolated MaterialParameterAnimation reader. Same32samples
fully accounted for, unchanged prefix/graph metadata and deterministic output;
all raw sound0,30empty arrays and2single241-key animations.22Python tests plus
Java guards pass. No product parser/UI change or semantics claim. Per-rule sources,
thresholds/conflicts/budgets and next D11 contract: docs/material-tail.md.

D11 independent inspect_tail consumes the documented tail layout using one shared
bounded prefix/graph/tail walk and animation metadata helper. All32canonical tail
samples exact including earlier fields; graph32/prefix32regression exact. Four
new authored cases and85Rust tests/fmt/Clippy/native build pass. No sound/animation
semantics or GUI change. Source provenance/tests/reproduction and next D12 native
metadata contract: docs/material-tail.md.

D12 presents independent tail metadata natively with explicit optional request,
raw sound/virtualized animation summaries/JSON and earlier-stage fallback. Three
real RTX3080/Vulkan captures corroborate empty and both populated sampled arrays;
87Rust tests/fmt/Clippy/build pass. No new layout/semantics inference or parser/
oracle changes. Evidence/interaction limits and next D13 census: material-tail.md.

D13 broadens local acceptance measurement to5,163available unique materials with
all prefix/graph/tail stages parsed; zero residuals,18revisions/28rawsoundvalues and
arraysup to7. This is independent-reader acceptance, not new oracle parity. Repeated
report deterministic;91Rust/24Python tests andfmt/Clippy/build pass.40metadata-only
follow-up samples selected, covering new primary features and24graph shapes; pending
external validation. Exact scope/provenance/limits: docs/material-census.md.

D14 adds independent complete-field parity40/40on the census follow-up set with
pinned guarded external readers and deterministic repeat. Combined72unique samples
cover all18revisions/28rawsounds/7arraycount cohorts;29of76graphshapes andlargest
componentarray603validated(1143observed). No new rules/semantics or source changes.
Provenance/reproduction/remaining sample-boundary task: material-census.md.


D15 adds complete-field pinned oracle parity on 48 count-shape/boundary samples.
Combined 120 unique samples cover all 76 observed box/wire count pairs and largest
component array (1,143). No new rules or semantics inferred. Repeated selection/
oracle deterministic; 27 Python tests pass. Count-shape coverage is not topology
or full-corpus validation. Provenance/reproduction: docs/material-census.md.


D16 visualizes already validated material endpoint metadata without new format
inference. Native head935/head931 diagrams (5/19 boxes) corroborate presentation;
95 workspace tests plus final 21 viewer tests/fmt/Clippy/build pass. Layout is an
independent index grid; shader/port/animation semantics remain UNKNOWN. Source,
resolved development failures and test limitations: docs/material-diagram.md.


D17 corroborates export of cached material fields against the existing pinned
oracle on head949. No new format research or interpretation. Native JSON repeats
identically; 99Rust tests/fmt/Clippy/build and envelope validation pass. Cached stage
availability differs from current-mode request state; no fresh source validity
claim. Provenance and verification: docs/material-export.md.


D18 projects cached supported texture metadata without new format research. Two
native wrapper samples match earlier C4 parser output and all externally supplied
header fields; no full external equality claimed for fields absent from its report.
101Rust tests/fmt/Clippy/build and schemas pass; Material regression unchanged.
No pixel export or semantic inference. Evidence: docs/texture-export.md.


D19 exports cached mesh summaries without new format inference. Native list/strip
fields match prior validated parser metadata and canonical external oracle using
existing IEEE bounds-bit comparison; static remains unsupported.103Rust tests/
fmt/Clippy/build, schema and deterministic repeat pass. No payload export or visual
fidelity claim. Exact provenance/limitations: docs/mesh-export.md.

D20 is authored UI navigation with no new format knowledge or external oracle work.
108Rust tests/fmt/Clippy/build pass; native synthetic route/tab roundtrip and real
mesh regression inspected. Inputs and pinned reference unchanged. Contract and
limitations: docs/navigation.md.

D21 adds authored keyboard navigation, no format rule.111Rust tests and native help
inspection pass. Existing C1 census was read to choose next useful format work:
one32-bit TEX/DDS standard-mask texture and three linear GTF0xa5 textures. These
observations do not establish pixel compatibility; use existing pinned oracle and
public DDS specification before implementation. See docs/navigation-shortcuts.md.

D22 independently implements documented standard-mask BGRA8, corroborates local
legacy LINEARSIZE with pinned oracle and preserves raw header. One local sample
matches all retained DDS fields and RGBA;67combined pinned samples regress cleanly.
115Rust tests/native/schema/repeat pass. Padded rows synthetic-only; no GTF swizzle
or higher mip evidence. Full rule provenance/conflict: docs/bgra-preview.md.


D23 establishes modern ANMb prefix ordering/gates from pinned RAnimation and bounded
external readers:33prefixes across20revisions,23actual-reader field matches.748local
locations include five oldhead844 layouts. Fat1 meaning UNKNOWN; no keyframe/playback
semantics established. Full evidence: docs/animation-prefix.md.


D24 independently implements D23's modern animation prefix and exactly matches
33local oracle samples, keeping raw fields/bits/nulls and opaque byte counts.
Oldhead844 remains Unsupported.119Rust tests and32material regression pass. No
new payload semantics; full implementation/verification: docs/animation-parser.md.


D25 exposes verified animation prefix fields natively; no new payload interpretation.
Three native supported views exactly match cached pinned-oracle metadata; old layout
remains unsupported. Synthetic longer protection buffers exposed caller/helper
length mismatch; fixed leading4byte classification, no new signature. Evidence and
verification limits: docs/animation-inspector.md.


D26 exports already verified Animation metadata without new format claims. Native
reports equal pinned D23 oracle fields; old layout remains unsupported/null. Version1
schema and deterministic repeat verified; docs/animation-export.md. Next evidence
work moves to level/plan prefixes rather than speculative animation playback.


D27 establishes level/plan header/dependency census and bounded modern plan sample
manifest from existing snapshot, not new payload evidence. Pinned RLevel immediately
recursively reads Thing; RPlan first stores inner revision and opaque byte array.
Independent bounded plan envelope is the next useful slice, contingent on external
oracle. Inventory/Thing semantics unknown. Local reference used after web cache miss.
Full rule provenance and conflicts: docs/level-plan-recon.md.


D28 replaces plan-prefix inference with48bounded local unchanged-reader experiments.
Clear wrapper buffers agree; rawinner/bytearray bounds/hash/tail measured, all28sampled
modernhead cohorts supported. Full RPlan not called; source establishes field order,
MemoryInputStream establishes primitive behavior. Inventory and Things still unparsed.
Storedinner==outer is observed only. docs/plan-prefix.md records exact limits.


D29 independently implements bounded plan prefix and matches48D28oracle records,
including opaque SHA256/bounds/tail. Rawinner mismatch retained; zero arrays do not
prove empty objects. No inventory/Thing evidence gained. docs/plan-parser.md.


D30 exposes alreadyverified planprefix natively; no newdiskformat rules. Small/large
MAP and entrycachedobjects equal D28oracle; old622unsupported. Inventory/Thing payload
still unparsed. Native/lifecyclecoverage and limits: docs/plan-inspector.md.


D31 exports verified Plan prefix metadata without new formatclaims. Nativecached
reports equal pinned oracle; schema/repeat/outcomes verified. Thing/inventory bytes
excluded and no extra reads. Next moderninventory scalarprefix evidence before first
recursive reference; title/description semantics remain unknown. docs/plan-export.md.


D32establishes modernInventoryItemDetails scalarprefix ordering frompinnedsource,
48boundedlocal primitive/SlotID classprobes before firstrecursive reference. Date/
slot/sound all0 locally; nonzero/extremes onlysynthetic. Pinned s64compactread conflict
reproduced; rawencoded64hex chosen, date semanticsunknown. docs/plan-inventory-prefix.md.


D33 independently decodes moderninventory scalarprefix and matches48pinnedoracle
records. Encodeddatehexavoids knowns64bug without semanticnormalization. Rawflags/
keys/slotvalues retained; optionalfailure retainsbase. Privateprefixrefactor original
48record report byte-identical. Inventoryreferences still UNKNOWN. plan-inventory-parser.md.


D34 exposes already verified inventory scalars through an optional native worker.
Three local supported inventory caches exactly match D32 oracle; four base caches
match, old622 unsupported. No new format or semantic knowledge. Encoded date bits
and raw flags/keys remain uninterpreted; recursive references remain UNKNOWN.
Verification limits and provenance: docs/plan-inventory-inspector.md.


D35 adds a versioned metadata envelope only. No new disk-format evidence; rawdate/
flags/keys remain uninterpreted. Native projections match the established D32 oracle.
Recursive inventory/Thing compatibility remains UNKNOWN. docs/plan-export-v2.md.


D36 improves snapshot-only browsing; no new format knowledge or community reader
work needed. Ordering keeps duplicate aliases and original indices. Native sort
summaries match independent snapshot ordering. docs/resource-sorting.md.


D37 is snapshot-only archive presentation; no new format knowledge. Native single-
archive first10/order/count/alias projections match independent expectations; authored
multiarchive cases cover global ties. Known highertexture mip-offset conflicts remain
unresolved, so folder browsing precedes that expansion. docs/archive-sorting.md.


D38 is literal metadata-prefix grouping with no new disk-format claim. Aliases remain
individual rows; bounded unusualpath handling does not authorize filesystem access.
Native counts/order match independent snapshot calculations. folder-browsing.md.


D39 introduces no new mesh-format knowledge. Surface normals are generated UI values,
not recovered original attributes; averagecamera-depth ordering is approximate.
Parsed geometry/export unchanged, nativeJSONbyte-identicalD19 verified baselines.
Posed/material/texture fidelity still UNKNOWN. docs/mesh-surface.md.

## D41 renderer integration — 2026-10-06

No new proprietary format rules or oracle work. Shared lbp-preview consumes the
established geometry/UV/material/texture findings; Bevy has an independent display
adapter. Renderer API documentation, authored policies, native synthetic/floor evidence
and unknown game-fidelity boundaries: [Bevy preview](docs/bevy-preview.md).

## D42 primitive partitions — 2026-10-06

Existing pinned Primitive descriptors and RMesh.getTriangles(primitive/start,count)
corroborate triangle ownership per primitive: 36 meshes/82 partitions (16 meshes
with multiple primitives), exact spans and canonical index SHA256. Existing flat
geometry metadata unchanged. No new material graph interpretation; supported
bindings reuse D40 and unsupported groups remain explicit. Authored mixed-binding
native scene verifies group alignment; actual board has four unsupported graphs.
No decryption/decompilation needed, no original code/assets copied. Provenance,
applicability/restart conflict/test scope: [D42](docs/primitive-groups.md).

## D43 diffuse route evidence — 2026-10-06

Pinned RGfxMaterial output/port helper behavior and GfxAssembler's per-output path
selection support a narrow direct-diffuse projection. Existing 32 sample routes
match exactly; 14 eligible, ten in larger graphs. Controlled first-48 linked
materials: zero route mismatches/eight eligible; sets may overlap. Eligibility is
not texture/mesh compatibility. Signed-zero offsets corroborated; no new disk
layout or arbitrary graph evaluator. Bottle cap/light native captures demonstrate
visible projection; board has an occluded supported group. Source/pin/methods,
strict duplicate policy and original-renderer unknowns: [D43](docs/diffuse-projection.md).
No decryption/recovered code/source translation or proprietary distribution.

## D44 UV channel selection — 2026-10-06

Pinned TEXTURE_SAMPLE parameter4 selects second pair when1; product restricts0/1
rather than adopting fallback for unknown selectors. 32/48 material channel/route
records match; 36 full-channel UV digests match and remain D40-identical. Selected
UV1 copies preserve original bits and never substitute UV0. Native Valkyrie has two
supported UV1/BC3 groups; original pose/lighting/alpha fidelity unknown. No newdisk
layout/recovered-source use/decryption. [Provenance/conflicts/test scope](docs/uv1-preview.md).


## D45 static sampler scale/offset — 2026-10-06

Question: order and field interpretation for direct unanimated subtype-zero diffuse
samplers. Pinned Toolkit MaterialBox constants and GfxAssembler static branch
support component-wise UV × scale + offset. Animated branch conventions differ
and are deliberately excluded. Independently expressed arithmetic preserves raw
arrays and IEEE bits; diagnostic numeric bounds are authored policy. Existing32/48
external route/channel/raw-parameter records match with zero mismatches; eligibility
increases15→16 and14→16 (sets may overlap). Toro2× strap and CliveUV1 half-scale
native evidence; nonzero offset remains authored-fixture evidence, not original-game
pixel parity. No decryption/recovered source. Full per-rule provenance, conflicts,
applicability, limits and tests: [D45](docs/affine-preview.md).

## D46 static texture × color diffuse route — 2026-10-06

Bounded metadata census of the established 80-material sample set found four
texture×static-COLOR multiply routes among unsupported diffuse roots. Pinned
Toolkit BoxType/BrdfPort/GfxAssembler COLOR/MULTIPLY behavior establishes parameter
meaning and component multiplication; independent bounded implementation rejects
other graph shapes. Authored external helper queries on the unchanged reader
confirm source/color/multiply indices and raw tint bits, 32/48 comparisons with
zero mismatches (eligibility16→18 in each set). No shader execution or decryption.
Khronos sRGB conversion informed authored linear tint baking; original game color
calibration and post-filter GPU rounding remain UNKNOWN. Raw pixels/UVs preserved.

Measured same36-mesh baseline:36→36viewable GUIDs,15→15all-diffuse GUIDs,
0newly verified complete. Three existing assets gain6nonempty textured groups.
Separate native vulture shows0.14901961gray tint and4× tiling; one image viewed,
secondary monitor chosen before visibility. No original inputs/code/keys/assets
distributed; no recovered/community source translation or strict clean-room claim.
Full per-rule evidence, applicability, conflicts and coverage: [D46](docs/tinted-preview.md).

## D47 static COLOR-only diffuse — 2026-10-06

Pinned GfxAssembler::resolve COLOR case and BoxType/BrdfPort establish static four
float parameters and direct diffuse route. Existing independent parser/raw oracle
supplies them; no new disk layout. Reused D46 external records compare48/32routes
and exact raw color bits with zero mismatches; eligibility18→36/18→23. Authored
bounded selector, explicit SolidAsset IR and independent GPU/UI adapters avoid
invented textures or UV decoding. Animated/wired/ThingColor/HDR paths stay unsupported.
Source limits/cancellation and raw colors retained. No recovered/translated source.

Same36-mesh before/after:36→36viewable GUIDs,15→23all-diffuse coverage.13assets
gain23solid groups;0newly verified complete. Native fez has4solid/0textured groups;
one capture inspected, secondary monitor selected before visibility. Color calibration,
alpha/pose/lighting/full shader correctness remain UNKNOWN. No original inputs
modified/assets or keys bundled/publishing. [Evidence/contract](docs/solid-color-preview.md).
## 2026-10-06 — D48 static sampler matrix behavior

Question: which unsupported subtype-one diffuse samplers can be previewed without
animation evaluation? Pinned Toolkit GfxAssembler::resolve static TEXTURE_SAMPLE
branch uses coefficients parameters0/1/2 and3/6/7 for the two UV row dot products;
parameters4/5 retain channel/slot roles. Independent arithmetic implementation;
no shader source copied/translated/executed. Pin/source URL, evidence labels,
applicability, conflicts and coverage: [docs/matrix-preview.md](docs/matrix-preview.md).
Three roots observed in existing80metadata records: one static, two animated.
Named/keyed animation stays unsupported. Reused48/32 material oracle records give
zero mismatches,36/24eligible (previous36/23). Tutorial floor0001a560 gains one
968triangle BC3 stitching group; four multiply routes remain neutral. Before/after
same37mesh hashes/37GUIDs remain37viewable/23all-diffuse/0verifiedComplete. Added
mesh selected through catalog linkage, without external geometry oracle. Original
36sample cohort unchanged. Private before/after reports and one native capture in
local-analysis; second-monitor-before-visible log. No decryption/recovered code,
original writes or proprietary asset/key bundling; game GPU/pixel parity unknown.
