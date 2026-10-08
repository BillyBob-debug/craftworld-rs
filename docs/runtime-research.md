# Sackboy, physics and logic in Rust/Bevy: research and plan

2026-10-08 America/Chicago. Cloud research only: no game data was read. Repo state read:
main `c4e4998` (after the level-research merge). Format evidence is Toolkit/cwlib (MIT) at
the project pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`. Labels follow docs/formats.md;
everything is CONFIRMED_FROM_COMMUNITY_CODE unless marked INFERRED / PUBLIC / UNKNOWN.
Sibling plans: `docs/level-research.md` (Thing reader, scene, shapes) and the Pod/menus/Popit
plan (ui-pod-popit, project files, not yet in the repo). Full cited notes, with per-field
orders, revision gates and URLs, are in `docs/runtime-evidence/{sackboy,physics,logic}.md`;
this document is the synthesis.

## 1. Short answer

- **Data is well specified, behaviour is not.** Toolkit gives exact layouts for every
  runtime part (PCreature, PYellowHead, PCostume, PNpc, PControlinator, PBody, PJoint,
  PShape, RMaterial, RJoint, PPhysicsTweak, PSwitch, PSwitchInput, PMicrochip, PScript,
  RScript) and for skeletons, skinning and keyframes. It gives **no** movement speeds,
  jump arcs, creature state enum, animation-per-state table, solver details, tick rate,
  or logic evaluation order. Those must be measured on the PC (RPCS3 frame-stepping,
  video timing, in-game test rigs) and labelled as local observations.
- **Order of work:** animation and costumes first (they fit the current asset-viewer
  scope and need no level), then physics on top of the level Thing reader, then logic,
  then the Sackboy controller once constants are measured. Scripts last.
- **Recommended stack:** our own skinning/animation via Bevy 0.19 `SkinnedMesh` +
  `AnimationClip`; **avian2d 0.7.0** (MIT/Apache, `bevy ^0.19`) with one 2D world and a
  per-body z-slab collision filter; logic as a deterministic, single-threaded
  `FixedUpdate` pass over a wire graph built at load.
- **Hard dependency on the level plan:** parts have no length prefix, so every runtime
  part must be parsed anyway for a level to parse at all (level-research.md §4.1). The runtime
  parts below are therefore the same work as level-research.md L6..Ln; this plan only adds the
  behaviour layer on top.

## 2. Scope decision needed first

AGENTS.md defines the product as an *Asset Explorer*; "the future compatible runtime is
context, not the current implementation scope", and rule 6 says "No GUI, physics,
gameplay ... during scanner work". Everything in §5 phases R2 onward (physics, logic,
controller) is gameplay runtime. Before any of it is queued, the user should decide
whether to amend AGENTS.md (suggested wording: runtime work allowed in separate crates
that the parsing crates never depend on, dev-only behind a viewer mode until stated
otherwise). Phase R1 (skinning, animation playback, costume assembly in the viewer) is
asset viewing and fits the current rules.

## 3. What the data says (summary; details in runtime-evidence/)

### 3.1 Sackboy (runtime-evidence/sackboy.md)
- **Skeleton:** 43 bones, 3ds Max Biped names (Dummy01 root, Bip01..., Bone_jaw, eye,
  tongue and zip-pull bones), 29 morph channels. Decoded from Toolkit's bundled
  `template.anim`. That blob is probably game-derived: **evidence only, never vendor or
  ship it**; confirm against the real Sackboy mesh on PC (check S1).
- **Anim hash** (`RAnimation.calculateAnimationHash`) verified against all 43 names.
  Animations bind to mesh bones by hash, not index.
- **RMesh bones:** name[32], flags, animHash, parent/firstChild/nextSibling, model-space
  `skinPoseMatrix` + `invSkinPoseMatrix` (column-major m44 with optional u16 mask), OBB,
  per-bone collision hull verts, bounds. No local TRS: derive `parent⁻¹ · self` after
  normalising scale. Our `lbp-mesh::geometry::skip_bone` already walks these bytes.
- **Skin streams:** joints = bytes 3/7/11/15 of stream 1; weights `(b2,b1,b0,255-Σ)/255`
  with the `b2==0xFF` special case. Streams 2.. are morph targets (v3 offset + packed
  normal), max 32, names in the header. Tail has `regionIDsToHide`,
  `costumeCategoriesUsed`, `hairMorphs`, mirror tables and softbody data.
- **ANMb keyframes (head ≥ 0x378, all local files):** big-endian blob: bone list, locator
  keys (4 bytes each, structure UNKNOWN), animated-track index lists, then i16x3
  rotations (/0x7FFF, w sign in x's LSB) and f16 position/scale/morph. Frame 0 is the full
  base pose; later frames store only animated tracks. `posOffset/posScale` (>0x38b) and
  `fat` (>0x3b1) are read but unused by Toolkit: meaning UNKNOWN (check S5). Avoid two
  Toolkit bugs: `getMorphFrames` indexes by morph not track; `getBaseTransform` uses y
  scale for z.
- **Costumes:** PCostume {body mesh, material, meshPartsHidden, costumePieces[14 categories]}.
  Assembly as Toolkit's SceneExporter does it: hide body primitives whose region is in
  meshPartsHidden, swap default fabric gmat GUID 9698 for PCostume.material, skin each
  piece to the body joints (bind by animHash; Toolkit's remap is unfinished), skip
  placeholder GUIDs 9876/9877, drive piece morphs through `morphParamRemap[32]`.
- **Instancing:** player = Thing with YELLOWHEAD + CREATURE + COSTUME + RENDER_MESH +
  POS + BODY (INFERRED combination). Sackbot = CREATURE + NPC (behaviour PATROL/FOLLOW/
  FLEE/IDLE/ACT/WAYPOINT, jump solver, recorded input). Possession = PControlinator.
  Spawning = PCheckpoint.spawningList; LBP2 has no "start point" flag.
- **Not in Toolkit:** PCreature.`config` points to a SETTINGS_CHARACTER `.cha` resource
  (type 12) that nothing public parses: the most likely home of movement constants
  (check S8). Creature `state` values, anim-per-state mapping, grab/slap, IK, collision
  capsule are UNKNOWN. Sackboy is also a softbody mesh (springs/clusters in RMesh).

### 3.2 Physics (runtime-evidence/physics.md)
- **PBody** stores velocity (v3), angular velocity (one f32 → 2D), `frozen` i32. No mass:
  INFERRED mass = polygon area × PShape.massDepth × RMaterial.density.
- **PShape:** 2D polygon (v2 or v3 verts + loops → concave and holes), physical material,
  thickness (default 90), massDepth (1.0), lethalType, flags (collidable game/poppet/with
  parent), stickiness/grabbability (≥0x3bd). Several layout changes inside the LBP2 band
  (0x346, 0x38a, COM matrix absent for 0x341..0x38d).
- **Layers:** no layer enum anywhere. A layer = PPos world-z ± PShape.thickness. INFERRED
  collision = 2D contact between shapes whose z-slabs overlap. Thick/thin spacing
  UNKNOWN (check P5).
- **RMaterial:** traction, density (20), restitution (0.3), sliding (0.8) and rolling
  (1.8) friction, gravity and air-resistance multipliers, collideType/collideIgnore
  bitmasks, grabbable, dissolve/explode/break thresholds, maxForce.
- **PJoint** types 0..10: LEGACY, ELASTIC, SPRING, CHAIN, PISTON, STRING, ROD, BOLT,
  SPRING_ANGULAR, MOTOR, QUANTIZED; patterns WAVE/FORWARDS/FLIPPER. CHAIN = winch and
  QUANTIZED = wobble bolt are INFERRED (check P1). RJoint = per-type template
  (elasticity, plasticity, FCurves, breakResistance).
- **PPhysicsTweak** = all movers/anti-grav/rotators/followers in one part, selected by an
  unnamed `configuration` int (names need check P6). Attract-o-gel block (config 0xE) is
  skipped by Toolkit: a format gap.
- **PWorld:** maxVel 100 / maxAVel 1 clamps, frame/simFrame counters, gravity blending,
  water level.
- **PUBLIC behaviour** (GDC 2009 David Smith talk, via a news article, facts only):
  custom rigid-body engine tuned for worst case, no sleeping, off-screen objects not
  simulated, Sackboy fully physical, objects destroyed when the solver hits an
  unresolvable contradiction. Solver type and tick rate not public.

### 3.3 Logic and scripts (runtime-evidence/logic.md)
- Every logic source is a Thing with **PSwitch**; `type` (SwitchType 0..54 for LBP2,
  INFERRED range) picks the behaviour: sensors, gates (AND 14, OR 21, XOR 22, NOT 23),
  timer, counter, toggle, randomiser, selector, battery, microchip, sequencer...
- **Wires live on the sender only:** `outputs[port] = {signal, targets[(Thing, in_port)]}`.
  A runtime must build the reverse index. Signal = {analog f32, ternary, player id};
  analog and digital share it. Gadget state (timers, counters, current outputs) is saved,
  so levels carry live logic state.
- PSwitch layout changes at ten revisions inside the LBP2 band (0x368..0x3ee);
  `logicType` exists only below 0x398.
- Receivers on non-logic Things use **PSwitchInput** (`updateType`: activation, destroy,
  lethal, mover axes, sackbot behaviour/waypoint/jump/grab/layer...).
- **Switch and gate behaviour in LBP2 is native, not scripted** (strong inference from a
  Toolkit comment: LBP2 removed the LBP1 fish scripts on switches, checkpoints,
  emitters, keys and enemies). Scripts still referenced include scoreboard 11599, gun
  66090, magic mouth 18420, music trigger 18256 and the world-thing script 19744; the
  full story-level list is UNKNOWN (check G3).
- **PScript** carries its field layout and values inline, so level parsing never needs
  RScript. **RScript** ("FishFingers") is a register VM: u64 instructions, 203 opcodes,
  string/constant tables, cross-script references by name. No public interpreter;
  zishzingers (no licence, behaviour reference only) shows scripts are classes extending
  Thing that call a large native engine API by name.
- **Public semantics** (fan docs, facts only): analog AND = min, OR = max, every gate has
  invert; NOT digital-on only at max; toggle/randomiser/selector take digital input;
  timer takes signed speed; counter/timer output a ratio; combiner = |a−b|; battery
  −100..+100. **UNKNOWN:** per-gate frame delay, evaluation order, multi-wire fan-in, analog XOR.

### 3.4 Shared unknowns
1. **Simulation tick rate.** All counters are integer frames; 30 Hz is INFERRED, not
   confirmed. Physics, logic timers, emitters, piston periods and animation playback all
   depend on it. One measurement (check X1) unblocks all three areas.
2. Determinism expectations: the game's own replays (Sackbot recordings, motion
   recorder) imply reproducible input playback, not bit-identical physics. Our target is
   deterministic within our own build.

## 4. Rust/Bevy design

**Crates.** Parsing stays in `lbp-resource` (or the `lbp-level` crate the level plan may
create): part structs only, no behaviour. New runtime crates, none of which the parsers
depend on:
- `lbp-anim`: bone/skin/morph/keyframe IR + sampling, CPU only (testable without Bevy).
- `lbp-sim` (Bevy plugins): `PhysicsPlugin` (avian2d glue), `LogicPlugin`,
  `CreaturePlugin`. Viewer gets a dev "play" mode that adds these plugins to the level
  scene from level-research.md L2/Lb.

**Animation.** One joint entity per bone (local bind transform, `Name` = bone name);
`SkinnedMesh` with inverse bind poses; joint index/weight attributes; morph targets via
Bevy morph weights. Clips built from decoded ANMb (per-bone TRS curves at 1/fps, linear),
played through `AnimationGraph`/`AnimationPlayer`; loop-from-`loopStart` needs a split
clip or a small custom driver. Verify exact 0.19 API names when implementing (the
research agent wrote them from 0.15-0.18 knowledge). Costume pieces share the body's
joint entities.

**Physics.** avian2d 0.7.0, `enhanced-determinism`, no `parallel`/`simd` if replays must
be bit-exact; fixed 30 Hz (pending X1), 4-8 substeps. One 2D world; component
`ZSlab { z, half_thickness }`; a `CollisionHooks` pair filter rejects non-overlapping
slabs (thin layers and cross-layer joints then work). collideType/collideIgnore →
`CollisionLayers`. Mass set explicitly from area × massDepth × density. Colliders:
triangulate polygon-with-holes (same triangulator as level-research.md Lx shape extrusion), merge
to convex pieces, cache per shape hash. Joint mapping:

| LBP joint | avian2d |
|---|---|
| BOLT | RevoluteJoint |
| MOTOR | Revolute + velocity motor (strength → max torque) |
| SPRING_ANGULAR, QUANTIZED (wobble) | Revolute + spring motor, target animated by pattern |
| PISTON | PrismaticJoint + position motor between tweakTargetMin/Max |
| CHAIN (winch) | DistanceJoint, animated length |
| ROD | DistanceJoint min = max |
| STRING | DistanceJoint 0..max |
| ELASTIC, SPRING | DistanceJoint with compliance from RJoint |

Our own systems either way: plasticity, FCurve-driven animation, normalizedForces,
joint breaking (read JointForces), destroy-on-contradiction / CRUSH, maxVel clamps,
movers (PPhysicsTweak), effector zones, water. Fallback: bevy_rapier2d 0.36 (Apache-2.0,
native rope/spring joints) if avian's stacking proves unstable. Risks: avian and Bevy
APIs churn (Bevy 0.20 is at rc.2), so pin exact versions; behaviour will differ from
Media Molecule's solver, so plan golden comparison scenes against captures.

**Logic.** `LogicNode` component per PSwitch Thing (kind, params, state, outputs);
`Signal { analog, ternary, player }` mirrors SwitchSignal so saves round-trip. A
`WireGraph` resource built at load (forward + reverse index), rebuilt on spawn/despawn.
Double-buffered signal arrays; topological order over SCCs, ties by Thing UID; a cycle
reads the previous tick. "One tick per gate" vs "combinational" is a config flag until
check G7 settles it. Schedule per fixed tick: Sensors (read last physics step) → Logic
(single-threaded exclusive system) → Actuators (PSwitchInput adapters, emitters, movers,
joints, checkpoints, sackbot inputs) → Physics. Microchips flatten to an enable gate over
their children. Randomisers seeded from level seed + UID.

**Sackboy controller.** Kinematic-feeling but physical body in the same avian world
(the talk says Sackboy is fully physical; air control is not). Collision from the
per-bone hulls in RMesh (INFERRED) or a capsule placeholder. State machine (idle, walk,
jump, fall, grab, layer-shift, dead) is our own design driven by measured constants;
grab = runtime joint to the grabbed body (PCreature.grabJoint). Layer shifting moves the
ZSlab. Sackbots reuse the controller with PNpc input instead of a pad. Controlinator
reroutes a player's pad input to a Thing's logic outputs.

**Scripts: user decision (not decided here).**
- (a) Interpret the user's own RScript bytecode at runtime. Fidelity for unknown
  scripts, but it *executes* original game code from user data (AGENTS.md covers
  shipping code, not this) and still needs the native API rebuilt by name, which is the
  larger job.
- (b) Reimplement each scripted gadget natively and read PScript member values as
  configuration. Clear legal footing; unknown scripts show as unsupported.
- Hybrid: (b) by default, (a) behind an off-by-default feature. Recommendation: (b)
  first; revisit after check G3 shows how many distinct scripts story levels use.

## 5. Phased path

**Phase R0, PC census (no product code).** The checks in §7, folded into the levels L0
oracle run where possible (same Toolkit RLevel pass gives part histograms). Output:
Sackboy mesh/anim inventory, joint/switch/script histograms, material dump, tick rate.

**Phase R1, animated Sackboy in the viewer (asset scope).** Bone parse in `lbp-mesh`,
skin/morph streams, ANMb keyframe decode, CPU sampler, Bevy skinned preview with a clip
picker, then costume assembly from a PCostume (first from a saved profile/plan, later
from level Things). Counting unit: animations that decode with exact byte consumption;
meshes shown skinned. Needs no Thing reader except for the costume step (PCostume
lives in Things).

**Phase R2, static physics sandbox.** Needs levels Lb (level parsed by our reader) and
Lx (polygon triangulation). Load a level, build colliders for every PShape, simulate
PBody things with gravity, PJoint mapped joints, materials; dev-only "play" toggle in
the level scene. Success = story level stands still without exploding; simple
contraptions (bolted wheel, piston) move.

**Phase R3, logic.** Wire graph + core gadgets: battery, AND/OR/XOR/NOT, toggle,
timer, counter, randomiser, selector, combiner/sign split, microchip enable; actuators:
emitter, destroyer, mover/rotator via PSwitchInput, piston/winch/motor speed, checkpoint
activation. Sensors needing players come in R4. Success = known story-level mechanisms
behave as in captures.

**Phase R4, playable Sackboy.** Controller with measured constants, pad input, camera
follow, layer shift, grab, death/respawn at checkpoints, player sensors, collectables
(score bubbles, PGameplayData), then Sackbots and Controlinator. Popit/Pod/menus come
from the sibling plan.

**Phase R5, fidelity.** Softbody wobble on Sackboy, facial expressions/emotes, sticker
and costume decals, scripts (per decision), water/effectors, attract-o-gel, lethal
effects, sequencer audio.

## 6. Candidate backlog items (unnumbered; Integrator assigns D-numbers)

| # | Item | Where | Done when |
|---|---|---|---|
| R0 | Sackboy/anim census: Sackboy mesh GUID, bone set vs the 43-bone list, morph names, ANMb inventory by bone set, fat/posOffset values, `.cha` presence (checks S1-S8) | PC | Report in `local-analysis/`, summary in milestone doc |
| Ra | RMesh bone parse (replace `skip_bone`) + mirror tables + region/costume tail | cloud-ok + PC parity | Synthetic tests; corpus: all meshes still parse, bone trees valid, `skin·invSkin≈I` |
| Rb | Skin and morph stream decode (joint indices, weights, morph offsets/normals/names) | cloud-ok + PC parity | Weights sum to 1; morph counts match header on corpus |
| Rc | ANMb keyframe decode + dense `[frame][bone]` TRS, CPU sampler, oracle compare vs Toolkit frame dump | cloud-ok + PC oracle | All local ANMb consume their blob exactly; frame values match oracle |
| Rd | Bevy skinned preview + clip picker (mesh viewer gets "Animate" for meshes with bones) | cloud-ok (synthetic rig) + PC QA | Sackboy walks in the viewer; counts of animated meshes in panel |
| Re | Costume assembly from PCostume (region hide, 9698 swap, pieces on body joints, morph remap) | needs Thing parts PCostume/PRenderMesh; PC QA | A saved costumed Sackboy renders like the game |
| Rf | Runtime part structs: PBody, PJoint, RJoint, RMaterial, PPhysicsTweak, PEffector, PEmitter (shared with levels L6..Ln) | cloud-ok + PC parity | Parts round-trip vs oracle |
| Rg | Physics sandbox: avian2d dependency (user licence OK), ZSlab filter, colliders from PShape, mass, joints table above, clamps | cloud-ok (synthetic scenes) + PC QA | Story level stable at rest; test rigs move |
| Rh | Movers/tweakers, joint animation patterns, breaking, crush/destroy policy | cloud-ok + PC QA | Captured rigs match within tolerance |
| Ri | Logic part structs: PSwitch (all LBP2 splits), PSwitchInput, PSwitchKey, PMicrochip, PSequencer, PTrigger, PCheckpoint, PGameplayData (shared with levels) | cloud-ok + PC parity | Parts round-trip vs oracle |
| Rj | Logic runtime core: wire graph, buffers, ordering, gates/timers/counters/random/selector/battery, microchip enable, actuators | cloud-ok (synthetic graphs) | Truth-table and timing tests; config flag for gate delay |
| Rk | Sensors needing players/physics: proximity, tag, grab, impact, pressure, score, death | after Rl; PC QA | Story mechanisms trigger as in captures |
| Rl | Sackboy controller v1: input, walk/jump/fall/layer-shift, camera, checkpoint respawn, clips by state | PC QA with measured constants | User plays through the first story level section |
| Rm | Grab, slap, collectables/score, lethal deaths | PC QA | — |
| Rn | Sackbots (PNpc behaviours, recorded input replay) + Controlinator | PC QA | — |
| Ro | PScript instance-data reader (needed by both script options) | cloud-ok + PC parity | All PScript parts in story levels parse |
| Rp | Scripted gadgets, per the user's decision (native reimplementation first) | split later | — |
| Rz | Softbody, expressions/emotes, water, attract-o-gel, sequencer audio | split later | — |

Dependencies: Ra→Rb→Rc→Rd (no level work needed). Re needs the Thing reader (levels L3,
L4 + PCostume/PRenderMesh parts). Rf/Ri/Ro are part of levels L6..Ln and should be
scheduled in the part-histogram order L0 produces, not separately. Rg needs levels Lb and
the Lx triangulator. Rl needs X1 and S8/S9.

## 7. PC checks for the Integrator (need game data)

**Shared**
- X1. Tick rate: frame-step a known timer/piston in RPCS3 (or video-time it) and compare
  with stored frame ints (timerCount, animationTime default 60, emitter frequency).

**Sackboy**
- S1. Sackboy body mesh GUID/path; bone count, names, parents and hash order vs the
  43-bone list; any skinPose with non-unit scale.
- S2. Body morph count and names; mirrorMorphs; is it 29?
- S3. Do GUIDs 9698 (fabric gmat) and 9876/9877 (placeholder pieces) exist on 01.33?
- S4. ANMb files whose bone set equals the Sackboy hashes: count, paths, fps, loopStart,
  morph tracks.
- S5. `fat` and `posOffset/posScale` values across heads; does the blob size match the
  f16/i16 layout exactly?
- S6. Samples with `locatorKeys != 0`: raw records and locator names.
- S7. Costume piece meshes: same bone order as the body or hash-matched only;
  regionIDsToHide / costumeCategoriesUsed / hairMorphs values; body primitive regions.
- S8. Any SETTINGS_CHARACTER (`.cha`, type 12) resource: count, size, head.
- S9. Movement constants by observation: walk/run speed, jump height and air time, fall
  speed, layer-shift time (RPCS3 frame-step + memory watch of PCreature fields, or video).
- S10. Sackbot PNpc.jumpSolver values in a level that has one (parabola fit → implied gravity).

**Physics**
- P1. PJoint.type and animationPattern histogram; confirm CHAIN = winch, QUANTIZED =
  wobble by placing each tool and diffing the saved part.
- P2. Dump every RMaterial with names (density, frictions, restitution, gravity sign of
  floaty, collide masks, grabbable, dissolvable, maxForce).
- P3. Stock RJoint templates per joint type.
- P4. Non-zero PBody velocities and `frozen` values in story levels.
- P5. Histograms of PShape.thickness, massDepth and world-z → layer centres and spacing.
- P6. PPhysicsTweak.configuration values (name by placing each tweaker), zBehavior; any
  WIND_TWEAK in heads > 0x3e2 (expect none).
- P7. ShapeFlags, lethalType, stickiness/grabbability/grabFilter value ranges.
- P8. PWorld maxVel/maxAVel/gravity across levels; frame vs simFrame deltas.
- P9. Polygons with holes (loops > 1) and the largest vertex count per shape.

**Logic and scripts**
- G1. RScript count, revision histogram, className per GUID.
- G2. Opcode histogram; count and names of NATIVE functions (size of the engine API).
- G3. PScript GUID histogram in story levels, with member field names per GUID.
- G4. SwitchType histogram over story levels and creator-mode plans.
- G5. Level revisions vs the PSwitch/FieldLayoutDetails split points (0x368, 0x369, 0x382,
  0x38f, 0x398, 0x3a4, 0x3ad, 0x3d9, 0x3ec, 0x3ee).
- G6. Wire stats: outputs per switch, max port, targets without PSwitch/PSwitchInput,
  feedback cycles, microchip nesting depth.
- G7. Gate delay and ordering: in-game chains of N NOT gates and a feedback loop, timed by
  frame-step.
- G8. SwitchSignal ternary values and player id range seen.
- G9. PSwitchInput.updateType usage.

## 8. User decisions

1. **Scope:** amend AGENTS.md to allow gameplay runtime work (R2 onward). R0, R1 need no change.
2. **Physics dependency:** avian2d 0.7.0 (MIT OR Apache-2.0); rapier fallback is Apache-2.0.
3. **Scripts:** native reimplementation (recommended first) vs interpreting the user's
   own RScript bytecode vs hybrid.
4. **Measurement method for gameplay constants:** RPCS3 frame-stepping and memory watch
   of the user's own copy (allowed as research by AGENTS.md; results recorded as local
   observations, no code copied) vs video timing only.

## 9. Overlaps with the sibling plans

- **level-research.md:** all part structs here are levels L6..Ln work; L0 oracle should also emit
  the R0/P/G histograms; Lx triangulator is reused for colliders; the level scene (L2/Lb)
  is where play mode lives.
- **ui-pod-popit.md:** Popit costume menu, emotes/expressions menu, sticker placement,
  Controlinator prompts, the Pod's Sackboy, profile avatar (RSyncedProfile.sackboyAvatarWorld)
  and the gadget/tool inventory are UI scope there. This plan provides the costumed,
  animated Sackboy and the gameplay systems those menus act on.

## 10. Provenance and licensing

- Toolkit/cwlib (MIT) at the pin: format specification and oracle; independent Rust,
  nothing translated. Its bundled `template.anim` / `characters.boneset` are probably
  game-derived: research evidence only, never vendored.
- avian2d/avian3d MIT OR Apache-2.0; bevy_rapier2d/rapier2d Apache-2.0 (crates.io,
  checked 2026-10-08).
- zishzingers (Beyley, archived, no licence) and LBP Asset Studio (no licence): behaviour
  reference only. Fan wikis (CC BY-SA) and forum posts (no licence): facts only.
- Leaked debug/prototype builds (Hidden Palace listings): not used.
- Decompilation of the user's own copy: allowed as evidence for behaviour constants only,
  output kept in `local-analysis/`, never copied into product code.
