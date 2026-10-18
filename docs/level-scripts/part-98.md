# Level scripts, part 98: a left overlay shrinks out, and the cards go when Story closes (2026-10-17, D351)

Part 97 left open whether a badge overlay that stops being shown shrinks out, and whether
each Story opening is a first fill. This part reads the slot's free test and the card's
erase and rebuild paths.

### What was found

- FOUND (01.33, card slots): slot n's mesh at card+0x34+4n, scale (target, current,
  previous) at +0x68/+0x6c/+0x70 + 0xc n, spin at +0x104/+0x108/+0x10c + 0xc n, a "wanted"
  byte at card+0x1a0+n; 13 slots.
- FOUND (0x6fcc68, in 0x6fca08): a slot that exists but is no longer wanted gets scale
  target 0 and wanted 0; its mesh is kept. A new slot gets target 1 and wanted 1.
- FOUND (0x6fec94-0x6fecb0, 0x6ff324, in 0x6fe974): after the spring steps, a slot not
  wanted is freed (0x6fba24) once its current scale is 0 or below (r2+0x6c50 = 0.0);
  until then it is drawn at its shrinking scale. So a left overlay shrinks out: from rest,
  1, 0.75, 0.3875, 0.0369, then freed on the fourth update (the spring overshoots 0).
- FOUND (0x6fcfd0): slot 0 (not one of the twelve overlays) is freed at once.
- FOUND (0x42a92c, 0x42b3e0): the fill loop erases a card once its scale target and
  card+0x210 are 0 or below (0x4221d8, then 0x6fbb74 and 0x6fbaa0 free all its slots).
  The target is the level's scale times planet+0x114/+0x118.
- FOUND: the planets' rebuild 0x42324c (frees every card's slots, clears the grow flag)
  runs from `SetDecorations` (0x6162f4 to 0x4235fc) and when a newly requested planet
  resource finishes loading (0x43d474, 0x43c25c, through 0x436b50), not on every opening.
- INFERRED: closing the Story search sends the cards' scale to 0, so they are erased with
  their slots and the next opening builds new cards (constructors clear the grow flag,
  part 97): each opening is still a first fill, by new cards rather than the rebuild.

### What changed

- `OverlayGrow::leave`: one update towards scale 0, drawn while the scale is above 0 (the
  game's slot model; no viewer path drops a single overlay yet, the set is the profile's).
- `grow_badge_overlays` notes that a hidden overlay is a closed search (the card is gone),
  so overlays vanish with it.

### Left out

- The card's own scale spring when the search opens and closes (its constants), the
  dispatcher above CPlanets slot 0x1c (per tick or per frame), and whether a slot wanted
  again while shrinking is rebuilt only after it is freed (INFERRED from 0x6fca08).

### Verification

`d351_left_overlay_shrinks_out`, `d350_first_fill_is_settled`. `cargo fmt --all --check`;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
