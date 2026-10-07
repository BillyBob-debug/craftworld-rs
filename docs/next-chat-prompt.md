> D45 static diffuse UV scale/offset implemented on 2026-10-06. Read latest
> STATUS.md and docs/affine-preview.md. Next D46: choose one common unsupported
> diffuse route from bounded diagnostics and establish pinned behavior evidence
> before adding it. Matrix/animated samplers remain deferred. Older prompts below
> are historical; the clickable launcher and browser remain available.

> Post-D44 usability request implemented on 2026-10-06: clickable Windows launcher
> and searchable Bevy asset menu with hide/show unavailable files. Read the latest
> STATUS.md and docs/bevy-preview.md. D45 affine sampling below remains deferred;
> this usability change does not implement additional material operations.

> D44 was implemented and verified on 2026-10-06. Read latest STATUS.md and
> docs/uv1-preview.md. Next D45 is bounded finite affine scale/offset sampling for
> direct UV0/UV1 texture routes, with original CPU arrays preserved and derived
> coordinates labeled. Subtype1 matrices, animation and wired sampler inputs remain
> deferred. Older milestone prompts below are historical.

> D43 was implemented and verified on 2026-10-06. Read the latest STATUS.md and
> docs/diffuse-projection.md. Next D44 is bounded direct-diffuse UV1 channel selection
> using verified interleaved UV arrays, with a local native example, unchanged raw
> CPU arrays and explicit channel diagnostics. Transforms/new graph operations remain
> separate. Older milestone prompts below are historical.

> D42 was implemented and verified on 2026-10-06. Read the latest STATUS.md and
> docs/primitive-groups.md. The following D41 prompt is historical. Next select one
> additional common material subset from the new per-group unsupported diagnostics
> and gather bounded format/behavior evidence before expanding the interpreter.

> D41 was implemented and verified on 2026-10-06. This prompt is retained as the
> historical acceptance contract. Read the latest STATUS.md entry and
> docs/bevy-preview.md for results and launch instructions. D42 primitive ownership
> and material groups is the following milestone.
# New-chat handoff prompt — D41 Bevy preview

Continue this project by implementing D41: the first working Bevy asset-preview
frontend. Work through implementation and verification, not another planning-only
pass. Make small, reviewable changes and preserve the existing working explorer.

## Read first

- AGENTS.md (current research permissions supersede earlier blanket prohibitions)
- STATUS.md, especially the latest entries
- RESEARCH.md
- docs/architecture.md
- docs/bevy-transition.md
- docs/textured-mesh.md
- docs/mesh-geometry.md
- docs/texture-preview.md

## Current state

This is an independent LBP2 Asset Explorer, not yet a compatible game runtime.
The eframe/egui explorer has verified archive/catalog/resource readers, texture
decoding, stored mesh geometry, UV decoding, and a narrow material-linked Direct
diffuse preview. A Bevy frontend has been planned but is not implemented or built.

Known working chain:

- Mesh GUID 000150e5: vin_floor_courtyard_03.mol
- Material GUID 00014c31
- Texture GUID 000150eb: 512×256 GTF BC1
- Mesh: 24 vertices, 8 triangles, one primitive
- Material: one texture sample connected directly to diffuse output, identity UV0

36 UV samples match the pinned community oracle. The floor texture's top-mip
pixels match exactly. D40 recorded 153 passing Rust tests, including 67 viewer
tests. Verify the current state rather than assuming these totals remain current.

Relevant code:

- crates/lbp-mesh/src/geometry.rs
- crates/lbp-mesh/src/uv.rs
- crates/lbp-texture/src/lib.rs
- crates/lbp-catalog/src/attachment.rs
- crates/lbp-viewer/src/app/mesh.rs
- crates/lbp-viewer/src/app/mesh/linked.rs

Local input:

- Snapshot: local-analysis/a2-inventory-verified.json
- Extracted data: game-data/a1-disc
- Build: BCUS98245 01.00

Activate the existing toolchain with `. ./scripts/use-local-rust.ps1`.
This workspace currently uses Windows GNU Rust. Do not assume a root
rust-toolchain.toml exists. Inspect Git/toolchain state before changing anything.

## Research permissions and distribution boundaries

The user permits local decryption, protected-data inspection, disassembly and
decompilation of their own game copy when needed to understand formats or behavior.
This explicitly supersedes the earlier blanket prohibition on those techniques.

Keep original ZIP/ISO/game/executable inputs read-only. Store decrypted files,
raw decompiler output and other derived research artifacts only under ignored
local directories such as local-analysis/. Game trees remain under ignored
game-data/. Never commit or distribute proprietary research outputs, original
assets, recovered game code or game-specific decryption keys.

Use research findings to write an independent implementation. Record independently
expressed format/behavior notes and provenance; do not copy, adapt or translate
recovered proprietary code into the product. Absence of copied code alone does
not establish a strict clean-room process.

