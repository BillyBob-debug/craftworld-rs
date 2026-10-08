# LBP2 asset knowledge map

D59 adds guarded interior-only nearest preview for source BORDER axes,not general
border-color behavior. Same39GUIDs:polaroid gains32triangles,+0newly viewable/
+0Complete;full-diffuse34→35. [Evidence and inference](interior-border-preview.md).

D58 supports fixed same-sample alpha-factor MIX metadata and nearest texel baking.
Local polaroid BORDER wrapping remains unsupported. Same39GUIDs:+0newly viewable/
+0Complete/0improved assets;full-diffuse34 unchanged. [Evidence](alpha-mix-preview.md).

D57 accepts a static COLOR/single-texture MIX using existing graph fields.
Same38GUIDs (prior37plus bouncepad):one232triangle group improved,+0newly viewable/
+0Complete;full-diffuse34 unchanged. [Evidence](single-mix-preview.md).

D56 reuses existing animation metadata for opt-in local base-value snapshots of
two named XYZ matrix rows. Game timing/global uniform resolution remain UNKNOWN.
Same37GUIDs:two groups/223triangles improved,+0newly viewable/+0Complete;
static full-diffuse34 unchanged,opt-in35. [Evidence](animation-base-preview.md).

D55 supports one static COLOR/two-texture product MIX using existing graph fields.
Same37GUIDs:lamp gains190triangle group,+0newly viewable/+0Complete;
all-diffuse33→34. Instance/named animated routes remain explicit unsupported.
[Rule evidence, policy, bounds and coverage](static-mix-preview.md).

D54 preserves independent existing wrapS/wrapT fields for static single/tinted
diffuse samples,accepting raw1/3 on each axis. Same37GUIDs:3assets/3groups/
81triangles improved,+0newly viewable/+0Complete;all-diffuse31→33.
[Sources, policy conflicts, scope and coverage](axis-wrap-preview.md).

D53 supports a fixed four-texture diffuse chain using two exact coordinate classes.
No new disk rule or recursive traversal. Same37GUIDs:handkerchief gains one group/
98triangles,+0newly viewable/+0Complete; all-diffuse30→31.
[Sources, bounds, scope and coverage](four-product-preview.md).

D52 accepts exactly two static diffuse multiplies with three distinct texture
leaves and exact coordinate sharing for the secondary/third samplers. No new disk
rule or general traversal. Same37GUIDs:1asset/2groups/160triangles improved,
+0newly viewable/+0Complete; all-diffuse30 unchanged.
[Sources, bounds and coverage](nested-preview.md).

D51 permits independent UV0/UV1 channel pairs for Bevy diffuse products. Existing
sampler selector and UV layout rules retained, no channel fallback or grid baking
across channels. Same37GUIDs:4assets/4groups improved,+0newly viewable/+0Complete;
all-diffuse26→30. [Evidence/scope/coverage](channel-pair-preview.md).

D50 independently samples bounded static matrix/affine pairs in Bevy using the
existing graph/UV rules. No new disk layout or coefficient snapping. Pinned rule
sources, targeted raw-bit parity, frontend scope and GPU unknowns:
[dual-preview.md](dual-preview.md). Same37GUIDs:1improved/1newgroup,
+0newly viewable/+0verifiedComplete; all-diffuse25→26.

D49 supports one bounded static two-texture MULTIPLY→DIFFUSE route. No new disk
layout: existing raw graph/descriptor/UV fields retain source provenance. Pinned
community multiplication behavior, local route/asset evidence, sRGB specification,
authored nearest-grid policy and unknown game fidelity are distinguished in
[product-preview.md](product-preview.md). Same37GUIDs:7assets/13groups improved,
+0newly viewable/+0verifiedComplete; all-diffuse coverage23→25.

Evidence snapshot: 2026-10-05. Labels refer to a specific claim, never blanket
correctness of a whole format. Local samples are this BCUS98245 disc only.

D47 supports static COLOR→DIFFUSE without inventing a texture or decoding UVs.
Existing raw node parameters/routes are CONFIRMED_FROM_COMMUNITY_CODE and
CONFIRMED_FROM_LOCAL_DATA on 23 additional material hashes. Same36-asset baseline:
13 improved assets/23color groups; +0newly viewable/+0verified complete; all-diffuse
coverage15→23. Original game calibration/fidelity remains UNKNOWN.
[Rules, limits and UI counting](solid-color-preview.md).

D46 extends existing graph behavior to one static TEXTURE_SAMPLE × COLOR diffuse
route. No new disk layout: indices/tint bits are CONFIRMED_FROM_COMMUNITY_CODE /
CONFIRMED_FROM_LOCAL_DATA on four newly eligible material hashes. Linear sRGB tint
baking is authored renderer policy; original game calibration/pixel fidelity remains
UNKNOWN. Measured asset delta: +0 viewable/+0 complete, three sampled assets with
six additional textured primitive groups. [Rules, provenance, bounds](tinted-preview.md).

