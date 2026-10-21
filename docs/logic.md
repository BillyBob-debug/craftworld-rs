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
  5 at a found target (0x1c3e04); modelled since D1144 (Part 46).
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
(0x1c8038) or the tweak is a follower (0x1c81a4; D1163, Part 64). Each row is a unit axis, a target speed, limits, and 1/mass.

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
  motion recorder 45 (142; Part 24), angle 41 (126; Part 25), wormhole 57 (123;
  Part 26), pocket item pedestal 65 (77; Part 27), lever 1 (74; Part 28),
  three-way switch 2 (7; Part 29), sticker 5 (68; Part 30, its manual value),
  score 30 (51; Part 31), movinator pad 44 (50; Part 32, the control pad's rule),
  type 55 (11; Part 33, the pad without a player).
- Kept as stored (the world signal is not modelled yet): no placed type is left.
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
  turn it to the nearest quarter turn). Not modelled: a Thing under a cursor chip
  (object kind 0x96, Part 45; 0x3c594 returns its cursor record), which reads
  0x1d3700 or 0x1d315c on that record instead.
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
- The driver comes from the controlinator on the pad's grandparent (Part 38, D1136).
  INFERRED: a player is matched by Thing or by `piloting` (the controlinator's Thing, or
  the switch's Thing where there is none); circle and cross are not swapped (0x6a0ea8
  taken as 0); the mirror sign of outputs 10, 12, 14 and 16 (0x6a1140) is +1; an
  inverted output's held ternary turns back as the inversion does. Not modelled:
  recorded values (+0x54 → +0x60 entries, 0x6a122c), the mask 0xff0fff7f, the second
  table 0x6a163c, the right stick's curve 0x38b3ec and the d-pad and tilt fallbacks
  (0x386c78, 0x386fa8).
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

## Part 25: the angle sensor switch (D1123)

The angle sensor (type 41, 126 placed) turns on as the object it sits on turns back to
upright. compute_output's table 0x11be00 entry 41 is 0x11cd3c; the store 0x11bf18
inverts the result through the standard tail (table 0x11bfd4 entry 41 is 0x11c16c).
There is no end step (0x124934 sends types above 0x27 past table 0x124b0c). The output
index is not read, so every output gets the same signal; the player is always -1.

- The object is the switch Thing's parent (Thing+0x5c), else the Thing. 0x1141f4 takes
  its world matrix's Y axis (m[4..6]), turned by the shortest arc that takes its Z axis
  (made unit length) onto +z (0x113fb0), and makes its x, y part unit length: b. For an
  object in the level plane b is (m[4], m[5]) made unit length.
- b is turned by PSwitch+0xdc radians, the other way when the matrix is mirrored
  (det < 0); angle = atan2(-b.x, b.y), the turn about z, counterclockwise positive.
- r = angle / (angleRange (PSwitch+0x7c) / 180.0 × π). By bulletsRequired
  (PSwitch+0xcc): 1 (0x11d940) 1 + r when r < 0, else 0; 2 (0x11d9c8) 1 - r when
  r > 0, else 0; any other value 1 - |r|. The activation is that kept to [0, 1], the
  ternary 1 when it is above 0, else 0.
- A switch Thing that belongs to a creature (0x3c594, unless Thing+0x78 has 0x100) uses
  b = (sin φ, cos φ) of the creature's facing (creature+0x140 → +0x84) instead.
- Census (census-all-8.json): 126 sensors, 8 inverted, all with one output and
  bulletsRequired 0; output 0 is wired in all 126 and stores on in 17; every sensor has
  a parent. Ranges: 7.5 (58), 45 (26), 30 (11), 90 (9), 15 (5), 12.5 (4), 2.5 (3),
  0.5, 112.5 and 67.5 (2 each), 1, 5, 22.5 and 60 (1 each).
- Not modelled: PSwitch+0xdc (not in the serialized field order found so far, so 0
  here, which also makes the mirror test change nothing) and the creature case (no
  facing angle in our state). Without a world matrix the output is off here.
- lbp-runtime: `angle_sensor` in angle.rs.
- Tests: `angle_sensor_reads_its_objects_turn`, `angle_sensor_modes_and_inversion` in
  angle.rs.

## Part 26: the wormhole switch (D1124)

The wormhole switch (type 57, 123 placed) reports a wormhole that has just finished.
compute_output's table 0x11be00 entry 57 is 0x11d104. The store 0x11bf18 inverts the
result through the standard tail: 0x11bfac sends types above 0x37 to 0x11c16c, past
table 0x11bfd4. There is no end step (0x124934 sends types above 0x27 past table
0x124b0c). The output index is not read, and the player is always -1.

- The Thing is the switch's own (PSwitch+8), not its parent. The output is off
  (0x11bf08) unless bit 49 of the Thing's 64-bit part mask (Thing+0x10) is set and part
  slot 49 of its part block (Thing+0x34 → +0xc4) is not null.
- Otherwise it is on (1.0, ternary 1, r2-0x4174) while byte +0x16 of that part is set,
  else 0.0 and 0 (r2-0x41cc).
- Slot 49 is the WORMHOLE part (INFERRED: the runtime part enum shifts the indices
  above 0x28 by one, so serialized WORMHOLE 0x30 is slot 49; every type-57 Thing in the
  census has the part).
- PWormhole (ctor 0x137944, 0x3c bytes) sets +0x16 to 0. Its update 0x138040 (called by
  the manager 0xcfbb4 at 0xcfc04) clears +0x16 when it was 1, and sets it for one tick,
  clearing +0x17, when the wormhole stops being active (+0x17): when the countdown +0x1c
  passes 10 (0x138094), or when no player is inside. +0x17 is what the IsActiveWormHole
  native (0x6d3a2c) reads; TriggerWormHole (0x641208, 0x13883c) sets +0x15.
- Census (census-all-9.json): 123 switches, none inverted, all with one output; output
  0 is wired in 40 and stores on in 0; 56 have no parent. Parts on these Things: POS,
  WORMHOLE and SWITCH 123 each, RENDER_MESH 81, SCRIPT, SHAPE and TRIGGER 79 each, BODY
  57, DECORATIONS 2.
- Not modelled: the wormhole's own update, so `ThingState.wormhole_ended` stays false
  and a wormhole switch reads off here (the stored signal is no longer kept).
- lbp-runtime: `wormhole_sensor` in wormhole.rs.
- Tests: `wormhole_switch_reads_its_ended_byte`,
  `wormhole_switch_inverts_through_the_standard_tail` in wormhole.rs.

## Part 27: the pocket item pedestal switch (D1125)

Switch type 65 (77 placed) is named the pocket item pedestal (INFERRED: the switch-type
name table 0xd6cb58). compute_output's table 0x11be00 entry 65 is 0x11d164. The store
0x11bf18 inverts the result through the standard tail (0x11bfac sends types above 0x37
to 0x11c16c). The player is always -1.

- FUN_0010b0d8 returns the switch's value block at PSwitch+0xe4, allocating its 0x40
  bytes on first use (init 0x10b134-0x10b1b8): +0 an id (default from
  *PTR_DAT_00d95f34), +4 a wide-string name, +0x18 a string, and two vectors, empty at
  init: floats at +0x28 (count +0x2c) and s8 values at +0x34 (count +0x38).
- Output `o` reads activation = floats[o] and ternary = s8[o], each 0 past its count
  (0.0 is r2-0x41cc).
