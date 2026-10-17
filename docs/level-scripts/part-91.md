# Level scripts, part 91: the Story badge's size (2026-10-17, D344)

Part 90 drew the Story view's badge card about 1.18 times the game's height. This part reads
the game's values in RPCS3 and sizes the card from them.

### What was found

- FOUND (0x4226b0 -> 0x6fe974 -> 0x6ffff4): a badge's radius is the planet's +0x1c plus a
  per-type table entry (TOC r2 0xdaa0c0 -0x7bf0, 0 for Earth's type 1); its scale is that
  radius x the result's +0x210 / 240 (240 at TOC +0x6e78).
- MEASURED (RPCS3 memory, Story view settled on The Cosmos, read only): Earth's planet
  object (type 1, +0x1c = 1750) holds six group results; each one's +0x210 is 0.27, and
  their planet-space matrices have a scale of 1.96875 at a radius of 1750. Part 81's 0.18
  (0x16afb0) only fills a settings block (+0x1b4, +0x1d4); 0.27 is 1.5 times that.
- MEASURED: the matrices' axes match `badge_matrix` (X along (0, 1, 0) x the direction).
- MEASURED: the badge's instance holds the colour 0xffceb424 twice (instance +0x10,
  +0x14), the profile's first colour (the `colours` line of `--profile-records`). For D345.
- INFERRED: at the Earth Thing's scale (0.961) a radius of 1750 lands near the Earth
  model's surface (about 1620); the Earth model's 1.35 put the card 1.46 radii out.

### What changed

- Viewer: `GROUP_BADGE_ANGLE` is 0.27; `add_badge` uses the Earth Thing's scale (with its
  rotation, about the Earth model's centre) instead of the Earth model's.

### Viewer run (part 80's flags, `--profile-records`, held at tick 760)

The card's top edge is at y 268 in both frames (1280 x 720); its height is 157 pixels
against the game's about 161 (was 194). Captures stay in ignored `local-analysis/d344/`.

### Left out

- The frame's tint (the profile's first colour, found above), the goodies icons along the
  card's bottom, and the info bubble's overlap.

### Verification

`d343_badge_matrix_is_the_games_basis` (now with 0.27). `cargo fmt --all --check`; `cargo
clippy -p lbp-bevy-viewer --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
