# D191: level scenes use the D188 defaults, with each Thing's editorColor

Date: 2026-10-09 (America/Chicago).

## Problem

D188 made single-mesh preview and the mesh censuses default to a white instance
colour and animation base values. Level scenes still loaded every mesh with the
caller's source-only `PreviewOptions`, so materials that need a THING_COLOR input or
an animation base drew neutral there. The Bravery Test level-report count was 195
Things drawn with neutral groups (PC, main 983e05e).

## Change

`LevelSceneOptions::defaults` (`StandaloneDefaults`, default `ON`):

| Input | Model Things and decorations | Shapes | Backdrop |
|---|---|---|---|
| Instance colour | Thing's own `PRenderMesh.editorColor` | explicit only (unchanged) | D188 white |
| Animation base | on | on | on |

- Explicit inputs (`--instance-rgba`, `--animation-base`) win, as in
  `StandaloneDefaults::apply`. With an explicit colour, every Thing uses it.
- `editorColor` is packed ARGB (`-1` = opaque white); `editor_color_tint` turns it
  into linear-looking RGBA by dividing each byte by 255 (no sRGB conversion; the
  game's conversion is `UNKNOWN`).
- Cache: `build_scene_with_colors` keys loaded meshes by (source, colour variant).
  Each source first loads with the base inputs (white). A Thing with a non-white
  editorColor gets its own load only when that base load actually projected the
  instance colour (`MaterialAsset::instance_projection()` on some group), so
  colour-blind meshes stay one shared load. `build_scene` (backdrop plans, tests)
  keeps one load per source.
- Shapes: `PShape.color` is not wired to shape materials (not researched); their
  instance colour stays explicit-only. Backdrop plan Things use the white default,
  not their own editorColor.
- Viewer: `--no-instance-default` / `--no-animation-base` also turn the level
  defaults off (the old scene for before/after comparisons).
- `--level-report` adds: "Drawn with a non-white editorColor instance colour: N
  Things/decorations, M mesh+colour variants".

## Evidence

- `PRenderMesh.editorColor` field layout: `CONFIRMED_FROM_COMMUNITY_CODE`
  (docs/d80-thing-color.md E7), read by `crates/lbp-level/src/parts.rs`.
- 96% of placed RENDER_MESH Things carry `ffffffff`: `CONFIRMED_FROM_LOCAL_DATA`
  (docs/instance-color-census.md, B176 run on the PC).
- editorColor feeding the material THING_COLOR input: `INFERRED`. No RenderDoc
  capture shows it; the oracle scene files (lbp2/oracles/scenes) carry no colours.
- Animation base without game time: the existing D61 snapshot (`INFERRED` for time).

## Tests

- `cargo test -q -p lbp-preview --lib level_scene`: `d191_editor_color_variants_only_for_instance_meshes`
  (variant loads only for instance-colour meshes, shared loads, counts) and
  `d191_editor_color_is_packed_argb`.
- `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`, `cargo fmt --all --check`.

## Not tested (cloud container, no game data)

Level counts on game data. PC verification: `--level-report` for Bravery Test and
tutorial_23 with and without `--no-instance-default --no-animation-base` (neutral
Things before/after, colour variants), and one before/after Bravery Test capture.
