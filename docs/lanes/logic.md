# Lane state: logic (the game's gadgets and circuits) (2026-10-19)

## Read first
1. docs/logic.md: Part 1 (census of the Rookie Test's logic and the fix order), Part 2
   (anti-gravity tweakers, what the EBOOT does)
2. crates/lbp-sim/src/scene.rs (`AntiGravity`, the tweak pass in `build_scene`) and
   lib.rs `spawn_scene`, `follow_tweaks`
3. crates/lbp-runtime/src/switches.rs module doc (what the switch pass does not model)
4. docs/logic.md Parts 4-18 (pistons; winches; motors; gyroscopes and rotators;
   movers and followers; tags; every switch type; the random, impact, camera finished,
   magic mouth, projectile and paint switches)

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
- D1113 (branch `logic-impact`, PR 256): the impact switch (docs/logic.md Part 15).
- D1114 (branch `logic-camera`, worktree `E:\Projects\wt-d324`, stacked on PR 256):
  the camera finished switch and the cutscene timer (docs/logic.md Part 16).
- D1115 (branch `logic-mouth`, stacked on D1114): the magic mouth switch
  (docs/logic.md Part 17).
- D1116 (branch `logic-projectile`, stacked on D1115): the projectile and paint
  switches (docs/logic.md Part 18).
- Decision numbers: D1100-D1199, next free D1117.

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
  after D1116 x 24,464 on tick 2004, unchanged).
- Switch census (Part 14): `level_scripts` example with `--all` over the 01.33 catalog
  into local-analysis/logic/census-all-1.json (378 levels, 358 read, 141,572 switches).
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
1. The switches kept as stored (Part 14), by count: velocity 42 / 43, grab 6, control
   pad 38, key remote 54, button 0, then the rest. The host must fill
   `LevelWorld.contacts` from avian2d (Things by `lbp_sim::ThingIndex`) for the impact
   switch (Part 15) to see anything, and call `projectile_hit` for projectile and paint
   sensors (Part 18).
2. Sensors: the cone and layer tests (player, tag, grab, impact, proximity).
3. Outputs not yet modelled: configurations 3-5 and 12, the exact 0x1c21f4 speed, the
   mirror flag and the follower's +0x220 angle; SWITCH_INPUT, MATERIAL_TWEAK, EMITTER.
4. Motors' leftovers: the position term and strength cap of row 0x95f494, the turn sign,
   the type 4 / angle-wrap question at 0x962590, the switch type bucket in 0x900730.
5. Sequencers, controlinators, creatures.
