# D41 Bevy asset preview — 2026-10-06

The independent Bevy frontend consumes the same CPU preview preparation as the
eframe explorer. No game assets, recovered code, decryption keys, or intermediate
asset files are bundled. Inputs are the existing validated schema-2 snapshot and
the user's extracted directory. Direct ISO loading remains deferred.

## Launch and first test

Double-click **Launch Asset Explorer.cmd** in the project root. It opens the
built Bevy viewer with the local verified inventory and extracted game directory,
initially showing the Valkyrie mesh. The launcher locates the checkout from its own
path, so it also works when started from another directory. It uses a process-only
PowerShell execution-policy override; it does not change machine policy.
The existing local executable, inventory, game directory and Rust runtime paths
must be present. Missing prerequisites print an error and keep the launcher console
open. This is a development-checkout launcher, not a standalone distribution.

Use **Browse assets** to search paths, filenames or GUIDs and click a row to load.
Hover a row for its full path, MAP identity and preview/failure explanation.
**Show files without a current preview** is off by default. Enabling it includes
other asset types and meshes whose last attempt failed. Unchecked ordinary mesh
candidates remain visible: header/type evidence does not prove renderability.
Confirmed types take precedence over extension hints. A successful preview means
geometry displayed; unsupported material groups can still be neutral. Failed
attempts are scoped to the attached source and can be retried from the revealed
row or the GUID field. Cancelled work does not mark a file unavailable.
All catalog rows/aliases are preserved; the list uses virtual scrolling and cached
filters. Browsing metadata does not eagerly read game assets.

Leave the GUID blank and click Load / reload to attach and browse without opening
an initial asset. Attachment survives an unsupported initial selection. Source path
changes clear cached outcomes. CLI filters: `--browse-query TEXT` and
`--show-unavailable`; `--select-guid` is optional.

To rebuild or launch manually, activate the existing local GNU environment:

~~~powershell
. ./scripts/use-local-rust.ps1
cargo build -p lbp-bevy-viewer
./target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 000150e5
~~~

The status should report the courtyard floor, 24 vertices, eight triangles,
one primitive, UV0 and a 512×256 BC1 texture using Repeat.
Drag outside the controls with the left mouse button to orbit, right/middle to pan,
and wheel to zoom. R resets; F frames the selected mesh. Reset and Frame selected
are also buttons. Edit the GUID and click Load / reload to select another resource.
Snapshot/source edits take effect at Load / reload and validate the new attachment.
Cancel discards pending work. Unsupported linked materials retain neutral geometry
with an explicit reason. Unsupported geometry reports an error.

Authored renderer verification scene:

~~~powershell
./target/debug/lbp-bevy-viewer.exe --synthetic
~~~

Left tile: UV0 asymmetric red/green top and blue/yellow bottom. Middle tile:
repeat UV range 0..2. Right tile: clamp-to-edge for the same coordinates.
Below them a smaller red foreground square must occlude the blue background,
even though the foreground is submitted first.
--qa-capture local-analysis/NEW.png captures the native render and exits.
Captures are diagnostics, never distributed fixtures. Scripted Bevy captures hold
the camera fixed so live mouse/keyboard input cannot alter the verification view.

## Separation and lifecycle

lbp-preview owns established bounded mesh reads, geometry decoding and the D40
direct-diffuse graph/binding/UV/texture route. It has no rendering/UI dependencies.
Both frontends consume LoadedMesh; linked-stage errors preserve geometry.
D40 predicates, budgets, conflicting-candidate behavior and evidence are unchanged;
see [textured-mesh](textured-mesh.md). A new short-page guard prevents read-loop stalls.

The Bevy worker loads/indexes the bounded snapshot and validates the read-only
attachment off the UI thread. Validated context is reused for same-source GUID
changes, with existing stability checks. Source path changes invalidate that context.
Replacing/cancelling a job cancels its token and drops its receiver. Obsolete results
cannot reach the scene. Replacing the scene despawns its entity and explicitly
removes owned Mesh, StandardMaterial and Image assets. One current scene is retained.

