# B176: instance color census

Question: which instance colors are meshes (and shape materials) actually placed with
in the game? This is a read-only count of stored values. It does not render anything
and does not change the viewer.

## What it collects

`crates/lbp-level/src/instance_color.rs` (aggregation, unit-tested) and
`crates/lbp-level/examples/instance_color_census.rs` (driver) walk every
magic-confirmed `level` MAP row (236 level content hashes on 01.33) with
`lbp_level::read`, and every magic-confirmed `plan` MAP row with `lbp_level::read_plan`
(the existing D119 plan Thing reader; no new parsing). Each file is grouped by content
hash. Levels and plans are reported in separate sections.

For each read Thing:

| Histogram | Key | Value |
|---|---|---|
| `render_mesh_by_mesh` | RENDER_MESH `mesh` descriptor (`guid:XXXXXXXX`, else `sha1:<hex>`) | `PRenderMesh.editorColor` |
| `shape_by_gfx_material` | GENERATED_MESH `gfxMaterial` (the material the D114 extrusion renders with) | `PShape.color` |
| `shape_by_physics_material` | `PShape.material` descriptor | `PShape.color` |

Colors are the stored u32 as 8 hex digits (ARGB per Toolkit); `ffffffff` is the
default opaque white (-1). Pre-0x38a `PShape.color` floats would appear as
`v4:r,g,b,a` (not expected in the LBP2 band). Per key: Thing count, distinct file
count, and per color the Thing and file counts. Totals per histogram: keys, Things,
Things without a usable descriptor, Things with default white, distinct colors, keys
with a single color, keys whose only color is default white, keys with several colors.

Counting unit: one Thing with the part, per containing level (or plan) content hash.
Partially read files (`partial_things`) contribute the Things read before the stop.
Back-referenced PShape/PRenderMesh parts are counted once per Thing that carries them.

## Evidence

- Field layout of `PRenderMesh.editorColor` and `PShape.color`:
  `CONFIRMED_FROM_COMMUNITY_CODE` (docs/d80-thing-color.md row E7; Toolkit
  `structs/things/parts/PRenderMesh.java` lines 32, 99-108, `PShape.java` lines 55-85,
  179-226), implemented in `crates/lbp-level/src/parts.rs` (D107/D114).
- Stored values in the census output: `CONFIRMED_FROM_LOCAL_DATA` once run.
- Scope note (also in the JSON): placement colors from level Things; the link from
  editorColor to the material THING_COLOR input is `CONFIRMED_FROM_COMMUNITY_CODE` for
  the field only, not yet shown.

## Command (PC, PowerShell, project toolchain)

```powershell
cargo run -q --release -p lbp-level --example instance_color_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/b176-instance-colors.json
```

Output: a new JSON file under ignored `local-analysis/` (the tool refuses existing
files and other directories); schema_version 1, BTreeMap key order. A one-line summary
per section is printed.

## Tests and limits

- `cargo test -q -p lbp-level --test instance_color`: authored in-memory Things (no
  game bytes) check keys, Thing/file counts and the single-color / default-white totals.
- Not tested: the census was not run on game data (no game data in the cloud container
  where it was written); counts are unknown until it runs on the PC.
- Not shown: whether editorColor or PShape.color reaches the material THING_COLOR input,
  or how the game combines them with brightness. MATERIAL_OVERRIDE/TWEAK, decoration and
  sticker colors are not collected.