D22 adds standard-mask32-bit TEX/DDS BGRA8 top-mip support. Mask/pitch rules are
CONFIRMED_FROM_DOCUMENTATION; one local sample's legacy LINEARSIZE convention and
canonical pixels are CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE.
Other masks, uncompressed GTF, higher mips and GPU fidelity remain unsupported or
UNKNOWN. Explicit conflict, bounds and provenance: [BGRA8](bgra-preview.md).

D4 adds external material-prefix evidence: [per-field rules and provenance](material-prefix.md).
32clear PS3 GMT prefixes have corroborated scalar bits, shader boundaries, texture
descriptors and wraps (CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA).
D5 independently reads this prefix, with32canonical external matches. Graphs, shader meaning and texture roles
remain UNKNOWN; header dependency order does not establish sampler roles.

D7 graph metadata evidence: [pinned layouts/conflicts/sample scope](material-graph.md).
Box/wire/embedded-animation layouts and explicit texture-slot links are
CONFIRMED_FROM_COMMUNITY_CODE, corroborated on32local materials. D8 independent
graph metadata reader matches all32canonical samples. Graph evaluation, shader meaning, rendering and animation playback
remain UNKNOWN. The Toolkit extreme signed-integer oracle bug is not adopted.

- `CONFIRMED_FROM_LOCAL_DATA`: measured bytes/structure in the local copy.
- `CONFIRMED_FROM_COMMUNITY_CODE`: reader implementation explicitly contains rule.
- `CONFIRMED_FROM_DOCUMENTATION`: published documentation describes rule.
- `STRONG_INFERENCE`: supported reasoning, not established format behavior.
- `UNKNOWN`: no sufficient evidence yet.

## Pipeline

A1 implements SFO/MAP/FARC; A2 now inspects magic, clear binary headers and dependency
tables, with location candidates and catalog links. See [A1 contract](scanner.md)
and [A2 contract/oracle](resource-metadata.md). Full local table field parity with
unmodified pinned Toolkit is established; 24 clear binary samples agree on headers
and normalized dependencies. C1 adds a bounded texture census and external top-mip
decode oracle: [texture evidence and conflicts](textures.md). C2 verifies 19 plain
GTF BC1 top mips and native preview: [contract](texture-preview.md). C3 verifies
15 TEX/DDS BC1 top mips, including raw DDS fields: [contract](tex-preview.md). Original renderer
fidelity, other variants and higher mips remain unverified.

PARAM.SFO: CONFIRMED_FROM_COMMUNITY_CODE and CONFIRMED_FROM_LOCAL_DATA. Toolkit
`types/ParamSFO.java::build` at d6a2ef80 establishes magic 00 50 53 46, LE version
0x101, LE key/value table offsets and record count (20-byte header), 16-byte records
with u16 key offset/format and u32 length/capacity/value offset. Offsets are relative
to their respective tables; formats 0x4 raw text, 0x204 NUL text, 0x404 LE integer.
Independent Rust reader validates bounds and exports only selected identity fields.
Its synthetic SFO recipe and all truncations are tested. Build fields are declared
metadata, not executable authenticity or verification of every asset.

```text
User-owned data [CONFIRMED_FROM_LOCAL_DATA]
  ZIP -> readable ISO9660 listing -> PS3_GAME/USRDIR
    DATA.FARC -> SHA1/offset/size index [CONFIRMED_FROM_LOCAL_DATA]
    OUTPUT/BLURAYGUIDS.MAP -> path/GUID/SHA1/size [CONFIRMED_FROM_LOCAL_DATA]
    loose GAMEDATA files [CONFIRMED_FROM_LOCAL_DATA]
       GUID -> MAP -> content SHA1 -> FARC or loose candidate
         [CONFIRMED_FROM_COMMUNITY_CODE; partial local join corroboration]
       resource magic + serialization method + revision
         [CONFIRMED_FROM_LOCAL_DATA for sampled clear binary/GTF resources]
       dependency table -> typed GUID and/or SHA1 references
         [CONFIRMED_FROM_LOCAL_DATA for sampled binary resources]
       plain payload / bounded chunked zlib stream
         [CONFIRMED_FROM_COMMUNITY_CODE; local bounded texture decompression verified]
          GTF/TEX -> BC1/BC3 top pixels [CONFIRMED_FROM_LOCAL_DATA; sample scope]
          MSH -> bounded stored geometry [CONFIRMED_FROM_LOCAL_DATA; 36 samples]
          SMH -> mesh IR          [COMMUNITY_CODE; local headers only]
          GMT/MAT -> materials    [COMMUNITY_CODE; local headers only]
          ANM -> animation        [COMMUNITY_CODE; local headers only]
          LVL/PLN -> things/parts [COMMUNITY_CODE; local references only]
          trans -> localization  [COMMUNITY_CODE; local raw prefix only]
          FSB/MP3/FEV -> audio    [LOCAL names; detailed compatibility UNKNOWN]
          FSH/shaders/SPU/etc.    [COMMUNITY type map; payloads UNKNOWN locally]
    DATA.SDAT NPD container [CONFIRMED_FROM_LOCAL_DATA marker]
       protected payload [UNKNOWN contents; stop at encryption boundary]
```

