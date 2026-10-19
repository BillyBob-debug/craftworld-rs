# Lane state: logic (the game's gadgets and circuits) (2026-10-19)

## Read first
1. docs/logic.md: Part 1 (census of the Rookie Test's logic and the fix order), Part 2
   (anti-gravity tweakers, what the EBOOT does)
2. crates/lbp-sim/src/scene.rs (`AntiGravity`, the tweak pass in `build_scene`) and
   lib.rs `spawn_scene`, `follow_tweaks`
3. crates/lbp-runtime/src/switches.rs module doc (what the switch pass does not model)
4. docs/logic.md Parts 4-11 (pistons: length, clock, saved mode, ends, speed mode;
   winches; motors; gyroscopes and rotators)

## State
- Owner (2026-10-19): "i think we might need to work on getting the games logic and
  circuits accurate in order to go further". The level1 lane is paused for this one.
- D1100 merged (PR 241), D1101 (PR 242, e3337c2), D1102 (PR 243, 447b32d), D1103 (PR 244,
  8f435e8), D1104 (PR 245, 0b2d19a: each piston's saved mode and held flag).
- Owner paused (2026-10-19 "pause for now") and resumed ("unpause") the same day.
- D1105 merged (PR 246, e84bd7a), D1106 (PR 247, c632357), D1107 (PR 248, 67fd1db:
  winches), D1108 (PR 249, 6c79cd5: motors). PR for D1109 (branch `logic-mover`,
  worktree `E:\Projects\wt-d324`): gyroscopes and rotators turn their bodies.
- Decision numbers: D1100-D1199, next free D1110.

## Facts
- Route run: as docs/lanes/level1.md Facts, with `--native-scripts`; captures under
  local-analysis/logic/ (a new name each run). Before D1100 Sackboy stopped at x -385;
  after it he stands at x 22,785 on tick 2012 (x 22,770 on tick 2005 after D1102;
  after D1103 x 22,059: he waits about 25 ticks longer near x -3,000, and the inputs end
  at tick 1970; after D1104 x 25,657 on tick 2006; after D1105 x 25,645; after D1108 x 24,635 on
  tick 2008: the wobble bolt on microchip 57 swings at once, Part 10).
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
  Stop-in-time helper 0x1c21f4. Followers (11) and 6-8 via 0x957df4, unread.

## Next
1. Motors (docs/logic.md Part 10): the position term and strength cap of row 0x95f494,
   the turn sign; check thing 58's swing in RPCS3 (Part 10). The type 4 / angle-wrap question at
   0x962590. The switch type bucket in 0x900730.
2. Followers and movers (configurations 11, 8, 6-7) through 0x957df4; then 3-5 and 12;
   the gyroscope's exact 0x1c21f4 target and the flip flag.
3. SWITCH_INPUT, MATERIAL_TWEAK (friction, restitution) and EMITTER.
4. The 38 stored-signal switches; the sensors' cone and layer tests.
5. Sequencers, controlinators, creatures.
6. Extend the Rookie Test route past x 22,785 and see what stops it next.
