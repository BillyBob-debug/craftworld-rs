# Level scripts, part 92: the Story badge's frame colour (2026-10-17, D345)

Part 91 found the profile's first colour (0xffceb424) twice in the Story badge's instance.
This part draws the card's frame in it.

### What was found

- MEASURED (the mesh's gmats, `mesh_group_probe` and `material_graph_dump`):
  `cosmos_polaroid.mol` has two groups. Group 0 (gmat 0x6a16) is the picture: the slot
  icon mixed over a grey (0.30) by its alpha. Group 1 (gmat 0x6a17) is the frame: a solid
  grey (0.486). Neither graph has a ThingColor box or any other per-instance input, so the
  viewer's instance colour (`PreviewOptions::instance_color`) does nothing on this mesh.
- INFERRED: the game applies the instance's colour outside the material graph, as a
  multiplier on the draw; how its shader takes it was not read.

### What changed

- `lbp_preview::tint_solid_group` multiplies a solid group's rgb by a colour (alpha kept).
- Viewer: `--profile-records` also gives the level loader the profile's first colour, and
  `load_badge` multiplies the frame group (`BADGE_FRAME_GROUP`, 1) by it. The picture
  group is left as it was. The `Story badge` line says whether the frame took the colour.

### Viewer run (part 80's flags, `--profile-records`, held at tick 760)

The frame is gold, as in the game's frame; it was pale grey. Captures stay in ignored
`local-analysis/d345/`.

### Left out

- The game's card shows a light mat inside the gold rim; the viewer's frame is gold up to
  the picture. Whether that mat is the picture group's grey (the icon not filling it) or an
  untinted part of the frame was not checked.
- The goodies icons along the card's bottom, and the info bubble's overlap.

### Verification

`d345_tint_solid_multiplies_rgb`. `cargo fmt --all --check`; `cargo clippy -p
lbp-bevy-viewer -p lbp-preview --all-targets -- -D warnings`; viewer story tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