- FUN_0010e4f0 fills both vectors from a registry under FUN_00399b04's global (+0x4cc,
  entries at +0x4dc/+0x4e0, stride 0x30: name +0, id +0x14, float vector +0x18, s8
  vector +0x24) when an entry matches the block's id and name, and empties them on a
  miss. It runs when the key is set (FUN_0010e714, from the property dispatcher
  0x5c556c) and at level setup (FUN_001ab508 at 0x1ac3a0). No native writes the block.
  FUN_0010b1c0 saves the block only when +0xe4 is set (INFERRED: the vectors are a cache
  refilled from the registry). What fills the registry is not found (INFERRED: pocket
  item or profile state).
- Census (census-all-10.json): 77 switches, 8 inverted, all with a parent. Outputs: 3
  in 59, 2 in 10, 1 in 3, 4 in 3, 5 in 1, 6 in 1. Wired: output 0 in 19, output 1 in
  21, output 2 in 55, output 3 in 3. Stored on: output 0 in 4, output 1 in 3 (one at
  0.25709596), output 2 in 4 (one at 0.9419894), output 3 in 1. Parts on these Things:
  SWITCH 77, POS 9, RENDER_MESH 4.
- Not modelled: the registry, so `SwitchState.pedestal_activations` and
  `pedestal_ternaries` stay empty and every output reads off (on when inverted). The
  stored signal is no longer kept, so the 12 outputs stored on now start off.
- lbp-runtime: `pedestal_sensor` in pedestal.rs.
- Tests: `pedestal_reads_its_value_arrays_per_output`,
  `pedestal_inverts_through_the_standard_tail` in pedestal.rs.

## Part 28: the lever switch (D1126)

Switch type 1 (74 placed) is the lever. compute_output's table 0x11be00 entry 1 is
0x11cad0, which calls 0x11a6a4. The store 0x11bf18 inverts the result through the
standard tail (table 0x11bfd4 entry 1 is 0x11c16c). The player is always -1, and every
output gets the same signal.

- The handle is the Thing at PSwitch+0x90, a weak reference (ctor 0x10c930 zeroes the
  node {Thing*, prev, next}; link 0x511a8, unlink 0x511f8). INFERRED: it is the
  serialized referenceThing, the thing_ref read right after the switch type. Without
  one the signal is {0.0, 0, -1}.
- 0xf13a0 gives a position for each of the handle and the switch Thing (PSwitch+8):
  when the Thing's object at +0x30 is set, its local point +0x50 under the world
  matrix, else the world matrix's translation (+0x70 of the block at Thing+0xc). The
  handle's position less the switch's is taken into the switch Thing's frame by the
  inverse of its world matrix ((Thing+0xc)+0x40); x is the first component.
- Activation = (x + 100) / 200 (100.0 r2-0x3f6c, 200.0 r2-0x3eac) kept to [0, 1].
- The ternary is the default rule 0x11873c on output 0 (0x118874): 0 when |a| is at
  most 0.25 (r2-0x3f10), 1 at or above 0.75, -1 at or below -0.75, and in between what
  output 0 already holds.
- Census (census-all-16.json): 74 levers, 4 inverted, all with one output (67 wired)
  and a referenceThing; 12 without a parent, 18 with a child. Parts: POS, RENDER_MESH,
  SHAPE and SWITCH on all, BODY on 12. Stored activation of output 0: 0 in 57,
  0.0639 in 12, 0.7401 in 1, 1 in 4.
- With the translation as the position, x is -112 on 33 levers and -94.5 on 39 (the
  handle at rest at the left end of two lever designs), 54 on one and -100 on one.
  The rule's activation equals the stored one on 34 of 74; the rest differ by at most
  0.064 (0.028 against 0 on 27, 0.028 against 0.064 on 12, 0.770 against 0.740 on
  1). The ternary band agrees on 73 of 74. INFERRED: the gap is the body point
  (+0x30, +0x50) that the translation leaves out, or physics settling before the save:
  levers with the same x store both 0 and 0.064, so the saved geometry alone does not
  give the stored value.
- Not modelled: the body point; the position is the world matrix's translation. A
  handle or switch without a world matrix reads off.
- lbp-level: `Switch.reference_thing`. lbp-runtime: `lever`, `default_ternary` and
  `LevelWorld::reference_offset` in lever.rs.
- Tests: `lever_reads_its_handle_offset`,
  `lever_holds_its_ternary_between_the_bands_and_inverts` in lever.rs.

## Part 29: the three-way switch (D1127)

Switch type 2 (7 placed) is the three-way switch. compute_output's table 0x11be00
entry 2 is 0x11caf0, which calls 0x11a3b4, built like the lever's 0x11a6a4 (Part 28):
the same handle (PSwitch+0x90), the same two 0xf13a0 positions (0x11a41c, 0x11a43c)
and the same x in the switch Thing's frame.

- Activation = x / 100 (r2-0x3f6c) kept to [-1, 1] by two fsel pairs (1.0 r2-0x4174,
  -1.0 r2-0x4168). The ternary is the default rule 0x11873c, as for the lever. The
  player is -1, and every output gets the same signal.
- Inverted, table 0x11bfd4 entry 2 is 0x11c234: `fneg f31,f31; neg r30,r30`, so both
  the activation and the ternary are negated (not 1 - a as for the lever). The held
  ternary is output 0's negated back before the rule.
- Census (census-all-17.json): 7 three-way switches, 1 inverted, all with one output
  (3 wired) and a referenceThing; 3 without a parent, 1 with a child. Parts: POS,
  RENDER_MESH, SHAPE and SWITCH on all, BODY on 3. Stored activation of output 0: 0
  in 3, and -0.0098, -0.0109, -0.0146, -0.0153 on the others; all stored ternaries 0.
- With the translation as the position x is about 1 on all 7 (the handle centred); the
  rule gives 0.011 to 0.013 against stored values within 0.028 of it. The ternary band
  agrees on 7 of 7. INFERRED: the gap is the same unmodelled body point as the lever's.
- lbp-runtime: `trinary` in lever.rs; `default_ternary` now takes the held ternary.
- Tests: `trinary_is_signed`, `trinary_inverts_by_negation` in lever.rs.

## Part 30: the sticker switch (D1128)

Switch type 5 (68 placed) is the sticker switch. compute_output's table 0x11be00
entry 5 is 0x11c820, shared with types 9, 10, 17 (toggle), 28 (battery) and 29: it
reloads ManualActivation (PSwitch+0xc activation, +0x10 ternary, +0x14 player) and
stores it through the standard inversion tail. So the sticker switch outputs its own
ManualActivation, as the battery does.

- The writer is 0x53c584, run when a sticker is placed (not each frame). Through the
  setter 0x10a5ec (trampoline 0x89882c, call at 0x53c908) it sets ManualActivation to
  {1.0 (0xda6778), 1, -1} on each sticker switch whose Thing is within PSwitch+0x74 of
  the placed sticker (squared distance, 0x53c744..0x53c770).
- The reference is PSwitch+0x9c, INFERRED the serialized stickerPlan. Without a valid
  plan reference (resource type 0x26) any sticker matches; with one, the placed
  sticker's descriptor must equal it (0x53c9f0..0x53ca38; the hash compare 0x8959fc
  INFERRED). No code that clears it again was found.
- On a match it also triggers the switch's effect list (PSwitch+0x50) or the
  "gameplay/sticker_switch/match" sound; INFERRED, not modelled.
- Census (census-all-18.json): 68 sticker switches, none inverted, all with one
  wired output; ManualActivation {0, 0, -1} on all 68, and the stored output equals
  it (through the inversion) on 68 of 68.
- Not modelled: placing stickers, so ManualActivation stays as loaded.
- lbp-runtime: the `5 | 17 | 28` arm in switches.rs.
- Test: `sticker_switch_passes_its_manual_activation` in switches.rs.

