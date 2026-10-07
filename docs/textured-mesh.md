> D45 now allows bounded static scale/offset with separate derived sampling UVs;
> the identity-only sampling restriction below is historical. Raw UV preservation,
> channel selection and other route guards remain. See [D45](affine-preview.md).

# D40: first material-linked textured mesh — 2026-10-06

The Mesh tab now has **Direct diffuse**, alongside Wireframe and Neutral solid.
This is a small working asset-viewer slice: mesh → material graph → texture slot →
resolved texture → verified UV0 → unlit textured triangles. It is not an LBP shader
implementation. The previous proposed camera-control task was superseded by the
user's request to move on to substantive asset compatibility.

## Supported contract

Existing ordinary clear MSH geometry and texture contracts apply. Exactly one mesh
primitive, zero material-reference flags, and no alternative material descriptor.
The GMT graph must have exactly two nodes and one wire: TEXTURE_SAMPLE → OUTPUT,
source port 0, diffuse destination port 0, zero swizzle, subtype 0. Texture sampling
must use UV0, exact identity scale and zero offset, and no named/keyed animation.
Selected texture descriptor flags must be zero. Both sampler axes must agree:
repeat (1) or clamp-to-edge (3). The established decoder must support the top mip.

No silent first-candidate selection: GUID/hash lookup retains all aliases, requires
one shared MAP hash and one available location, rejects conflicting IDs/locations,
null IDs and unavailable bytes. Duplicate aliases for the same location deduplicate
only for this bounded read. Existing reference menus still retain the aliases.
There is no recursive material evaluation or arbitrary shader execution.

Selecting Direct diffuse loads linked resources on a cancellable worker. Source
replacement/navigation cancels and clears CPU/GPU caches. A linked-stage failure
retains successfully parsed geometry and its metadata, reports the reason, and
leaves Wireframe/Neutral solid usable. An unsupported texture is never replaced by
an invented fallback image. Wireframe/solid do not initiate linked reads.

## Format provenance

All community references below are pinned to Toolkit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Independent authored code uses established
field facts and an external reader; no community shader implementation was copied,
adapted or translated. Existing license/distribution policy remains unchanged.

| Rule | Evidence and source | Applicability / conflicts / coverage |
|---|---|---|
| Per-vertex attributes interleave UV channels, each two fixed big-endian IEEE f32 words; stride = channel count × 8 | CONFIRMED_FROM_COMMUNITY_CODE: [RMesh serialize/init/getUVs(channel)](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java), [MemoryInputStream v2](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java) | Existing mesh-prefix revision/flags contract; exact attribute extent required. Synthetic interleaving/negative zero/truncation/nonfinite/cancellation checks; 36 local sample channel digests match. |
| Texture sample parameter indices 0/1 scale, 2/3 offset, 4 channel, 5 slot | CONFIRMED_FROM_COMMUNITY_CODE: [MaterialBox constants/constructor](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java) | Existing GMT heads 920–951, branch zero. First slice only identity UV0. Synthetic nonidentity/channel rejection. |
| Node kinds OUTPUT=0/TEXTURE_SAMPLE=1, diffuse port=0, zero swizzle means unchanged channels | CONFIRMED_FROM_COMMUNITY_CODE: [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java), [BrdfPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BrdfPort.java), [GfxAssembler getWire/texture-sample case](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java) | This semantic subset is narrower than graph metadata parsing. Synthetic invalid endpoints/ports/swizzles/extra nodes reject. |
| Animation activation uses nonempty decoded name; string reader removes NULs | CONFIRMED_FROM_COMMUNITY_CODE: GfxAssembler above, [MaterialParameterAnimation serialize/getName](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialParameterAnimation.java), MemoryInputStream str above | All-zero name bytes count as empty, rather than rejecting the local padded empty name. Named animations/key values reject. No animation evaluation. |
| Wrap=1, clamp-to-edge=3 | CONFIRMED_FROM_COMMUNITY_CODE: [TextureWrap](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java) | Matching S/T axes only; egui's existing repeat/clamp sampler. Other modes remain unsupported in this slice. |
| Floor mesh 000150e5 → material 00014c31 → slot 0 / texture 000150eb | CONFIRMED_FROM_LOCAL_DATA: bounded native material graph diagnostic, mesh/UV/texture oracle and final linked preview below | BCUS98245 01.00 only; mesh 24 vertices/8 triangles, two UV channels; material revision 931, simple direct diffuse graph. |

Pinned `RMesh.getUVs(start,count,channel)` incorrectly seeks using `start` as a byte
offset instead of vertex stride. The authored Java synthetic guard demonstrates
this conflict. Oracle comparison uses only `getUVs(channel)` (start zero). Rust
uses explicit checked extent/stride calculations. UV min/max metadata is not used
as a quantization rule; no compressed-UV theory was invented.

## Display and budgets

CPU UV arrays belong to `lbp-mesh`; no UI dependency or serialization of UV arrays.
`app::mesh::linked` is a CPU orchestration layer over existing parsers/catalog/source;
GPU texture handles stay in the UI. `surface` preserves each triangle's original
vertex/UV correspondence during existing average-depth sorting. White vertex tint
avoids applying the generated neutral-solid shading. UVs are supplied unchanged:
no guessed V inversion, game lighting, material flags, alpha-mode reproduction,
normals, bone/morph transforms, higher mips or color-space fidelity claim.

