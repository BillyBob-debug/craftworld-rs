# Logic, Part 52

See docs/logic.md for Parts 1-49.

## Part 52: the followers' speed (D1150)

Part 12 read the follower's (configuration 11) wanted speed in 0x957df4 as a stop-in-time
rule, sqrt(2 x accel x range x distance), like the gyroscope's. Part 51 showed the
turning tweaks use the shaper 0x1c21f4 instead; the followers do too.

FOUND, the call (`bl` at 0x9586cc, the shaper's only call outside the turning packer):
the first argument is half = 0.5 (0xd97fc4) x accelStrength (+0x54) x velRange (+0x50) x
the activation (0xcaf9c); the second the body's velocity along the unit aim direction,
read from (Thing+0x64 -> +0x110)+0x10; the third to fifth are 0, so there is no lead; the
sixth the distance |error|. The error is (target - own) - unit(target - own) x r, with
r = minRange (+0xe8), or maxRange (+0xec) when the flee byte (+0xf5) is set. A zero
distance skips the call and the speed is 0. INFERRED: that the second argument is the
velocity.

FOUND, after it (0x958724 to 0x958738): the speed is clamped to +-velRange x |0xcbed4|.
The wanted velocity is the unit direction times the speed, renormalised into the row
(0x958068 to 0x958124, row +0x30 = |v|), so a negative speed flips the row's axis. The
limits stay +-accel x range (0x95839c), scaled by the activation. Configurations 6 and 7
do not call the shaper (6 stores |v| x velRange, 7 |v| x 0xcbed4), and 8 copies its
velocity.

Model: `MoveTweak::rows` takes the activation and asks `SpinTweak::speed(half, w, 0,
distance)` for a follower. The test `d1150_followers_ease_in_with_the_turn_shaper` checks
full acceleration from rest, half of it at half activation, the top speed, a stop at the
kept distance without overshooting, and braking when moving away.

Census (census-all-39, followers by ticks to full speed, ceil(1/accel)):

| 1 tick | 2-10 | 11-60 | over 60 | none |
|---|---|---|---|---|
| 1,061 | 904 | 241 | 25 | 63 |

Not modelled: the tweak+0xe5 == 0 branch (0x9587ac), INFERRED to subtract an offset
rotated by the +0x220 angle; the +0xe5 lvsl at 0x9585ac; the +0x5c rotation by 0xcc484
(0x9582b4), INFERRED to be a 2D rotation; the input scale 0xcbed4, read as 1.