## Part 31: the score switch (D1129)

Switch type 30 (51 placed) is the score switch. compute_output's table 0x11be00 entry 30
is 0x11c834, which calls 0x11ab74 and stores through the shared tail 0x11c2c4.

- The score: over the game state's player records (count +0x1e8, by index 0x156b74),
  only those whose number (+0xdc) passes the team filter (PSwitch+0x60, 0xf9a64). By
  the PWorld's game mode (the chain 0xf9a64 reads too): mode 0 the sum of their scores
  (+0x30); mode 1 (versus) the highest above 0.0 (r2-0x41cc), the first record keeping
  a tie, with its player; any other mode, teams (5) included, 0.0.
- The target is PSwitch+0xcc, the field the counters read as bulletsRequired. With 0
  the signal is {1.0 (r2-0x4174), 1, player}. Otherwise the activation is score /
  target, not clamped; the ternary is 0x11873c's rule for type 30 (0x1187e8): 1 at or
  above 1.0, -1 at or below -1.0, otherwise 0; the player is -1.
- Inverted through the standard tail (table 0x11bfd4 entry 30 is 0x11c16c).
- Census (census-all-19.json): 51 score switches, none inverted, all with one wired
  output and team filter 0. Targets from 50 to 300,000 (none 0). The stored output is
  {0, 0} on all 51, what the rule gives with no score.
- Not modelled: the switch Thing's owner (Thing+0x50) given to the team filter; as for
  the button it is nobody.
- lbp-runtime: `score_switch` in score.rs reads `LevelWorld.player_records`.
- Tests: `score_switch_sums_or_takes_the_best_score`,
  `score_switch_is_not_clamped_and_inverts` in score.rs.

## Part 32: the movinator pad (D1130)

Switch type 44 (50 placed, 25 outputs each) runs the control pad's handler (Part 21).
compute_output's table 0x11be00 entry 44 is 0x11d378, the same as entry 38.

- Each output's activation is 0x6a1004 and its player 0x6a0728 on the driver's pad,
  as for the control pad. With no driver, or a pad that is not live: 0.0 (r2+0x57e8 of
  the callee's TOC) and the player -1.
- The ternary is 0x11873c's jump for types 38 and 44 (0x118830): outputs 6 and 9 are 1
  above 0, else 0; the others take the pad's dead-zone rule.
- Inverted: outputs 10 to 24 are negated (table 0x11bfd4 entry 44 is 0x11c24c, which
  checks output - 10 against 14), the others take the standard tail.
- Census (census-all-20.json): 50 movinator pads, none inverted, all with 25 outputs
  and a parent; 13 also carry POS and RENDER_MESH. Wired outputs: 0 (35), 9 (24), and
  one each of 3, 8, 18 and 19. All 1,250 outputs store {0, 0, -1}, the no-driver
  signal.
- Not modelled: 0x6a1004's values for outputs 18 to 24 (0.0 here) and the button
  prompts set after it for type 44 (0x11d448).
- lbp-runtime: `pad_sensor` serves both types; `negated` in pad.rs gives the range.
- Test: `movinator_pad_reads_the_pad_and_negates_to_24` in pad.rs.

## Part 33: switch type 55 (D1131)

Switch type 55 (11 placed, 20 outputs each) reads the same pad as the control pad
(Part 21) without a player. compute_output's table 0x11be00 entry 55 is 0x11d098.

- The pad lookup and the activation call are the control pad's (0x89033c → 0x6a1004
  with the output and 0), but the player is kept at -1 (r28, 0x11d0f4) and no button
  prompts are set; the signal is stored through 0x11bf18.
- The ternary is 0x11873c's default rule (0x118878; type 55 is not in its list), so
  outputs 6 and 9 take the dead-zone bands as the others do.
- Inverted: outputs 7 to 19 are negated (table 0x11bfd4 entry 55 is 0x11c25c, which
  checks output - 7 against 12), the others take the standard tail. 0x900730 already
  marks the same outputs as signed. No end-of-update step (0x124b0c's table covers
  types up to 39).
- Census (census-all-21.json): 11 switches, none inverted, all with 20 outputs and a
  parent; one also carries POS and RENDER_MESH. Wired outputs: 13 (2) and 15 (4). All
  220 outputs store {0, 0, -1}, the no-driver signal.
- Not modelled: 0x6a1004's values for outputs 18 and 19 (0.0 here). The name is not
  known (Part 14: past the A4 table's band).
- lbp-runtime: `pad_sensor` serves the type; `negated` in pad.rs gives the range.
- Test: `type_55_reads_the_pad_without_a_player` in pad.rs.

## Part 34: switch inputs (D1132)

SWITCH_INPUT (PSwitchInput, part 0x20; 15,521 placed in 168 levels) is the adapter a
switch output drives on a Thing that is not a logic part. It is not polled: when an
output's on state or player changes, the switch pass (0x1244a4) calls 0x11ddf0 for each
target, which jumps on the target's object type (+0x7e) through the table at
`*(0xd962f8)`. Object types 68, 69 and 70 reach the three handlers below. Nothing reads
dataSource or the activation float, and no code compares updateType with 0, 8 or 9.

- LETHAL (updateType 2, 0x11ecf4): in the default mode (part +0x30 = 0) each visit sets
  lethalActivationFrame (+0x1c) through 0x12e3a8. It becomes the current frame while
  (port 0 ternary != 0) XOR lethalInverted (+0x38), and 0 otherwise, also when the
  Thing is deactivated (+0x7f) or port 0 is not wired. In mode 1 the frame is taken
  only on the edge. Then 0x12e114 resolves the group: the input with the largest
  nonzero frame gives its lethalType (+0x34); with none on, an input whose port 0 is
  not wired applies (it is always on); else 0. The game writes the type to the
  material (0x0fdc48) unless the current value is 6 to 11, and plays the lethal sound
  (0x2dcb3c) unless disableLethalAudio (+0x3a) is set.
- DESTROY (updateType 1, 0x11ec6c): on the edge only, 0x12f354 destroys the owner's
  group, with the rigidly joined Things when includeRigidConnectors (+0x3b) is set, with
  effectDestroy (+0x3c), credited to the signal's player.
- SACKBOT_BEHAVIOR (updateType 10, 0x11ebf8): on the edge, 0x12eccc on the owner's
  parent and its behaviour object. Not modelled.
- INFERRED: object types 68/69/70 are the Things whose updateType is 2/1/10, so the
  runtime picks the handler by updateType. The group is taken as the Things with the
  same root of the parent chain. Mode 1 is taken to be the stored `behavior` = 1.
- Census (census-all-22.json): 10,432 DESTROY (10,088 with port 0 wired, 4,590 with
  rigid connectors), 1,578 LETHAL (1,448 wired, 144 inverted, 196 with a stored frame;
  lethal types 1 ×795, 2 ×480, 13 ×289, 0 ×14; 367 resolve lethal at load), 3,463
  SACKBOT_BEHAVIOR (2,306 wired) and 48 CIRCUITBOARD (7, no handler, none wired). None is
  in mode 1 and none is on a deactivated Thing at load.
- lbp-level: `PartData::switch_input`. lbp-runtime: switch_input.rs, reached from
  `apply_signal`; `LevelWorld::lethal_type` and `destroy_requests` are for the host,
  which does not apply them yet.
- Tests: `lethal_input_follows_its_port_and_inverts`,
  `latest_lethal_input_wins_and_unwired_is_always_on`,
  `destroy_input_asks_on_the_edge_only` in switch_input.rs.

## Part 35: the material tweaker (D1133)

PMaterialTweak (part bit 0x22) sets the physics material of its Thing's rigid group.

- Offsets (reader 0xb585a4): +0x0c restitution, +0x10 frictionScale, +0x14
  hideInPlayMode, +0x15 stickiness (low 7 bits; bit 7 a separate flag, 0xadb84),
  +0x16 grabbability, +0x17 grabFilter, +0x18 noAutoDestruct, +0x1a
  disablePhysicsAudio, +0x1b hasShadow; +0x19 caches the on state.
- Kind: 0x3ed2c gives a Thing with a tweak kind 0x4d, or 0x9f when its stickiness is 3.
  0x11ddf0's entry for 0x4d (0x11e394) only recomputes the body (0xadd84 -> 0xadc64 ->
  0xfb258); the signal value is not used there. Kind 0x9f has no entry.
