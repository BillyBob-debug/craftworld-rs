# Logic and circuits: the game's gadgets against the runtime

Lane: logic (docs/lanes/logic.md), decisions D1100-D1199. The oracle is game data, the
EBOOT (Ghidra bridge) and RPCS3 memory reads; nothing here needs a decrypted file.

## Part 1: census of the Rookie Test (D1100)

Counted from the level's Things (Rookie Test 0001e786, 3,247 Things; the Pod 00018fdb is
a shell of 6 Things with no switches, joints or chips, so its behaviour is all script).

Parts: SWITCH 914, SWITCH_KEY 507, MICROCHIP 262, PHYSICS_TWEAK 267, SWITCH_INPUT 31,
MATERIAL_TWEAK 61, EMITTER 24, JOINT 124, TRIGGER 113, CHECKPOINT 8, BODY 368, SCRIPT 221,
SEQUENCER 2, CONTROLINATOR 2, CREATURE 4, NPC 4, ENEMY 4, CAMERA_TWEAK 22.

Switches (type, count, what the runtime does):
- Computed (876 of 914, 96%): AND 14 ×37, countdown 15 ×73, timer 16 ×78, toggle 17 ×24,
  OR 21 ×48, XOR 22 ×2, NOT 23 ×27, selector 35 ×2, microchip 36 ×262, circuit board
  37 ×262, player sensor 3 ×25 and tag sensor 4 ×36.
- Sensors: the cone and layer tests always pass, and so do the player and team filters.
- Stored signal only (38): button 0 ×2, sticker 5 ×3, random 19 ×3, cutscene camera
  finished 32 ×11, magic mouth 34 ×17, control pad 38 ×2.

Other gadgets, before this lane:
- PHYSICS_TWEAK 267: read and dropped; movers, rotators, followers and anti-gravity
  tweakers were inert. Fixed for anti-gravity in Part 2.
- Joints 124 (piston 33, string 25, bolt 51, chain 4, spring-angular 4, motor 5, legacy
  2): simulated, but no switch signal drives a piston, motor or winch.
- MATERIAL_TWEAK 61: dropped; every shape uses friction 0.8, restitution 0.3, density 20.
- EMITTER 24, SWITCH_INPUT 31, SEQUENCER 2, CONTROLINATOR 2, creatures: dropped.
- BODY: all 368 have frozen != 0 (153 on a joint); they start asleep and wake on contact.

Fix order: 1 physics tweakers; 2 switch signals into pistons, motors, movers and
emitters; 3 SWITCH_INPUT, MATERIAL_TWEAK and EMITTER; 4 the 38 stored-signal switches and
the sensors' cone and layer tests; 5 sequencers, controlinators and creatures.

## Part 2: anti-gravity tweakers hold the walkway up (D1100)

The level's 267 PPhysicsTweak Things are now kept (`PartData.physics_tweak`). By
configuration: 0 ×169 (anti-gravity), 1 ×56, 11 ×35, and a few of 5, 8, 9 and 12.

The Rookie Test stop at x -385 (docs/level1-play.md Part 16) came from a frozen walkway
row falling as one island once Sackboy bumped column T2739. That column carries
anti-gravity tweaker T3090 (configuration 0, gravity 1, dampening [1, 1, 0],
lastKnownActivation 1), and so does T2736 at the row's other end.

What the game does (EBOOT 01.33, Ghidra):
- PPhysicsTweak is part bit 30; configuration at +0x0c, input mode +0x10, gravity +0x18,
  buoyancy +0x1c, dampening +0x20. Activation getter 0xcaf9c: mode 0 digital, mode 1
  the analog value's magnitude.
- The body update 0xc5f1c sums gravity × activation over every tweak in the body's
  hierarchy and sets the gravity scale to clamp(1 - sum, 0, 1) (0xc6308-0xc63b0). So
  gravity 1 at activation 1 cancels gravity; gravity 0 leaves it. Buoyancy and dampening
  are summed the same way.
- Other configurations go to SPU jobs through the packer 0x1c7ed0 (configuration above
  11 skipped): 1-5 and 9 through 0x95fb20, 6-8 and 11 through 0x957df4 (11 adds a
  position target). Their force math runs on the SPU and is not read yet; which gadget
  each number is stays INFERRED. No maximumMass limit was found on the PPU side.

lbp-sim now does the same for configuration 0: each body's gravity scale is
`clamp(1 - Σ gravity × lastKnownActivation, 0, 1)` over the tweaks on its hierarchy,
with the saved activation standing in until switch signals reach tweakers (item 2).
Dampening and buoyancy are kept but not applied. With that, the row stays up and the
same route inputs take Sackboy from x -385 to x 22,785 (tick 2012).

Test: `d1100_an_anti_gravity_tweaker_holds_its_body_up` (lbp-sim sandbox, synthetic).

## Part 3: tweakers follow their switch signals (D1101)

Wiring in the Rookie Test (switch outputs whose target is a gadget): pistons 27 of 33
(25 from microchip outputs, 2 from countdowns), one other joint, anti-gravity tweakers 3
(one NOT, two microchips), movers (configuration 1) 22, and single tweaks of
configurations 8 and 11. The other 166 anti-gravity tweakers have no input.

What a tweak sees (EBOOT 01.33, 0xcaf9c, constants 1.0 at 0xd952c4 and 0.0 at 0xd952c8):
- No input wired: 1, so an unwired tweaker is always on.
- Input mode 0 (digital): 1 when the signal's ternary is non-zero, else 0.
- Input mode 1 (analog): the magnitude of the signal's activation.
- A flag at Thing +0x7f forces 0 (not identified; not modelled).
The input mode sits at +0x10 of the part in memory; reading it as the stored `behavior`
is INFERRED. Every tweak in the Rookie Test stores 0, so all of them are digital.

The runtime gives this as `LevelWorld::tweak_activation`. Each body keeps its tweaks
(`AntiGravity.tweaks`), and the viewer's `follow_tweaks` system, after the tick's switch
pass, sets the gravity scale from the live activations and wakes a body whose scale
changed. Before the first tick the saved lastKnownActivation stands in.

Tests: `d1101_a_tweak_reads_its_input_digitally_or_by_magnitude` (lbp-runtime
level_world) and `d1101_a_switched_off_anti_gravity_tweaker_lets_its_body_fall` (lbp-sim
sandbox), both synthetic.

