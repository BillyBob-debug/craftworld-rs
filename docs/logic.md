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
- Configurations 6-8 and 11 (the follower, 35 in the level with followKey set) go
  through 0x957df4 and are next.

lbp-sim (`SpinTweak`, `drive_spins`): per tick, in order, each gyroscope (1) and rotator
(9) moves its body's turn speed towards its target by at most the limits × activation.
The gyroscope's target is sign(e) × sqrt(2 × accel × velRange × |e|) clamped to
±velRange (INFERRED shape). The 1/mass factor is left out (INFERRED: the solver turns
the limit back into a speed change). The sign against avian's counter-clockwise angle is
UNVERIFIED for rotators; the gyroscope is symmetric.

Test: `d1109_gyroscopes_right_their_body_and_rotators_turn_at_their_speed` (lbp-sim
sandbox).