- On (0xadbac): off on a deactivated Thing (+0x7f); on with no wired input; otherwise on
  while the wired output's value is nonzero (INFERRED: port 0, the activation).
- The body (0xfb258, over the group from 0x3dae4): the kind 0x4d tweaks that are on give
  the mean restitution (body+0x74, shape byte v * 255) and mean friction scale
  (body+0x78),
  the largest grabbability (+0xc8), grab filter (+0xc7) and stickiness (+0xc9), and the
  OR of noAutoDestruct (+0xc0 flag 0x40), disablePhysicsAudio (flag 0x100) and
  hasShadow. With none on, the body keeps its material's restitution and stickiness and
  friction scale 1.0.
- Census (census-all-24.json): 8,827 tweaks, 8,825 with a parent, 588 with an input
  wired, 8,302 on at load. Grabbability 0 x6,291, 1 x53, 2 x2,366, 3 x117; stickiness
  0 x7,702, 1 x855, 2 x195, 3 x75; grab filter nonzero 16; friction scale not 1.0
  4,376; restitution not 0 472; noAutoDestruct 3,046; physics audio off 2,741; shadow off
  1; none hidden in play mode.
- Not modelled: kind 0x9f (stickiness 3 writes friction and restitution times 100 and
  flag 0x8000 to a child's body), the meaning of grabbability, grab filter and flag 0x40,
  the load fixup for stickiness 3 (0x350c10), and hideInPlayMode (no reader found).
- lbp-runtime: material_tweak.rs, `LevelWorld::tweak_on` and `body_material`. INFERRED:
  the rigid group is the Things with the same root of the parent chain.
- Tests: `group_tweaks_average_and_take_the_largest`,
  `wired_tweak_follows_its_input_and_deactivation`,
  `stickiness_three_is_kept_apart_and_bit_7_is_dropped`.

## Part 36: the emitter (D1134)

PEmitter (part bit 0x14) emits a copy of its plan on a clock or when a switch reaches it.

- Offsets (reader 0xb8a228, visitor 0xb8a3b8): +0x40 clock state, +0x44 angVel, +0x48
  frequency, +0x4c phase, +0x50 parentRelativeOffset, +0x60 lifetime, +0x64 maxEmitted,
  +0x68 currentEmitted, +0x6c maxEmittedAtOnce, +0x70 behavior, +0x74 plan, +0xa1
  recycleEmittedObjects, +0xec objects alive. Clock state: +4 time made, +8 frame the
  scale changed, +0xc scale, +0x14 player.
- Clock (0x9bee4, every frame t): returns early on another world, a paused world
  (0x88fb1c), a deactivated Thing (+0x7f), behavior 1, or behavior 0 or 2 with frequency
  0. The scale is 1.0 unwired; wired with behavior 2 the input's activation (output
  +0x18); wired with behavior 0, 1.0 while the input's ternary (+0x1c) is nonzero, else
  0.0 and no emit. A scale change keeps the time made: time += old*(t-base) - (t-base),
  base = t. Time(t) = (t-base)*scale + base + time made. It emits when
  floor((time(t-1)+phase)/frequency)*frequency + frequency <= time(t) + phase.
- Behavior 1: 0x11ddf0's entry for kind 0x4b (0x11eac8) emits once when the switch
  reaches it (0x9c3b0 -> 0x9baf8).
- Gates (0x9baf8): unless recycling or maxEmittedAtOnce is 0, no emit while that many
  are alive; then 0x93780: (maxEmitted 0 or currentEmitted < maxEmitted) and
  maxEmittedAtOnce nonzero, so maxEmittedAtOnce 0 never emits. An emit (0x9a53c) gives
  the object the lifetime and counts currentEmitted up.
- Owner: the input's player when it has one, else the group owner (0xf8e70).
- Census (census-all-25.json): 3,881 emitters, 3,435 with port 0 wired; behavior 0
  x1,973, 1 x1,902, 2 x6; frequency 0 116; maxEmittedAtOnce 0 7; maxEmitted set 774;
  recycling 2,545; lifetime 0 2,006; currentEmitted nonzero 421; editor emitters 131;
  hidden in play 5; 3,874 pass the gates at load.
- Not modelled: the thermometer budget (world+0x8d0), the flag at +0xf6, the group
  owner, placing the object (0x97dd4). NOT FOUND: where a lifetime runs out and whether
  recycling destroys the oldest.
- lbp-runtime: emitter.rs, `ThingState.emitter`, `LevelWorld::emitter_tick` and
  `emitter_trigger`. INFERRED: the input is port 0.
- Tests: `unwired_emitter_emits_every_frequency_frames_after_the_phase`,
  `at_once_and_total_limits_gate_emits`,
  `wired_input_sets_the_speed_and_behavior_one_waits_for_the_switch`.

## Part 37: the sequencer (D1135)

PSequencer (part bit 0x2000000000, slot +0x94) moves a play head across its microchip's
circuit board and switches the board's contents on in the columns under the head.

- Offsets (reader 0xb57d20, ctor 0xf58c0): +0xc tempo (125), +0x10 swing, +0x14..+0x1c
  echo feedback, time and mix, +0x20 reverb (3), +0x24 loop, +0x28 start point, +0x2c
  channels, +0x30..+0x44 volumes, +0x48 play head, +0x4c playing, +0x4d music pending,
  +0x50 frame count, +0x54 music sequencer (1), +0x64 trigger player, +0x68 behavior (2).
  Kind (0x3ed2c): 4 for a music sequencer, 5 otherwise.
- Board length: the microchip's +0x3c (0xf454c via 0xae380). INFERRED: it and +0x40
  are the two f32 stored after the component list (CircuitBoardSizeX/Y).
- Step (0xf45e4): base = tempo * 7/960 (0xd9593c). Unwired and not on another
  sequencer: base while playing. Unwired on another sequencer: input 1, or 0 when the
  Thing is deactivated. Wired (port 0 activation a, ternary t): behavior 0 and 6
  base*t; 1 base*t, or -base when t is 0; 2 base while playing; 3 -base while playing;
  4 base*a; 5 |a|*length - head. A music sequencer holds still while not pending and
  frames <= 29.
- Move (0xf5008): head + step below 0 stops at 0 (not playing) or loops to the end;
  at or past the end it loops to 0 or stops at the end. Behaviors 2 and 3 and an
  unwired top-level sequencer keep their playing flag (and stop playing at the end);
  the rest play while the head is not 0. The setter (0xf4f20) works out the flag of
  every Thing on the board again when the head changed (0x409dc).
- Wires (0x11f3a0, kinds 4 and 5): a rising edge on port 0 starts behavior 2 from 0 and
  behavior 3 from the end unless playing (0xf675c), and records the player; on port 1
  it stops at 0 (0xf52b8). INFERRED: port 1 is the reset input.