## Part 4: pistons with no input run the game's wave (D1102)

The Rookie Test's 33 pistons all store tweakTargetMin/MaxLength 0, so the old scene held
each one at its rest length. What drives them is the motion block: animationRange (signed,
31.5 to 1,858), animationTime (6 to 180 frames), animationPause (0 or 15) and pattern 0.

What the game does (EBOOT 01.33, read with capstone; Ghidra shows nothing there):
- The joint length function 0xc95ec (jump table 0xc9658) sends types 3, 4 and 9 to
  0xc9790; other types return the base length at +0xf0.
- PJoint offsets (serializer 0xb41850): animationRange +0xc8, Time +0xcc, Phase +0xd0,
  Speed +0xd4, Pause +0xd8, modStartFrame +0x144, modDeltaFrames +0x148, modScale +0x14c,
  tweakTargetMax +0x1c4, tweakTargetMin +0x1c8 (not read on this path).
- No input on the Thing (+0x60 empty), or mode +0xa0 not 1, 2 or 4: free running on the
  retimed frame t (pointer +0x190; read here as the tick count, INFERRED) plus the phase.
  Pattern 0: base + range × (0.5 - 0.5 cos(π × wave)), the cosine INFERRED (0x88f64c).
  Pattern 1: base + t × speed. Pattern 2: a sawtooth from base + range to base over time,
  then base for the pause. Other patterns: base.
- The wave 0xc79c8: 0 when time < 1; over a cycle of time + pause it holds 0 for
  pause / 2, rises over time / 2, holds 1 for pause / 2 and falls over time / 2.
- Wired (mode 1 or 4): base + 2 × t × range / time on a retimed clock, so a ramp after the
  switch changes; mode 2 clamps t to time + pause then uses the same patterns. Who sets
  the clock (+0x190) and the mode on a switch change is not read yet.
- 0xcaf9c (the tweak activation) is not used by pistons; they only test for an input.

lbp-sim now gives each piston a `PistonDrive`; the viewer's `drive_pistons` sets the
prismatic joint's length every tick for the unwired ones (`SceneJoint::free_length`) and
wakes both ends. Wired pistons (27 of 33 here) keep their rest length until the
switch-driven clock is read. The base length is the stored length (INFERRED).

Test: `d1102_an_unwired_piston_moves_out_and_back_on_its_wave` (lbp-sim sandbox).

## Part 5: switched pistons run their clock while the signal is on (D1103)

The retimed frame from Part 4 is the piston's own clock (EBOOT 01.33, FOUND unless
marked):
- `+0x190` points into the joint itself (0xc80a0: `+0x110`; 0xc76e0 may swap in a copy at
  `+0x150`), so the clock is the saved modStartFrame, modDeltaFrames and modScale:
  frame = (now - start) × scale + start + delta, with now the game frame (0xc7548).
- SetScale 0xc7648: delta += (old scale - 1) × (now - start), start = now, scale = new.
  The frame stays continuous, so a signal changes the rate and never moves the piston.
- The per-frame pass 0xcfedc skips joints with no input. For a wired one it sets the scale
  by the mode (`+0xa0`, jump table 0xcffa4): 0 on/off, scale 1 while the signal's
  ternary is set, else 0 (the piston holds where it is); 1 direction, +value or -1 (or 0
  after a signal-type test 0x900730); 2 always 1, with a restart at the cycle's end
  (0x11f26c, 0xc7574); 3 the analog value; 4 speed (0xc9e80, not decoded). With no live
  signal object the clock resets to 0, 0, 1 (0xd0054).
- The tweak menu setter 0x1110e0 sets the mode (pattern 0: choice 0 mode 0,
  1 mode 3, 2 mode 1); it is also saved (Part 6).

In the Rookie Test every piston saves modStartFrame 5,306,444 and modScale 0. Play starts
from the reset clock 0, 0, 1 instead (INFERRED; with the saved values nothing would move).

