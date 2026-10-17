# Level scripts, part 90: the Story view's badge card (2026-10-17, D343)

Part 89 found that the Story view's planet badge is a mesh, `cosmos_polaroid.mol`, with the
slot's icon bound into its picture. This part draws that card on Earth for the default
result (The Cosmos) and places it as the game does.

### What was found

- FOUND (01.33, 0x6ffff4, the path badges take from 0x6feb90): a badge's matrix in its
  planet's own space, with `d` the slot's `location` and `s` = radius x angle / 240
  (240 at TOC 0x6e78): X = `s` x normalised (0, 1, 0) x `d` ((1, 0, 0) when parallel),
  Y = -`s` x `d` (into the planet), Z = `s` x normalised (`d` x X), at `d` x radius.
- FOUND (0x6fcfd0, 0x6fe450, the draw loop 0x6fee00..0x6ff0d8): the polaroid's own slot
  adds no scale, offset or spin; only `cosmos_sphere.mol` (0x10c2d) gets x2.45 and a lift
  of 100. The overlay slots 1..12 (goodies icons) are attached by 0x6fca08 with a random
  spin about Y of up to pi/8 and polaroid-specific offsets (TOC 0x6d4c..0x6d74).
- FOUND (0x9227c): the matrix goes unchanged into the planet's instance list (planet
  +0x24). INFERRED (0x6ffa64 multiplies the same output by the list's node matrices): the
  planet's world matrix places it.
- MEASURED (viewer): the Pod's Earth Thing's POS (translation (-1250, 0, 0), scale 0.961)
  is not the Earth model's matrix (centre (-2296, 1132, 40), scale 1.35; the model is
  placed by a bone). The planet's turn (part 82) works in the Thing's frame: with the
  Earth model's rotation the card ended up behind the machine on top.
- MEASURED (the card mesh): local bounds x -163..165, y -55..-1, z -190..190 (the picture
  faces -Y, out of the planet).

### What changed

- lbp-preview: `retexture` / `retexture_group` draw a single-texture group with another
  texture through the material's own colour chain (the chain is now `display_pixels`,
  shared with the load); `story::decode_icon_texture`; `LevelScene::positions` (each
  Thing's POS); `payload_bounds` is public.
- Viewer: the worker loads the polaroid with the default group's icon on group 0
  (`story_world::load_badge`); `add_badge` puts it on the Pod's Earth Thing as an alternate
  item (shown while Story is up) at `badge_matrix` in the Thing's rotation about the Earth
  model's centre, at the Earth model's scale. The load prints a `Story badge:` line.

### Viewer run (part 80's flags, `--profile-records`, held at tick 760)

The card shows with The Cosmos's icon; its centre is within a few pixels of the owner's
game frame's (640, 348 at 1280 x 720). It is drawn about 1.18 times the game's height
(194 against 164 pixels); without the Earth model's scale it was 0.66 times. Captures stay
in ignored `local-analysis/d343/`.

### Left out

- The card's frame colour: the game's is the profile's yellow, the viewer's the mesh's
  cream (INFERRED an instance tint, not traced).
- The goodies icons along the card's bottom (overlay slots), and the remaining size gap.
- The info bubble overlaps the card: its measured place (part 86) is left of the game
  frame's here, where its tail points at the card's right edge.

### Verification

`d343_badge_matrix_is_the_games_basis` (viewer). `cargo fmt --all --check`; `cargo clippy
-p lbp-preview -p lbp-bevy-viewer --all-targets -- -D warnings`; lbp-preview and viewer
tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