- Gating (0x409dc): a Thing on the board with no wired input is deactivated unless its
  columns (s, e] hold head/52.5 (0xd92e94). Columns (0xae53c): x/52.5 from the
  component list (+0x5c, 36-byte entries, words 3, 6 and 7 read as f32), w = word 6 /
  word 7, at least 1; m = 2 for kinds 4, 0x48 and 0x49, else 1; s = floor(x - m + 0.5),
  e = floor(w*m + x + 0.5); kinds 0x2f, 0x31, 0x32 (switch types 39, 15, 16) use w for
  m in s and 1 in e. Behavior 6 with port 0 wired and off deactivates every Thing on
  the board. INFERRED: words 3, 6, 7 are the stored f32 #0, #3, #4 (the Thing pointer
  takes 3 words); 0x88f6cc is floor.
- Census (census-all-26.json): 1,736 sequencers, port 0 wired 1,541, port 1 wired 207;
  behavior 0 x46, 1 x8, 2 x1,556, 4 x4, 5 x122; loop 493; music 21; playing at load
  198; head nonzero 132, past the end 26; length 0 once. 19,936 Things on their boards,
  14,909 in the component list, 97 wired; 14,768 are outside the head at load.
- Not modelled: the music (0x2cebcc, 0xf6db8), the pending byte +0x4d, 0x900730 on a
  wired input with ternary 0, the other gating path when the board Thing's +0x10 bit 4
  is set (0xae26c: local matrix x instead of the list, so 5,027 unlisted Things are not
  gated), and the kind '%' call 0x368c40. NOT FOUND: the output a sequencer gives.
  INFERRED: the frame count +0x50 counts `sequencer_tick` calls.
- lbp-runtime: sequencer.rs, `ThingState.sequencer`, `LevelWorld::sequencer_tick` and
  `sequencer_input`; lbp-level keeps PMicrochip's component list and board size.
- Tests: `unwired_head_runs_at_tempo_and_stops_or_loops_at_the_end`,
  `port_zero_starts_and_port_one_stops`,
  `board_things_play_while_the_head_is_in_their_columns`.

## Part 38: the controlinator (D1136)

PControlinator (part bit 0x26, slot +0x98, kinds 6 to 8 at 0x3ed2c) is the seat a player
sits in to drive a contraption; the control pads under it read that player's pad.

- Offsets (reader 0xb73008, ctor 0x6a0844): +0xc driver, +0x18 a Thing, +0x28 link,
  +0x34 attached player (each a Thing reference), +0x40 mode, +0x44 input scale, +0x48
  bool, +0x4c state, +0x50..+0x52 bytes, +0x54 the recorded-inputs Thing, +0x49 i8 from
  revision 0x3ed (1 before). The reader drops the stored state; the ctor sets it to 0.
- Pad (0x11d378): the control pad's Thing's parent's parent holds the controlinator.
- Driver (0x6a0134): mode 8 in state 2 with an attached player reads that player;
  otherwise the driver of the controlinator on the link Thing, or none. With no driver
  every output but 0 reads 0.0 (0x6a1004).
- Group (0x69ffec): another controlinator holds the mode when it is not deactivated, in
  the same mode, in state 1, its port 0 off, and has a driver. Attached controlinators
  push their mode to the others in the world list (0x12d2a4). INFERRED: the input read
  is port 0's ternary.
- Census (census-all-27.json): 596 controlinators; mode 0 x530, 1 x17, 2 x17, 3 x15, 8
  x17; link to itself 499, none 97; driver stored 149, attached 0; stored state 0 x499,
  1 x33, 2 x64. All 546 control pads (535 type 38, 11 type 55) have their controlinator
  on the grandparent, none on the parent or the pad's own Thing.
- Getting in and out: Part 40. Not modelled: the
  recorded inputs read first from the +0x54 Thing, the Thing type 8 path of 0x6a1004
  and the mirror sign from the link Thing's group matrix. NOT FOUND: what +0x18, +0x48
  and +0x50..+0x52 mean.
- lbp-runtime: controlinator.rs, `ThingState.controlinator`,
  `LevelWorld::pad_controlinator`, `controlinator_driver` and `controlinator_holds_group`;
  `pad_sensor` reads the driver through it.
- Tests: `pad_reads_the_link_driver_of_its_grandparent`,
  `mode_eight_in_state_two_reads_the_attached_player`,
  `a_driven_controlinator_holds_its_mode`, `pad_reads_the_driver_of_its_controlinator`.

## Part 39: the creature (D1137)

PCreature is a creature's body; PEnemy is a creature piece (a brain or other part) on
it. This part covers how a creature that is not a player dies.

- State (FOUND): creature+0xc14 (0 at load, ctor 0x829b4), set by 0x7b454; +0xc18
  frames in the state; +0xd68 the lethal type last applied. INFERRED: 5 dying, 6 dead.
- Contact (0x81824, called from the update 0x8d2bc): 14 contact slots at
  state(+0xbf0)+0x180+i*12; the strongest type 1 to 14 wins and gas 6 to 11 reads as 6.
  With +0xf4c clear the type is 0xe (INFERRED: unclaimed). A creature with no player
  part calls 0x7d1a4(c,1) and goes to state 5; a player goes to 0x63f28 instead.
- Dying: state 5 ends in state 6 after cfg(+0xbf4)+0x40 frames for fire, +0x48 for
  electricity, +0x50 for type 0xe and +0x4c for the rest. State 6 skips the processor.
- Crush (0x7d6ec): while the penetration passes cfg+0x74 the count +0xd80 grows; past
  cfg+0x78 the creature dies with type 4. 0x952cc also writes type 4 (not examined).
- Early outs of 0x81824: Thing+0x7e not 0x88 or 0x89, Thing+0x78 & 2, state 6,
  0x88f7dc, world+0x804, 0x63b80; a player part with +0x848 set skips the scan.
- PEnemy (stored, `PartData.enemy`): part type, radius, snap vertex, centre offset, anim
  Thing, anim speed, source player, new-enemy flag, smoke colour, smoke brightness (from
  revision 0x39a). HasPEnemy is Thing+0x10 bit 0x1c; IsEnemyAI (0x6d6e54) needs the
  creature bit 0xd. NOT FOUND: the PEnemy reader in the EBOOT, what part types mean, and
  the brain rules (what kills a creature through its brain).
- Census (census-all-28.json): 1504 creatures, 189 with a PEnemy on the creature's own
  Thing, 181 with a parent. 3484 enemies: type 1 x2713, 2 x189, 3 x39, 4 x543; new 0 for
  all; anim Thing set x39; source player 0 for all; smoke colour 0xffffffff x3310. The
  189 of type 2 sit on the creature itself; of the rest, 1 has its creature on the
  parent and 6 on the grandparent.
- Not modelled: the player path, the early outs, the fire stun (state 1, cfg+0x58),
  0x952cc, and the brain. NOT FOUND: the config values, so the host supplies them.
- lbp-runtime: creature.rs, `CreatureState`, `CreatureConfig`, `strongest_lethal`,
  `ThingState.creature` and `ThingState.enemy`.
- Tests: `the_strongest_type_wins_and_gas_reads_as_six`,
  `contact_kills_after_the_types_duration`, `an_unclaimed_creature_takes_type_e`,
  `crushing_counts_deep_frames_then_kills`.

## Part 40: getting in and out of a controlinator (D1138)

The update is 0x6a3264 (0x6a3edc is inside it; caller 0x89014c). All FOUND in 01.33
unless marked.

