# Level scripts, part 89: the profile's colour, and what the badge card is (2026-10-17, D342)

Part 88 left the Story view's colour open: the game frame of part 86 draws a green title and
info bubble, while the player's first colour in memory is a yellow. This part finds where the
Story view's Gooey takes its colours, reads them from the owner's profile, and records what the
badge card beside the bubble is, for the next step.

### What was found

- FOUND (01.33, 0x7006c0 -> 0x4927b8 -> 0x487490, the switch at 0x4874c0): the filled
  bubble's fill is the Gooey's colour at +0x1c0; the bubble draw sets none of its own.
  0x48e9f8 is the one writer found: four packed colours to +0x1c0 (fill), +0x1e0, +0x1d0
  and +0x1f0.
- FOUND (the Pod's per-frame update 0x4381d8, block 0x438278..0x438318): each frame, before
  the bubble is built (0x7010b8 at 0x438520) and placed (0x701780 at 0x4387fc), the planets'
  Gooey (planets+0xbc) takes the local profile's colours: `*(profile+0x514)` (0x18ae48),
  words +0x4c to +0x58 (0x15894c, stored words, no palette).
- MEASURED (the owner's profile in RPCS3, a run not kept): `*(profile+0x514)` is the same
  object the player record points at (+0x16c), so the colours are the player's: 0xffceb424
  (yellow), 0xffc331a0 (pink), 0xff3bcfc1, 0xffe60f64. The owner's own game frame of the
  Story view (part 81, The Cosmos at 100%) is yellow with a pink goodies dot.
  INFERRED: part 86's green frame (Da Vinci's Hideout at 0%) came from another profile; the
  green is 0xff96dd3c, what a new profile takes (`DEFAULT_PLAYER_COLOUR`), which is what the
  viewer drew without a profile colour.
- FOUND (the badge, for the next step): the planet's badge is a mesh, not a Gooey card. A
  result's mesh GUID is at result+0x220: `cosmos_polaroid.mol` (0x6a1a), `cosmos_sphere.mol`
  (0x10c2d) or `badge_zip_entrance.mol` (0xe91); its instance is made at 0x6fcfd0, and each
  frame 0x6fe974 (from 0x4226b0) puts the slot's icon (result+0x2a8) in mesh 0's texture
  slot. `native_DrawAsPolaroid` (0x63aa64) tests for 0x6a1a. INFERRED: the cream frame and
  the coloured back are the mesh's own materials (no engine tint found).
- FOUND: `Planets.ActivateBadgeTether(Gooey, int)` (0x62d6cc) fills a tether record at the
  Gooey frame +0x20 +0x180 (0x6ff9c8): an active byte, the badge looked up by id (0x43612c),
  its flags OR'd with the argument. INFERRED: the bubble's placement (0x701780) points its
  tail at that badge.

### What changed

- lbp-runtime: `parse_profile_records` returns `ProfileRecords` (goodies and colours) and
  reads a line `colours C0 C1 C2 C3`; `LevelWorld::set_profile_colour` gives every player
  record the profile's first colour, also to records made later, and the session hands it
  to the shell level. `SearchSlot` keeps the slot's icon; `LevelWorld::story_info_slot` is
  the bubble's slot.
- `scripts/rpcs3_profile_records.py` writes the colours line from `*(profile+0x514)`.
- Left out: the badge mesh with the slot's icon (next), the other three colours' uses (the
  goodies dot is the second).

### Verification

`d342_profile_colours_line` (lbp-runtime), `d342_profile_colour_colours_the_players`
(level_world). Viewer capture (scripted Pod, held at tick 760, `--profile-records` with the
owner's colours, not kept): the info bubble is filled yellow, as the owner's game frame, where
it was green. `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D
warnings`; lbp-runtime tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