lbp-sim keeps a `PistonClock` per piston. The viewer gives every wired piston mode 0
(INFERRED: the menu's first choice) through `LevelWorld::tweak_activation(thing, 0)`, so
it runs while its switch is on and holds while off; unwired ones run at 1 as in Part 4.

Test: `d1103_a_switched_piston_clock_freezes_while_off_and_never_jumps` (lbp-sim sandbox).

## Part 6: each piston's saved mode (D1104)

The mode at `+0xa0` is saved after all (EBOOT 01.33, FOUND): it is the last field the
PJoint serializer 0xb41850 registers (0xb41bfc, through 0xb3c7d4, an int). The bool just
before hideInPlayMode is `+0x1cc`, which holds the piston (scale 0). lbp-level now keeps
both as `JointParams::mode` and `held`.

In the Rookie Test all 27 wired pistons save mode 1 (direction), as do one unwired piston
and the wired type 9 joint; every other joint saves 0 and none is held.

What a mode does (FOUND unless marked):
- The rate (0xcfedc, jump table 0xcffa4): held 0; mode 0 1 while the ternary is set,
  else 0; mode 1 the ternary as a float (1 or -1), and with the ternary 0 either 0 or -1
  by the port test 0x900730 (taken here as -1, INFERRED: an off switch sends a direction
  piston back); mode 2 1; mode 3 the activation; mode 4 is 0xc9e80 (Part 8).
- The length (0xc9790's branch at 0xc993c, wired and mode 1 or 4): base + 2 × frame ×
  range / time, the base for time < 1, with no phase, pattern or wave. Other modes use
  the Part 4 rule on the clock.
- Neither function clamps the frame; the ends are the solver's (Part 7).

lbp-sim: `scene::piston_scale` gives the rate, `SceneJoint::length` the length; the
viewer passes the signal on each piston's input (`LevelWorld::input_signal`).

Test: `d1104_a_direction_piston_ramps_out_while_on_and_back_while_off` (lbp-sim sandbox).

## Part 7: a piston's ends are a joint limit, not a clock clamp (D1105)

What the game does (EBOOT 01.33, FOUND unless marked):
- Only three places write a piston clock: the mode 2 restart 0xc7574 (called from
  0x11f28c), SetScale 0xc7648 and the reset at 0xd0054. Nothing bounds the frame, so a
  direction piston's clock runs on past either end while its rate holds.
- The constraint solver loop 0x1c89d4 gives joint types 3 and 4 a limit row (0x960988,
  0x962590) between min and max of base and base + range (0xc7350, 0xc7330; base
  `+0xf0`, range `+0xc8`). The measured length is tested against both at 0x960d84 and
  0x960d8c, so the joint stops at its ends whatever the ramp asks for.
- So a direction piston left on for long stays out, and when switched off it waits
  until its clock has run back to frame time / 2 before it moves. Type 9 (angular) has
  its own limit row (0x95f494, INFERRED).
- The port test 0x900730 gives 0 for an ordinary switch output, so an off switch runs a
  direction piston back at -1 (INFERRED: the switch's type bucket is not mapped).

lbp-sim: `SceneJoint::length` keeps every piston's length between base and base + range
and the clock is no longer clamped (the D1104 `ramp_frames` and `PistonClock::clamp` are
gone).

Test: `d1104_a_direction_piston_ramps_out_while_on_and_back_while_off` (lbp-sim sandbox,
updated for the limit).

## Part 8: speed pistons servo to their signal (D1106)

What the game does (EBOOT 01.33, FOUND unless marked):
- The mode 4 case of the joint pass (0xd00c8) calls 0xc9e80 with the signal's analog
  value (input `+0x18`, not the ternary). For types 3 and 4 the target is s = max(value,
  0), stored as Speed (`+0xd4`) and flipped to 1 - s when the range is negative and the
  pattern is not 2. The current length (measured, 0xc9d38) as a fraction of the way from
  min to max of base and base + range gives the error e = s - fraction; the clock rate is
  1 above 0.1, -1 below -0.1, else 10 × e (SetScale 0xc7648). The length then follows
  the Part 6 ramp, so the piston moves at full speed and eases into the target.
- Type 9 runs the same servo on a wrapped angle (0xca008, INFERRED).
- The other cases, confirmed: mode 0 the ternary set, mode 1 the ternary or 0 / -1 by
  0x900730, mode 2 1, mode 3 the analog value.
- 0x900730 is a component validity test on the signal's type (table 0x900778: types 2,
  10 and 20 always, 36-38, 44 and 55 by index), not a live-signal test. Which type a
  switch output has is still not mapped, so mode 1's off rate stays -1 (INFERRED).
- Motors and winches (Toolkit names, INFERRED: type 3 winch, 9 motor) share the length
  function 0xc9790; a motor spins on pattern 1 (base + frame × Speed) inside angle limits
  about `+0xe0` (0x95f494; unbounded when range is 2π or more); strength is `+0xc0`
  clamped to [0, 1] (0xca760). Not applied yet.

lbp-sim: `SceneJoint::servo_scale` gives the mode 4 rate, with the commanded length
standing in for the measured one (INFERRED). The Rookie Test has no mode 4 piston.

Test: `d1106_a_speed_piston_settles_at_its_signal_fraction` (lbp-sim sandbox).

## Part 9: winches run the piston rule (D1107)

What the game does (EBOOT 01.33, FOUND unless marked; TOC base r2 = 0xd9a0f8):
- 0xc95ec sends types 3 (winch), 4 and 9 to the same length function 0xc9790, so a
  winch's length follows the piston clock, modes and ramp (Parts 4-8).
- The solver's type 3 case 0x1c98b4 builds its row with 0x960988 (strength 0xca760, lo
  0xc7350, hi 0xc7330) and a stack flag of 1 (0x1c9964). Inside, the measured length is
  compared with hi (0x960d84) and lo (0x960d8c); with the flag clear one bound of the row
  is -FLT_MAX (0x9611ec), with it set 0. INFERRED: the set flag makes the row one-sided,
  so the rope only pulls. The flag-clear caller at 0x1ca288 (lo = hi = `+0xf0`, strength
  FLT_MAX) looks like a rod.
- The type 4 case 0x1c9988 (only when `+0x28` is set and `+0x29` clear) goes through
  0x962590, which wraps an angle by 2π before the builder 0x962b9c; the type names may
  be off by one there (open).
- Type 9 (0x1c9eac): range 2π or more (or pattern 1) is unbounded; pattern 0 limits
  `+0xe0` ± range / 2; pattern 2 `+0xe0` to `+0xe0` + range; rows 0x95ae04 and 0x95f494
  with the strength as a cap. The turn rate is in Part 10.

lbp-sim: a CHAIN joint gets a `PistonDrive` and a distance joint held at 0 to the length
(`drive_winches`, INFERRED one-sided); `PistonDrive::step` is shared with pistons.

Test: `d1107_a_winch_follows_the_piston_rule` (lbp-sim sandbox).

## Part 10: motors turn on the piston clock (D1108)

What the game does (EBOOT 01.33, FOUND unless marked):
- 0xc99d0 (types 7 and up) returns a motor's target angle: the length rule 0xc95ec less
  the base `+0xf0`, plus the centre `+0xe0`. It is not stored in a field.
- The row builder 0x95f494 (called at 0x1ca050) calls 0xc99d0 at frame and frame + 1 and
  drives the joint by that change per frame (wrapped by 2π); the error to the measured
  angle 0xc9250 also feeds the row (INFERRED, clamps 0x95f91c-0x95f944). Strength from
  0xca760 is the cap (f2); f1 is 0, or 5 × strength for pattern 2.
- Pattern 1 (0x95f5f0 to 0x95fa44, 0xc7444, 0xc7370): Speed `+0xd4` × the clock rate
  (`*(+0x190)+0x3c`), in radians per frame. Pattern 0 with time 1 or more:
  range / (time × 0.5) × rate, a sweep of centre ± range / 2.
- 0xc9af4 (from 0xcab7c) moves the centre by measure - target; INFERRED a sync when the
  joint first runs, so only the angle's change drives the motor.

lbp-sim: a MOTOR joint gets a `PistonDrive`; `PistonDrive::turn` steps the clock and
returns (angle(frame + rate) - angle(frame)) × TICK_HZ in rad/s, with
`SceneJoint::motor_angle` the length rule less the base, clamped to the range except for
pattern 1 or a range of 2π or more. `drive_motors` sets the motor's target velocity each
tick. This replaces `MOTOR_SPEED_SCALE` (1, unverified) with the found unit (× 30).
Left out: the position term and the strength cap. The sign (game angle against avian's
counter-clockwise) is UNVERIFIED.

The Rookie Test has one MOTOR joint (thing 58: mode 1, pattern 0, range π, time 180,
pause 30, so a wobble bolt), wired to microchip 57 (type 36), whose port is on from tick
0. By the rule it swings its half turn in 90 frames and holds; before it crept at 0.017
rad/s. Sackboy then drops at tick 1012 near x -1,977 (y 3,610) instead of staying up,
and the route ends at x 24,635 (tick 2008) instead of 25,645. A control run with the old
spin gives 25,645 again. Which is right needs RPCS3 (open).

Test: `d1108_a_motor_turns_at_its_speed_on_its_clock` (lbp-sim sandbox).

## Part 11: gyroscopes and rotators turn their bodies (D1109)

Census of the Rookie Test's tweakers other than anti-gravity (by configuration): 1 ×56,
11 ×35, 5 ×2, 9 ×2, 12 ×2, 8 ×1. Part 1 called configuration 1 the mover; the packer
says otherwise. All 56 have velRange 0.756 or 0.209 (π/15), accel 1 or 0.124, decel 0.

What the game does (EBOOT 01.33, SPU row packer 0x95fb20, configurations 1-5 and 9):
- Each tweak becomes one turning row on its body: a target turn speed (+0x20, per tick)
  and change limits (+0x24, +0x28), default ±accel × velRange.
- Configuration 1 aims at world up (direction (0,1,0,0) at c463b0): the angle error is
  wrapped to [-π, π] (0x960790), fed to 0x1c21f4 (a stop-in-time rule, shape INFERRED),
  and the result is clamped to ±velRange × input scale (0x960858). So 1 is the
  gyroscope. 3 aims along the body's velocity, 4 along controller stick slots 1 and 2,
  5 at a found target (0x1c3e04); none of these is modelled.
- Configuration 2 (target = slot 1 analog × velRange) and 9 (target = middleVel z as
  stored, no velRange) are rotators; the limit on the slowing side is decel × velRange
  (0x95fd4c).
- Last (0x95fdc8): both limits × activation (0xcaf9c) × 1/mass. Gravity, buoyancy and
  dampening are not read here. Input scale 0xcbed4 is 1 unless input mode is 2.
- A flip flag (body matrix determinant < 0, XOR the byte at +0x224) negates the target;
  not modelled.
- Configurations 6-8 and 11 (the follower, 35 in the level, mostly with followKey set)
  go through 0x957df4 (Part 12).

lbp-sim (`SpinTweak`, `drive_spins`): per tick, in order, each gyroscope (1) and rotator
(9) moves its body's turn speed towards its target by at most the limits × activation.
The gyroscope's target is sign(e) × sqrt(2 × accel × velRange × |e|) clamped to
±velRange (INFERRED shape). The 1/mass factor is left out (INFERRED: the solver turns
the limit back into a speed change). The sign against avian's counter-clockwise angle is
UNVERIFIED for rotators; the gyroscope is symmetric.

Test: `d1109_gyroscopes_right_their_body_and_rotators_turn_at_their_speed` (lbp-sim
sandbox).

## Part 12: movers and followers push their bodies (D1110)

0x957df4 (EBOOT 01.33) packs a tweak's linear rows for the packer 0x95fb20. It is called
twice (0x1c8030, 0x1c80b0); the second call happens only when decel × velRange is not 0
(0x1c8038). Each row is a unit axis, a target speed, limits, and 1/mass.

Row 1 (0x958068-0x958124): the configuration gives a wanted velocity v (x, y). The axis is
v / |v|, the target |v|, the limits -decel × velRange and +accel × velRange. A zero v
brakes: axis along the body's velocity (or world up), target 0, ±decel × velRange
(0x958260). Configurations other than 11 scale the target by the input scale 0xcbed4
(0x95813c). The limits are then × activation × 1/mass (0x958188).

Row 2 (0x958354): the cross axis (a.y, -a.x), target 0, ±decel × velRange; the follower
uses ±accel × velRange (0x95839c).

- Mirror (not 11; 0x957fb4): v.x negated when the body's determinant sign differs from
  byte +0x224. Local space (byte +0x5c, localSpace; 0x9582b4): v turns with the body.
- 8, the mover (0x9583c4): v = middleVel x, y. The census's one mover is [0, 35, 0].
- 6 and 7 (0x957efc, 0x95840c): v = stick slots 1 and 2 (0xcb02c, 0xcb074), squared to a
  circle. 6 multiplies the target by velRange and 7 does not.
- 11, the follower: the target is found by 0x1c3e04 (min_range +0xe8, max_range +0xec, a
  cone of angle_range degrees +0xf0, followKey +0xf4). followKey 0 searches players, and
  1 searches keyed things. It takes the nearest with min_range < distance < max_range in
  x, y; a negative max_range means 1e8, and a cone applies under 180 degrees. The goal is
  the point min_range short of the target (max_range with flee, +0xf5). With allowUpDown
  off (+0xe5), the push along the body's up axis turned by +0x220 is dropped. The speed
  is 0x1c21f4 (stop-in-time; accel × velRange × activation × 0.5 at 0xd97fc4), clamped
  to ±velRange × |input scale|. No target means the brake row with ±accel × velRange.

lbp-sim (`MoveTweak`, `drive_moves`): per tick, in order, each row moves the body's speed
along its axis towards its target by at most the limits × activation. INFERRED or not
modelled:
- The follower's speed is sqrt(2 × accel × velRange × distance), as the gyroscope's
  (D1109).
- The input scale is read as 1, and stick movers see no stick.
- The mirror flag and the +0x220 angle are not modelled. The cone is a half-angle about
  the body's up axis.
- Followers with followKey (keyed things) follow tags since D1111 (Part 13).
- The follower follows the play-mode Sackboy. 1/mass is left out as in D1109.

Test: `d1110_movers_reach_their_velocity_and_followers_stop_short_of_their_target`
(lbp-sim sandbox).

## Part 13: keyed followers follow tags (D1111)

With followKey set, 0x1c3e04 asks FUN_000ceb8c(world, key) for the tags of that key. It
reads a map at world+0x3a4+0x1c (falling back to world+0x328, 0xceb58) through
FUN_000ceac0: 0x18-byte entries matched on two ints, read here as the tag's colour and
name (INFERRED). Each key holds a list of 12-byte entries (x, ?, thing) sorted by x; the
search binary-searches [x - max - r, x + max + r]. For each tag it needs
min_range < distance < best (best starts at max_range; distance in x, y), the relation
filter 0xf9a64, a thing whose root (+0x5c) is not the follower's own, and the cone test.
It keeps the nearest. 0xf9a64 is a game-mode filter on the relation (0..4); in normal play
0, 1, 3 and 4 pass and 2 fails. It is not the key match.

lbp-sim: `build_scene` gives each (colour, name) a key number and lists every SWITCH_KEY
as a `SceneTag`. A tag with no position of its own (a tag placed on a microchip or a
part has only SWITCH_KEY) sits where its nearest positioned parent is. A tag on a body
is kept in that body's frame; `spawn_scene` puts the tags of a body on it as `TagMarks`,
and loose tags on one entity. `tag_spots` turns them back into world points each tick,
and `drive_moves` sends a keyed follower to the nearest matching tag it sees that is not
on its own body. INFERRED or not modelled:
- Two followers' bodies count as "own" only when they are the same body; the game
  compares the things' roots (+0x5c).
- The relation filter is read as "pass" (normal play).
- A keyed follower with no matching tag in range brakes.

Test: `d1111_keyed_followers_find_tags_with_their_colour_and_name` (lbp-sim sandbox).


## Part 14: every switch type across all levels (D1112)

The census runs the `level_scripts` example with `--all` over every level resource in the
01.33 catalog (378 levels, 358 read; 141,572 PSwitches). Counts by type, with how
lbp-runtime's switch pass (`compute_output`) handles each:

- Computed from inputs: microchip 36 and circuit board 37 (28,278 each), timer 16
  (15,513), countdown 15 (7,090), NOT 23 (5,794), AND 14 (5,727), battery 28 (4,599),
  OR 21 (4,089), selector 35 (2,869), toggle 17 (1,107), circuit node 40 (772), XOR 22
  (648), direction 20 (344), sign split 27 (130), random 19 (1,213).
- Sensors computed from the world: tag 4 (16,953), player 3 (5,627), impact 18
  (3,944; Part 15), camera finished 32 (2,760; Part 16), magic mouth 34 (1,426;
  Part 17), projectile 39 (816) and paint 8 (51; Part 18), linear velocity 42 (771)
  and angular velocity 43 (122; Part 19), grab 6 (666; Part 20), control
  pad 38 (535; Part 21), key remote 54 (497; Part 22), button 0 (224; Part 23),
  motion recorder 45 (142; Part 24).
- Kept as stored (the world signal is not modelled yet):
  angle 41 (126), 57 (123),
  65 (77), lever 1 (74), sticker 5 (68), score 30 (51), movinator pad 44 (50), 55
  (11), trinary 2 (7).
- Gate modes: AND 5,641 mode 0 and 56 mode 1; OR 4,084 mode 0 and 5 mode 1; NOT 5,686
  of 5,794 inverted. Timer behaviours 0-4: 4,194, 2,410, 5,159, 3,710, 40.
- The names come from docs/runtime-evidence/logic.md A4; types 55, 57 and 65 are past
  that table's LBP2 band and are named there as later additions (INFERRED: the 01.33
  table has 0x42 types, so the band runs to 65).