- Gate (0x6a096c): shut while port 0 (Thing+0x60 entry 0) is on, which also clears +0x28
  and +0x34; also shut on world+0x804, world+0x820, T+0x7f or 0x550e70(T,0). With a
  driver, the gate stays open only while the driver is in creature state 4.
- Reach (0x6a2938): the candidate's Thing type is 0x88 or 0x89; its creature state is
  one of 0, 1, 2, 3, 7, 8, 9, 10, 11, 12, 16; its player part +0x848 is clear; it is not
  +0x18; and it is closer to the seat than +0x44 (a radius in world units, as well as the
  scale of inputs 9 to 19). The update only looks when there is no driver (+0xc).
- Sitting (0x6a2fc4): with +0x51 set the nearest creature in reach sits at once and is
  kept in +0x18; otherwise a player only gets the prompt flag (+0x24) and sits from the
  player's own code (INFERRED), and a creature with no player sits when its brain
  +0x58 is not 4 or on buttons 0x1001 (0x89586c). Sitting writes player+0xecc = T, puts
  the creature in state 4 (0x7b454) and sets the driver (0x6a0b78).
- Leaving (0x89950c): when the gate shuts with a driver, a driver still in state 4 is
  put out of it; once it is not in state 4 the seat clears +0xc (0x6a0b78). No leave
  button is read in the update; INFERRED: it is in the player's state 4 code.
- +0x4c: the reader keeps the ctor's 0 (0xb52e3c reads into a local and puts the old
  value back), and the update does not write it. NOT FOUND: what sets 1 or 2 in play.
- Census (census-all-29.json): radius (+0x44) <=0 x5, <100 x11, <500 x370, >=500 x210;
  +0x51 set x298 (flags [0,1,1] x183, [1,1,1] x115); port 0 wired x199; +0x18 set x49.
- Not modelled: the gate's world and Thing flags, the player prompt and button, the
  brain path, the nearest-candidate choice (the host passes the distance).
- lbp-runtime: controlinator.rs `controlinator_open`, `controlinator_in_reach`,
  `controlinator_auto`, `controlinator_enter`, `controlinator_seat_tick`, `SEATED`.
- Tests: `a_creature_in_reach_sits_and_drives`, `the_seat_lets_go_once_the_driver_stands`.

## Part 41: player death, respawn and checkpoint lives (D1139)

All FOUND in 01.33 unless marked.

- Lethal contact on a player (0x81824) calls 0x63f28, which only notifies: tag 0xc9516493
  to the killer for fire, tags to the players at part+0x65/+0x67 when game+0x180 is 0,
  0x55e49554 for type 0xe, and a script hook on the player's Thing. It never sets state 5;
  who does is NOT FOUND (INFERRED: the script hook or the player manager).
- Dying (state 5) ends in state 6 after cfg+0x40 (fire), +0x48 (electric), +0x50 (type
  0xe) or +0x4c frames. The strings FramesFrom{Fire,Ice,Electric,Crush,Drowned}DeathTill
  Respawn suggest +0x40..+0x50 in that order (INFERRED).
- Respawn 0x7fb00 sets state 0 and calls 0x5a2d8; its only caller is the message handler
  0x7feb8 (sender NOT FOUND). 0x5a2d8 spawns when 0x5a134 allows: spawningDelay
  (+0x34, INFERRED) is 0, and a life is left (+0x10 > 0) unless lives are unlimited or
  the spawn is forced. A plain spawn takes one life when counted; a forced one tops +0x10/+0x14 up
  toward game+0x1e8 + 2 by the multiplier (three times it when +0x10 was negative).
- Lives are unlimited (0x5a0c4) when game+0x180 != 0, in the pod (game+0x3b5), or when
  0x5a038 is true (+0x41, INFERRED instanceInfiniteSpawns, or 0x3ad08, 0x59e94, a global).
- Activation 0x599dc sets +0x40 and, while +0x10 is -1, +0x10 = +0x14 = lifeMultiplier
  (+0x38, INFERRED) x (game+0x1e8 + 2). INFERRED: game+0x1e8 is the player count; the
  stored 3/3 lives on multiplier 1 and 6/6 on multiplier 2 fit one player.
- Scripts: native_IsPlayerDead 0x5fe12c is true in state 5 or 6; the death count
  (game+0x1c4) is bumped by the script (native_IncDeathCount 0x5f4d2c), not the engine.
- Census (census-all-30.json, 1314 checkpoints): multiplier 1 x751, 2 x563; lives -1/0
  x1087, 3/3 x171, 6/6 x35, 4/4 x14, 8/8 x5, 5/5 x1, 12/12 x1; infinite x458; delay x41.
- Not modelled: the spawn slots (+0x28/+0x2c), the type 's' checkpoint and body test in
  0x5a134, creature+0xc48, the lives_zero sound (0x18c948) and the game over.
- lbp-runtime: checkpoint_lives.rs `CheckpointLives` (`activate`, `can_spawn`, `spawn`,
  `respawn`), `LivesRules`, `is_player_dead`; `ThingState.checkpoint`. lbp-level keeps
  spawningDelay and lifeMultiplier.
- Tests: `the_first_activation_gives_lives_once`,
  `respawns_take_lives_until_none_are_left`, `unlimited_lives_and_the_delay`, `a_forced_spawn_tops_up_toward_players_plus_two`.

## Part 42: a seated player whose controlinator is destroyed (D1140)

All FOUND in 01.33 unless marked.

- Of the 50 calls to the state setter 0x7b454, four pass state 5. Three only run with no
  player part (0x81824, 0x7d6ec at 0x7db48, 0x36eb38 at 0x36f2ec). The fourth, in the
  creature update 0x8d2bc at 0x8d8fc, has no player check.
- Sitting (0x6a2fc4 through the thunk 0x89f82c) calls 0x7d5dc: it points the weak
  reference +0xecc at the controlinator's Thing (0x511a8), clears +0xed8, sets +0xf12 to
  0 or 6 by 0x5fab8 and puts the creature in state 4.
