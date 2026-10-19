# Lane state: logic (the game's gadgets and circuits) (2026-10-19)

## Read first
1. docs/logic.md: Part 1 (census of the Rookie Test's logic and the fix order), Part 2
   (anti-gravity tweakers, what the EBOOT does)
2. crates/lbp-sim/src/scene.rs (`AntiGravity`, the tweak pass in `build_scene`) and
   lib.rs `spawn_scene`
3. crates/lbp-runtime/src/switches.rs module doc (what the switch pass does not model)
4. docs/level1-play.md Part 16 (why the route stopped at x -385)

## State
- Owner (2026-10-19): "i think we might need to work on getting the games logic and
  circuits accurate in order to go further". The level1 lane is paused for this one.
- PR for D1100 (branch `logic-census`, worktree `E:\Projects\wt-d324`): PPhysicsTweak
  kept, configuration 0 applied as gravity scale; for the coordinator.
- Decision numbers: D1100-D1199, next free D1101.

## Facts
- Route run: as docs/lanes/level1.md Facts, with `--native-scripts`; captures under
  local-analysis/logic/ (a new name each run). Before D1100 Sackboy stopped at x -385;
  after it he stands at x 22,785 on tick 2012.
- Census: a scratch example (not committed) prints counts; the PPhysicsTweak histogram by
  configuration is in docs/logic.md Part 2.
- EBOOT 01.33: PPhysicsTweak part bit 30; activation getter 0xcaf9c; body update 0xc5f1c
  (gravity scale clamp(1 - Σ gravity × activation, 0, 1) at 0xc6308); SPU packer 0x1c7ed0
  (configurations 1-5 and 9 via 0x95fb20; 6-8 and 11 via 0x957df4).
- Movers, rotators and followers (configurations other than 0) run on the SPU; their math
  needs SPU disassembly or RPCS3 memory reads of a moving body.

## Next
1. Switch signals into physics: tweaker activation (input mode 0 digital, 1 analog),
   pistons, motors and winches from their switch inputs (D1101).
2. Movers and rotators (configurations 1 and 11): read the SPU job or measure a body
   in RPCS3, then apply them.
3. SWITCH_INPUT, MATERIAL_TWEAK (friction, restitution) and EMITTER.
4. The 38 stored-signal switches; the sensors' cone and layer tests.
5. Sequencers, controlinators, creatures.
6. Extend the Rookie Test route past x 22,785 and see what stops it next.