### The random switch (type 19)

Its settings sit at PSwitch+0xa8..+0xbc: behaviour, pattern, on time min / max and off
time min / max, in ticks (the AND / OR gates read their mode from the +0xb8 slot). Its
state is a seed (+0x1c), a phase (+0x24) and an on byte per output (SwitchOutput+0x24).

- Output (0x11c2e0): on (1.0, ternary 1) while that output's byte is set, with output
  0's player, then the shared inversion tail.
- Dice: seed = (seed × 0x2E90EDD + 0x0E908FC1) & 0x3FFFFFFF, then R(k) = k × (seed >>
  16) >> 14, in 0..k. Where the seed is first set was not found; it starts at 0 here
  (INFERRED).
- Pick (0x10df10, n outputs): pattern 0 turns only R(n) on; 1 turns on one of the
  outputs that are off (nothing, and no roll, when all are on); 2 is 1 but with all on
  clears them and turns R(n) on; 3 toggles R(n); other values do nothing.
- Step at the end of the update (0x12414c, table 0xd96504 entry 19): with port 0 wired,
  behaviour 0 clears every byte while the input is off, behaviour 1 stores the input and
  picks only on its rising edge (apply_signal 0x11f038); unwired on a sequencer,
  behaviour 1 stores on; otherwise the phase ticks. The step only runs for a Thing that
  is not deactivated (0x1244a4), so the deactivated branches do not arise.