- Each update, a creature in state 4 whose +0xecc is null (0x5fa90) calls 0x7d1a4 (it
  lets go of the seat's Thing) and goes to state 5. The lethal type +0xd68 is not set, so
  the dying time is the last type's. The weak reference is nulled when the seat's Thing
  is destroyed (INFERRED: 0x511f8 unlinks it, the Thing's +8 list holds it).
- 0x7b454 has a special case only for state 0xb; per-state tables at 0xd93c08 (old) and
  0xd93c0c (new) are not decoded. The thunks 0x89821c and 0x8a469c and the computed
  state at 0xdacb4 (0xdab6c) were not traced.
- So a player still dies from contact by a path NOT FOUND (Part 41); this is the only
  engine path that puts a player creature in state 5 found so far.
- No census: it applies to every controlinator (Part 38).
- lbp-runtime: `CreatureState.seat`, `CreatureState::seated_tick`,
  `LevelWorld::seat_destroyed`; `controlinator_enter` sets the seat.

## Part 43: the cone and layer tests of the tag and player sensors (D1141)

All FOUND in 01.33 unless marked. Census (378 levels): player sensors 5,627, of which
1,811 have AngleRange < 180 and 212 IncludeTouching 1; tag sensors 16,953, of which
2,975 and 489. IncludeTouching is only 0 or 1 on these two types.

- Order per candidate (0x118d5c, 0x119910): range, then the cone when AngleRange < 180,
  then the layer test when IncludeTouching is not 0.
- Cone: passes when (dir . offset) / distance >= cos(AngleRange), AngleRange the half
  angle in degrees. dir (0x1141f4) is the base of the proxy Thing (0x113fb0): its local
  +Y, turned by the shortest arc that takes its local Z onto world Z, flattened and
  normalised. That base is turned clockwise by PSwitch+0xdc, negated for a mirrored
  proxy. 0x110140 (from 0xaec1c, a load or edit pass, INFERRED) sums +0xdc over the
  sensor and the Things above it on the chip: each adds the inverse of its in-plane
  rotation and a mirrored one negates the sum. So a plain sensor looks along its own
  up, one on a chip along the chip's up turned by its rotation on the board.
- A Thing without a PPos and a sensor under a cursor chip: Part 45. INFERRED: a
  candidate at distance 0 passes.
- Layer, tag sensor (0x1196b0, 0x11984c): the body is the proxy's when the proxy has the
  part at bit 31 of its part mask (INFERRED: the PShape), else its parent's. A sensor
  without a body accepts; a tag with neither part nor parent is rejected, one whose
  body is empty accepted; else the z spans of the two bodies' boxes (body+0x160) must
  overlap, touching faces not counting.
- Layer, player sensor (0x11a13c): a sensor span of 20 or less sees no player. Else
  floor(min / 200 + 0.5) <= floor(player's middle z / 200) <= floor(max / 200 + 0.5)
  (INFERRED: 0x758784 is floor). The checkpoint rule at 0x11a31c is not modelled.
- INFERRED: the body box is the PShape slab, local z -thickness..+thickness, and the
  local x / y box stands in for the outline of a tilted Thing.
- lbp-runtime: `ThingState.shape_box`, `LevelWorld::z_span`,
  `LevelWorld::cone_direction`; the tests in `tag_sensor` and `player_sensor`.
## Part 44: the grab and impact sensors have no cone or layer test (D1142)

All FOUND in 01.33. The "proximity" switch is the player sensor (type 3), Part 43.

- Impact sensor (type 18, body 0x11ae80..0x11b570): it never reads PSwitch+0x7c
  (AngleRange) and never calls the cone direction 0x1141f4, the proxy base 0x113fb0 or
  the rotation sum 0x110140. It copies the body's box (*(body+0x160)) to the stack and
  hands it to the per-contact function 0x900f54 (calls at 0x11b2fc, 0x11b3e4), which
  loads no floats and reads no +0x7c. IncludeTouching (+0x8c) only admits touching
  contacts (0x9011e8); it is not a layer test.
- Grab sensor (type 6, 0x10f000..0x10fd00): no read of PSwitch+0x7c (the two `sth` to
  +0x7c at 0x10f1e4 and 0x10f230 write another struct) and no call to the helpers above.
  Mode 2's extra test 0x1d31c0 follows player+0x144 to the grab record and compares the
  grabbed Thing's tag filter (+0xc9, 0x10f250) with the other Thing's u16 +0x7a: an
  identity check, not a spatial one.
- Callers of 0x1141f4: the tag sensor (0x119718), the player sensor (0x11a230,
  0x11a2b8), the angle sensor (0x11cd48), the linear velocity sensor's resetWhenFull path
  (0x11da60), and 0x114de4, 0x115088 (the editor's cone drawing, Part 45).
- lbp-runtime: no change to the rules; the tests `impact_sensor_has_no_cone_or_layer_test`
  and `grab_switch_has_no_cone`.
## Part 45: the cone's chip table, cursor chips and the editor's cone drawing (D1143)

FOUND in 01.33 unless marked.

- Rotation sum 0x110140, a Thing without a PPos (0x1103f8): when the walk reaches a Thing
  whose grandparent has the MICROCHIP part (part bit 0x21) and the Thing has no PPos
  (+0x2c), it looks the Thing up in that PMicrochip's table (+0x5c: a count, then 9-word
  entries from word 4, word 0 the Thing). Found: the sum adds the float in word 5, then
  negates when the byte at word 8 is set (and XORs PSwitch+0xe2). Not found: nothing.
  INFERRED: the table is the level file's `components` {thing, x, y, angle, scaleX,
  scaleY, flipped} with x at word 3, so word 5 is `angle` and word 8 `flipped`.
- Cursor chip: object kind 0x96 (0x3ed2c, case 0x3f45c, at 0x3f460) is a type-36 switch
  with IncludeTouching (+0x8c) not 0. The cone direction 0x1141f4 first calls 0x3c594
  when Thing+0x78 bit 0x100 is clear: it walks Thing+0x50 up through kinds 3, 5, 0x40
  and 0x50, and on a kind 0x96 Thing (or the Thing itself) returns 0xd1df8: the record
  in world+0xc0 (count +0xc4) whose +0x1b4 is that Thing, plus 0x70. Otherwise it sets
  the bit and returns 0, the normal path 0x1142e0 (Part 43). With a record the
  direction is the sincos (0x1d3594) of record+0x1b0, turned by PSwitch+0xdc (negated
  for a mirrored sensor, 0x1147b8; else 0x114634). Not modelled: the record is runtime
  state of the cursor, not in the level file.
- 0x114930 (holding the calls 0x114de4 and 0x115088) is the editor's drawing of sensor
  cones, render only: 0x567990 calls 0x564d3c twice (flag 0 and 1, after the depth
  functions 0x203 and 0x206 are written), whose thunks 0x898f7c, 0x898f8c (0x12c988)
  and 0x898f9c (0x12d2a4) reach it. It returns unless (parent or self)+0x10 bit 4 is
  set and switches on Thing+0x7e.
- Census (census-all-31.json, 358 of 378 levels read): the cone walk reaches a chip table
  for 3,265 of 5,627 player sensors and 14,515 of 16,953 tag sensors; the entry adds an
  angle or flips for 958 and 2,893 of them. 5 player and 25 tag sensors sit under a cursor
  chip, none of them with a cone (angle range below 180), so leaving the cursor record out
  changes no placed cone.
- lbp-runtime: `ThingState.chip_components` (`ChipAngle { thing, angle, flipped }`),
  read by `LevelWorld::cone_direction`; the test
  `cone_rotation_reads_the_chip_table_without_a_ppos`.

## Part 46: tweaks that aim along velocity, stick or target, and the layer switch (D1144)

What the game does (EBOOT 01.33, packer 0x95fb20; the switch at 0x95fd0c-0x95fd38
builds an aim vector per configuration):
- 3 aims along the body's velocity (state+0x10 x, y, where state = *(body+0x64)+0x110).
- 4 aims along stick slots 1 and 2 of the controller block *(tweak+8)+0x60 (0xcb02c,
  0xcb074); a missing slot reads 0. The flip negates the aim's x (0x960108).
- 5 aims at the nearest target of 0x1c3e04 (min +0xe8, max +0xec, cone +0xf0, key from
  byte +0x14 >> 4 and +0xd8): target minus body (state+0x20), plus the target's
  point's velocity (the lead, Part 50). No target gives speed 0.
- Dead zone (0xc46400, 0x95fd88): an aim with x² + y² under 0.1, or accel × velRange 0,
  sets the target speed to 0 and skips 0x1c21f4 and the clamp; the limits stay, so the
  body brakes.
- Otherwise 0x1c21f4 gets 0.5 × accel × velRange and two wrapped atan2 terms (0x960588
  to 0x960790), the second adding ±tweak+0x220 by the flip; which one is the error is
  INFERRED. Configurations 1, 3 and 5 flip only that sign; 2 and 9 negate the target
  and swap the limits (0x95ff4c-0x95ff90).
- Dispatch (0x1c7ed0, mask 0x23e): only 1-5 and 9 reach this packer; 6-8 go to
  0x957df4, 11 has its own branch. Configuration 12 is not a physics row: 0x1cc660
  (about 0x1cc730) calls 0xce1f8 when the byte at *(tweak+0xd0)+0x24 is set, which moves
  the body between layers through 0xcd770 (tweak+0x68: 0 back, 1 front, 2-4 by depth);
  Part 47 models it.

lbp-sim (`SpinTweak::aim`, `target_along`, `sees`; `drive_spins` in lbp-sim and the
viewer): 3 aims along the body's velocity in game units per tick, 4 along its stick
(Part 50), 5 at the player or, with followKey, the nearest tag of its key not on
its own body, searched as the follower's (Part 12). The error is the wrapped angle from
the body's up axis to the aim (INFERRED), shaped as configuration 1. The stick, flip and
lead are Part 50. followKey as 5's key is INFERRED (the same byte and field the
follower reads).

Census (census-all-32.json, 358 of 378 levels read), PPhysicsTweak by configuration:
0 ×42,984 (anti-gravity), 1 ×2,574, 2 ×124, 3 ×213, 4 ×80, 5 ×577 (491 with followKey),
6 ×105, 8 ×4,310, 9 ×2,177, 11 ×2,294 (2,180 with followKey), 12 ×2,632, 13 ×142,
14 ×268. That 5 is mostly keyed, as the follower is, backs reading followKey as its key.

Test: `d1144_aim_tweaks_turn_up_along_velocity_stick_and_target` (lbp-sim sandbox).

## Part 47: the layer switch (D1145)

What the game does (EBOOT 01.33):
- Each tick 0x1cc660 calls 0xce1f8 for every configuration 12 tweak whose wait byte
  (*(tweak+0xd0)+0x24) is set, unless a frame flag (sp+0x247, meaning unknown) is set.
  0xce1f8 clears the byte, reads the input's count n (*(*(thing+8)+0x60)[4]+0x1c) and
  stops if it is 0. Turning off moves nothing back.
- zBehavior (+0x68): 0 moves -n layers, 1 moves +n, 2 to 4 move by zBehavior - 1 -
  trunc((z + 679) / 200) from the body's depth z (+0x78), which reaches layers 1 to 3
  from the back (INFERRED). A layer is 200 deep (0xd953ac).
- 0xcd770 moves the root Thing's glued group (0xcc854, gathering with 0xcc1f4 under
  maximumMass at +0x60, 0 for none) only if there is room: the new depth stays in -511
  to 110 (0xd95360, 0xd95364), no shape lands on another solid one, and the penetration
  test (0x8d0af0) is at most 1. Then every body's matrix z gains the step at once.
