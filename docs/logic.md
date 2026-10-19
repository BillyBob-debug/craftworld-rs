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
- The mode is set only by the tweak menu setter 0x1110e0 (pattern 0: choice 0 mode 0,
  1 mode 3, 2 mode 1) and is not saved. How a loaded piston gets it is not found.

In the Rookie Test every piston saves modStartFrame 5,306,444 and modScale 0. Play starts
from the reset clock 0, 0, 1 instead (INFERRED; with the saved values nothing would move).

lbp-sim keeps a `PistonClock` per piston. The viewer gives every wired piston mode 0
(INFERRED: the menu's first choice) through `LevelWorld::tweak_activation(thing, 0)`, so
it runs while its switch is on and holds while off; unwired ones run at 1 as in Part 4.

Test: `d1103_a_switched_piston_clock_freezes_while_off_and_never_jumps` (lbp-sim sandbox).