The delivered product must contain no original LBP code or assets. Runtime assets
come from the user's own ISO/game data. Direct ISO support is a later input feature;
D41 uses the existing extracted directory without pretending ISO loading exists.

Prefer documented specifications and existing licensed community readers, then
controlled local experiments, then targeted decryption/executable investigation
when the preceding routes cannot answer a concrete question. These techniques are
permitted, not required by default. D41 already has accessible verified input and
does not need executable research. Online service modification is outside scope.

Preserve the default-deny .gitignore and inspect prospective tracked files.
No commits, pushes, releases or publication. The project's distribution license
is undecided. Follow community-code licensing obligations; public visibility is
not reuse permission. Toolkit oracle pin:
`d6a2ef80b5a09999f08f23a803b516096055e5c9`.

## Implementation order

### 1. Prove Bevy feasibility before refactoring working code

Choose compatible released Bevy/bevy_egui versions using official documentation
and the integration's support table. Pin the dependency resolution in Cargo.lock.
Use only features needed for this desktop preview.

Build and run a minimal authored synthetic 3D scene on the existing toolchain.
Verify the actual renderer initializes and depth testing works. A successful
cargo check alone is insufficient.

If the GNU build fails, diagnose the concrete failure and document the smallest
necessary remedy. Do not silently replace the working workspace toolchain or
install a large unrelated development stack. Existing eframe success does not
establish Bevy compatibility.

### 2. Share the existing CPU preview preparation

Extract the established CPU-only mesh/material/texture loading and binding logic
into a small shared module/library when Bevy becomes its second consumer. A crate
such as lbp-preview is reasonable if justified.

Both frontends should consume the same supported-format rules and resolution
behavior. Do not duplicate or broaden the material interpreter during this move.

Keep parsing, CPU intermediate structures and renderer/GPU representations
separate. Parser/catalog/preview libraries must not depend on Bevy, egui or GPU
handles. Preserve cancellation, budgets, ambiguous candidates and explicit
unsupported/protected outcomes. Permitted research does not mean normal viewer
loads should unexpectedly invoke decryption or executable-analysis tools.

Verify the existing eframe floor preview and diagnostic exports still work.

### 3. Implement the real Bevy floor preview

Add a small frontend, for example lbp-bevy-viewer, accepting
`--snapshot FILE --game-dir DIRECTORY --select-guid HEX`.

Load through existing approved read-only source access. Convert verified CPU
geometry, UVs and decoded pixels into Bevy Mesh/Image/material assets. Do not
export intermediate game assets to glTF, PNG or another asset directory.

Provide:

- A depth-tested perspective viewport
- Orbit, pan, zoom, reset and frame-selected controls
- Minimal GUID selection/load/reload controls
- Visible loading, success and unsupported/error states
- Asset identity and basic geometry/texture statistics

Use the established unlit direct-diffuse subset first. Keep the existing explorer
available as a diagnostic reference. Do not port the entire browser or every
inspector panel in this milestone.

Geometry should remain inspectable when a linked material is unsupported, with
the limitation clearly shown.

### 4. Verify the renderer adapter and lifecycle

Explicitly check:

- Position/index/UV correspondence
- Coordinate, winding and culling policy
- UV orientation using an asymmetric synthetic texture
- Repeat and clamp-to-edge behavior
- Depth occlusion using overlapping opaque synthetic geometry
- Finite bounds, camera framing, clipping and degenerate geometry
- Explicit color-space and alpha display policy

Preserve raw CPU geometry and UVs. Display centering/scaling belongs in the
renderer. Do not guess axis conversions or flip UVs merely to make a sample look
plausible.

If the shader requires normals, generate bounded diagnostic normals and label
them accordingly. Do not claim original LBP normals or lighting are reproduced.

Background loading must keep the application responsive. Selection/source
replacement, cancellation and reload must prevent stale results from publishing
and release obsolete scene entities and GPU assets.

## Scope and completion

Do not implement levels, physics, gameplay, animation playback or multiplayer.
Do not expand to general material graphs or multiple textured primitives yet.
Use authored synthetic fixtures and existing pinned community oracles. Private
captures/logs/diagnostic outputs stay under ignored local-analysis/.

Run meaningful focused tests, existing regressions, cargo fmt --check, Clippy
and an actual native build/run. Inspect native captures; do not equate compilation
with visual correctness or claim manual interaction without evidence.

Update STATUS.md, MODLOG.md and relevant architecture/transition documents. Record
exact commands, resolved failures, remaining limitations and blockers. Record source
URL, pinned commit/section, evidence category and coverage for new format knowledge.

Deliver:

- A working Bevy preview of the known floor asset
- The exact launch command
- Verification results and rendering limitations
- A short first-user-test walkthrough

When ready, open the interactive Bevy preview for me. Do not close unrelated
running applications.

The following milestone should extend primitive ownership and material groups
using this same proven pipeline. Finish and validate the Bevy slice first.

