# Level scripts, part 134: a badge's radius (2026-10-19, D387)

Part 133 drew the links with a level badge's angular radius of 0.09 (INFERRED). This
part traces where the game sets that radius. No code changes: the value comes from a
loaded settings resource, and the disc run's link conflicts with the obvious reading,
so it needs a measurement.

### Found (EBOOT)

- The link drawer's badges are the planet's 0x2c0-byte elements (planets+0xf8, looked
  up by SlotID through 0x426c64), the same ones as the cards with the scale spring
  (part 99): the angular radius at +0x210 and the card's scale are one field.
- Two stores write +0x210: 0x42af8c snaps it to the target (when
  `*(planets+0x20)+0xb0` is set, zeroing the speed at +0x21c), and 0x42b534 is the
  spring in the per-frame update 0x42a92c (speed 0.7 at 0xda2724, pull 0.25 at
  0xda2728). +0x214 and +0x218 follow from it through 0x89568c.
- The target (0x42ae44-0x42aeb0) is `f(O) * k`: `O` the element's object (+0x30,
  through 0x6fb9a4), `f` its virtual at +0x24 (0 when `O` is missing or 0x428174 says it
  is hidden), `k` planets+0x114 when 0x420890 holds for the element's key (type 2, or
  type 1 with 3 or 4 at key+4), else planets+0x118.
- planets+0x114 and +0x118 start at 0 (0x425d8c, 0x425d94) and are loaded from the
  settings resource at planets+0x94, its +0xc4 and +0xc8 (0x42f834, 0x42f840). No 0.27
  constant is in the EBOOT.
- `f` is 1.0 for two classes (0xa3f870, constant 0xda3084). A third class (vtable
  0xd765a0, 0x453584) scales a value of its own by global settings (+0x1e4 to +0x1f4)
  chosen by its flags.
- The polaroid's own size multiplies +0x210 by +0x29c (0x42a22c).

### Inferred

- Part 81's seeds (0x16afb0) are 0.18 for a group and 0.12 for a level, and the groups
  measured 0.27 (part 91), 1.5 times the seed. The same factor gives a level 0.18; even
  0.12 alone makes two levels' radii (0.24) reach across the disc run's link (13.5
  degrees, 0.236), which the drawer would then skip. Either a level's radius is smaller
  while the unlock sequence plays, or the game does not draw this link. In the groups-
  only Story view, both ends of a link inside one group resolve to the group's badge,
  which the drawer also skips.
- The viewer keeps 0.09 (part 133) until a level badge's +0x210 is measured.

### Next

Measure a level badge's +0x210 (and planets+0x114, +0x118) in RPCS3 while the unlock
sequence shows a link, and whether the link is drawn there.
