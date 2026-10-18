# Level scripts, part 99: the Story card opens at full size and shrinks out on closing (2026-10-17, D352)

Part 98 left open the card's own scale spring when the Story search opens and closes.
This part reads the spring in the planets' fill loop and the card constructor.

### What was found

- FOUND (0x42b4c8-0x42b534, in the fill loop 0x42a92c): the card's scale is a spring at
  card+0x210, its speed at card+0x21c: speed = 0.7 speed (TOC-0x799c) + 0.25 (target -
  scale) (TOC-0x7998), then scale += speed. The same constants as the slot springs.
- FOUND (0x42ae84-0x42aeb0): the target is the slot's scale (its vtable+0x24) times
  planet+0x114 or +0x118 (picked by the slot's type, 0x420890); 0 when the card has no
  slot or the planets' object at +0x94 is missing.
- FOUND: planet+0x114/+0x118 start at 0 (CPlanets constructor, 0x425d8c/0x425d94) and are
  copied from `(planets+0x94)+0xc4/+0xc8` when that object's dirty byte (+0xdc) is set
  (0x42f834/0x42f840), which come from a settings object; they are not open and close
  targets and are not eased.
- FOUND (0x42bc50 to 0x6fc414): a new card is made at its target with speed 0. Opening
  the search therefore shows the cards at full size at once; only closing animates.
- FOUND (0x42afa4): a card whose scale and target are both 0 or below is erased with its
  slots (part 98).
- FOUND: a snap branch sets scale = target and speed 0 when `*(planets+0x20)+0xb0` is set
  (meaning not identified), and a CPlanets virtual (0x4279a8) adds 0.05 to the speed.
- INFERRED: closing drops the slot's scale to 0, so from 1 the card goes 0.75, 0.3875,
  0.0369, then -0.2178, where it is erased; it is drawn while above 0.

### What changed

- `story_world::CardScale` (made at its target, `step`, `erased`) and `StoryCard`, the
  card shrinking out after closing.
- `follow_story_world` starts the shrink on closing and keeps the badge's card and
  overlays drawn until it is erased (`wanted_closing`); the rest of the Story world goes
  at once, as before. A line `Story card at tick N: shrunk out and erased`.
- `grow_badge_overlays` scales the card and its overlays about the card's place.

### Left out

- The snap flag and the 0.05 kick, what the card+0x214/+0x218 values computed from the
  scale are (two calls to 0x89568c), and whether the rest of the Story world also stays
  up while the cards shrink. The dispatcher above CPlanets slot 0x1c (0x43d900) is still
  unplaced (per tick or per frame).

### Verification

`d352_card_opens_full_and_shrinks_out`, `d352_a_shrinking_card_stays_drawn_after_closing`,
`d352_card_scales_about_its_place`. `cargo fmt --all --check`;
`cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
