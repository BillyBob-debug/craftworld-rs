# Level scripts, part 132: the links' growth (2026-10-18, D385)

A pending link now grows the way the game grows it, and its progress is kept for the
link's drawing. Part 130's step (INFERRED: 1/30 a tick) is replaced by the game's own.

### What changed

- lbp-runtime `level_unlocking.rs`: `unlock_query` binds
  `PodComputer.GetLinkAngularSeparation__Q8SlotLink` and
  `SetLinkProgress__Q8SlotLinkf`, the two calls `PendingLink.Update__` makes each tick.
- FOUND (`PendingLink.Update__`, 01.33): `Time` grows by 2 a tick up to
  `120 * (pi^2 - (pi - a)^2) / pi^2`, `a` the link's angular separation, and `Progress`
  is `SmoothStep(0, that end, Time)`, handed to `SetLinkProgress`.
- FOUND (`PendingLinkCluster`, state 2): the links below `Tick / 10` (the tick before
  it grows) update, then `Tick` grows; the cluster is done when its last link reaches 1.
  The native thread now does the same.
- FOUND (EBOOT): `SetLinkProgress` clamps to [0, 1] and stores it in the link's record
  (a sorted 0x50-byte vector on the planets, keyed by the two SlotIDs). The angular
  separation is the angle between the two badges' directions (INFERRED: the arc cosine
  of their dot), 0 for a link it does not know.
- lbp-runtime `lib.rs`: `LevelWorld.link_progress` holds each link's last progress.
- lbp-script-vm: the two natives are routed to the world.

Run on the disc with the scripts (Pod menu, Cross on the selected level at tick 720,
finish at 30): back in the Pod, the thread found the link at tick 41, cleared it and
unlocked (0, 125365) at tick 61 (53 with the old step).

Not found: the engine's drawing of a link from that progress (no pod script draws one).

### Next

Draw the links from `link_progress`.
