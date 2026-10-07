# Asset Explorer architecture decision

D59 BorderDomain retains original BORDER axes and whole-channel sampling UV range
bits. Nearest alpha MIX only,validated1/4096interior margin before texture decode;
display axes use equivalent ClampToEdge without folding UVs or choosing bordercolor.
No new buffer/shader/device feature. Original/display metadata and app policy label
remain explicit;Complete still requires review. [Evidence](interior-border-preview.md).

D58 AlphaDiffuseMix stores fixed same-sample alpha factor metadata separately
from constant DiffuseBlend. Existing derived pixel buffer bakes nearest texels;
both frontends explicitly select nearest filtering.192KiB heap LUT/2KiB transfer
table,existing16MiB output;no shader or source-budget change. Local BORDER wrap
remains unsupported,so metadata selection unlocks0sampled assets.
[Evidence](alpha-mix-preview.md).

D57 extends static MIX to COLOR plus one sampler,using DiffuseBlend and the
existing derived diffuse buffer. No second sampler/multiply index;original
pixels/UVs remain intact.1024byte LUT,existing16MiB output budget,cancellation
and both frontend display paths reused. Viewable/Complete counts stay separate
from preparation coverage. [Evidence](single-mix-preview.md).

D56 adds an opt-in local animation base snapshot for direct matrix diffuse samplers.
Snapshot metadata retains original static parameters, local row bits and key counts;
derived UVs reuse existing bounded transforms. Default and legacy loading remain
static-only. Bevy mode changes reload and clear session verification; snapshot
outcomes display separately and cannot be confirmed Complete. No motion or game
time inferred, no new shader/resource budget. [Evidence](animation-base-preview.md).

D55 DiffuseBlend retains static MIX index/color/weight bits/input direction for
one COLOR and one two-texture grid product. Baker performs product/mix in linear
space before encoding,with a bounded192KiB lookup table;no extra image/source/UV
array or GPU shader change. Existing nearest-grid frontend support and separate
Complete review remain. Instance ThingColor/named animation remain unsupported.
[Evidence](static-mix-preview.md).

D54 Asset stores typed U/V TextureWrapping rather than one repeat boolean.
Single/tinted bindings preserve each axis independently; Bevy maps to separate
sampler modes. Legacy eframe rejects mixed axes while retaining geometry.
Multi-texture products still require Repeat on both axes; budgets unchanged.
UI labels each axis and keeps Complete confirmation separate.
[Evidence](axis-wrap-preview.md).

D53 adds a fourth original texture/key for a fixed three-multiply chain with
exact first/fourth and second/third coordinate sharing. GPU fourth sample reuses
primary coordinates, no additional UV arrays; cleanup owns all four images.
Four-source read/pixel ceilings and source/cycle guards remain explicit. UI labels
four samplers and separates full diffuse coverage from Complete.
[Evidence](four-product-preview.md).

D52 retains a third original texture/key only for a bounded three-leaf nested
product whose third sampler shares exact original channel/transform with the
secondary sampler. Bevy's optional third GPU sample reuses the secondary attribute;
all three GPU images are removed on replacement. No additional coordinates or
parser/UI coupling. Source/pixel ceilings now cover three textures; manual Complete
confirmation and legacy geometry fallback retained. [Evidence](nested-preview.md).

D51 each sampler selects/transforms its own original UV0/UV1 channel. Matching
channels remain required for periodic grid baking; other pairs use existing dual
sampling. GPU adapter separately validates secondary coordinate correspondence
without changing primary UV bits/indices. UI labels both original channels.
Memory/source/pixel/shader budgets unchanged; legacy fallback retained.
[Evidence](channel-pair-preview.md).

D50 DualAsset stores a second original texture/key and independently derived UVs.
Bevy feeds primary/secondary coordinates into GPU UV0/UV1 attributes and multiplies
two filtered samples with an authored unlit ExtendedMaterial extension. Original
channel identities/raw matrices remain CPU metadata. D49 grids retain nearest;
dual samples use linear. Scene cleanup owns both GPU images, mesh and material;
legacy eframe explicitly retains geometry without this binding. No parser/render
dependency introduced. [Bounds, API pin and scope](dual-preview.md).