- Tick (0x124298): on (phase > 0) counts down; at 0, pattern 0 clears and an off time
  of off_min + R(off_max - off_min) starts (one shorter when both on times are 0). Off
  counts up; at 0 the switch picks and an on time of on_min + R(on_max - on_min) starts
  (one shorter when both off times are 0). Neither time is ever 0.
- Tests: `random_dice`, `random_pick_patterns`, `random_switch_times`,
  `random_switch_inputs` in switches.rs.

## Part 15: the impact switch (D1113)

The impact sensor (type 18, 3,944 placed) is worked out each tick from that tick's
physics contacts. compute_output (FUN_0011bd58) sends it to 0x11c2a8, which calls
0x11ae80; nothing is stored and there is no end-of-update step (table 0x124b0c entry 18
jumps to the epilogue 0x124940).

- Body: the sensor's proxy (0x11b000), or its parent when the proxy has no shape
  (0x11b47c); with neither, off (0x11b488).
- Contacts: the world's pair cache (world 0xdf5b28, 64-byte records), visited by
  0x900f54. A contact with the body itself is skipped (0x900fb0).
- Impact: the record's two vectors at +0x10 and +0x20 are not both zero (0x90103c to
  0x901090), with no tolerance. With IncludeTouching (+0x8c) a contact that only
  touches also counts (0x9011e8). "ImpactTolerance" belongs to the sound part, not here.
- Who counts: with the tag filter (+0xc9) only an object carrying a tag of the sensor's
  key (+0x80; 0x11af34, 0x9010a0); otherwise by PlayerMode (+0x64): 0 the first player
  only (0x900fc0 to 0x901124), 2 any object, others any player (0x11b6d8, 0x900fcc).
- Signal: activation 1.0, ternary 1 (0x11873c, threshold DAT_00d961e8 = 0.25) with the
  player of the first player hit; inverted (+0xc5) through the shared tail.