## Renderer policy and provenance

These are authored diagnostic display policies, not new LBP format rules.
Bevy 0.19.1 and bevy_egui 0.42.0 are exact dependency pins; Cargo.lock pins resolution.
[Integration support table](https://github.com/vladbat00/bevy_egui/blob/v0.42.0/README.md)
documents this compatible pair. Official APIs and local downloaded crate source
guided independent implementation; no example asset or implementation was copied.

| Policy | Evidence / coverage |
|---|---|
| Triangle-list positions, indices and UV0 correspondence | CONFIRMED_FROM_DOCUMENTATION: [Bevy 0.19.1 Mesh](https://docs.rs/bevy/0.19.1/bevy/mesh/struct.Mesh.html); adapter tests preserve index/UV pairing. Original CPU arrays remain untouched. |
| Right-handed display, Y up, camera looking down local -Z; no axis conversion | CONFIRMED_FROM_DOCUMENTATION: [Transform](https://docs.rs/bevy/0.19.1/bevy/transform/components/struct.Transform.html). Original LBP axis/front-face convention UNKNOWN; no conversion guessed. Triangle order retained; culling disabled. |
| Row-major RGBA with UV (0,0) top-left; UVs unchanged | CONFIRMED_FROM_DOCUMENTATION: [official custom mesh example, tag v0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/3d/generate_custom_mesh.rs); asymmetric authored native fixture. No V flip. |
| Linear filtering, repeat or clamp-to-edge on both axes | CONFIRMED_FROM_DOCUMENTATION: [ImageSamplerDescriptor](https://docs.rs/bevy/0.19.1/bevy/image/struct.ImageSamplerDescriptor.html); explicit sampler tests and authored native fixture. D40 wrap mapping retains its pinned game-format evidence. |
| Rgba8UnormSrgb, unlit StandardMaterial, opaque alpha, no tonemapping | Authored policy using [StandardMaterial](https://docs.rs/bevy/0.19.1/bevy/pbr/struct.StandardMaterial.html). Alpha bytes retained but blending/cutout ignored. Original game color-space/alpha fidelity UNKNOWN. |
| Depth-tested perspective, clipping 0.01..100 | Official Camera3d/StandardMaterial opaque pipeline; overlapping opaque synthetic native scene. No painter-depth sorting. |
| Display-only centering/uniform normalization using f64 bounds | Authored policy, finite/extreme/zero-span/index/UV tests. Raw CPU geometry retained. Degenerate faces skipped; no faces/zero-span gives explicit unsupported. |
| Diagnostic normal attribute | Bounded area-weighted normals from normalized triangles satisfy the unlit pipeline, with +Y fallback for unused/cancelled normals. UI labels them diagnostic; not original normals or lighting. |

Limits remain 100,000 vertices, 50,000 triangles, 4096 per texture dimension and
4,194,304 pixels, plus the existing loader/parser budgets. Camera distance is bounded
0.15..30, pitch ±1.5, pan target ±10. Display normalization does not prove game scale.
Since D148 single meshes are shown Z-up model space rotated to Y-up (front toward the
+Z camera; `--model-up y` keeps raw axes) and the home view is yaw 0 / pitch 0.15
(docs/d148-camera-sackboy.md).
No skinning, morphs, animations, multiple-material binding, general shader graphs,
levels, physics, gameplay, multiplayer or direct ISO loading.

## Verification

Native feasibility and final verification evidence are recorded in STATUS.md.
Private logs/captures remain under ignored local-analysis. No manual mouse/keyboard
interaction or other GPU/platform is claimed unless explicitly recorded there.


## D42 group support — 2026-10-06

Bevy now supports independently bound primitives within the same direct-diffuse
subset. Unsupported groups stay neutral with per-primitive reasons; shared cached
bindings merge for drawing. At most eight distinct material loads and nine draws.
Try GUID 00013051 for four unsupported material groups, or `--synthetic-groups`
for the authored red/green/neutral mixed scene. The floor launch and camera controls
above remain valid. [Group contract/provenance and verification](primitive-groups.md).

## D43 visible diffuse projections — 2026-10-06

Try GUID 00003114 for the colored bottle cap (three ignored graph nodes/wires), or
00013edb for the garage-light housing with a neutral unsupported center. Selection
and controls use the same launch command above with the new GUID. Both frontends
label diffuse-only projection limits. Other branches are not reproduced.
[Contract and native evidence](diffuse-projection.md). UV0 identity only; original
game shading, lighting/alpha/pose fidelity remain unknown.

## D44 UV1 — 2026-10-06

Use GUID00010e8d in the launch command for Valkyrie: two independently textured UV1
parts with different BC3 textures. GUID00020155 has one UV1 plus neutral unsupported
groups. Raw channels remain unchanged; selected pair feeds the preview's single
diffuse attribute. [Channel contract and verification](uv1-preview.md).


## D45 additional material support

D47 additionally displays static COLOR-only diffuse materials. No diffuse image is
required; the panel distinguishes textured versus solid-color groups and shows
assets ready for completeness review. Same36-asset baseline:13improved/23newsolid
groups, all-diffuse15→23; +0newviewable/+0verified complete. Try fez0001f915 or
mixed texture/color garage light00013edb. [D47 evidence/counts](solid-color-preview.md).

D46 additionally supports one texture multiplied by an unwired static COLOR.
Both frontends label tint and preserve original pixels. Three meshes in the fixed
36-asset sample gain six textured groups; +0 newly viewable/+0 verified complete.
Vulture `00027987` also shows a non-white tint. [D46 evidence/counts](tinted-preview.md).

Static diffuse sampler scale/offset is supported on UV0/UV1, with raw coordinates
preserved and derived sampling values labeled. Use GUID00002635 (2× strap) or
00016c87 (UV1 half-scale groups); other unsupported groups remain neutral.
Run --synthetic-affine for authored scale/offset/reflection checks.
Bounds, evidence and fidelity limits: [affine preview](affine-preview.md).

## Compatibility status and test placement — 2026-10-06

The clickable launcher's Bevy browser distinguishes **Viewable** (geometry was
prepared and installed in the scene, potentially with neutral groups) from
**Complete** (all textures showing and geometry visually verified correct).
Renderer completion, game lighting and full material/shader fidelity are separate.
The preview panel reports supported textured and solid-color primitive groups. Successful
diffuse loading does not automatically prove all textures or geometry correct.
Two explicit visual confirmations establish Complete; neutral groups or skipped
geometry block confirmation. Confirmations apply to the actually displayed GUID,
not an edited GUID field, and reload replaces its previous evidence.

Totals count distinct attempted GUIDs in the current attached-source session,
across all search results. Complete is a subset of Viewable. Failed attempts and
unchecked candidates remain distinct; MAP aliases remain individual browser rows
and do not multiply the attempted-GUID totals. Source changes/restart clear the
evidence; counts are not a persisted census or a claim about unchecked assets.
The old eframe inspector has no shared compatibility ledger yet.

Every future compatibility milestone must report newly viewable and newly complete
assets against an explicit baseline, using the same source, asset unit and tested
set. Whole-corpus totals remain unknown until measured. D45's three additional
material samples do not establish three newly viewable or complete assets.
This status/UI change adds **0 newly viewable / 0 newly complete assets**.

The Bevy window starts hidden, then centers on an OS-reported non-primary monitor
before becoming visible. On a two-monitor setup this is the second monitor.
Without a non-primary monitor it stays hidden; do not launch native tests in that
configuration. Multiple non-primary monitors use the first reported one. Physical
placement has not been verified in this change; headless tests avoid opening a
test program. Any future eframe/other test launch must also explicitly target the
second monitor. Use image inspection only to answer concrete visual questions.
