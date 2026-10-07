# D19 cached mesh diagnostic summaries

Completed 2026-10-05 (America/Chicago). The Mesh tab offers an explicit destination
and Save action under "Export cached mesh diagnostic JSON". It projects the cached
validated prefix and topology metadata plus vertex count: counts, bounds/IEEE bits,
digests, primitive descriptors/material references, bone count, raw primitive type,
triangle count and parsed/unparsed lengths. Explicit field lists exclude vertices,
triangles/index buffers, bone records/transforms, geometry payload, GPU state and
images. Camera and navigation state do not change the report. No additional source
read or parser invocation occurs during save.

Version1 envelope/schema: `schemas/mesh-diagnostic.schema.json`, scope
`cached_mesh_diagnostic_metadata_not_asset_export`. Shared app::diagnostics retains
the same private canonical/exclusive/new-file `.json` rules and 8MiB budget as
Material/Texture exports. Relative location and prior-read scope are explicit.
Error categories exclude arbitrary host paths. Unsupported/blocked/cancelled/failed
mesh states have null metadata; the current geometry cache API provides no partial
prefix on failure, so one is not fabricated. Busy/unselected/uncompleted exports
are refused. Unknown remaining tails are counts, not interpreted or exported.
Reference GUID/hash candidates (including raw zero) remain distinct fields.

## Provenance and oracle evidence

No parser or format rule change. CONFIRMED_FROM_COMMUNITY_CODE: existing pinned
Toolkit `RMesh`, `Primitive`, bone structural traversal and stream rules at commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Per-field source links, revision
applicability and conflicts remain near lbp-mesh and in mesh-prefix.md and
mesh-geometry.md. CONFIRMED_FROM_DOCUMENTATION: list/strip topology ordering and
restart retain Microsoft provenance there. No posing, skinning, morph evaluation,
material shading or original visual fidelity claim.

CONFIRMED_FROM_LOCAL_DATA: exported native cached summaries:

| GUID | Existing sample | Counts / topology | JSON bytes / SHA256 |
|---|---|---|---|
| 000150e5 | vin_floor_courtyard_03.mol, SHA1 64d2941bd42e3ab30186bd2f1a00ed7139a93a8a | 24 vertices, 8 triangles, 1 primitive, list5 | 4440 / 636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1 |
| 00013051 | dc6a_circuit_board.mol, SHA1 da304a9720740a5bdac08ed8ccbbd4e2420c8a88 | 2560 vertices, 2640 triangles, 4 primitives, strip6 | 5899 / 785b49e56d3d59c1ba6cac419557e176c165b16d9c19765b57ad7c08b39fb9be |
| 000161b2 | unsupported static env_popov_background_squish.smh | unsupported, null metadata | 523 / f8cd1bf980f4cdafe5a42cb1a07604f3989ce674a86eff97f38274a95e42c4a1 |

Exact relative location is checked against the private D1 manifest for both supported
samples. Remove only the new vertex_count summary field, and the entire remaining
metadata equals the prior oracle-validated D2 Rust parity report's geometry object.
Vertex count equals declared prefix count0. Direct pinned community oracle equality
also passes using the existing canonical rule: decimal position_bounds is excluded
because Gson prints short f32 decimals while serde_json::Value widens to f64;
position_bounds_bits and every other field compare exactly. This is the already
documented D2 conflict, not a relaxed guard. No fresh decoder/oracle or pixel test.
Unsupported static data remains outside the ordinary MSH reader boundary.

## Verification

Two authored cache cases verify field whitelists, prefix/primitive fidelity, raw
zero GUID/hash and simultaneous alternative identifiers, IEEE bits, independence
from camera/payload values, error categories and busy/reset refusal. Existing shared
writer tests cover containment/exclusive output/budgets. All 103 workspace tests
(29 viewer tests), fmt, Clippy and native build pass; ignored log
local-analysis/d19-workspace-tests.log. No product dependency change.

Draft202012 schema validation passes for all three real reports using the existing
ignored jsonschema4.23.0 tool. Five invalid mutations per supported report are
rejected: positions, prefix index data, primitive bone data, wrong scope and
unsupported stage carrying available metadata. List report repeats byte-identically.

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','000150e5','--qa-tab','mesh','--qa-mesh-json','local-analysis/d19-mesh-list-diagnostic.json','--qa-capture','local-analysis/d19-mesh-list-export.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d19-mesh-list-export.stdout.log -RedirectStandardError local-analysis/d19-mesh-list-export.stderr.log
$qaProcess.WaitForExit(30000)
```

Other runs use GUID00013051 / strip output names, GUID000161b2 / unsupported names,
and GUID000150e5 with new repeat paths. All four native runs exited zero. Inspected
list/strip/unsupported screenshots on RTX3080/Vulkan: status, material reference
controls and existing neutral wireframe fit the pane. QA invokes the same production
export function as the GUI button. No manual Save/path-editing/error interaction,
other GPU/platform, directory-race, full mesh corpus or new posed/shaded preview
claim. Python source/tests unchanged; their prior27tests were not repeated. No new
format failure or approval rejection. Original SFO/MAP hashes, FARC size/ReadOnly,
clean pinned reference and default-deny audit unchanged. All game data/reports/
captures/tools remain ignored; authored Rust/schema are trackable. No Git
initialization, commits/publication or license decision.

## Next useful task: D20 bounded Back/Forward navigation

The current UI has Back only for MAP selections; archive entries and overview
cannot participate, and Back mutates selection after binding the old detail resource
for that frame. Improve browsing with bounded route history covering MAP aliases,
archive-entry selection and overview, plus Forward and stored inspection-tab state.
Centralize route transitions so changed selection cancels/discards stale workers,
caches, raw/hex details and pending jumps. Clear both histories on snapshot replacement;
clear Forward after a new route, validate indices and avoid duplicate route entries.
Keep restored targets visible by clearing hiding filters. Stop rendering the old
detail after a navigation event to avoid mixing old-resource UI with new selection.
Use authored state/toolbar event tests and native captures; no parser/source-format
work, asset writes or new dependencies are needed. Inspect existing handlers first.