D49 adds optional ProductAsset to a textured binding, retaining both original
textures/keys and a separate bounded nearest-grid product. Original sampler bits
remain in DiffuseSelection/second_sample; product display consumes raw UVs, source
routes consume derived UVs. Display extents/pixels methods separate original
texture metadata from product grids. Both frontends select nearest for products;
single textures remain linear. Parser stays UI-free. Eight-binding memory/read
ceilings now account for two sources; [policy and evidence](product-preview.md).

Decision date: 2026-10-05. A1/A2 established four crates; B1 adds catalog/viewer;
B2 splits shared lbp-io when it gains its second consumer.

## Scope and crate boundaries

D47 preview IR separates textured MaterialAsset from SolidAsset. The solid variant
retains material identity/raw color bits and has no texture/UV fields; loaders return
it without texture resolution/UV decoding. Renderers handle it explicitly, and the
Bevy session ledger counts textured versus supported solid groups separately.
All-diffuse coverage enables review; Complete remains visual confirmation.
[Contract and measured coverage](solid-color-preview.md).

The launcher's Bevy frontend owns a source-session compatibility ledger, separate
from parser/scanner metadata: Viewable means scene geometry installed; Complete
requires visual confirmation of all textures and correct geometry. Neutral groups
and skipped geometry prevent confirmation. Distinct attempted GUID totals include
complete in viewable, retain failed/unchecked distinctions, and do not depend on
search filters. Reload replaces evidence; source changes/restart clear it. This is
not a whole-corpus census. See [status policy](bevy-preview.md#compatibility-status-and-test-placement--2026-10-06).

Start with four crates; create further crates when real consumers need them:

| Crate | Responsibility | Depends on |
|---|---|---|
| lbp-core | GUID/hash newtypes, resource descriptors, evidence/confidence, diagnostic categories, build identity and plain intermediate structures | no UI |
| lbp-archive | FARC table reader and MAP reader; bounded reads, raw rows/aliases; later FAR4 only if relevant accessible inputs arrive | core, standard Read/Seek |
| lbp-resource | magic/type/header/revision inspection, dependency descriptors, shared ordinary-binary payload handling and GMT diagnostic prefix | core, serde, flate2; byte slices/readers, no disk path/UI assumptions |
| lbp-cli | game-directory discovery, SFO identity, scan orchestration, deterministic terminal/JSON output and errors | the three libraries |
| lbp-io | shared extracted-directory discovery, safe read-only opens, output containment and SFO identity | core, serde; no UI |
| lbp-catalog | bounded snapshot projection/indexes, source binding with existing readers, approved byte ranges and stable bounded hex reads | serde/json, io, archive, resource, core, SHA-256; no UI |
| lbp-viewer | native metadata/entry/hex, texture/mesh previews, material prefix/slot browsing, cancellable workers, virtualized lists and QA capture | catalog, resource, texture, mesh, eframe/egui with wgpu, image for QA PNGs |
| lbp-texture | bounded clear GTF/TEX BC1/BC3 top-mip CPU decoding and raw header metadata | serde, flate2; no UI |
| lbp-mesh | bounded clear MSH prefix and stored position/topology IR, diagnostic metadata | core, resource, serde, SHA-256; flate2 used by synthetic tests; no UI |

`lbp-io` now contains the source/SFO modules formerly local to CLI. B2 uses the same
readers and containment logic through catalog attachment, avoiding duplicate format
parsers. ISO and overlay support remain deferred. No library depends on GUI.
`lbp-texture` and `lbp-mesh` now have real consumers; `lbp-level` remains a later
crate. The original policy avoided empty
crate proliferation now. Texture and mesh modules can first live under resource;
split when codec dependencies and IR reuse justify it. Defer level parsing until
resource references and simple visual assets are reliable.

```text
DirectorySource -> SFO identity + MAP rows + FARC entries + loose-file catalog
                           |
                   resolver/catalog
               GUID -> MAP aliases -> SHA1 -> candidate locations
                           |
            bounded resource header/dependency inspector
                           |
              plain metadata and diagnostic IR
                    /                  \
             CLI / JSON          later viewer
                                     |
                          texture/mesh IR -> GPU adapter
```

FARC and MAP are complementary, not two interchangeable resource archives. GUIDs
name logical resources; content SHA-1 keys locate bytes. One content hash can have
many MAP paths. No parser library owns a graphics device or egui context.

## Scanner contract

Accept `--game-dir PATH`; normalize root case and nested PS3_GAME/USRDIR discovery
without changing input. File handles are read-only. Disallow symlink traversal that
escapes the selected source root; never execute scripts or launch EBOOT. An output
path must not resolve inside the selected game source or overwrite an input.

Phase 1: build metadata, container table inventory, aliases and loose path matches.
Phase 2: cheap bounded resource headers and dependency tables, with no payload decode
unless the supported wrapper requires it. A1 explicitly supports local MAP 0x100/FARC;
A2 adds clear binary headers through locally observed revision 951. Other revisions are errors or
per-resource unsupported diagnostics. Resource version is a per-resource value.

Use a stable schema version and sort by source-relative archive path, SHA1, GUID,
and alias path. Emit identifiers as fixed-width hex strings (unsigned GUID 8 hex
digits, SHA1 40), integer offsets/sizes, raw and normalized type, revision/branch,
resolution state and reasons. Separate filesystem file counts, MAP row counts,
distinct content counts, and header-confirmed type counts.

Statuses: known/parsed, identified-but-unparsed, unknown-magic, unsupported-revision,
missing, ambiguous, malformed, blocked-encrypted. Dependencies distinguish no table,
not inspected, parsed empty, and parsed edges. Include cycle-safe forward/reverse
queries later; do not recursively decode entire levels during a scan.

Global I/O failures return failure; partial scans retain explicit failures and an
appropriate non-success/partial outcome. Expose limits on table entries, read sizes,
decompressed bytes, traversal depth, and total work. No auto-fetching missing assets.
JSON writes should be atomic with explicit overwrite policy. Build identity initially
means SFO values and useful small metadata fingerprints, not whole-disc authentication.

## GUI stack evaluation

| Stack | Advantages for this app | Costs | Decision |
|---|---|---|---|
| wgpu + winit + egui directly | Full viewport/event control, straightforward custom mesh rendering | Own event loop, surface lifecycle, egui integration; unnecessary setup for metadata browser | Good underlying technology, defer direct host plumbing |
| eframe/egui with wgpu backend | Existing native egui host, metadata/tree/table UI, background jobs; custom wgpu paint callbacks allow later 3D viewport | Custom 3D renderer still required; large resource tables need virtualization | **Recommended for B/C** |
| Bevy + egui | Scene graph, asset scheduling, cameras and ready 3D rendering; valuable for later gameplay runtime | ECS/engine lifetime and integration-version churn for a diagnostic browser | D41 recommendation: staged 3D frontend after D40; keep parsers independent |
| Slint or other desktop UI toolkit | Strong declarative UI and widgets | Extra bridge for a custom debug 3D viewport; no observed project-specific benefit | No present reason to choose |

Official references inspected: [eframe](https://docs.rs/eframe/latest/eframe/),
[egui](https://github.com/emilk/egui), [Bevy introduction](https://bevy.org/learn/quick-start/introduction/).
Choose compatible stable versions at implementation time and pin Cargo.lock;
do not encode current ecosystem versions into this long-lived design.

The recommendation is an engineering inference from the app's needs. No GPU stack
or native GUI was installed during reconnaissance. B1 now builds/renders the native
wgpu GUI on the local GNU toolchain; verification is in docs/viewer.md. Milestone B needs
virtualized 30k+ row browsing, cancellable workers, lazy loading and bounded caches.
Milestone C first renders decoded 2D textures; a shaded LBP material interpreter is
not necessary. Future mesh rendering consumes standard positions/normals/UVs/indices
and material metadata, not raw serialized format structures.

## Licensing and implementation choice

Do not adopt cwlib-rs wholesale: incomplete and no explicit license found. Java
Toolkit is the strongest permissively licensed reference, but translating every class
would inherit too much scope and dependencies. Implement only proven required readers,
preserve notices for adapted code, and compare output through a pinned external oracle.
The reconnaissance pass created no Rust files. The continuation implements only A1.

## A1 implemented design

- Four crates; serde/serde_json for versioned output, SHA-256 for small metadata
  fingerprints, tempfile for atomic no-overwrite JSON publication. No rendering dependencies.
- Strict bounded MAP 0x100 parser and seek-based FARC footer reader. Preserve all
  rows, duplicate GUIDs/hash candidates and aliases, without choosing overlay precedence.
- SFO reader validates tables and known scalar encodings, emits only selected build
  identity fields, and distinguishes metadata fingerprints from full asset verification.
- Input may be a disc root or PS3_GAME root. Enumerate metadata inside PS3_GAME,
  skip nested symlinks/reparse points, and read only SFO, MAP and FARC tables.
- Keep type hints explicitly extension-derived. Resource headers/dependencies are
  `not_inspected` in A1; lbp-resource contains the type-hint boundary only.
- Deterministic arrays/keys, no absolute source path, timings or current timestamp
  in JSON. Report archive candidates, declared/actual size mismatches, unresolved
  external rows, duplicate IDs and skipped filesystem entries.
- Never write into selected source; create-new atomic output only. CLI errors fail;
  unresolved catalog references are reported without pretending they are corruption.
- Synthetic parser and CLI tests, then two local scans plus full field comparisons
  against the earlier independent Python table probe. Actual Java oracle belongs to A2.

No MSVC installation was found. For this all-Rust CLI, install a project-local GNU
Rust toolchain with its bundled linker rather than a system-wide Visual Studio install.
MSVC remains the preferred target to evaluate before native GUI/C dependencies.
The B1 eframe/wgpu dependency tree now builds on GNU without installing MSVC;
this resolves the immediate GUI toolchain question, not other-platform portability.
Rustup supports both Windows ABIs; see the
[official Windows notes](https://rust-lang.github.io/rustup/installation/windows.html).
## C1-C3 texture boundary

C1's texture_census lives only in lbp-catalog/examples and uses the existing
validated attachment. flate2 is a dev dependency for diagnostic prefix inflation;
the catalog/UI product has no texture decoder. Java Toolkit stays an ignored
external oracle. No format-specific GPU dependency was added.

C2 introduces lbp-texture for bounded wrapper/chunk parsing and independent
BC1 CPU RGBA output, initially simple GTF top mip. It accepts resource bytes
or a bounded read callback, without depending on catalog, egui or wgpu. Viewer workers
obtain approved candidate bytes and call the decoder; the UI owns GPU upload/cache.
Raw dimensions/flags and unsupported layouts are preserved;
unknown mip tails remain unparsed. Higher mips require a trustworthy layout oracle.
See texture-preview.md, textures.md and roadmap.md for scope and acceptance criteria.

C3 reuses shared bounded chunk inflation and BC1 pixels for TEX/DDS top mips.
DdsHeader preserves raw fields and zero mip declarations; optional GTF metadata
remains absent for TEX. CPU IR still has no GUI/GPU dependency. Native preview
uses the existing single-resource worker/cache path; [C3 contract](tex-preview.md).

C4 adds BC3 in the same CPU crate, explicit BlockFormat and format-correct labels.
No new UI/rendering dependency; [BC3 evidence](bc3-preview.md).

D1 adds lbp-mesh for independent MSH prefix diagnostics, depending on resource/core,
flate2/serde/hash only. Catalog examples use it as a dev dependency; no viewer
mesh dependency yet. Disk prefix metadata precedes future geometry IR;
[verified contract](mesh-prefix.md).

D2 makes lbp-mesh a viewer dependency and adds non-serializable CPU Geometry IR.
MeshPrefix/GeometryMetadata export diagnostic fields/digests only; original positions
and triangles are not serialized as fixtures/assets. Native egui wireframe owns
projection and draws through existing wgpu UI, with no format logic in GPU code.
No new engine or renderer dependency. Independent job/cache is source-bound and
cancellable; parser and geometry remain free of UI. Mesh contract: mesh-geometry.md.

D3 material navigation uses catalog GUID/hash indexes with separate candidate groups
and existing navigation history; no material codec/renderer dependency. Metadata
census/oracle remains diagnostic-only. See material-navigation.md.

D4 material-prefix oracle remains an ignored external Java execution with authored
adapter and metadata consistency checks; no new product crate/dependency. A future
prefix codec should share bounded binary inflation through lbp-resource, not depend
on lbp-mesh. Contract and explicit unknown tails: material-prefix.md.

D5 implements the small material prefix directly in lbp-resource::material. Shared
binary inflation now lives in lbp-resource::binary and serves mesh/material without
UI/catalog dependence. No new material crate until a substantial graph codec/IR
justifies it. Only diagnostic scalar/interval/descriptor/wrap metadata escapes;
opaque bytes are skipped. Native material UI remains next. See material-prefix.md.

D6 adds native MaterialView with one source-bound cancellable metadata worker/cache,
direct lbp-resource dependency and shared UI reference candidate grouping. All I/O
uses attachment's validated read-only pages. Prefix types remain diagnostic structs;
GPU/shader/material graph representation is absent. See material-viewer.md.

D8 optional material::inspect_graph remains inside lbp-resource, with diagnostic
box/wire/embedded-animation structs, no graph execution or renderer dependencies.
One bounded prefix/graph expansion; prefix API/schema/UI unchanged. SHA-256 hashes
component bytes instead of returning animation keys. Future graph UI consumes raw
metadata/range flags via existing attachment workers. See material-graph.md.

D9 integrates optional graph diagnostics in MaterialView without changing parsing
library APIs. One validated read worker; prefix and optional graph expansion are
sequential and bounded. Separate outcomes preserve prefix on graph failure. Cache
reset binds metadata to source/selection; graph-only cancellation retains cached
prefix, graph preference survives reset. Selected-row metadata and virtualized
lists consume diagnostic structs, with no renderer/shader graph representation.
Slot navigation shares separate GUID/hash candidate menus. See material-graph.md.

D10 changes external oracle diagnostics only. Proposed D11 optional material tail
API stays in lbp-resource, sharing bounded prefix/graph/animation metadata readers
without a renderer dependency. Existing APIs/schemas remain stable. Keep raw sound
and names/bits/digests, array counts and residual bytes; no animation key output,
sound simulation or semantic binding. Native integration follows parity in a
separate milestone. See material-tail.md.

D11 implements optional inspect_tail inside lbp-resource. Graph internal decoder
returns its bounded expanded payload for tail continuation; no duplicate expansion
or parser/UI coupling. TailInspection flattens unchanged prefix/graph metadata and
adds raw sound/parameter-animation metadata/residual counts. Shared animation
helper returns hashes/counts rather than key data. Existing APIs remain stage-specific;
GUI still uses prior graph API. Native optional tail integration is next. See
material-tail.md.

D12 adds native tail mode to the existing MaterialView source worker/cache. Parse
requested deepest stage first; success expands once, failure falls back through
bounded earlier APIs. Independent errors preserve valid prefix/graph metadata.
UI presents diagnostic array metadata and raw sound; no sound/animation/GPU binding.
Mode preference survives reset; cached earlier stages survive deeper cancellation.
See material-tail.md. Next is bounded coverage measurement using existing parsers.

D13 adds an authored headless coverage example, using existing catalog attachment
and material APIs without new format logic. Exact snapshot path/offset/size keys
retain all identity observations/aliases/conflicts; hint-only records stay unconfirmed.
Version1private diagnostic JSON contains compact per-stage metadata and distributions,
with count/resource/aggregate/deadline/cancellation limits. No GUI/library change.
Follow-up sample selection consumes metadata only; see material-census.md.


D16 keeps material graph layout/routing/hit testing in the viewer's material/diagram
module. Existing plain parser metadata is unchanged; shared parser count caps are
publicly reexported for UI budgets. No dependency or GPU/parser boundary changes.
Diagnostic grid positions are GUI state, not original format interpretation.
Verification and next cached JSON export: material-diagram.md.


D17 adds a UI-only cached material diagnostic envelope/private output module.
It reads no source bytes and invokes no parser. Versioned JSON preserves stage
availability independently of current UI mode; stable error categories avoid host
path leakage. Exclusive canonical local-analysis output and8MiB serialization
budget. No product dependency change. Second-consumer texture export should factor
these output safeguards rather than duplicate them; see material-export.md.


D18 factors app::diagnostics for the second cached-export consumer. It owns private
canonical/exclusive/bounded JSON output and stable stage/location/error helpers;
resource envelopes stay under their Material/Texture UI modules. Texture projection
explicitly whitelists header and byte counts, independent of decoded pixels/GPU
state. No parser/dependency changes. Third-consumer mesh summaries are next;
verification/scope: texture-export.md.


D19 adds the third cached diagnostic consumer under mesh/export. Explicit prefix/
primitive/topology field projection excludes CPU geometry arrays, leaving Geometry
non-serializable. Vertex count is a cache summary; all remaining fields come from
existing diagnostic IR. Shared private writer unchanged, no parser/dependency work.
Next improve route history and transition cache invalidation; see mesh-export.md.

D20 centralizes checked transitions in app::navigation with bounded Back/Forward
route/tab visits. All resource-cache/hex cancellation happens on route change;
validated source attachment persists. Toolbar navigation runs before detail binding.
History stores no assets or camera/candidate state. See navigation.md; next add
keyboard access through the same transition helpers.

D21 handles exact Alt history shortcuts before panels bind selected resources,
consumes pressed events without repeat navigation and retains ordinary text-editing
keys. Reuses route/cache cancellation, no new dependency. Next expand supported
texture IR only after existing oracle/spec evidence; navigation-shortcuts.md.

D22 replaces the block-only Rust enum with PixelFormat including BGRA8. Texture
CPU decode adds documented row pitch and retains bounded RGBA IR; UI/GPU cache
and exported metadata use existing generic paths. BC serialized names unchanged;
legacy top_mip_block_bytes now also covers stored scanline bytes. No new dependency.
Known legacy DDS flags handled explicitly; contract/provenance: bgra-preview.md.


D23 prepares external animation prefix evidence; product unchanged. The small
forthcoming CPU diagnostic reader belongs under lbp-resource and reuses clear-wrapper/
integer guards. No animation engine/crate or GUI dependency needed. animation-prefix.md.


D24 adds CPU-only animation metadata under lbp-resource. Shared binary payload
guards reused; unsigned BE32/ULEB32 helper privately factored from authored Material
reader. Prefix IR contains no keyframe/blob bytes and no UI/filesystem dependency.
33animation/32material oracle records match. Native inspector next; animation-parser.md.


D25 adds app::animation cancellable prefix worker/cache and Animation tab for MAP
and entry routes. Source/route/snapshot reset and busy/capture polling integrated;
IR/parser untouched. Private QA logging projects cached metadata only. Shared
protection helper now accepts longer buffers by reading leading4bytes. Next fourth
cached diagnostic-export consumer; animation-inspector.md.


D26 adds app::animation::export, fourth cached metadata-only consumer of the shared
private writer. Explicit IR projection and schema stay UI-side; no source reads or
parser/rendering dependency changes. QA capture invokes production saver. Bounds,
outcomes and verification in animation-export.md.


D27 research favors future lbp-resource::plan bounded metadata rather than creating
lbp-level or a Thing engine immediately. Existing catalog dependencies already serve
level reference browsing. Plan envelope IR would retain raw inner revision and opaque
array bounds/hash; UI and payload bytes remain separate. External oracle next,
implementation gated on evidence. level-plan-recon.md.


D28 external bounded plan-prefix oracle now defines a CPU-only diagnostic IR candidate:
rawinnerrevision, opaqueThing array bounds/hash and trailinginventory bytes. No
Thing graph engine or separate lbp-level needed for this slice. Independent reader
next, UI later. FullRPlan not invoked; exact primitive coverage in plan-prefix.md.


D29 adds CPU-only lbp-resource::plan, existing wrapper/integer helpers and sha2 reused.
IR has metadata/digest only, no opaque byte buffer or UI/filesystem dependency.
Read-only catalog parity example validates48externaloracle records. Nativeprefix
inspector next; no separate Thing engine/lbp-level. plan-parser.md.


D30 adds app::plan bounded validatedsource worker/cache and Plan tab on MAP/entry
routes. Source/route/snapshot reset and busy/capturepolling integrated. UI displays
CPU IR and digest; parsing remains UI-independent. Private QAlogs project cache
without another source read. Next shared privateexportconsumer. plan-inspector.md.


D31 adds app::plan::export, fifth metadata-only sharedprivate writer consumer. UI-side
explicit IR projection/schema; no filesystem/parser dependencies introduced to CPU
library. NativeQA invokes productioncachedsave. Optionalinventory diagnostic stage
may follow only after pinned-reader evidence; prefix retained on deeper failures.
plan-export.md.


D32 favors optional CPUplaninventory diagnosticstage rather than fullThingengine:
rawencoded64hex/rawuint32 fields andprefixextents, no recursive refs oropaque buffers.
ExistingPlanPrefix remainsseparate/available on deeperfailure. Pinned SlotIDclass/
primitives supplybounded externaloracle; independentCPUstage next. plan-inventory-prefix.md.


D33 factors privatePlanprefixhelper and adds CPUoptionalcombined inventoryinspection.
One boundedexpandedbuffer/digest, no publicopaque bytes; validbase anddeeperResult
remain separate. ExistingbaseAPI/schema unchanged. UI/native optionalmode next,
version2export afterwards. plan-inventory-parser.md.


D34 adds optional Plan inventory inspection using the combined CPU stage after one
bounded source read. Separate deeper errors preserve the valid base cache; cancellation
and navigation/source resets clear both. UI uses raw IR only. Version1 cached export
remains base-only; explicit version2 stage projection follows. No CPU/UI dependency
inversion or new crate. See plan-inventory-inspector.md.


D35 upgrades the Plan UI-side cached exporter to version2, separate base/inventory
stage outcomes and explicit scalar projections. Shared privatewriter unchanged,
no parser calls or original rereads. Historical version1 schema retained. Next
MAP-list sorting changes presentation order while preserving catalog indices.
plan-export-v2.md.


D36 adds UI-only resource_list ordering over filtered catalog indices, cached by
filter/sort and invalidated on snapshot replacement. No snapshot mutations, source
reads, parser coupling or new dependencies. Selection/history/candidates retain
original indices. Next analogous archive-entry presentation. resource-sorting.md.


D37 adds UI-only entry_order over original archive/entry pairs, filter/sort caching
and snapshot invalidation. Global keys use archive/table ties; no source or parser
calls. MAP/entry sort preferences stay independent. Next literal metadata-prefix
folder navigation should remain separate from filesystem resolution. archive-sorting.md.


D38 adds UI-only one-level folder metadata projection over existing filtered indices,
with bounded link lengths/depth, no filesystem resolution and no catalog mutation.
Folder/sort/filter cache invalidation and navigation-reveal/reset policy explicit.
Next optional mesh surfaces should consume verified CPU IR without parser changes.
folder-browsing.md.


D39 adds app::mesh::surface, a UI-only bounded projection/flatshading/depthordering
consumer of existing verified Geometry IR. Single untextured egui trianglemesh through
existingwgpu; no parser/source/material dependencies. Wireframe retained, explicit
occlusionlimits, metadataexport invariant. Next focus/preset controls. mesh-surface.md.

D40 supersedes focus/preset polish with a material-linked preview. `lbp-mesh::decode_uvs`
returns CPU arrays plus digest metadata, with no UI/serialization of asset arrays.
`app::mesh::linked` independently recognizes a narrow direct-diffuse graph, resolves
all catalog candidates without ambiguous selection, and orchestrates bounded existing
parsers/source access. Shared Arc catalog avoids cloning the large inventory. A linked
failure retains base geometry; UI owns GPU handles. Existing surface painter supplies
verified UVs/white tint using the existing egui/wgpu stack. Metadata export stays v1
and unchanged. Next preserve primitive ownership before multiple material bindings.
See [contract and provenance](textured-mesh.md).

2026-10-06 user steering: recommend a staged Bevy preview frontend as D41, building
on the verified D40 CPU assets. Extract shared preview orchestration at its second
consumer, add a Bevy renderer adapter and a real depth-tested perspective scene,
then migrate inspector panels through bevy_egui. Retain eframe during validation.
Implementation/build pending; multi-primitive bindings move to D42.
[Decision and acceptance target](bevy-transition.md).

## D41 implemented boundary — 2026-10-06

Two consumers now justify lbp-preview: CPU-only bounded mesh preparation and existing
D40 linked graph/UV/texture resolution, depending on catalog/resource/mesh/texture.
lbp-viewer delegates to it and retains existing UI/GPU/export ownership.
lbp-bevy-viewer depends on lbp-preview and Bevy/bevy_egui only at the frontend,
adapting plain CPU data to owned Mesh/Image/StandardMaterial and cancellable scene
lifecycle. No parser/library gains Bevy/egui dependencies. Existing read-only attachment
and protected outcomes remain unchanged. Native verification passed; see
[Bevy contract](bevy-preview.md). D42 extends primitive/material ownership, not runtime.

## D42 group boundary — 2026-10-06

Geometry retains nonserialized primitive triangle spans alongside flat geometry.
Shared lbp-preview returns cached Arc CPU bindings or explicit per-group failures,
with at most eight distinct material attempts. Bevy checks the complete partition,
groups equal bindings only for drawing, uses shared whole-mesh framing and owns
all generated GPU assets. Eframe metadata/export and single-primitive preview stay
available. Parsers/preview remain UI-free. [D42 contract](primitive-groups.md).

## D43 projection boundary — 2026-10-06

Shared lbp-preview selects one direct diffuse route, preserving selection/ignored
counts in CPU Asset. Other graph branches are never evaluated or loaded. Both UIs
label projection limits; Bevy still pools per-material group draws. Existing parser
metadata and geometry export stay unchanged. A read-only preview example compares
route metadata to bounded external helper evidence. [Contract](diffuse-projection.md).

## D44 explicit original UV channel — 2026-10-06

DiffuseSelection adds original uv_channel; CPU preparation checks/clones the chosen
verified array before texture loading and retains all-channel metadata. Renderer
sampling attributes receive that selected array without rewriting original UV IR.
Both UIs show identity; cache/group/export boundaries unchanged. Private census v2
adds channel metadata, not game arrays. [Contract](uv1-preview.md).


## Bevy browsing and development launcher — 2026-10-06

The Bevy frontend has a metadata-only, searchable, virtualized catalog menu.
Header/type hints identify unchecked mesh candidates, not proven renderability.
A source-scoped outcome cache records displayed geometry or the last failed
attempt; unsupported diffuse groups still count as geometry previews. Confirmed
non-mesh types and known non-MSHb wrappers have no current geometry handler.
A checkbox reveals all rows, including failed/unhandled rows, without deduplication.
Cancellation never installs an outcome. Source path changes invalidate the cache.
Catalog/source attachment completes independently of preview decoding, so an
unsupported first selection does not prevent browsing. The parser and shared CPU
preview crates have no new UI dependencies or format rules. The authored root CMD
launcher delegates to scripts/launch-viewer.ps1 and uses the existing ignored local
binary/data/toolchain; no proprietary bytes or game-specific keys are bundled.


## D45 separate static sampling coordinates — 2026-10-06

The CPU preview retains selected raw UVs and creates a distinct sampling array
using validated static scale/offset bits. Both display adapters consume that
array. Graph selection, transform bounds and cancellation precede texture decoding;
unsupported per-binding transforms retain neutral geometry. Mesh/UV IR and geometry
exports remain unchanged. Eight bindings add at most6.4MB of bounded sampling
arrays. UI labels original channel and raw/derived transform; no new parser or
shader evaluator. [Evidence and policy](affine-preview.md).
D48 SamplingTransform explicitly separates static scale/offset and 2×3 matrix
sampling. Both preserve raw UVs and create bounded cancellable sampling arrays;
display adapters consume the arrays and label the selected transform. Unsupported
animation/wired routes keep neutral geometry. [Evidence](matrix-preview.md).

D60: DualAsset owns typed secondary U/V wrapping; Bevy installs both samplers
independently. Only Repeat pairs enter the LCM grid. Product MIX/shared chains
stay Repeat-only. See [mirror-preview.md](mirror-preview.md).