Rendering uses existing egui/wgpu, linear filtering and its alpha/color pipeline.
Average-depth ordering is approximate and can misorder intersecting triangles.
Original-game visual equivalence, full sampler fidelity and shader fidelity remain
UNKNOWN. Native inspection confirms visible textured triangles, not pixel-equivalent
game rendering. No depth-buffer claim or full-corpus compatibility claim.

At most three selected resources are read, each through approved read-only pages
≤4096 bytes: mesh ≤2 MiB, material ≤1 MiB, texture ≤16 MiB (≤19 MiB stored total).
Expanded per-parser limits: mesh ≤16 MiB, GMT ≤8 MiB, texture ≤32 MiB; geometry and
UV APIs each inflate the same cached mesh bytes, without a second disk read.
Cumulative expansion work is therefore bounded by 72 MiB. Limits remain 100,000
vertices, 8 UV channels, 50,000 preview triangles, 4 million texture pixels.
Checks run between pages/stages and within decode loops; cancelled results cannot
publish. Protected input boundaries remain unchanged.

Version-1 cached mesh diagnostic JSON deliberately remains geometry metadata only,
byte-identical to D39. Linked outcome/UV channel digests appear in the UI/private QA
log; UV arrays and pixels never enter that export. A mesh export's successful stage
does not assert that its material preview succeeded.

## Verification and reproduction

```powershell
. ./scripts/use-local-rust.ps1
./scripts/run-mesh-oracle.ps1 -Uv -Output local-analysis/d40-uv-oracle.json
cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d40-uv-oracle.json local-analysis/d40-uv-parity.json --uv
./scripts/run-texture-oracle.ps1 -SampleManifest local-analysis/d40-texture-samples.json -Output local-analysis/d40-texture-oracle.json
cargo run -p lbp-catalog --example texture_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d40-texture-oracle.json local-analysis/d40-texture-parity.json
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 000150e5 --mesh-view textured
```

Outputs are exclusive-create private files; choose new filenames when rerunning.
Texture manifest selects GUID 000150eb's catalog location, not committed asset bytes.
UV oracle: 36 samples / 605,145 stored bytes, zero mismatches/failures; repeated oracle
byte-identical. Texture oracle: one 81,281-byte GTF BC1, 512×256, 10 declared mips,
only top mip decoded; exact RGBA SHA256
`87794649aaafcc5018ee2bb690f6ea8750b83f68f55548ef1a638a25dc1477e3`.

153 Rust tests pass, including 67 viewer tests; fmt/Clippy/build pass. Synthetic
coverage includes UV envelopes across old/modern and compressed/raw cases, every
truncation, protected/cancel boundaries, direct-graph predicate, aliases/conflicts,
UV ownership, linked failure preserving geometry, cancellation and reset.
This supersedes earlier aggregate totals; this count is the exact sum
of passed Rust tests in `local-analysis/d40-tests.log`.

Four native regression captures exit zero (RTX3080/Vulkan): floor direct diffuse,
four-primitive circuit board explicit unsupported slice, SMH explicit unsupported
layout, and floor wireframe. All four mesh JSONs are schema-valid and byte-identical
to corresponding D39 files. Floor/board/SMH PNGs inspected. No manual interaction
claim. A final floor-label capture also exits zero, shows UV0/512×256/BC1/Repeat,
and preserves the same export bytes. No maximum-budget redraw benchmark.
Initial compile rejected implicit LocationKey
serialization; explicit metadata fields fixed it. Clippy's constant chunk iterator
warning was fixed using array chunks; final checks pass. Initial PowerShell oracle
argument syntax was corrected before successful execution.

SFO/MAP hashes and FARC length/read-only attributes unchanged; pinned reference tree
clean. Ignore audit excludes private outputs/game bytes/reference binaries and
permits only authored additions. No root Git initialization or publication.

## Next substantive task

Support multiple mesh primitives using this same proven direct-diffuse contract.
Preserve primitive ownership in the CPU geometry IR, compare primitive triangle
partitions against the pinned oracle, bind each supported primitive independently,
and retain visible diagnostic geometry for unsupported material groups. Do not
expand to a general material evaluator or level renderer in that task.

## D41 shared consumer — 2026-10-06

CPU preparation now lives in lbp-preview and serves both frontends. The predicate,
pinned evidence and budgets above are unchanged. The Bevy frontend replaces painter
ordering with native depth-tested perspective while retaining eframe diagnostics and
the byte-identical metadata export. [Renderer policy and verification](bevy-preview.md).

## D42 Bevy material groups — 2026-10-06

The Bevy consumer now loads each primitive independently within this same graph
predicate. Equal resolved materials share CPU bindings and GPU draws; unsupported
groups remain neutral and explicit. Nonzero alternative descriptor flags reject
as well as nonzero kinds. Eframe retains its single-primitive reference behavior.
No general graph evaluator. [D42 budget/provenance/tests](primitive-groups.md).

## D43 projection extension — 2026-10-06

The historical two-node/one-wire restriction is superseded by selection of exactly
one output and unique direct texture diffuse wire within a larger graph. Selected
sampler inputs, transforms/animation/swizzle remain unsupported; other branches
are counted/visibly ignored. Either offset zero sign is identity, with raw metadata
preserved. Both frontends share this rule; eframe remains one primitive and Bevy
retains D42 group budgets. [Evidence and limitations](diffuse-projection.md).

## D44 original UV1 support — 2026-10-06

Direct projection now supports identity UV0 or UV1 with explicit channel selection,
missing-channel rejection and no raw coordinate mutation/fallback. Two real Valkyrie
UV1 texture groups verified natively in Bevy; eframe stays single-primitive, UV0
export regression unchanged. [Sources and limitations](uv1-preview.md).
