# Neutral solid mesh diagnostics

D39 completed 2026-10-06 (America/Chicago).

The Mesh tab offers Wireframe (default) and Neutral solid. CLI preference:
`--mesh-view wireframe|solid`. Mode changes redraw the already cached geometry,
without another source read or parser call. Source/route/snapshot resets discard
geometry and cancel workers while retaining the chosen mode. Reset view retains
mode and restores the existing camera. No new crate, dependency or format rule.

## Geometry and display contract

Implementation: `app/mesh/surface.rs`, `app/mesh.rs`, viewer options/constructor.
Geometry provenance and oracle pin remain in [mesh-geometry.md](mesh-geometry.md)
and [mesh-prefix.md](mesh-prefix.md): unchanged independently parsed stored position/
triangle IR, corroborated by pinned Craftworld Toolkit. No community rendering code
was copied or translated. The surface algorithm is authored UI-side presentation.

The existing normalization/orbit transform now exposes camera depth while preserving
wireframe x/y projection. Normalization uses f64 arithmetic, including extreme f32
bounds. Each triangle gets a geometric face normal from transformed positions;
lighting is fixed in camera space and two-sided. Original normals, UVs, materials,
textures, bones, morphs, animation and game lighting are not used.

Faces draw far-to-near by average camera depth, with original triangle number as a
deterministic tie break. A single egui colored triangle mesh is submitted through
the existing wgpu renderer. The pinned local epaint0.36.2 Mesh::colored_vertex /
add_triangle API was inspected; vertices are untextured, and egui allocate_painter
clips drawing to the preview canvas. There is no depth buffer in this approach.
Intersecting or cyclic overlapping faces can sort incorrectly; the UI states this
limit and retains Wireframe. No posed/textured/original appearance claim.

Build limits are50000 triangles and1000000 positions, matching existing preview/
parser bounds; at most150000 colored vertices and indices are emitted. Invalid
camera/bounds, nonfinite positions/normals, out-of-range indices, budgets or screen
coordinate overflow return explicit display errors. Repeated-index and collinear
zero-area faces are omitted and counted separately from parsed topology. Edge-on
projected faces may submit zero screen area; submitted count does not equal visible
pixels. Empty geometry has a valid empty scene. No validation was weakened.

Projection/shading/sorting is recomputed on redraw within those bounds. Maximum-budget
frame time was not benchmarked. This is a small diagnostic surface renderer, not a
new full 3D pipeline. Metadata JSON excludes display state and all geometry arrays;
its version1 contract and production shared private writer remain unchanged.

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All passed:154 tests including63 viewer tests. Two new authored cases cover complete
far/near/tied-face order, reversed-winding identical two-sided shade, repeated-index
and distinct-index collinear degenerates, valid mesh submission/indices, extreme
finite bounds, screen overflow, invalid camera/bounds/indices, triangle budgets,
nonfinite positions and empty geometry. Existing export invariance now includes
solid mode; reset checks mode retention, cancellation and cache discard. Original
wireframe projection test remains exact. After verification only provenance/module
comments changed. No parser/runtime failure in this milestone.

Four hidden, auto-exit native runs all exit0, RTX3080/Vulkan, bounded validatedsource
attachment with4704862 metadata bytes (31534 archive entries). Private artifact stems:

| Stem | Selection/mode | Parsed geometry | Surface submission |
|---|---|---|---|
| d39-mesh-list | GUID000150e5, solid |24 vertices/8 triangles|8 faces,0 skipped|
| d39-mesh-strip | GUID00013051, solid |2560 vertices/2640 triangles|2472 faces,168 skipped|
| d39-mesh-unsupported | GUID000161b2, solid |SMH unsupported|no scene|
| d39-mesh-wireframe | GUID000150e5, wireframe |24 vertices/8 triangles|no solid scene|

List, strip and unsupported PNGs were inspected. Neutral surfaces appear, mode and
limits are visible, material reference controls remain available and SMH remains
explicitly unsupported. The circuit-board sample is relatively small/edge-on at the
existing default camera and material list consumes preview height; this motivates
better viewing controls next. No manual orbit/zoom/mode click, all-GPU/full-corpus,
actual depth-correct occlusion or original material appearance validation.

All four private JSON reports validate against `schemas/mesh-diagnostic.schema.json`
and are byte-identical to respective D19 verified list/strip/unsupported baselines.
Solid QA counts agree with those unchanged parsed vertex/triangle counts, with
submitted + omitted faces = parsed triangles. Neither unsupported nor wireframe
run emits a solid-submission summary. Parser unchanged, so existing externaloracle
was not rerun merely for a rendering change.

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013051 --qa-tab mesh --mesh-view solid --qa-capture local-analysis/new-solid.png --qa-mesh-json local-analysis/new-solid.json
target/debug/lbp-viewer.exe --mesh-view invalid
```

Invalid mode returns expected exit1 with `Mesh view needs wireframe/solid` (the native
QA loop's final command exit1 reflects this expected rejection). Fixed an existing
mojibake dash in help text using explicit UTF-8 and an ASCII dash.

SFO/MAP hashes unchanged; DATA.FARC1023990298B/read-only; reference clean at
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Authored source/docs allowed; game data,
references, reports and captures remain ignored. No root Git/commit/publication/
license decision or current blocker.

## Next D40

Improve mesh viewing controls: a focus-preview toggle that frees canvas height and
orthographic axis presets for inspecting flat/edge-on resources, retaining orbit,
zoom, wireframe/solid mode and diagnostic metadata access. Presets are display-space
views, not inferred game orientation. Keep parser/source lifecycle/export unchanged.
Verify synthetic axis projections and mode/reset/navigation behavior plus native
circuit-board focused/preset and default/unsupported captures. No new format, material,
normal or skeleton parsing; no renderer dependency expansion.