Abbreviated COMMUNITY_CODE/LOCAL labels in the payload branches above mean the full
`CONFIRMED_FROM_COMMUNITY_CODE` / `CONFIRMED_FROM_LOCAL_DATA` labels, respectively.
The scanner resolves references; it does not execute payloads or simulate the game.

## Sources and pins

Toolkit pin **d6a2ef80b5a09999f08f23a803b516096055e5c9** (MIT). All following Java
paths are under `lib/cwlib/src/main/java/cwlib/`. Use the pinned source, not an
unqualified moving branch:
[cwlib source tree](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib).

| Knowledge | Source class/method | Local corroboration |
|---|---|---|
| FARC footer and rows | `types/archives/FileArchive.java` constructor; `Fat.java`, `Fart.java` | full local index parse and range checks |
| MAP versioned rows | `types/databases/FileDB.java::process`; `FileDBRow.java` | full revision 0x100 MAP consumed with no trailing bytes |
| resource IDs | `types/data/GUID.java`, `SHA1.java`, `ResourceDescriptor.java`; `io/streams/MemoryInputStream.java::guid` | MAP GUID/SHA1 rows and binary dependencies |
| resource types/magic | `enums/ResourceType.java`; `types/data/ResourceInfo.java` | MATb/GMTb/LVLb/PLNb/ANMb/MSHb/SMHb/GTF samples |
| wrappers/revisions/dependencies | `types/SerializedResource.java::process`, `processDependencies`; `enums/SerializationType.java` | clear binary revisions through 951; 24 samples compared with actual Toolkit; D69: heads to 0x3f8 (LBP2_MAX), 66 post-951 01.33 samples match the Toolkit oracle |
| compression | `util/Compressor.java::decompressData`; `enums/CompressionFlags.java`; stream classes | no real payload decompression in this pass |
| textures | `resources/RTexture.java`, `structs/texture/CellGcmTexture.java`, `util/DDS.java`, `enums/CellGcmEnumForGtf.java` | GTF header dimensions only |
| meshes | `resources/RMesh.java::serialize`, `RStaticMesh.java`; `structs/mesh/`, `structs/staticmesh/`; `io/exports/MeshExporter.java` | binary headers and material dependencies only |
| materials | `resources/RGfxMaterial.java`, `RMaterial.java`; `structs/gmat/` | binary headers only |
| animations | `resources/RAnimation.java::serialize`; `structs/animation/` | ANMb header, not keyframe decode |
| levels/plans | `resources/RLevel.java`, `RPlan.java`, `RSlotList.java`, `RPacks.java`; `structs/things/Thing.java`, parts/components | LVLb/PLNb header dependencies only |
| translations | `resources/RTranslationTable.java` constructor | raw local prefix, not strings |
| resolution | `singleton/ResourceSystem.java`, `types/archives/FartManyRO.java`, FileDB lookup | hash join and case-insensitive loose-path candidates only |
| FAR4/save archives | `types/archives/SaveArchive.java`, `SaveKey.java` | no local sample |