- Blocked, a set-layer move sets the wait again and tries one layer less, down to one.
  A blocked step back or front is dropped.
- NOT FOUND: what first sets the wait. INFERRED: it is waitingToMove beside
  lastKnownActivation, set when the activation changes. canPush (+0x64) is INFERRED.

lbp-sim (`LayerTweak`, `delta`, `tick`; one per configuration 12 tweak on its body):
the tick returns the depth step for a `fits` test the host gives. Not modelled: moving
the colliders (host work), the overlap and penetration tests, and the mass limit.

Census (census-all-32.json): 2,632 configuration 12 tweaks. Configuration 13 replays a
recorded move (Part 48), and 14 is read as attract-o-gel from cwlib only, not modelled.

Test: `d1145_layer_switches_step_a_layer_or_go_to_a_set_one_when_there_is_room`.

## Part 48: the recorded move player (D1146)

What the game does (EBOOT 01.33):
- 0xcb688 runs for every configuration 13 tweak with its activation from 0xcaf9c
  (Part 3). Above 0.001 it calls 0x3ced10; at or below, a digital input (tweak+0x10 0)
  resets the player (0x3ce348) so the next start is fresh, and an analog one pauses.
  It is called from 0xeacd4 (each logic frame) and from 0x1a1c88 at 0x1a3024 (inside
  the physics substeps); whether both move the head is NOT FOUND.
- 0x3ced10: on a fresh start it takes the Thing's matrix as the origin (0x3ce4a0), then
  moves the head (a frame index) by activation × speed each call. dir 0 plays forward,
  1 backward (len - 1 - head), 2 back and forth; kind 2 clamps the head to the last
  frame. Frames (0x6beba4, 0x6bec5c, 0x6bf140) hold a position offset and a rotation
  from the origin, frame 0 is zero, and samples are linear between neighbours.
- The frames live in the recording resource, not the level; the level saves only its
  descriptor and the player (playHead, kind, dir, speed, pathIsAbsolute and the start
  matrix, read in `physics_tweak`). INFERRED: the body is driven towards the sample
  through 0x42e6c (velocity terms in *(body+0x110)), not moved there outright, and past
  the end the head holds the last frame for every kind.

lbp-sim (`PlaybackTweak`, `tick`, `frame`, `sample`; one per configuration 13 tweak on
its body, with `RecordingPlayer` from lbp-level): the head and the sampled offset. Not
modelled: loading the recording resource, driving the body, the rotation, the two
callers.

Census (census-all-33.json): 142 configuration 13 tweaks, every one with a recording
set and pathIsAbsolute false; dir 0 ×140, 2 ×2; kind 0 ×10, 1 ×58, 2 ×74; behavior
0 ×26, 1 ×116; speed 1 ×116, the rest 0.4 to 2. That speed is mostly 1 backs reading
it as the head's step.

Test: `d1146_recorded_moves_play_by_activation_and_restart_or_pause_when_off`.
## Part 49: the attractor tweak (D1147)

What the game does (EBOOT 01.33):
- Configuration 14 runs no per-tick code of its own: 0x1c7ed0 skips configurations
  above 11, and 0xcb688 and 0xcb86c handle only 13 and 0. Setting 14 (0xcb534)
  allocates a 0x20-byte block at tweak+0xf8 (constructor 0x699abc) and takes a shared
  resource (0x699f00); setting anything else frees it (0x699ff8).
- While any configuration 14 tweak exists, 0x69be2c each frame calls 0x69a600, which
  widens the camera view rect (0xd6628) by a margin and calls 0x699b50. That queries the
  grid for things in the rect (0x1bfeb0) and builds a 0x28-byte record per thing for an
  SPU job. The job's force law, falloff and targets are NOT FOUND (SPU code).
- 0x610a8 fills the record: it walks the thing's group tree (+0x54 root, +0x48, +0x4c,
  +0x50) for configuration 14 tweaks whose owner group is that thing or whose block
  byte +0x1f is set, keeps the one with the highest block f32 +0, and adds an active
  flag, activation (0xcaf9c) above the constant at 0xd9333c (INFERRED 0.001, as Part 48).
- The block as saved (v > 0x3e3, serializer 0xb78a34): f32 +0, then two (i32, f32,
  f32) triples, then four u8 (constructor defaults 1, 1, 1, 0).

lbp-level keeps it as `AttractorBlock` (`priority`, `first`, `second`, `flags`,
`beyond_group`); lbp-sim puts an `AttractTweak` on its body, with `pick` and `active`.
Not modelled: the gather, the SPU job and any force.

Census (census-all-34.json): 268 configuration 14 tweaks, all behavior 0. The i32s
read as ARGB colours (INFERRED from the values): first 0xff00bfff ×233, white ×35;
second white ×266, with (0.5, 0.5) in 264. priority is a multiple of 52.5 in nearly
every tweak (315 ×86, 525 ×56, 210 ×52, 840 ×23), so it may be a size rather than a
rank; only its use as the winner is FOUND. flags [1, 1, 1] ×263; shared ×1.

Test: `d1147_attractors_pick_the_highest_priority_own_or_shared_block`.

Parts 50 on are one file each under docs/logic/ (docs/logic/part-50.md).
