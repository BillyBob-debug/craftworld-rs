# Logic, Part 51

See docs/logic.md for Parts 1-49.

## Part 51: the turn speed of 0x1c21f4 (D1149)

The turning packer 0x95fb20 (configurations 1-5) asks 0x1c21f4 for the turn speed it
wants. Part 11 read that as a stop-in-time rule, sign(e) x sqrt(2 x accel x range x |e|).
The EBOOT 01.33 read says otherwise.

FOUND, the call (0x9607ac to 0x9607e4): six float arguments. The first is half =
0.5 x accelStrength x velRange; the second the body's turn speed w (state+0x30); the
third and fourth are 0; the fifth the lead rate, the wrapped change in aim angle that
configuration 5's lead adds (vrfim wrap at 0x960790, 1/2pi at 0xd9836c), 0 for 1-4; the
sixth the angle error e, wrapped into [-pi, pi) at 0x96059c to 0x960604 (pi 0xd98364,
2pi 0xd98368): the aim's angle minus the up axis's, minus the +0x220 parent angle.
INFERRED: the error's sign, taken as positive anticlockwise as before.

FOUND, the helper 0x1c2128 (with the zero arguments dropped): for acceleration a and a
start speed m, t = 4a, U = -4a^2, W = t(lead - m), and the root
(-W - sqrt(W^2 - 4U((lead - m)^2 + 8ae))) / 2U, or -1e10 (0xd01502f9) with a negative
discriminant. 0x1c21f4 tries a = half, m = w + half and a = -half, m = w - half, and keeps
the one with the larger root h. With u = 0.25 x (2ha + lead - m) / a it wants:
- u > 1: w + 2a, full acceleration;
- else h > 1: w + 2a x u x (2 - u), easing in;
- else: e + lead, closing the gap in the tick.

FOUND, after it (0x9607ec to 0x960868): the speed goes to row +0x20, clamped to
+-velRange x |0xcbed4| (the input scale, read as 1 here), and the limits stay
+-accel x range.

Model: `SpinTweak::speed(half, w, lead, e)` and `shape_time`; `target_along` and `target`
take w. The test `d1149_turn_speed_speeds_up_eases_in_and_lands_in_a_tick` checks the
three cases and that a gyroscope tilted by 0.3 rad rights itself without overshooting.

Census (census-all-38, configurations 1-5 by ticks to full speed, ceil(1/accel)):

| configuration | 1 tick | 2-10 | 11-60 | over 60 | none |
|---|---|---|---|---|---|
| 1 gyroscope | 1,785 | 702 | 67 | 20 | 0 |
| 2 | 45 | 76 | 3 | 0 | 0 |
| 3 velocity | 94 | 107 | 9 | 2 | 1 |
| 4 stick | 68 | 12 | 0 | 0 | 0 |
| 5 target | 393 | 175 | 8 | 1 | 0 |

Most turning tweaks reach full speed in one tick, so for them the easing covers only the
last tick or two before the aim.

Not modelled: the lead rate from a host (passed as 0), the input scale 0xcbed4, and
whether the followers' speed in 0x957df4 is 0x1c21f4 too (still the sqrt rule, Part 12).