The old [LBPModding/Documentation](https://github.com/LBPModding/Documentation)
tree contains only README; its locally available wiki has only Home.md. It is not a
complete public specification corpus. No private Discord knowledge is claimed.

## FARC: sufficient for bounded implementation now

`CONFIRMED_FROM_LOCAL_DATA` and independently `CONFIRMED_FROM_COMMUNITY_CODE`:

- Last 8 bytes: big-endian u32 entry count, then ASCII `FARC`.
- Table start `file_length - 8 - count * 28`.
- Each row: SHA1 bytes[20], big-endian u32 offset, big-endian u32 size.
- Offset is relative to the FARC start, not the ISO start. Archive stores content
  identifiers and byte locations; it does not contain the path/GUID catalog.
- All local row ranges end before table start. Zero duplicate hashes locally.
- 19 sampled resource bytes hash to their table SHA1; not every payload was hashed.

`CONFIRMED_FROM_DOCUMENTATION`:
[Imagisphere File Archive](https://wiki.imagisphere.me/Filetype:File_Archive)
documents row fields and big endian. **Conflict:** its prose places magic before
count, contrary to its own diagram. Its claim about reverse table ordering was not
established locally. Neither reversed order nor contiguous payloads should be required
for a reader; trust each checked row offset. Archive members can themselves be
compressed resources even though FARC does not wrap all content in one compressor.

Implement overflow/negative-count/truncation/range/duplicate handling more rigorously
than permissive oracle code. The synthetic tests should cover invalid footer, impossible
table count, out-of-range payload, empty archive, overlapping/alias policy and truncation.
Only record overlap as a diagnostic unless a stronger validity rule is demonstrated.

## MAP/FileDB: sufficient for local 0x100 variant

`CONFIRMED_FROM_LOCAL_DATA`: big-endian revision 0x100, u32 count; each row consists
of u32 UTF-8 path byte length, path bytes, 8-byte timestamp, u32 declared size,
20-byte SHA1, u32 GUID. Probe used signed i64 timestamps like Toolkit's `s64`;
timestamp meaning/epoch is **UNKNOWN** for our inventory. Keep raw values first.

`CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit switches to 16-bit path length and 32-bit
timestamps when `(revision >> 16) >= 0x148`; Vita extension-only names also exist.
Do not support those variants based solely on an LBP2 test. MAP size is declared
resource metadata, not necessarily physical protected-container size (SDAT differs).
GUID zero/null interpretation appears in the reader; preserve raw zero in inventory.
GUIDs are unsigned 32-bit logical identifiers, distinct from content hashes.

Preserve raw rows, GUID conflicts and hash aliases. Toolkit skips duplicate GUIDs;
an inspector should report them instead. Resource resolution priority for updates,
mods/saves/patch overlays and hash collisions is **UNKNOWN** for this build until
tested with explicit additional inputs; do not impose a speculative last-file-wins rule.

D69 update: the official installed update is now an explicit second input. Its
`USRDIR/output/brg_patch.map` (also 0x100) replaces disc rows by GUID; the disc rows are
kept as shadowed records. `STRONG_INFERENCE` from 01.01..01.33 local data, see
[update-overlay.md](update-overlay.md). Mods/saves precedence remains UNKNOWN.

## Resource wrappers and dependencies

`CONFIRMED_FROM_COMMUNITY_CODE`: resource wrappers commonly begin with three-byte
type magic and a fourth serialization character: `b` binary, `t` text, `e` encrypted,
space compressed texture, `s`/`S` swizzled texture variants. Raw DDS/PNG/JPEG and
extension-dispatched resources are special cases; not every file has this wrapper.

Clear binary revision is at byte 4, BE u32. For revision >=0x109 the dependency
table offset is at byte 8. Header behavior is versioned:

- Below 0x132 the reader treats payload as uncompressed.
- At/above 0x189 a compression-state byte is read for ordinary resources.
- At/above 0x271 branch ID and branch revision appear.
- At/above 0x297 (and a special Leerdammer branch case) compression flags appear.
- STATIC_MESH has its own header metadata instead of the ordinary branch/flag fields.
- Later custom `ALSR` trailing data exists in Toolkit; not locally verified or needed now.

These are source-derived branch rules, not an assertion of support across all builds.
Resource revision and branch must be preserved separately. `e` is a **blocked** marker:
do not call Toolkit's decrypting constructor on such inputs. No encryption support.

`CONFIRMED_FROM_LOCAL_DATA` for sampled clear binary resources: dependency table
at resource-relative offset contains BE u32 count, then flag byte per descriptor,
optional u32 GUID if flag bit 1 (`0x02`) set, optional SHA1[20] if flag bit 0 (`0x01`)
set, then BE u32 resource-type ID. Flags can be combined in community code; preserve
both IDs. Flag zero/null and unknown type must remain explicit. Header dependency
inspection does not require full Thing/mesh payload parsing or recursion.

Type mapping `CONFIRMED_FROM_COMMUNITY_CODE`, locally corroborated for sampled
families: `TEX/GTF` texture ID 1; `MSH` mesh 2; `ANM` animation 5; `GMT` graphics
material 7; `LVL` level 9; `FSH` script 11; `MAT` physics material 16; `PLN` plan 38;
`SMH` static mesh 44. Full enum includes game-version-specific and custom types;
Toolkit's custom BST resource is not evidence of an original LBP2 format.

## Plain compression and textures

`CONFIRMED_FROM_COMMUNITY_CODE`: the chunk wrapper is two BE u16 values (first
often 1, then chunk count), followed by `(compressed_size, decoded_size)` u16 pairs
and chunk bytes. Equal sizes mean stored bytes, otherwise zlib. Count zero has a
raw-data fallback in Toolkit. The first flag's complete meaning is **UNKNOWN**.
Implement exact-length checks and bounded decompression; do not copy permissive
inflate behavior or assume every chunk must be compressed.

`TEX ` payload commonly expands to a DDS file. `GTF ` includes CellGcmTexture
metadata before compression, then PS3 GPU texture data requiring layout/format
conversion. The sampled local UI/scratch/minimal textures are **GTF**, not TEX.
`CONFIRMED_FROM_LOCAL_DATA`: GTF format byte, mip/dimension/cubemap fields at bytes
4–7, BE width/height/depth at 12/14/16 agree with the community header layout.
Full swizzle/channel-remap/mip decode is not tested here.
D91 (`CONFIRMED_FROM_COMMUNITY_CODE`, oracle parity on 20 local samples): uncompressed
2D 0x85 A8R8G8B8 / 0x81 B8, swizzled or LN (0x20) linear, top mip only; B8 remap
0xaae4/0xa9ff as opaque gray. Rules and conflicts: [d91-gtf-uncompressed.md](d91-gtf-uncompressed.md).
D95: B8 pixel budget 16 Mi; BC1/BC3 cube maps decode six top-mip faces via the layered
API only (face order COMMUNITY_CODE, 128-byte face stride LOCAL, conflicts with Toolkit's
unpadded pass-through). [d95-texture-rest.md](d95-texture-rest.md).
D98 (`CONFIRMED_FROM_COMMUNITY_CODE`, no oracle: Toolkit ignores remap and has no volume
layout): uncompressed GTF applies the GCM remap word (bits 0..7 source selects A,R,G,B =
0..3 for outputs A,R,G,B; bits 8..15 modes 0 zero, 1 one, 2 remap), so B8 0x02ff is an
alpha mask (RGB 0); swizzled volumes interleave x, y, z bits (x lowest, exhausted axes
drop out), slice 0 as 2D. [d98-last-textures.md](d98-last-textures.md).

Best texture oracle: Toolkit RTexture + DDS helpers. External cross-checks:
[Refresh TexStream](https://github.com/LittleBigRefresh/Refresh/blob/25f03e40416ebe73016b9e9972e0fa7b11a7f0b0/Refresh.Core/Importing/TexStream.cs)
and `Refresh.Core/Importing/Gtf/{GtfHeader,GtfMetadata,GtfDecoder,GtfFormat,GtfPixelFormat}.cs`
(AGPL-3.0) support clearer streaming/texture inspection; no source incorporated.
[lbptoolspy-lite tex_tools](https://github.com/Zhaxxy/lbptoolspy-lite/blob/9e044e0f068d9f3f4db3608e8a4305e1f2f76ce9/src/lbptoolspy/tex_tools.py)
(GPL-3.0) only handles TEX->DDS and always zlib-decompresses chunks, so it is not a
general GTF oracle or an oracle for stored/equal-sized chunk cases.

Milestone C: start ordinary 2D PS3 GTF with a verified common pixel format; add TEX/DDS,
more formats, mips, remap/swizzle only as evidence/tests support them. Cubemap/volume,
animated textures and complete material shading are later.

## Mesh, material, animation, level and other payloads

`CONFIRMED_FROM_COMMUNITY_CODE`:

- RMesh reads vertex/index/edge/triangle counts, vertex streams, attribute bytes,
  primitives, bones, morphs, UV bounds and other versioned fields; MeshExporter emits
  OBJ/glTF. RStaticMesh is a distinct packed representation. Metadata exists to
  support an independent mesh IR, but local full decode remains **UNKNOWN**.
- RGfxMaterial handles graphics material structures; RMaterial handles physical
  properties. Parsing nodes/texture references is feasible; reproducing PS3 shaders
  or physical behavior is neither established nor part of the viewer scope.
- RAnimation covers frame count/FPS, bone masks, packed rotation/position/scale/morph
  data and locators. Playback/interpolation/axis fidelity needs targeted oracle work.
- RLevel/RPlan refer to Things, parts and world state, with revision-conditioned
  layouts. Slots/packs link story/inventory data. A binary-to-JSON reader is evidence
  of serialization knowledge, not a complete gameplay implementation.
- RTranslationTable has a binary count + `(u32 key, u32 string offset)` table and
  UTF-16/BOM delimited content, plus a legacy text variant. GenderFluid `Trans.cs`
  independently documents similar behavior but its delimiter condition needs care;
  prefer Toolkit and validate exact text boundaries rather than port that loop blindly.

`CONFIRMED_FROM_LOCAL_DATA`: loose FSB/FEV/MP3/BIK filenames and indexed shader/SPU/
script extensions. Exact audio codecs/bank versions, dialogue references, compiled
shader layouts, script data location and `.m`/`.mp` semantics remain **UNKNOWN**.
No scripts executed. FMOD and Bink are diagnostic identities, not planned bundled runtimes.

## Explicit unknowns / boundaries

- Internal mastering/build revision beyond SFO fields; complete disc authenticity.
- Other ISO filesystem views; multi-extent extraction; all-platform/archive variants.
- DATA.SDAT contents; prefix is `NPD\0`, still reported as protected by current
  readers. Current AGENTS.md permits scoped local decryption research if needed;
  its relationship to missing scripts remains unconfirmed. No decryption performed.
- Why 933 MAP external entries lack a matching loose ISO path; update/region variants.
- Archive/loose precedence and patch overlays, saves/FAR4 and GUID substitution rules.
- Texture variants beyond C4's plain GTF/TEX BC1/BC3 top mip; higher mip/tail layout and
  full-corpus compatibility. C2-C4 establish sample-scoped Rust/Toolkit pixel parity.
- Full local mesh/animation/material/level decode and rendering agreement.
- Game logic, physics, multiplayer and original renderer behavior: intentionally deferred.

## Required provenance block for future parsers

```text
Format rule: <field/layout and supported revision>
Evidence: <one of the five labels>
Source: <repository URL, full commit, path, method>
Local corroboration: <build, sample category, check; metadata only>
Conflicts/limitations: <variant exclusions or disputed claim>
Reuse: <independent spec implementation / licensed adaptation + notices>
Verification: <synthetic test, oracle version, comparisons actually run>
```

C4 BC3 rule/oracle conflict and sample scope: [bc3-preview.md](bc3-preview.md).

D1 prefix fields/provenance/test scope: [mesh-prefix.md](mesh-prefix.md). Topology
and full mesh compatibility remain UNKNOWN locally.

D2 geometry/topology/material descriptor and bone traversal evidence: [mesh-geometry.md](mesh-geometry.md).
Full posed mesh, material shading and tail semantics remain UNKNOWN.

D3 graphics-material header/dependency census and identity navigation: [material-navigation.md](material-navigation.md).
Slot roles and material/shader payload semantics remain UNKNOWN locally.

## D9 presentation evidence — 2026-10-05

Native optional material graph tables corroborate D8 metadata for one local head935
sample (5boxes/4wires/8slots). CONFIRMED_FROM_LOCAL_DATA for native presentation;
no new disk-layout claims. OUTPUT/TEXTURE_SAMPLE labels are
CONFIRMED_FROM_COMMUNITY_CODE from pinned Toolkit BoxType, shown as community
annotations; all other kinds/ports retain raw values. Prefix remains available on
graph failure. Unknown tail/roles/execution semantics unchanged. Test/reproduction
scope and next external tail-oracle investigation: material-graph.md.

## D10 clear material tail evidence — 2026-10-05

CONFIRMED_FROM_COMMUNITY_CODE: after graph, raw sound integer (head>=0x15b) then
counted inline MaterialParameterAnimation array (head>=0x2a2), pinned Toolkit
RGfxMaterial::serialize / Revisions / Serializer::array. Current scope branch-zero
PS3 heads920–951 only. CONFIRMED_FROM_LOCAL_DATA: all32selected tails accounted
for with zero residual bytes;30empty arrays,2single241-key animations, all sound0.
This supersedes the selected-sample UNKNOWN residual classification from D7–D9;
Rust graph reader/UI still report those tails unparsed until an optional extension.
Sound meaning and animation binding/timing/interpolation remain UNKNOWN. No
full-corpus or visual correctness inference. Exact provenance and coverage:
[Material tail evidence](material-tail.md).

## D11 independent tail metadata parity — 2026-10-05

Documented sound/parameter-animation tail layout now has an optional independent
Rust reader, inspect_tail. CONFIRMED_FROM_COMMUNITY_CODE rules unchanged, with
CONFIRMED_FROM_LOCAL_DATA canonical32sample corroboration including all previous
prefix/graph fields. Graph32/prefix32regression exact. Four synthetic cases cover
bounds, wrappers, unknowns and cancellation;85workspace tests pass. Sound/animation
semantics remain UNKNOWN, original shader bytes opaque. Prefix/graph APIs preserve
existing unparsed counts until callers explicitly request tail inspection. Source,
limits/test coverage and exact next native task: material-tail.md.

## D12 native tail corroboration — 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: native empty head935 and populated head949/mask4 plus
head945/mask8 tail presentations agree with D11 metadata. No additional layout
claim or parser change. Parameter animation values remain counts/digests only;
sound/animation semantics UNKNOWN.87Rust tests and three native captures pass.
Exact display/lifecycle limitations and next census contract: material-tail.md.

## D13 broader local material acceptance — 2026-10-05

CONFIRMED_FROM_LOCAL_DATA:5,163uniqueavailable clear material locations accept
existing prefix/graph/tail readers with no residual bytes.18revisions,28rawsound
values,185nonempty arrays/307records, maximum7records and1,143component values.
These distributions exceed the32sample oracle scope; they do not establish new
semantics or independently prove all reader results.40follow-up samples selected
for pinned oracle validation. Format rules unchanged; sound/animation/shader
meaning remains UNKNOWN. Coverage bounds, caveats and verification: material-census.md.

## D14 expanded sample corroboration — 2026-10-05

CONFIRMED_FROM_LOCAL_DATA:40additional pinned oracle samples match all independent
prefix/graph/tail fields; no changes to CONFIRMED_FROM_COMMUNITY_CODE rules.
Combined72unique samples cover all observed18revisions/28rawsoundvalues/7arraycount
cohorts in the selected snapshot,29of76graphcount shapes. Sound and animation/shader
semantics remain UNKNOWN. Largest observed component-array1143isnotyetinoracle
set(max603validated); next bounded remaining-shape/boundary follow-up. See
material-census.md for applicability, evidence and exact commands.


## D15 remaining count-shape corroboration — 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: 48 additional complete prefix/graph/tail oracle matches;
union 120 samples covering all 76 observed box/wire count pairs and maximum array
of 1,143 values. CONFIRMED_FROM_COMMUNITY_CODE rules/provenance unchanged. Count
pairs do not prove all topologies, materials or rendering; sound/animation/port/
shader semantics remain UNKNOWN. Applicability and tests: material-census.md.


## D16 material graph presentation - 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: native 5/19-box diagrams present previously validated
signed endpoints and raw kinds/ports/swizzle. No new CONFIRMED_FROM_COMMUNITY_CODE
rule. Authored grid layout does not infer serialized editor position meaning;
port/shader/animation/sound semantics remain UNKNOWN. See material-diagram.md.


## D17 cached diagnostic export - 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: exported cached head949 prefix/graph/tail fields match
existing pinned oracle metadata, including compression/expanded size. No new format
rule; existing CONFIRMED_FROM_COMMUNITY_CODE provenance unchanged. Version1 envelope
tracks cache availability/request/error state; it does not describe new disk layout.
Sound/shader/animation semantics remain UNKNOWN. See material-export.md.


## D18 cached texture metadata export - 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: native cached GTF/TEX BC3 export fields match prior
oracle-validated C4 parser metadata and all externally supplied header fields.
DDS raw mip zero retained. No new CONFIRMED_FROM_COMMUNITY_CODE/DOCUMENTATION
rule or pixel-format extension. Higher-mip/layout unknowns remain unchanged.
Exact comparison limits and schema: texture-export.md.


## D19 cached mesh summary export - 2026-10-05

CONFIRMED_FROM_LOCAL_DATA: native list/strip diagnostic summaries match existing
D2 parser metadata and pinned community oracle with exact IEEE bounds bits. Static
SMH remains unsupported/null rather than empty geometry. No new format rule;
CONFIRMED_FROM_COMMUNITY_CODE/DOCUMENTATION provenance and unknown tails/posing/
shading remain unchanged. Evidence and comparison limits: mesh-export.md.


## D23 animation metadata evidence

Modern ANMb scalar/count prefix and optional position/fat gates are
CONFIRMED_FROM_COMMUNITY_CODE, corroborated on33local prefixes and23guarded full-
reader comparisons.748locations have CONFIRMED_FROM_LOCAL_DATA ANMb magic; aliases
counted separately. Oldhead844, keyframe/locator semantics and playback remain
unsupported/UNKNOWN. [Evidence](animation-prefix.md).


## D24 implemented animation prefix

The independent modern ANMb prefix reader matches33local external-oracle records
across20revisions (CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE).
Frames/fps/loop/morph are raw fields; optional vectors raw IEEE bits. Old layouts,
keyframes, locator records and playback remain unparsed/UNKNOWN. [Contract](animation-parser.md).


## D25 native inspection

Animation UI reuses D24 prefix knowledge without new format rules. Raw fields and
unparsed bytes stay separate from payload compatibility. Existing known protected
signatures now classify consistently for longer preflight buffers, using leading
4bytes only; synthetic coverage, no protected asset experiment. animation-inspector.md.


## D26 Animation diagnostic envelope

Cached JSON schema version1 projects existing prefix knowledge only; no animation
payload export or new disk-format rule. Raw scalars/IEEE bits, absent nulls, opaque
byte counts and explicit unsupported outcomes preserved. animation-export.md.


## D27 levels and plans

CONFIRMED_FROM_LOCAL_DATA:282clearbinarylevel locations/45headercohorts and7342plan
locations/97cohorts in accessible corpus. Header dependency entries45946/47672 parsed;
these do not establish Thing graph relationships. CONFIRMED_FROM_COMMUNITY_CODE:
RLevel enters recursive worldThing reference; RPlan begins with inner revision and
opaque Thing byte array, followed by versioned inventory metadata. STRONG_INFERENCE:
bounded modern plan prefix useful before recursive levels. Local payload fields and
Thing semantics UNKNOWN. [Provenance and sample scope](level-plan-recon.md).


## D28 plan prefix evidence

CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA:48modernbranch0PLNb prefixes
using pinned unchanged i32/bytearray and validated clearwrapper. Rawinner, opaqueThing
array bounds/hash and inventorytail counts; no Thing or inventoryfield interpretation.
Allobserved innerheads matchouter but mismatch is not declaredcorrupt. Old/branch/
streaming/extra trailers unsupported by oracle scope. [Provenance](plan-prefix.md).


## D29 implemented plan prefix

CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE: independent ordinary modern
branch0PLNb prefix exactly matches48bounded pinned-reader records. Metadata/digest
only; inner raw revision, opaquearray extent, inventorytail lengths. Payload semantics
and fulllevel/Thing compatibility UNKNOWN. [Contract/tests](plan-parser.md).


## D30 native plan metadata

Native Plan view reuses D29prefixknowledge, no newdiskformatclaims. Rawinner/opaque
Thingdigest/bounds/tail byteextents remain distinct from parsedinventory/Things and
payload compatibility. Oldlayouts stay Unsupported. plan-inspector.md.


## D31 plan diagnostic envelope

Version1cached JSON projects existingverifiedplan prefix/hash/bounds only; no new
PLNblayout rule, Thing/inventory data export or compatibility claim. Rawinner and
unsupportedstates preserved. Sourcevalidation epoch explicit. plan-export.md.


## D32 plan inventory scalar prefix

CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA: boundedmoderninventory
scalarprefix at D28tailstart, throughdescriptionkey, beforeCreationHistory reference.
Rawdateencoded64/slot/sound/colour/type/subtype/keys andoffsets; actualSlotIDclass fields
agree on48localsamples. Authoredsynthetic evidence exposes pinnedcompact s64shift
conflict; rawencodedhex preserved. Datesemantics/translatedkeys/recursiveinventory
remain UNKNOWN. [Rules/conflict/coverage](plan-inventory-prefix.md).


## D33 implemented optional plan inventory prefix

CONFIRMED_FROM_LOCAL_DATA / CONFIRMED_FROM_COMMUNITY_CODE: independentrawinventory
fields/offsets allmatch48externalprimitive/SlotIDoracle records. Rawencoded64 preserves
highbit/dateconflict; no datetime/enum/reference interpretation. Baseprefix output
unchanged, deeperfailure explicit. [Contract/tests](plan-inventory-parser.md).


## D34 native optional inventory metadata

No new disk-format claim. CONFIRMED_FROM_LOCAL_DATA: native optional cache matches
all scalar/extent fields from the bounded D32 oracle on three supported selections.
Base-only output stays unchanged. Recursive metadata, date/localization semantics
and full plan compatibility remain UNKNOWN. [Verification](plan-inventory-inspector.md).


## D35 version2 Plan diagnostic envelope

No new disk-format rule. Cached raw inventory scalars/encodeddate/extentmetadata and
base prefix have separate requested/outcome states. Original payload bytes remain
excluded. Native projections corroborate existing D32 evidence; recursive and date/
title semantics remain UNKNOWN. [Contract](plan-export-v2.md).


## D39 neutral mesh surface visualization

No new disk-format claim. Generatedface normals/shading and camera depth are display
values from existingverifiedpositions/topology. No parsednormal/UV/material/bone/morph
semantics. Existinglocal metadata regression unchanged; original visual fidelity and
depth-correct intersection rendering remain unverified. [Limits](mesh-surface.md).

## D40 UVs and direct-diffuse graph subset

CONFIRMED_FROM_COMMUNITY_CODE: interleaved per-vertex BE f32 UV pairs, MaterialBox
sample parameter indices, output/diffuse node/port values, NUL-padded empty animation
name decoding, repeat/clamp enums. CONFIRMED_FROM_LOCAL_DATA:36 UV channel digests,
floor mesh→GMT→texture chain and exact BC1 top-mip pixels. Pinned partial-UV getter
bug is explicitly avoided; metadata min/max UV fields are not a quantization rule.
Rendering/lighting/alpha/color-space/full sampler fidelity remain UNKNOWN. Existing
CONFIRMED_FROM_DOCUMENTATION block-compression rules apply; no new codec assumption.
Source pin, methods, applicability, conflicts and tests: [textured-mesh.md](textured-mesh.md).

D43 adds no disk layout. `CONFIRMED_FROM_COMMUNITY_CODE` output/port selection and
sampler parameter behavior, `CONFIRMED_FROM_LOCAL_DATA` bounded route comparisons
support a labeled diffuse-only projection. Other branch semantics/game rendering
remain `UNKNOWN`. Strict duplicate handling is authored policy rather than an
original-game claim. [Per-rule sources/conflicts/coverage](diffuse-projection.md).

D44 adds no disk layout. `CONFIRMED_FROM_COMMUNITY_CODE` sample parameter4 selects
UV pair; `CONFIRMED_FROM_LOCAL_DATA` material channel/route and mesh UV oracle matches
corroborate explicit UV1 sampling. Other selector intent/game fidelity `UNKNOWN`.
[Pin/methods/conflicts/coverage](uv1-preview.md).


D45 adds static subtype-zero diffuse scale/offset behavior using existing raw
material parameters, with original UVs preserved. Rule-level evidence, numeric
preview policies, animated-branch conflict and original GPU unknowns:
[static affine sampling](affine-preview.md). No new disk layout.
D48 adds static subtype-one UV matrices from existing raw material parameters.
`CONFIRMED_FROM_COMMUNITY_CODE` coefficient placement/row math and
`CONFIRMED_FROM_LOCAL_DATA` raw-bit parity plus one linked native preview are
documented with pin, applicability, conflicts and coverage in
[matrix-preview.md](matrix-preview.md). Original GPU fidelity remains `UNKNOWN`.

D60 retains MIRROR2 and independent two-source addressing; rule evidence,
applicability/conflicts/coverage: [mirror-preview.md](mirror-preview.md). No new disk layout.

## D61–D62 sampler inputs, ThingColor and subtype-0 animation

`CONFIRMED_FROM_COMMUNITY_CODE` (Toolkit d6a2ef80…, GfxAssembler): THING_COLOR (box 2) is the
per-instance color input. Subtype-0 sampler input ports add (0), multiply (1) and
subtract (2) after static scale/offset; with one named animation the sampled UV is
`(uv - t.zw) * t.xy`. Local data confirms ThingColor variant 8 and a W-only
subtype-0 curve. Instance values in the game remain unknown. Details:
[instance-preview.md](instance-preview.md), [animation-base-preview.md](animation-base-preview.md).

## D94 BLEND thresholds

`CONFIRMED_FROM_LOCAL_DATA`: BLEND (box 16) parameter words 0/1 are lower/upper
thresholds; compiled constants in two 01.33 materials equal 1/(p1 − p0) and
−p0/(p1 − p0), matching Toolkit's 100/15 literals for 0.15/0.16. RGB mask space stays
`UNKNOWN`. Details: [d94-blend.md](d94-blend.md).
