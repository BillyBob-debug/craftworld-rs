# D241: landing dip against the game's vertices (2026-10-12)

Owner (2026-10-11): "sackboy still seems extra bouncy like the model is stretching vertically a bit".

## Measured (RPCS3 vertex capture `local-analysis/d234-anim-1to1/jump.bin`, Moon, standing jump)
- Vertical arc: game centroid rise per tick 12.8, 31, 53, 78, 104, 129, 152, 172, 189, 203 ... apex +234;
  ours (body y + pose) 12, 31, 55, 80, 105, 130, 152, 171, 188, 202 ... +232. Matches; no change.
- Landing, lowest vertex relative to rest, one value per tick: -26.7, -29.0, -16.2, 0.0, +1.4, +2.6, +4.6,
  +0.7, -3.2, -3.1, -2.0, -1.3 ... (rest 0.0; idle holds -0.0).
- Ours before: dip table -26.2, -35, -35, -31.8, -25.3, -15.6, -8.6, -4.3, -2.5 ... (15 ticks) plus the jump
  clip tail's feet 4.5-6.0 below the root: lowest point about -41, 12 below the game, and 10+ ticks under rest.
- Idle: the game's top vertex moves 5.3 (201 -> 206.4) with feet fixed; ours moves 23, but that is the 408
  extra vertices of mesh 0000043f (2920 vs the game's 2512); the ICP fits of the stand clip stay 1.3-1.6.

## Change
`LANDING_DIP` (lbp-sim) = game lowest point series minus the pose's 5.5: -21.2, -23.5, -10.7, 0. Visual only.

## Open
- The physics body position dips 35 (d230); the drawn figure no longer follows it. Whether the game draws
  the root at the body position and cancels the clip's root offset is not resolved.
- Pose fit of the landing ticks against the clip frames is 3.5-3.8 (stop/turn were 3-5): landing tail poses
  still need tick-aligned captures. Take-off: game goes straight to a stretched pose (h 214-219); ours plays
  a 6-tick crouch first in the hold-6 test; not changed (needs aligned capture).
