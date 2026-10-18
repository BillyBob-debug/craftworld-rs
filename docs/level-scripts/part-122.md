# Level scripts, part 122: the player-count texts (2026-10-18, D375)

The native launch warnings now carry the slot's player bounds and give the popup's text
the way the game builds it. The Story run is unchanged: its slot has no player warning.

### What the game does (FOUND, 01.33 EBOOT)

- The launch check (0x402b24) calls 0x16e3b0 with the slot's minimum and maximum
  (slot+0xc5, +0xc6) when the party is under the minimum, and 0x16de78 when it is over
  the maximum, each into a 0x200 buffer that becomes the popup's body; world+0x3b0 gets
  0x10 or 0x20. The party's count is never put in the text, and there is no plural split.
- Too few (0x16e3b0): minimum and maximum differ, key 0x018707a4 with [min, max]; equal,
  key 0xf5796f9e with [min].
- Too many (0x16de78): bounds differ, key 0x16e7eeda with [max, min]; equal and not 1,
  key 0x0ada56d4 with [min]; both 1, key 0x1a2ff9c1 with no numbers.
- Formatting (0x518d0, 9 slots of value and type, count at +0x4c): `%` and a digit 1-9
  puts that slot; `%d`, `%s`, or `%l` and one more character put the next slot in turn;
  a slot past the count is written as the placeholder itself. Integers print in decimal
  (0x514d4).
- The popup: 0x3e995c with type 0x4b (INFERRED a message box), the body, and the two
  button texts 0x119fbf3b and 0x5162e2b2.

### What changed

- `lbp-runtime`: `LaunchWarning::TooFewPlayers` and `TooManyPlayers` carry (min, max);
  `LaunchWarning::text` gives the key and numbers; `format_lams` does the placeholders;
  `LevelWorld::launch_warning_text` reads the key from the loaded LAMS tables (from the
  disc, so no decrypted copy is needed) and gives `None` when no table has it.
  INFERRED: `%` before any other character is kept, and a numbered placeholder does not
  move the next-in-turn slot.
- Viewer: the level log prints each warning's text.

Parity 5 pass; the Story run is unchanged.

### Next

The warning popup's drawing, the entrance animation and the state 6/7 load.
