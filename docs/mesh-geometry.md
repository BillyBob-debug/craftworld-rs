# D2 bounded stored mesh geometry and native wireframe — 2026-10-05

D2 complete: ordinary clear MSHb, heads 567–951 and known flags 0/7. lbp-mesh
produces CPU position/triangle IR plus diagnostic metadata; libraries remain free
of GUI/GPU dependencies. Native Mesh tab shows neutral stored-geometry wireframe,
candidate selection, cancellable loading, metadata, reset, orbit and zoom. It supports
MAP candidates and archive entries. No bone/morph transforms, material shading,
normals/UV interpretation, animation or gameplay. There is no depth buffer/culling
in this first wireframe; original rendering/pose correctness is not established.

## Per-rule provenance

`CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`:
- [RMesh::serialize/getTriangles](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java): prefix -> primitives -> bones -> mirror arrays -> explicit primitiveType. Geometry stops at that field; softbody/culling/costume and remaining bytes stay unparsed.
- [Primitive::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/mesh/Primitive.java): graphics-material and optional texture-list descriptors followed by raw min/max vertices, first/count indices, region. Raw high bits are retained. Index ranges must fit declared face indices; no guessed topology from numTris.
- [Serializer::resource](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java): flags word for versions >0x22e, byte kind, GUID and/or SHA1. Current kind bits 2/1 retained; GUID uses integer encoding. Legacy swapped kind mapping before0x191 is excluded by explicit lower revision bound. Null and raw ID fields are preserved, not silently resolved.
- [Bone::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/mesh/Bone.java), AnimBone::serialize, MeshShapeVertex/Info::serialize: bounded structural traversal only. Fixed names/OBB/bounds fields, integer family fields, two matrices, bounded shape arrays. Parent/child values and matrices are not interpreted or applied.
- [MemoryInputStream::m44/shortarray/enumarray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java): flags7 matrix masks count serialized float words; fixed short/byte mirror entries. CellGcmPrimitive assigns 5=triangles, 6=triangle strip. Other topology values remain unsupported.

`CONFIRMED_FROM_DOCUMENTATION`: [Microsoft primitive topology](https://learn.microsoft.com/en-us/windows/win32/direct3d11/d3d10-graphics-programming-guide-primitive-topologies)
defines triangle list/strip ordering and fixed 16-bit restart0xffff. Independent
triangulation resets per primitive/restart and advances parity for degenerates.
Degenerate triangles are retained in metadata digest but repeated-index faces are
omitted from wireframe drawing. No original front-face convention is assumed.

`CONFIRMED_FROM_LOCAL_DATA`: 36 existing D1 samples match exact primitive metadata,
bone counts, explicit topology, parsed/unparsed byte counts, canonical triangle
index SHA256 and prefix metadata/IEEE position bounds. 32 strips, four lists;
8–2,560 vertices and 1–43 bones. Sample material kind is GUID (2); synthetic tests
cover kinds0/1/2/3 and rejection of unknown bits. Non-GUID actual materials are not
locally corroborated. Remaining geometry tails range91–17,060bytes and are unknown.
Oracle invokes Toolkit stream readers and RMesh::getTriangles on controlled raw
streams with declared primitiveType; it does not deserialize or export full meshes.
No community source was translated into the Rust implementation.

## Budgets, checks and native QA

D1 source/expanded/count limits retained; primitives and bones <=4096, shapes per
bone <=4096, morph mirror entries<=32, total triangles<=1,000,000. Only topology5/6;
odd/short index buffers, primitive ranges and vertex indices strictly checked.
Cancellation across source pages/chunks/bones/primitive/index loops. Native drawing
caps50,000triangles, one current-resource worker/IR; stale jobs canceled on source
or selection changes. Header preflight reads <=32bytes before whole-source budget
allocation, so SMH/other magic is explicitly unsupported. Protected candidates
remain blocked by attachment/mesh preflight with no protected decoding.

The 65-test workspace run passed before final descriptor/preflight additions;
final seven mesh tests pass, final nine viewer tests pass, workspace Clippy/fmt and
native build pass. Synthetic recipes exercise compact/fixed fields, compressed and
stored chunks, descriptors, bone traversal, topology/truncation/ranges, strip cuts,
degenerates, extreme finite projection and cancellation/reset. Four Python texture
report tests and synthetic Pillow checks pass from C4.

Initial native captures exited with 'Unknown QA tab': the new tab was omitted from
CLI allowlist. Corrected and rebuilt. First SMH capture hit size budget; header
preflight improved the diagnostic and final capture reports unsupported MSH-only
scope. All failed-run logs/capture retained ignored. Native list/strip/static views
inspected on RTX3080/Vulkan. No manual drag/scroll input or other backend tested;
orbit/zoom controls implemented, synthetic projection checked. Original fidelity,
full mesh corpus, other revisions/topologies and posed/skinned meshes unverified.

```powershell
. ./scripts/use-local-rust.ps1
./scripts/run-mesh-oracle.ps1 -SampleManifest local-analysis/d1-mesh-samples.json -Output local-analysis/NEW-geometry-oracle.json
cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-geometry-oracle.json local-analysis/NEW-geometry-parity.json
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
```

Select an MSH resource and Mesh tab; candidates and raw metadata remain inspectable.
QA flag --qa-tab mesh, local listGUID000150e5 / stripGUID00013051 /
unsupported staticGUID000161b2. Inputs/captures are private evidence, never fixtures.
Ignored evidence: d2-mesh-oracle.json, d2-mesh-parity-final.json,
d2-prefix-regression.json, d2-mesh-strip/list.png, d2-mesh-static-final.png and logs.
D1 prefix-only comparison is retained via --prefix-only.

Next bounded extension: material reference navigation and material metadata/oracle
research. Textured rendering, skinning and animation require separate evidenced
contracts; do not expand to full levels, physics or multiplayer from this milestone.

## D42 ownership — 2026-10-06

CPU Geometry additionally retains one triangle span per primitive (including empty
and degenerate spans), without serializing it or changing geometry metadata/digests.
36 local meshes/82 partitions match the pinned oracle exactly. Existing parsing,
topology/restart/budgets remain unchanged. [D42 evidence](primitive-groups.md).
