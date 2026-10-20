# Lane state: logic (the game's gadgets and circuits) (2026-10-20)

## Read first
1. docs/logic.md: Part 1 (census of the Rookie Test's logic and the fix order), Part 2
   (anti-gravity tweakers, what the EBOOT does)
2. crates/lbp-sim/src/scene.rs (`AntiGravity`, the tweak pass in `build_scene`) and
   lib.rs `spawn_scene`, `follow_tweaks`
3. crates/lbp-runtime/src/switches.rs module doc (what the switch pass does not model)
4. docs/logic.md Parts 4-44 (pistons; winches; motors; gyroscopes and rotators;
   movers and followers; tags; every switch type; the random, impact, camera finished,
   magic mouth, projectile, paint and velocity switches)

## State
- Owner (2026-10-19): "i think we might need to work on getting the games logic and
  circuits accurate in order to go further". The level1 lane is paused for this one.
- Owner (2026-10-19, later): "would it not be better to just work on logic on its own as
  a whole and then come back to trying to play levels". So the lane works on logic as a
  whole: every switch, gate and output kind, checked against the EBOOT's rules and a
  census across all levels, each with its own unit tests. The Rookie Test route is only a
  check, rerun and recorded at the end of each PR, never a goal; no level-specific gaps
  are chased. Playing levels comes back once the logic set is complete.
- Merged: D1100 (PR 241) to D1109 (PR 250, b351ff1); D1110 (PR 251, da39db8: movers and
  followers); PR 252 (849bf6a: follower rows in game units per tick).
- Merged: D1111 (PR 253, e164663): keyed followers follow tags.
- Merged: D1112 (PR 254, 5c1ef1c): the switch census (Part 14) and the random switch.
- Merged: D1113 (PR 256, 1acd570) impact, D1114 (PR 257, e44851a) camera finished,
  D1115 (PR 258, 9c12aea) magic mouth, D1116 (PR 259, a2ad940) projectile and paint.
- Merged: D1117 (PR 260, d94b874) the velocity switches.
- Merged: D1118 (PR 261, 4c34a07) the grab switch.
- Merged: D1119 (PR 262, 1aa3098) the control pad switch.
- Merged: D1120 (PR 263, 4242f1f) the key remote switch.
- Merged: D1121 (PR 264, 5f2ac7c) the button switch.
- Merged: D1122 (PR 265, e504be1) the motion recorder switch.
- Merged: D1123 (PR 266, 8a1e600) the angle sensor switch.
- Merged: D1124 (PR 267, bdd1329) the wormhole switch.
- Merged: D1125 (PR 268, 0f11073) the pocket item pedestal switch.
- Merged: D1126 (PR 269, 1f4c887) the lever switch.
- Merged: D1127 (PR 270, 5a9b6d8) the three-way switch.
- Merged: D1128 (PR 271, 3fdf7e2) the sticker switch.
- Merged: D1129 (PR 272, a2b3b59) the score switch.
- Merged: D1130 (PR 273, e480618) the movinator pad.
- Merged: D1131 (PR 274, 3fbcd7f) switch type 55.
- Merged: D1132 (PR 275, ae24c2d) switch inputs.
- Merged: D1133 (PR 276, 91e4066) the material tweaker.
- Merged: D1134 (PR 277, bd6deb2) the emitter.
- Merged: D1135 (PR 278, 0d8f6d4) the sequencer.
- Merged: D1136 (PR 279, af19bb7) the controlinator.
- Merged: D1137 (PR 280, 922909f) the creature.
- Merged: D1138 (PR 281, 9fb266e) getting in and out of a controlinator.
- Merged: D1139 (PR 282, 5593712) player death and checkpoint lives.
- Merged: D1140 (PR 283, 4ee668f) a destroyed controlinator kills its driver.
- Merged: D1141 (PR 284, 4976a39) the tag and player sensors' cone and layer tests.
- D1142 (branch `logic-d1142`, worktree `E:\Projects\wt-d324`): the grab and impact
  sensors have no cone or layer test (Part 44).
- Decision numbers: D1100-D1199, next free D1143.

## Facts
- Route run: as docs/lanes/level1.md Facts, with `--native-scripts`; captures under
  local-analysis/logic/ (a new name each run). Before D1100 Sackboy stopped at x -385;
  after it he stands at x 22,785 on tick 2012 (x 22,770 on tick 2005 after D1102;
  after D1103 x 22,059: he waits about 25 ticks longer near x -3,000, and the inputs end
  at tick 1970; after D1104 x 25,657 on tick 2006; after D1105 x 25,645;
  after D1108 x 24,635 on
  tick 2008: the wobble bolt on microchip 57 swings at once, Part 10; after PR 252
  x 25,657 on tick 2003; after D1111 x 24,464 on tick 2005; after D1112
  x 24,464 on tick 2005, unchanged; after D1113 x 24,464 on tick 2005,
  unchanged: nothing fills the contacts yet; after D1114 x 24,464 on tick 2005,
  unchanged: no cutscene camera on the route;
  after D1115 x 24,464 on tick 2004, unchanged;
  after D1116 x 24,464 on tick 2004, unchanged;
  after D1117 x 24,464 on tick 2004, unchanged: nothing fills the bodies yet;
  after D1118 x 24,464 on tick 2005, unchanged: nothing fills the grabs yet;
  after D1119 x 24,464 on tick 2005, unchanged: nothing fills the pads yet;
  after D1120 x 24,464 on tick 2004, unchanged;
  after D1121 x 24,464 on tick 2004, unchanged: nothing fills impulses yet;
  after D1122 x 24,464 on tick 2004, unchanged: nothing writes the word yet;
  after D1123 x 24,464 on tick 2004, unchanged;
  after D1124 x 24,464 on tick 2004, unchanged;
  after D1125 x 24,464 on tick 2005, unchanged; three earlier runs of the
  same build lost the GPU device near tick 675, cause not found, and no
  type 65 switch is reached on the route;
  after D1126 x 24,464 on tick 2004, unchanged;
  after D1127 x 24,464 on tick 2005, unchanged;
  after D1128 x 24,464 on tick 2005, unchanged;
  after D1129 x 24,464 on tick 2004, unchanged;
  after D1130 x 24,464 on tick 2004, unchanged;
  after D1131 x 24,464 on tick 2005, unchanged;
  after D1132 x 24,464 on tick 2005, unchanged;
  after D1133 x 24,464 on tick 2004, unchanged; a GPU device-lost crash near
  tick 680 hit 5 of 7 runs, the D1132 build too;
  after D1134 x 24,464 on tick 2004, unchanged, no crash;
  after D1135 a GPU device-lost crash near tick 685 in 4 of 4 runs, main
  without D1135 too; after D1136 the same crash near tick 682 in 3 of 3 runs,
  main without D1136 too).
- Switch census (Part 14): `level_scripts` example with `--all` over the 01.33 catalog
  into local-analysis/logic/census-all-1.json (378 levels, 358 read, 141,572 switches);
  census-all-2.json adds the grab counts (Part 20), census-all-4.json the pads
  (Part 21), census-all-5.json the key remotes (Part 22),
  census-all-6.json the buttons (Part 23),
  census-all-7.json the motion recorders (Part 24),
  census-all-8.json the angle sensors (Part 25),
  census-all-9.json the wormhole switches (Part 26),
  census-all-10.json the pocket item pedestals (Part 27),
  census-all-16.json the levers and three-way switches (Part 28),
  census-all-17.json the three-way switches (Part 29),
  census-all-18.json the sticker switches (Part 30),
  census-all-19.json the score switches (Part 31),
  census-all-20.json the movinator pads (Part 32),
  census-all-21.json switch type 55 (Part 33),
  census-all-22.json the switch inputs (Part 34),
  census-all-24.json the material tweaks (Part 35),
  census-all-25.json the emitters (Part 36),
  census-all-26.json the sequencers (Part 37),
  census-all-27.json the controlinators (Part 38),
  census-all-28.json the creatures and enemies (Part 39),
  census-all-29.json the controlinator seats (Part 40),
  census-all-30.json the checkpoint lives (Part 41).
- Census: a scratch example (not committed) prints counts (`WIRED=1` lists switch
  targets by gadget); the PPhysicsTweak histogram by configuration is in docs/logic.md
  Part 2, the wiring in Part 3.
- Piston length 0xc95ec (types 3, 4, 9 at 0xc9790), wave 0xc79c8; PJoint motion at
  +0xc8..+0xd8 (Part 4). A piston scratch dump prints each piston's motion and input.
- Tweak activation getter 0xcaf9c (Part 3): unwired 1, digital = ternary != 0, analog =
  |activation|.
- EBOOT 01.33: PPhysicsTweak part bit 30; activation getter 0xcaf9c; body update 0xc5f1c
  (gravity scale clamp(1 - Σ gravity × activation, 0, 1) at 0xc6308); SPU packer 0x1c7ed0
  (configurations 1-5 and 9 via 0x95fb20; 6-8 and 11 via 0x957df4).
- Row packer 0x95fb20 (Part 11): configuration 1 is the gyroscope (aims at world up),
  2 and 9 rotators, 3-5 aim along velocity / stick / target. Row: target turn speed
  +0x20, limits +0x24/+0x28 = ±accel × velRange, × activation × 1/mass (0x95fdc8).
  Stop-in-time helper 0x1c21f4. Followers (11) and 6-8 via 0x957df4 (Part 12); tag
  search 0x1c3e04 and map 0xceb8c (Part 13).

## Next
Logic as a whole, one component kind per PR, each with a census across all levels, the
EBOOT rule and unit tests:
1. Every placed switch type has a rule since D1131 (Part 14), and switch
   inputs since D1132 (Part 34), material tweaks since D1133 (Part 35),
   emitters since D1134 (Part 36), sequencers since D1135 (Part 37), controlinators
   since D1136 (Part 38), creature deaths since D1137 (Part 39). The host
   must call `CreatureState::touch`, `crush_frame` and `tick` with config values (NOT
   FOUND), and seat players with `controlinator_in_reach`, `controlinator_enter` and
   `controlinator_seat_tick` (Part 40), and respawn dead players with
   `CheckpointLives::respawn` after `activate` (Part 41), call `seat_destroyed` when
   it destroys a Thing and `seated_tick` each frame (Part 42). The host
   must carry out `LevelWorld.destroy_requests`, apply `lethal_type` and
   `body_material`, call `emitter_tick` each frame and spawn the plan when it
   says so, and call `sequencer_tick` each frame and `sequencer_input` when a
   switch reaches a sequencer. The host must
   fill `PlayerState.grabbing` for the grab switch
   (Part 20), `PlayerState.piloting` (the controlinator he sits in) and `pad` for the
   control pad (Parts 21 and 38) and
   `ThingState.body` from avian2d for the velocity switches (Part 19). The host must fill
   `LevelWorld.contacts` from avian2d (Things by `lbp_sim::ThingIndex`) for the impact
   switch (Part 15) to see anything, with `impulse` for the button (Part 23), and call
   `projectile_hit` for projectile and paint sensors (Part 18).
2. Sensors: the cone of a sensor on a cursor (0x1d3594) and the chip-table angle of a
   Thing without a PPos (0x1104ec); the cone direction's callers 0x114de4 and 0x115088.
   Grab and impact have no cone or layer test (Part 44), tag and player do (Part 43).
3. Outputs not yet modelled: configurations 3-5 and 12, the exact 0x1c21f4 speed, the
   mirror flag and the follower's +0x220 angle; the sackbot input, the
   stickiness-3 tweak, the emitter's lifetime and recycling, the
   sequencer's output, music and bit-4 boards (Part 37).
4. Motors' leftovers: the position term and strength cap of row 0x95f494, the turn sign,
   the type 4 / angle-wrap question at 0x962590, the switch type bucket in 0x900730.
5. The creature brain (PEnemy reader NOT FOUND; leads: the TOC entry for string
   "PEnemy" 0xc79058, bit 0x1c tests); who puts a player in state 5 on contact (Part
   42 has the seat path) and sends the respawn (0x7feb8, Part 41); the controlinator's
   recorded inputs (Part 38) and what sets its +0x4c (Part 40).
