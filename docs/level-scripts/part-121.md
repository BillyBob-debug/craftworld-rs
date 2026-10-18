# Level scripts, part 121: the cursor arrives (2026-10-18, D374)

The native planets now answer `PodComputer.HasCursorArrived` and keep the planets'
land-at-once flag. Neither changes the Story run: Story never asks whether the cursor
arrived, and every Story and group search passes the flag as false.

### What the game does (FOUND, 01.33 EBOOT and listings)

- `HasCursorArrived` (0x615998) calls the focus planet's vtable +0x3c (0x420868), which
  answers Planet+0x140 == 0: the cursor is not in flight. The scripts call it in
  `PodStateSearch` on its developer level move and in `PendingLinkCluster`.
- `SetWantSearchDetails`' first flag goes to Planets+0x5d9 (native 0x613574 -> 0x43c530
  -> 0x43c484). On each planet change 0x43c9dc copies it to Planets+0xb0; 0x43cd64 sets
  +0xb0 to 1 when a request is accepted and a new planet is queued, then clears +0x5d9.
  A nonzero +0xb0 lands a cursor flight on its first frame (0x4298e8). The Story and
  group searches, the decorate states and Tweak pass false; Stream and SearchList pass
  a computed value.
- The cursor's height and blend (Planet+0x1a0, +0x1b0): the constructor (0x425b14) sets
  both to 255.0, with +0x1a4/+0x1a8 the cosine and sine of +0x1a0. A flight to a node
  takes +0x1a0 from node+0x210 and +0x1b0 as 4.0 (0xd924ac) when node+0x220 is 0x6a1a,
  else 255.0; in flight both are lerped by the step from their start copies (+0x170,
  +0x180). Idle, they follow runtime tunables (0xdc3e54, 0xdc4160, 0xdc4328) that are
  not readable offline. No reader of either was found.
- Planet+0x1c, the size in the flight's speed, is 0x420814 of the planet's kind
  (+0x18): kind 1 the float at 0xd9246c, kind 2 the float at 0xd92468, else 255.0.
  The two floats were not read.
- The cursor's drawing was not found in the EBOOT; INFERRED script-side, from
  `GetCurCursorDir` (0x6108e4, Planet+0x190).

### What changed

- `lbp-runtime`: `PodComputer.HasCursorArrived__` answers no flight; `PlanetsSearch`
  keeps `land_at_once` from `SetWantSearchDetails`' first flag and `cursor_flight_step`
  lands at once while it is set. INFERRED: the flag takes effect at the call, not at the
  next planet change.

Parity 5 pass; the Story run is unchanged (no capture needed: neither path runs in it).

### Next

The player-count texts, the warning popup's drawing, the entrance animation and the
state 6/7 load.
