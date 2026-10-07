# Bevy preview frontend decision — 2026-10-06

Recommendation: make a Bevy 3D preview the next implementation milestone. The
verified D40 geometry/UV/texture/material slice now provides enough real input to
evaluate an engine frontend. Original B/C eframe choice served metadata browsing;
the current triangle painter's approximate depth ordering is no longer the right
foundation for growing model previews.

This is an architecture decision, not a completed Bevy implementation. No Bevy
dependencies were installed or built in this decision pass. The running eframe
explorer and its user test session were left intact.

## Reuse and separation

- Keep all original-format parsers, catalog, approved source attachment, read-only
  access, budgets and community-oracle fixtures. No Bevy dependency in these crates.
- Extract D40's CPU-only linked loading and direct-diffuse predicate from the viewer
  into a shared preview library when the second frontend consumes it. Its output
  is geometry/UV/pixels plus explicit supported/unsupported outcomes; no GPU handles.
- Add a small `lbp-bevy-viewer` consumer. Convert CPU positions, triangle indices,
  UVs and RGBA into Bevy Mesh/Image/material assets in a separate renderer adapter.
- Keep eframe usable as a diagnostic reference during the transition. Move browsing
  and inspector panels into Bevy/egui after the 3D slice passes; eframe host/event
  lifecycle code requires replacement, while catalog/filter/navigation logic remains
  reusable. Never run two competing window/render loops inside the same frontend.

Bevy provides an existing 3D rendering/scene/camera framework. The decision to use
it now is an engineering inference from the project's increasing viewport needs.
[Mesh API](https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html) and
[StandardMaterial](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html)
support the adapter route. The first direct-diffuse mode should be unlit; a lit
inspection mode may use explicitly generated normals and diagnostic lighting.
Neither mode proves original LBP material fidelity or decodes original normals.

[bevy_egui](https://github.com/vladbat00/bevy_egui) documents side-panel/perspective
viewport and render-to-image-widget examples. At research time its support table
lists Bevy 0.19 with bevy_egui 0.40–0.42; its example uses 0.19.0/0.42.0. These are
candidate compatible releases, not a dependency pin or a claim of latest release.
Verify released versions/features/toolchain compatibility when implementing, pin
the chosen Cargo.lock resolution, and use that version's API documentation.
Larger dependency/build cost and integration-version churn are accepted tradeoffs.

## D41 acceptance target

1. Shared CPU preview loader passes existing predicate/ambiguity/cancellation tests;
   the existing explorer still loads the floor with unchanged diagnostic exports.
2. A Bevy window loads the same user-owned floor mesh/material/texture directly
   through approved source access, with perspective orbit/pan/zoom and reset/frame.
3. Depth-tested opaque geometry passes an authored overlapping-triangle check.
   Asymmetric synthetic texture/UVs check orientation and repeat/clamp behavior.
   No new proprietary fixtures or exported asset files.
4. Native floor preview is inspected. Unsupported mesh/material states are visible;
   reload/selection/source replacement cancels stale work and drops GPU assets.
5. Record narrow tests, fmt/Clippy/build, native evidence and render limitations.
   No claim of automatic LBP shader compatibility, skinning, levels or gameplay.

Keep this first task to one proven asset and the established direct-diffuse subset.
Multiple-primitive ownership and material groups follow as D42; broad material
evaluation follows evidence later. Bevy's ECS may manage preview entities, but no
gameplay/physics/multiplayer work is introduced by this frontend decision.

## Handoff review: implementation order — 2026-10-06

The next-chat review retains Bevy as the recommended viewport direction, but adds
a build/render feasibility gate before extracting working loader code. Existing
eframe success on Windows GNU does not establish Bevy compatibility. Bevy's
[official Windows setup](https://bevy.org/learn/quick-start/getting-started/setup/)
describes MSVC/C++ tools; try the existing project-local toolchain with a minimal
released-version Bevy scene and report actual results. Do not silently replace the
working workspace toolchain or assume a rust-toolchain.toml exists (it does not).

Order: minimal synthetic Bevy scene and depth check → shared CPU preview extraction
with eframe regression → known floor in Bevy with minimal selection/status/camera
controls → lifecycle/native verification. Full browser/inspector migration and
multiple-primitive material work remain subsequent milestones. Minimal controls
should permit testing another GUID without hardcoding the floor into the loader.

Renderer adapter concerns are explicit acceptance checks, not new format claims:
axis/handedness/winding, UV orientation, wrap, color-space/alpha policy, degenerate
faces, finite camera/bounds and clipping. Preserve raw CPU geometry/UVs; any display
centering/normalization belongs in the renderer. If the chosen shader needs normals,
generate bounded diagnostic normals without claiming original normal compatibility.
Use unlit direct diffuse first; attractive PBR lighting is not an LBP material oracle.
The [official mesh example](https://bevy.org/examples/3d-rendering/generate-custom-mesh/)
documents normal attributes and winding; use chosen-version documentation at build
time. Dependency/version compatibility and native operation remain untested here.

## Research permissions revision — 2026-10-06

Current AGENTS.md permits local decryption/decompilation research on the user's own
copy when needed. Replace the earlier next-chat prompt's blanket prohibition with
that policy. Originals remain read-only; research/decrypted/recovered files and
game-specific keys remain ignored and excluded from distribution. Product source
must not copy/adapt/translate recovered LBP code or bundle original assets. Bevy D41
does not need these techniques; use the already verified clear-data route. Eventual
assets come from user-owned ISO/game data; direct ISO support is not implemented yet.

## D41 implementation result — 2026-10-06

The feasibility gate and first frontend are now implemented, built and natively
verified on the existing GNU toolchain. lbp-preview supplies both eframe and Bevy;
157 workspace tests pass. The floor loads directly from validated read-only game
data; authored native fixtures verify depth, UV orientation, wrap and opaque alpha.
The eframe diagnostic export remains byte-identical. Detailed policy, launch and
limitations: [Bevy preview](bevy-preview.md); exact check evidence: STATUS.md.
Multiple-primitive ownership/material binding is now D42. Earlier pending/build
statements above are historical. No executable research was needed.

## D42 implemented — 2026-10-06

Primitive ownership and independent bounded material outcomes now feed pooled Bevy
texture/neutral draws. 36 sample partitions match the oracle; mixed authored native
groups align, real circuit board retains all four unsupported groups, floor and
eframe export regression pass. 160 workspace tests and fmt/Clippy/builds pass.
[D42 evidence and limits](primitive-groups.md). Browser migration and runtime remain
later; complex graph expansion requires a separate evidenced contract.

## D43 implemented — 2026-10-06

A bounded shared diffuse projection now handles direct texture routes alongside
ignored branches. Bottle and garage-light native captures show visible asset
improvement; eframe reference and floor export remain working/unchanged. 160 tests,
fmt/Clippy/both builds pass. [Contract](diffuse-projection.md). UV1 direct sampling
is next; larger graph evaluation and runtime remain separate tasks.

## D44 implemented — 2026-10-06

Explicit UV1 direct sampling now works through shared preparation. Native Valkyrie
verifies two supported textured groups; UFO verifies neutral fallback. Eframe
retains single-primitive scope and unchanged floor export. 161 tests, fmt/Clippy/
build pass. [Contract](uv1-preview.md). Affine sampling remains a separate task.
