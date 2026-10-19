# Level scripts, part 135: a level badge's radius (2026-10-19, D388)

Part 134 traced a badge's angular radius at +0x210 to `f(O) * k`, `k` from the
settings at planets+0x94. This part follows `k` to its default: 0.08 for both a level
and a group. The viewer's level badges take 0.08 (was 0.09, INFERRED).

### Found (EBOOT)

- planets+0x94 is zeroed by the planets constructor (0x425cc0) and set only by 0x455dd8
  (0x455df8), called at 0x410284 from a loop over 0x28-byte entries (key at +0/+4, the
  settings object at +0x1c).
- That object is built by 0x456aa0: +0xc4 and +0xc8 copy +0x1a0 and +0x1a4 of a global
  settings block (pointer at 0xe9b9b0; 0x456b84 to 0x456bb0), +0xcc is 1.0.
- The block's constructor (0x16abac, its TOC 0xd9a0f8) stores 0.08 (0xd97560) to
  +0x1a0, +0x1a4, +0x1ac, +0x1b0, +0x1bc and +0x1c0 (0x16afd4 on), 0.2 to +0x1a8
  (planets+0x11c), and part 81's seeds 0.18 and 0.12 to +0x1b4 and +0x1b8.

### Inferred

- A level badge's `f` is 1 (the classes at 0xa3f870), so its radius is 0.08. With two
  levels at 0.16 the disc run's link (0.236) is drawn, which resolves part 134's
  conflict. No loader for the block was found, so a settings file could still change
  it.
- The groups' measured 0.27 is not `1 * 0.08`: their class is the third one (part 134),
  which scales a value of its own.

### Next

Measure a level badge's +0x210 in RPCS3 during an unlock to confirm 0.08.
