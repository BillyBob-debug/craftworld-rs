# D42 primitive ownership and material groups — 2026-10-06

The Bevy preview now binds primitives independently using the existing identity-UV0,
unanimated direct-diffuse subset. Unsupported groups remain neutral and retain their
triangle ownership. Geometry metadata exports and the eframe single-primitive preview
contract remain unchanged. No general material graph, game shader or runtime is added.

## Provenance and evidence

`CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`,
[RMesh.getTriangles(Primitive), getTriangles(start,count)](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java),
[Primitive.serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/mesh/Primitive.java).
Each primitive's first/count selects an index interval; the primitive carries its
material descriptor. Existing D2 triangulation resets strip parity at each interval.
No min/max vertex masking or region semantics are inferred. Overlapping intervals
retain separate ownership; equal material IDs do not erase primitive identity.

This changes the CPU representation, not disk parsing. `primitive_triangles` holds
one contiguous triangle range per metadata primitive, including empty groups. Flat
triangles/digest/order remain identical. Ownership includes degenerate triangles;
only the renderer filters them. Applicable to the existing clear MSH revision/flags/
topology contract; [D2 limits and conflicts](mesh-geometry.md) still apply. Pinned
Toolkit's strip routine does not handle arbitrary leading/short restart patterns as
the independent bounded implementation does; no new agreement on those patterns is
claimed. Synthetic tests cover list/strip, empty/short strips, duplicate intervals,
degenerates, cancellation; existing restart/parity tests remain.

`CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, 36 existing samples / 82 primitives (16 multi-primitive meshes) / 605,145 stored
bytes: per-primitive start/end and big-endian triangle-index SHA256 all match the
pinned Java reader, zero mismatches/failures. This is partition corroboration, not
material/rendering fidelity or full-corpus compatibility.

Material/UV/wrap rules remain the [D40 contract](textured-mesh.md). Per-group zero
material flags and absent alternative descriptors (including zero alternative
flags) are required. GUID/hash candidate ambiguity remains explicit. Original game
lighting, alpha, pose, alternative materials and graph evaluation remain `UNKNOWN`.
No decryption or executable investigation was necessary or performed. Reference
source was read as evidence; no recovered/community renderer code was translated.

## CPU and GPU bounds

`lbp-preview::load_mesh_groups` reads one mesh and returns one outcome per primitive.
Up to eight distinct resolved material locations are attempted in primitive order.
Successes and failures are cached within that load; shared successes use Arc CPU
assets. Resolution precedes caching, preserving conflicting aliases/identifiers.
Failed/unresolved/over-budget groups never substitute an invented texture.

Limits: 4096 parser primitives, 100,000 vertices, 50,000 preview triangles. Mesh
stored input <=2 MiB; each attempted material <=1 MiB and texture <=16 MiB, hence
<=138 MiB stored reads across eight attempts. Existing per-parser expansion limits
bound cumulative work by <=464 MiB (16 mesh + 8*(8 material + 32 texture + 16 UV)).
Each texture <=4 million pixels; retained CPU pixels <=128 MiB, UV0 <=6.4 MiB across
eight successful bindings. Renderer image preparation temporarily duplicates pixel
buffers; these limits are not claims of total process/GPU memory usage. UV decoding
currently repeats once per successful distinct material and may be optimized later.
Cancellation checks run between primitives and existing pages/decoder loops. Final
source stability/cancellation checks precede publication; stale receiver behavior
is unchanged. Normal loads never invoke research tools.

The Bevy adapter checks that ranges completely partition flat triangles and match
binding count before allocating GPU assets. Groups sharing the same cached binding
merge only for drawing; all unsupported groups share one neutral draw. At most
nine owned Mesh/material/entity sets and eight Images. Full-mesh positions supply
identical display centering/scaling to every draw, preserving group alignment and
shared vertex/UV correspondence. Per-primitive stats/reasons remain in a scrollable
status panel. Empty/all-degenerate draws are skipped with diagnostics. Scene reset
removes every owned entity and GPU asset; the lifecycle test exercises four groups.
Existing depth, double-sided, unlit, opaque, sRGB, top-mip policies remain unchanged.

## Reproduction

```powershell
. ./scripts/use-local-rust.ps1
./scripts/run-mesh-oracle.ps1 -Groups -Output local-analysis/NEW-groups-oracle.json
cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-groups-oracle.json local-analysis/NEW-groups-parity.json --groups
./target/debug/lbp-bevy-viewer.exe --synthetic-groups
./target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013051
```

Outputs must be new ignored files. The authored group scene uses red left geometry
(two primitives sharing a binding), green upper-right and neutral unsupported lower-
right. All use the production bucket/mesh/image adapters. This checks display
ownership/alignment, not proprietary material graph decoding. Read-only source,
material predicate and resolver tests plus local real-asset captures cover those
separate boundaries. Native verification results and exact checks are in STATUS.md.

Eframe remains the single-primitive diagnostic reference. Direct ISO, full browser
migration, other material graphs, alternative descriptors, alpha/game lighting,
skinning/animation and level rendering remain later milestones.

## Verified native results

160 workspace tests pass, with fmt/Clippy and both frontend builds. Inspected native
captures: authored mixed bindings align correctly; real circuit board 00013051 has
892/1062/415/271 triangles in its four neutral groups, each explicitly rejecting its
more complex graph; floor 000150e5 retains 24 vertices/eight triangles/512x256 BC1
Repeat. Eframe floor JSON remains byte-identical to D41. A real asset with multiple
supported texture bindings was not verified; that claim relies only on the authored
adapter fixture. Manual mouse/keyboard interaction, rapid native reload stress,
other GPUs/platforms and original-game visual fidelity remain untested.
