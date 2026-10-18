# D415: Sackboy's head follows the Popit cursor (Popit part 11)

Before: with the Popit cursor shown, Sackboy's head stayed on its neutral look. After: his
head turns and tilts toward the cursor, easing in over about ten ticks, and eases back when
the cursor goes.

## The rule (FOUND, EBOOT 0x20e28c, block 0x20e6a0..0x20f040)
- Head look = lerp(stick look 0x868..0x870, target look 0x874 / 0x878 / 0x87c, weight
  anim+0x880), then the D233 head-look path (0x200aa0 -> 0x2007e4: each value clamped to
  [-1, 1] picks the additive look clips' frame).
- Branches, in order: (1) the Popit cursor is active (0x1ec2dc -> 0x511788 on the poppet,
  not on his own Sackboy, not blocked by 0x13960c): target = poppet+0x1940 (INFERRED: the
  cursor's world place, written by 0x513e98); direction = target - head, x and y only,
  normalised, times 0.8 (0xd99470); each tick (0x1f70b0) left-right eases 0.1 toward
  0.8 dx, up-down toward -0.8 dy, roll 0; weight +0.1 a tick to 1. (2) creature state
  +0x41: look -(state+0x450), weight to 1. (3) a script / UI target Thing (0x3e515c):
  weight to 0.6. (4) else the weight falls 0.1 a tick.
- Body facing (0x2002ac): no Popit branch turns the body; the only pi (back to camera) case
  is the pod computer's user. The Popit exits (poppet modes 0x1e / 0x26 / 0x27 in game mode
  4 / 5, state +0x40 / +0x41) return 0.
- Arms (INFERRED): 0x2097e0 uses the same cursor test and poses both arms (0x20936c).

## Port
- `SackboyAnim::set_look_target` takes the direction from his head to the cursor; branch 1
  only (stick look 0, branches 2-3 not modelled). The direction is the screen one from the
  head's projected place (`popit_look`; head 165 units over the feet, INFERRED), standing in
  for the world x / y difference.
- Left-right sign MEASURED in the viewer: the clip's + value turns his face toward screen
  left, so a cursor to the right takes -dx. Up-down v < 0 = up (INFERRED, looks right).
- Check (disc + update only): `--level-path gamedata/levels/palettes/borders/blank_level_large.bin
  --play-sackboy --popit-cursor --qa-hide-ui --window-16-9 --preview-lighting` with the
  `d79-inventory-0133.json` snapshot and `game-data/a1-disc;game-data/a1-update-0133`:
  the cursor up and right of him, his face turned up and right toward it.

## Open
- The level frames' back view with the card open (D235 MEASURED, kept in `popit_turn`)
  has no EBOOT body-turn rule behind it; the Pod frame's look up at the card is not
  branch 1 (no cursor). Which Popit screens count as cursor-active is OPEN.
- The arms' pose toward the cursor.