- Not modelled: IncludeRigidConnectors (+0xca; on 0x11b0d0 to 0x11b478, off 0x11b584),
  the mask and shape tests of a touching contact, the game-state check of the tag
  filter, creatures as players and the tag's own player.
- lbp-runtime: `impact_sensor` in sensors.rs reads `LevelWorld.contacts` (`Contact { a,
  b, force }`). The host does not fill it yet, so impact switches read off for now.
- Tests: `impact_sensor_counts_hits_by_player_mode`,
  `impact_sensor_touching_tags_and_body` in sensors.rs.

## Part 16: the camera finished switch and the cutscene timer (D1114)

Camera finished (type 32, 2,760 placed) sits on a cutscene camera. compute_output sends
it to 0x11c854, which calls 0x10ee78. It reads the camera tweak and the cutscene
manager (update 0x369130, start 0x368c40, switch 0x368680), so the runtime now keeps a
small copy of that manager.

- Cutscene camera: a camera tweak with cameraType 3 (PCameraTweak+0x30).
- Time since used (PCameraTweak+0x5d): set to 0 while the camera's input is on or it is
  the cutscene's camera, otherwise up by one a tick while below 10; 0xff never counts.
- Start: with no cutscene, a cutscene camera with time above 9 and input on becomes the
  cutscene's camera; ticks, ending and its time go to 0 (0x368c40, 0x368680, 0x549f8).
- Hold: while the ending count (manager+0x18) is not above 0, ticks (+0x14) go up by
  one; once cutSceneHoldTime (ticks; below 0 for ever) is below the ticks, ending = 3.
  Each later tick takes one off; at 0 the cutscene ends.
- Signal: off without a cutscene camera, off while it is the cutscene's camera with
  ending not above 0, on ({1.0, 1, -1}) while time is at most 9, else off. So it fires
  for the last ticks of the ending and 9 ticks after.
- INFERRED: the input is the signal on the camera's port 0 (ternary not 0); the step runs
  before the switch pass.
- Not modelled: the trigger box of an unwired camera (0x56808), the transitions, a hold
  from the camera's audio (0x55160), skipping, chained cameras (0x365470), single use
  (+0x72) and the paused checks.
- lbp-runtime: cutscene.rs (`cutscene_step`, `camera_finished`); lbp-level reads
  `CameraTweak` (camera type, hold time, time since used).
- Tests: `camera_finished_fires_as_the_cutscene_ends`,
  `time_since_used_and_other_camera_types` in cutscene.rs.

## Part 17: the magic mouth switch (D1115)

The magic mouth (type 34, 1,426 placed) gets its signal from its script, but not by
asking it. compute_output's case 0x11c874 takes the Thing's script part and reads the
integer member `OutputEndSignalFrames` through the stub 0x89032c, which jumps to
0x61fd88. The member's name is cached at TOC 0xdaa0c0+0x2e38, and the name string's
address is at +0x2e3c.

- Signal: {1.0, 1, -1} while the member is above 0 (1.0 from 0xd95f84), else off
  (0x11bf08). Both go through the shared tail, so an inverted mouth is turned round.
- The bubble script sets the member to 2 when its message stops and counts it down in
  `Update__` (docs/level1-play.md), so the output is a 2-tick pulse.
- Before: the runtime stored the script's `SwitchGetNewActivation__i` result as the
  output. That skipped the inversion and kept the stored player. Type 34 now reads the
  member in the switch pass. Other scripted switches still store the script's value.
- INFERRED: with no script, or no such member, off.
- lbp-runtime: `magic_mouth` in switches.rs reads `ThingState.members`.
- Test: `magic_mouth_reads_its_end_signal_frames` in switches.rs.

## Part 18: the projectile and paint switches (D1116)

The projectile sensor (type 39, 816 placed) and the paint sensor (type 8, 51 placed)
count hits like the counter. compute_output's jump table (0x11be00) sends types 8, 15
and 39 to 0x11ca04.

- Activation: BulletsDetected (PSwitch+0x18) / BulletsRequired (+0xcc, at least 1),
  clamped to 0..1. The player is the one stored with the count (+0x14).
- Ternary (0x11873c): the counter is on when full (0x1187e8); paint and projectile take
  the timer's rule (0x118874): 0 at 0, 1 when full, otherwise held from the output.
- End of update (table 0x124b0c entries 8 and 39 → 0x124c1c → 0x10d5f8): a running
  BulletRefreshTime (+0x1c) goes down by one, and at 0 the sensor empties (0x10a504:
  count 0, player -1). With none running, a projectile sensor with ResetWhenFull
  (+0xe0) whose count has reached BulletsRequired starts one of 6 ticks. A paint sensor
  first checks the hits it holds (0x10d690, not modelled), so it never starts one here.
- Hits (0x11dcb8): the count goes up by n, never above BulletsRequired or below 0 (such
  a hit changes nothing), and the player is kept unless it is -1. A projectile hit
  (0x373914, in 0x37349c) passes 1; a message (0x11f650) passes its own amount. The
  game also checks a tag key (0x11dd0c) and a player mask (0xf9a64), not modelled.
