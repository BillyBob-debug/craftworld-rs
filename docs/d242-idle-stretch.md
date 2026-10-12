# D242: idle breathing amplitude (2026-10-12)

Owner (2026-10-12): the body stretches vertically a little too much during the idle animation.

## Measured (game `idle.bin`, 20 ticks vs our stand clip, per joint, y relative to joint 3)
| joint (body part) | game range | ours range | game mean | ours mean | bind |
|---|---|---|---|---|---|
| 12 chest/neck | 2.2 | 10.5 | 109.2 | 103.2 | 111.8 |
| 23, 24 head top | 2.9 | 11.3 | 117.4 | 114.0 | 123.2 |
| 22 head | 2.3 | 7.4 | 79.6 | 77.8 | 82.8 |
| 8, 10 hands | 3.2, 3.5 | 6.4, 6.7 | -38.3 | -34.5 | -34.6 |
| 25, 26, 30, 31 legs | 0.8-2.2 | 0.8-1.9 | 13.5 / 2.9 | 16.2 / 5.5 | 14.3 / 2.5 |

Same 20-tick period. The game moves the upper body about a quarter as far and holds it nearer the
bind pose (head 3-6 higher). Fit of `lerp(bind, stand frame, w)` to the 20 ticks: w 0.3-0.8 (about
0.5), RMS 0.9-1.4 against 1.3-1.6 for the full-weight stand clip.

## Change
`STAND_BASE_WEIGHT` = 0.5: the stand clip is lerped over the bind pose at half weight (only when the
base is Stand; walks/jumps lerp over it as before). Weight INFERRED from the fit, amplitude MEASURED.

## Open
- Why the game's weight is below 1 (mood-stand layers? 0x294 clips at neutral?) and whether the idle
  chain clips need the same; the jump layer's "10-40% stand under it" (D234) may be the same effect.
- Verify in RPCS3 with a tick-aligned idle capture right after spawn (stand, not idle chain).
