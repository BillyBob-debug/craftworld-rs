# D194 — level shapes take their own PShape.color (2026-10-09)

Follows D191 ([d191-level-thing-defaults.md](d191-level-thing-defaults.md)), which gave
RENDER_MESH Things their own `PRenderMesh.editorColor` but left shapes explicit-only.
Scene assembly (`level_scene.rs`) and the viewer report; no parser or shader change.

## Question

After D193, Bravery Test (GUID 0001e795) still draws a grey square on the central beam
and a grey panel. The PC level report at b8331fd (this PR's first commit, which adds the
reason line) names two neutral shape batches, both failing with "Unsupported: diffuse
multiply requires two direct textures or one texture and static COLOR":
`shapes gfx 00012ba7 batch 0` (9 Things) and `shapes gfx 00018401 batch 0` (59 Things),
68 shapes in flat fallback colour in total.

## Rule

- With the level defaults on (`--no-instance-default` not given) and no explicit
  `--instance-rgba`, every shape batch loads its gfx material with the batch's
  `PShape.color` as the instance colour (ARGB bytes / 255, same conversion as D191's
  editorColor; no sRGB step). An explicit colour still wins for every shape.
- Shape batches are keyed by (gfx material, `PShape.color`), so shapes of one material
  but different colours no longer share a batch. A non-white batch's label gains
  `colour AARRGGBB` (`shapes gfx 00012ba7 colour ff4d4d4d batch 0`).
- Pre-0x38a float colours are clamped to 0..1 and rounded to bytes (none expected in
  the LBP2 band).
- `--level-report` prints `neutral (1 groups): <error>` for a failed shape or backdrop
  batch and `shape Things (N): ...` (first 16, reader order) for each shape batch.
- `PShape.brightness`, `colorOff`/`brightnessOff` and player colour are not applied.

## Evidence

- The refusal is the `_` arm of the multiply route in `linked.rs`: the inputs are not
  texture/COLOR. An unprojected ThingColor box (type 2) lands there, and the instance
  projection turns it into a static COLOR (`instance.rs`), so a supplied colour selects
  the existing texture × COLOR route. That these two materials contain a ThingColor is
  INFERRED from the reason and the D191 shape rule; the PC report after this change
  confirms or refutes it.
- `PShape.color` layout and default white: CONFIRMED_FROM_COMMUNITY_CODE
  (docs/d80-thing-color.md E7). Common values white, ff1b1b1b, ff4d4d4d
  (docs/d108-shape-geometry.md M7).
- `PShape.color` feeding THING_COLOR: INFERRED (D108 M7 STRONG_INFERENCE, the same
  standing as D191's editorColor link); no RenderDoc capture shows it.

## Tests

`shapes_split_per_shape_color_d194`, `shape_color_argb_packs_and_clamps_vectors`,
`shape_batch_neutral_reason_is_its_material_error` (level_scene.rs).

## Not tested (cloud)

No game data: Bravery Test level report, corpus census and the gameplay-distance
before/after capture on the beam are the PC verification.
