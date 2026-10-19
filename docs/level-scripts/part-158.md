# Level scripts, part 158: clearing a pending link (2026-10-19, D1011)

When the level unlocking cutscene ends without playing its links (no search up), the
Pod clears them with `Game.ClearPendingLinks__`. The viewer then marked their targets
unlocked as a PLACEHOLDER, guessing that the cutscene's `UnlockBadge` was what opened a
level.

### Found

- `ClearPendingLinks__` (0x5f4e80 -> 0x19ac2c) removes the links from the last, each
  through 0x19ab60, the same path as `ClearPendingLink`. 0x19ab60 calls 0x19aa7c with
  the link's target, then erases the link.
- 0x19aa7c acts when the target's SlotID type is below 2 or is 8: it finds or makes the
  target's record (0x199c9c, table +0x280, stride 0x80) and sets bit 1 of its +0x32,
  marks the profile changed (+0x3f bit 0) and recounts the records with that bit
  (0x19a8a0).
- That bit is the one the lock query 0x1a7efc reads (part 141): a slot that is
  `InitiallyLocked` is locked while it is clear. So clearing a link opens its target,
  played or not.
- `AddPendingLink__` (0x6008ac -> 0x1a8764) only appends to the list (after the lock
  test 0x1a7efc and a duplicate check); it opens nothing.
- `UnlockBadge` (0x63d4d0 -> 0x5037dc) finds the badge by the slot in a global map and
  calls a method of its object (0x43507c, slot +100); it writes no lock.

### Inferred

- What the badge's method does was not read; it is taken as the badge's own look.

### Code

- `StoryProgress::clear_pending_link` removes a link and opens its target for the types
  above. `ClearPendingLink`, `ClearPendingLinks__` and the native cutscene's cluster use
  it; `UnlockBadge` no longer opens a slot.
