# D1 bounded MSH prefix diagnostic — 2026-10-05

D1 complete: UI-independent lbp-mesh reads ordinary clear MSHb prefixes through
existing resource header/dependency validation. It preserves counts, fixed raw morph
name bytes (hex), UV/area float bits, source stream offsets, position bounds/bits,
position/attribute/index byte digests, parsed prefix length and remaining byte count.
It does not decode topology, bones, materials, animation or static SMH payloads.
No geometry/asset export and no speculative interpretation of remaining fields.

## Evidence and rule provenance

All following prefix layout rules are `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit
commit `d6a2ef80b5a09999f08f23a803b516096055e5c9`:
- [RMesh::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java): seven counts (vertices, face indices, edge indices, triangles, streams, attributes, morphs), 32 fixed 16-byte names, revision >=0x239 min/max UV float arrays and area scale; streamCount+2 offset fields, fixed numVerts*16 bytes per stream, then attribute/index bytearrays.
- [MemoryInputStream::i32/str/floatarray/bytearray](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java): flags bit 1 selects ULEB integer encoding; float bit patterns remain fixed BE words. Negative/overflow counts are never allocations.
- RMesh::getVertices: first stream's first 12 bytes per 16-byte record are three BE float32 position components. Bounds/digests are diagnostics, not posed or transformed geometry.
- [SerializedResource::process](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/types/SerializedResource.java) and Compressor::decompressData: bounded clear binary payload precedes dependency table, optionally uses chunk envelope. Existing lbp-resource validates revision/branch/header/dependencies; strict mesh inflation checks complete input/output/end.

Toolkit's read-side RMesh skips source offsets and uses fixed-size streams, whereas
its writer emits cumulative offsets with repeated final size. D1 preserves these
fields and does not require write-side equality. Offset meaning beyond this reader
behavior remains `UNKNOWN`; unknown is not corruption. Morph names remain raw hex,
not guessed text encoding. Prefix checks do not establish index/triangle validity.

`CONFIRMED_FROM_LOCAL_DATA`: scanner contains 2,716 MSH locations in 119
revision/flag/branch cohorts; flags 0:1,448, flags 7:1,268. Bounded manifest selected
36 smallest/median samples in 19 cohorts (most frequent plus extrema/branch),
605,145 archived bytes. All Rust prefix fields and canonical position bound bits
match external pinned Toolkit stream readers and RMesh::getVertices on isolated
first-stream data. This oracle never invokes full RMesh deserialization or infers
topology from constructor defaults. Not a full corpus mesh compatibility claim.

The initial parity report failed all 36 samples solely because Gson serialized f32
bounds as short decimals while serde_json::Value widened them to f64. Exact IEEE
bounds bits now compare, and display decimals are excluded from cross-tool equality.
Counts, raw fields, offsets and payload digests were already identical. Initial
report retained as ignored evidence. A compile error from treating Inspection as
Result was fixed by mapping its explicit diagnostic status.

## Limits and verification

Archived 2 MiB; expanded 16 MiB; 100,000 vertices, 34 streams, 8 attributes,
32 morphs, 16 UV array elements, 1,000,000 declared index/edge/triangle counts.
Known serialization flags 0/7 only. All actual source callbacks in the diagnostic
use validated read-only attachment and at most 4096 bytes; total manifest budget
64 MiB, at most 48 samples. Cancellation per chunk and 256 positions. Protected
magic blocks before parsing; unsupported revisions/flags remain explicit.

Four authored Rust tests cover pre/post-0x239 layouts, fixed/ULEB counts, raw/zlib/
stored chunk forms, float bounds, full truncation prefixes, ULEB overflow, counts,
non-finite positions, strict zlib ending/trailing bytes, protection and cancellation.
Workspace 61 Rust tests, Clippy and fmt pass. No GUI mesh, full topology, bone,
material, normals, UV decoding, original transforms or visual correctness tested.

```powershell
. ./scripts/use-local-rust.ps1
python scripts/mesh_report.py local-analysis/a2-inventory-verified.json local-analysis/NEW-mesh-samples.json
./scripts/run-mesh-oracle.ps1 -SampleManifest local-analysis/NEW-mesh-samples.json -Output local-analysis/NEW-mesh-oracle.json
cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-mesh-oracle.json local-analysis/NEW-mesh-parity.json --prefix-only
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Ignored evidence: d1-mesh-samples.json, d1-mesh-oracle-bits.json,
d1-mesh-parity-final.json; superseded d1-mesh-oracle.json/d1-mesh-parity.json.
Reference source/jar and game bytes remain ignored and unmodified.
Next D2: establish bounded primitive ranges, bone/mirror layout traversal and
explicit primitiveType, then compare triangle/strip/restart output with external
reader before adding any neutral mesh preview. Never guess topology from numTris.