- Which hits count (0x10b6f0, by the projectile's kind byte, not modelled yet): a paint
  sensor takes only paint (0x8f); a projectile sensor by BulletTypes (+0xc4): 0 any but
  paint, 1 0x8c, 2 0x8d, 3 0x8e, 4 0x8b, above 4 none.
- lbp-runtime: arm `8 | 15 | 39` of `compute_output` and `projectile_step` in
  switches.rs; the countdown is `SwitchMemory.refresh`, loaded from bulletRefreshTime.
- Test: `projectile_sensor_counts_and_empties_when_full` in switches.rs.

## Part 19: the velocity switches (D1117)

The linear velocity sensor (type 42, 771 placed) and the angular velocity sensor (type
43, 122 placed) read the motion of a physics body each tick. compute_output's table
sends both to 0x11d28c (linear 0x11d1d0, angular 0x11d2a8); nothing is stored.

- Body: the switch Thing's parent's (Thing+0x5c), or the Thing's own with no parent.
  With no body, or a body with +0x140 set, off (0x11bf08); linear is also off when that
  Thing has no position (Thing+0x2c).
- Scale: angleRange (PSwitch+0x7c, the float after minRadius at +0x74 and +0x78; the
  angle switch 0x11cd3c reads it too). Mode: bulletsRequired (+0xcc).
- Signal: value / scale, kept to [-1, 1]; ternary 1 at or above T, -1 at or below -T,
  else 0, T 1.0 for linear (r2-0x4174) and 0.999 for angular (r2-0x3e10); player -1.
  Inverted through the standard tail (table 0x11bfd4 entries 42, 43 → 0x11c16c).
- Linear (0x11db10): 0 when the body's halfword +0xe is set; else the velocity of the
  Thing's point, linear + s (r.y, -r.x), r the Thing's position less the body's centre
  and s the angular velocity. ResetWhenFull (+0xe0, 0x11da58) turns it into the frame of
  0x1141f4's matrix m: x' = x m.x + y m.y, y' = x m.y - y m.x. Modes: 1 y, 2 x, 3
  max(-y, 0), 4 max(y, 0), 5 max(-x, 0), 6 max(x, 0), 0 and above 6 the speed.
- Angular: the body's angular velocity w. Modes: 1 max(w, 0), 2 max(-w, 0), 0 and
  above 2 w.
- INFERRED: m is the unit x axis of the body Thing's world matrix (0x1141f4 may also
  turn it to the nearest quarter turn). Not modelled: a Thing under an ancestor of
  object kind 0x96 (0x3c594), which reads a joint's angle (0x1d3700) or motion
  (0x1d315c) instead.
- lbp-runtime: `velocity_sensor` in velocity.rs reads `ThingState.body`
  (`BodyMotion { linear, centre, angular, still, inactive }`). The host does not fill it
  yet, so velocity switches read off (or on when inverted) for now.
- Tests: `linear_velocity_modes_scale_and_threshold`,
  `linear_velocity_spin_and_local_frame`, `angular_velocity_modes_and_threshold` in
  velocity.rs.

## Part 20: the grab switch (D1118)

The grab switch (type 6, 666 placed) is on while a player grabs the object it sits on.
compute_output's table sends it to 0x11cb50, which calls 0x10f508; nothing is stored and
there is no end-of-update step (table 0x124b0c entry 6 is the epilogue 0x124940).

- Object: the switch Thing's parent (Thing+0x5c), or the Thing itself with no parent
  (0x10f580). With none, off.
- Mode: PSwitch+0x8c (the IncludeTouching slot), dispatched at 0x10f6e4: 1 (0x10fb64)
  and 2 (0x10f7ac, which adds a test 0x1d31c0) look for a player whose grab target is
  the object; 3 (0x10fae4) for one whose held object (*(sub+0x140)+4) is it; any other
  value falls through to off (0x10f700).
- A player counts while grabbing (player+0x1c & 0x1001 and the byte +0x180). PlayerMode
  (+0x64) 0: only the local player (0x228818, 0x240a88); otherwise any player. With the
  tag filter (+0xc9, read as RequireAll) he must also carry a tag of the key (0x1d2540).
- Signal: {1.0, 1, the grabber's player number (sub+0x230)} or off; the ternary is set
  directly. Inverted through the standard tail (table 0x11bfd4 entry 6 → 0x11c16c).
- Census (census-all-2.json): mode 0 271, 1 1, 2 14, 3 380; PlayerMode 0 607, 1 25,
  2 34; tag filter 7; inverted 79; every one has a parent. The 271 of mode 0 read off
  (on when inverted) in the game too, unless a load step sets the mode (not looked for).
- INFERRED: modes 2 and 3 read as mode 1; the local player is the first player
  record's. Not modelled: creatures that grab (world+0x114, kind 0xc), the joint walk
  when PSwitch+0xcc is set, and the branch for a Thing under an ancestor of object kind
  0x96 (0x10f350).
- lbp-runtime: `grab_sensor` in grab.rs reads `PlayerState.grabbing`. The host does not
  fill it yet, so grab switches read off (or on when inverted) for now.
- Tests: `grab_switch_sees_the_player_grabbing_its_object`,
  `grab_switch_modes_parent_and_tags` in grab.rs.

## Part 21: the control pad switch (D1119)

The control pad (type 38, 535 placed) is the controlinator's switch: 18 outputs that
read the pad of the player driving it. compute_output's table sends it to 0x11d378;
each output's activation comes from 0x6a1004 and its player from 0x6a0728, and the
signal is stored at 0x11bf18 (the button prompt 0x3dae04 does not change it).

- Driver: the object at +0x98 reached from the switch's Thing. With no driver, or a pad
  that is not live (pad+0x1c & 0x1001), every output is 0.0 with player -1.
- Outputs (table 0x6a15c0): 0 the constant 1.0; 1 square; 2 circle; 3 triangle; 4 cross
  or d-pad right; 5 L1; 6 L2 pressure / 255 while L2 is down; 7 L3; 8 R1 or d-pad up;
  9 R2 as L2; 10 d-pad right less left; 11 up less down; 12, 13 the left stick (pad+0x30,
  +0x34); 14, 15 the right stick (0x38b604); 16 tilt pad+0xf8 / 90; 17 tilt pad+0x100 /
  -90, both kept to [-1, 1]. The player is the driver's number (*(slot)+0xc).
- Ternary (0x11873c, type 38 at 0x118830): outputs 6 and 9 are 1 above 0.0; the others
  0 within 0.25 (r2-0x3f10), 1 at or above 0.75, -1 at or below -0.75, and between them
  the value the output holds (slot+0x1c, 0x1188c0).
- Inverted: outputs 10 to 17 negated (table 0x11bfd4 entry 38 → 0x11c228), the others
  the standard tail 0x11c16c. No end-of-update step (0x124b0c entry 38 → 0x124940).
- Census (census-all-4.json): every pad has 18 outputs and a parent; none inverted.
  Wired outputs: 0 154, 1 10, 2 41, 3 16, 4 78, 5 3, 6 2, 7 4, 8 85, 9 3, 10 7, 11 3,
  12 99, 13 68, 14 9, 15 7, 16 0, 17 12.
- INFERRED: the driver is the player whose `piloting` is the switch's Thing; circle and
  cross are not swapped (0x6a0ea8 taken as 0); the mirror sign of outputs 10, 12, 14 and
  16 (0x6a1140) is +1; an inverted output's held ternary turns back as the inversion
  does. Not modelled: recorded values (+0x54 → +0x60 entries, 0x6a122c), the mask
  0xff0fff7f, the second table 0x6a163c, the right stick's curve 0x38b3ec and the d-pad
  and tilt fallbacks (0x386c78, 0x386fa8).
- lbp-runtime: `pad_sensor` in pad.rs reads `PlayerState.piloting` and `PlayerState.pad`.
  The host does not fill them yet, so pads read 0.0 with player -1 for now.
- Tests: `control_pad_maps_buttons_sticks_and_tilt`,
  `control_pad_dead_zone_hysteresis_and_inversion` in pad.rs.

## Part 22: the key remote switch (D1120)

The key remote (type 54, 497 placed) passes on the strongest signal among the tags of
its key, wherever they are. compute_output's table sends it to 0x11cf18, which calls
0x10fc88(out, &PSwitch+0x80, object); the result is stored at 0x11bf18.

- Object: the switch Thing's parent (Thing+0x5c), or the Thing itself.
- Tags: the key's entries in the tag map 0xceb8c, each read as the tag sensor reads it
  (0x12f6f4: off when the tag's Thing is deactivated, else the port 0 signal or on).
- Skipped: a tag at activation 0.0, and a tag whose Thing's parent-or-self is the object.
- Player: a tag signal with player -1 takes the creature's player number when its Thing
  is a creature's (Thing+0x78 & 0x100, 0x3c594 → 0x1d2d30).
- Kept: a tag whose |activation| is above the best so far (starting at 0.0), so the
  first in x order keeps a tie. The output is its {activation, ternary, player}; with
  none, {0.0, 0, -1}.
- Inverted through the standard tail (0x11bfd4 entry 54 → 0x11c16c). No end-of-update
  step (types above 0x27 go to the epilogue 0x124940 at 0x124938).
- Census (census-all-5.json): 497 remotes, 5 inverted, all with a parent, one output
  each (488 wired), 17 storing on. 298 keys have no tag in the level (INFERRED: their
  tags come in emitted objects; not checked); tags per key 1: 148, 2: 6, 3: 26, 4+: 19;
  none on the remote's own object. Wired tags per key 0: 420, 1: 68, 3+: 9.
- INFERRED: the creature is the player whose Sackboy Thing is the tag's Thing. Not
  modelled: the player of a tag owned by something else (+0x10 / +0x50 → +0x30 →
  0xf8e70); the skip of tags whose Thing+0x78 & 2 is set while a global (+0x180) is 1;
  the override after 0x10fc88 (0x2a1388, 0x2a7d38: a value per Thing UID when the
  object's +8 & 0x4000 is set, taken when stronger; INFERRED network or replay state).
- lbp-runtime: `key_remote` in remote.rs over `LevelWorld.tags` and `tag_signal`.
- Tests: `key_remote_takes_the_strongest_tag_of_its_key`,
  `key_remote_players_unwired_tags_and_inversion` in remote.rs.

## Part 23: the button switch (D1121)

The button (type 0, 224 placed) is on while something presses its body hard enough.
compute_output's table sends it to 0x11caac, which calls 0x11a998(out, PSwitch, 1); the
shared tail 0x11c2c4 inverts it.

- Force (0x100da4): over this tick's pair-cache contacts of the button's bodies (the
  object PSwitch+0x90, bodies +0x58 and +0x5a), skipping a body's contact with itself,
  the sum of each contact's two impulse lengths (+0x10, +0x20), counting only contacts
  whose other body's player passes the team filter (PSwitch+0x60, 0xf9a64).
- Player: the counted contacts' players in order, each one with a player replacing the
  one kept, except that once the kept player is output 0's player it stays; else -1.
- Activation: the force over 10000.0 (r2-0x3e6c) kept to [0, 1]. Ternary (0x11873c,
  the default rule): 0 at or below 0.25, 1 at or above 0.75, between them held.
- Release hold: while PSwitch+0x24 is 3 or less and the ternary is 0, the output is
  output 0's signal (taken back through the inversion so the tail gives it back).
- End step (0x124b0c entry 0 → 0x124bb8): while output 0 is on before the inversion,
  0x11a998 runs again with mode 0; a press sets PSwitch+0x24 to 0, none adds 1. So a
  released button stays on four more ticks. Then 0x10ffd4 eases the glow PSwitch+0x20,
  which changes no signal (not modelled).
- Census (census-all-6.json): 224 buttons, 3 inverted, one output each (205 wired),
  3 storing on, all with team filter 0 and a shape, 63 without a parent, none with
  port 0 wired.
- INFERRED: the button's body is the switch's own Thing and a body's player the player
  whose Sackboy Thing it is. Not found: where PSwitch+0x24 starts (here 0, so a button
  never pressed keeps its stored signal). Not modelled: the team filter's owner.
- lbp-runtime: `button` and `button_step` in button.rs over `LevelWorld.contacts`, whose
  new `Contact.impulse` the host fills (nothing fills it yet).
- Tests: `button_force_threshold_and_presser`,
  `button_holds_four_ticks_after_release_and_inverts` in button.rs.

## Part 24: the motion recorder switch (D1122)

The motion recorder (type 45, 142 placed) has three outputs, each read from one word of
the switch, PSwitch+0xcc (the slot other types call bulletsRequired). compute_output's
table 0x11be00 entry 45 is 0x11d4d0, which branches on the output index; the store
0x11bf18 inverts the result through the standard tail (table 0x11bfd4 entry 45 is
0x11c16c). There is no end step (table 0x124b0c stops at 0x27). The player is always -1.

- Output 0 (0x11d8bc): the low four bits over 15.0 (r2-0x4100), the ternary 1 when they
  are not 0, else 0.
- Output 1 (0x11d914): bit 4 as 0.0 or 1.0, the ternary the same bit.
- Output 2 (0x11d4e8): bit 5, the same way. Any other output is off (0x11bf08).
- Census (census-all-7.json): 142 recorders, none inverted, all with three outputs;
  the word is 0 in 78 and 1 in 64; only output 1 is wired (6), no output stores on,
  port 0 wired in 118.
- Not found: what writes the word while the game runs (the recorder's own update), so
  here it keeps the value the level file stores.
- lbp-runtime: `motion_recorder` in motion.rs.
- Tests: `motion_recorder_reads_its_word`,
  `motion_recorder_inverts_through_the_standard_tail` in motion.rs.
