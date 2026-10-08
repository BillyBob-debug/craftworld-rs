# PHYSICS: LBP2 data model, known behaviour, Rust/Bevy 0.19.1 plan
Scope: LBP2 band only, head 0x341..0x3f8, branch 0, subVersion 0. Researched 2026-10-08.
Toolkit pin: ennuo/toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9 (MIT, (c) 2024 ennuo).
`B` = https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/
Labels: CONFIRMED_FROM_COMMUNITY_CODE = read in cwlib source (field names, order, gates); the meaning is still the community's
interpretation. INFERRED = my reasoning. PUBLIC = a public article/wiki. UNVERIFIED = needs the game data (see the PC checks).

## A. Data (CONFIRMED_FROM_COMMUNITY_CODE unless marked)

### A1. Part index / gating (B enums/Part.java, enums/PartHistory.java, structs/things/Thing.java#serialize)
Physics parts: BODY idx 0x0, JOINT 0x1, POS 0x4, GENERATED_MESH 0x9, SHAPE 0x12, EMITTER 0x14, PHYSICS_TWEAK 0x1e,
MATERIAL_TWEAK 0x22, MATERIAL_OVERRIDE 0x23, WIND_TWEAK 0x2c. Thing.serialize caps the parts revision:
head<=0x3e2 -> max PartHistory.CONTROLINATOR (0x2f), so WIND_TWEAK (0x36) cannot exist. Head 0x3e3..0x3f8 -> max STREAMING_HINT,
so PWindTweak is *serializable* in late-LBP2 files, but it is an LBP3 feature (UNVERIFIED that LBP2 content ever uses it).
Body/shape hierarchy: Thing.parent, Thing.groupHead (B structs/things/Thing.java), PGroup (plan, emitter, lifetime, flags).
Every Thing that carries PShape and moves is a rigid piece. INFERRED: things glued together (same groupHead/parent tree
without joints) form one rigid body.

### A2. PBody (B structs/things/parts/PBody.java, PBody#serialize)
LBP2 layout: `posVel: v3` (linear velocity, xyz), `angVel: f32` (2D: only z rotation), `frozen: i32` (>=0x147),
`editingPlayer: Thing ref` (read when head>=0x22c && subVersion<0x84, so it is present in LBP2).
-> Saved levels/plans DO carry velocities (posVel/angVel). There is no mass or inertia field: INFERRED that the game derives
mass from shape area * massDepth * RMaterial.density. The meaning of `frozen` (an int, not a bool) is UNVERIFIED; it is
probably a freeze state/count.
PPos (B structs/things/parts/PPos.java): `worldPosition` m44 (localPosition dropped from 0x341, recomputed by the toolkit in PPos#fixup),
`thingOfWhichIAmABone`, `animHash`. Z is taken from the worldPosition translation; there is no layer index anywhere.

### A3. PShape (B structs/things/parts/PShape.java, PShape#serialize). Field order for LBP2:
1 `polygon: Polygon` (B structs/things/components/shapes/Polygon.java): from 0x341: `i32 count`, `bool requiresZ`, verts as v3
  if requiresZ else v2 (z=0), then `loops: intvector` (vertex counts per loop. Multiple loops = holes/multi-part; concave allowed).
2 `material: RMaterial ref` (default GUID 10724), 3 `oldMaterial` ref (>=0x15c), 4 `thickness f32` (default 90),
5 `massDepth f32` (>=0x227, default 1.0), 6 color: v4 RGBA float if head<=0x389, else i32 ARGB, 7 `brightness f32` (>=0x301),
8 `bevelSize f32` (default 10), 9 `COM m44` ONLY if head<=0x340 or >=0x38e (absent for 0x341..0x38d; recompute it),
10 `behavior i32` (>=0x303), 11 colorOff (v4 if <0x38a else i32) + brightnessOff f32,
12 `lethalType`: enum32 if head<=0x345, else u16, 13 `soundEnumOverride i32`, 14 `playerNumberColor` (i32 if <=0x367 else i8),
15 `flags`: i8 if <=0x345, else i16 (ShapeFlags), 16 `contactCache` (>=0x307: Contact[] {PShape ref, u8 flags} + bool sorted
  + bool cacheDirtyButRecomputed because subVersion<0x46), 17 `stickiness, grabbability, grabFilter: i8` (>=0x3bd),
18 `colorOpacity, colorOffOpacity i8` (>=0x3c1), 19 `canCollect bool` (>=0x3e2). The restitution float (0x29d..0x30b) does NOT appear in LBP2.
ShapeFlags (B enums/ShapeFlags.java): COLLIDABLE_GAME 0x1, COLLIDABLE_POPPET 0x2, COLLIDABLE_WITH_PARENT 0x4, DEFAULT 0x7.
LethalType (B enums/LethalType.java): NOT 0, FIRE 1, ELECTRIC 2, ICE 3 (deprecated), CRUSH 4, SPIKE 5, GAS 6..GAS6 11,
NO_STAND 12, BULLET 13, DROWNED 14.
Thickness/massDepth: the toolkit's ShapeAssembler derives them from a mesh bone's z extent:
`thickness = round((|maxZ|+|minZ|)/2)`, `massDepth = round1((|maxZ|+|minZ|)/100)`
(https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/toolkit/src/main/java/executables/ShapeAssembler.java, skeletate). INFERRED: thickness = half-depth in world
units around the shape's z; massDepth = depth in "100-unit" slabs, used as the mass multiplier. Default 90/1.0 suggests a thick
layer is ~200 world units deep with ~10-unit gaps (UNVERIFIED).

### A4. Layer/depth model
cwlib has NO layer enum: a layer is just the world-z of PPos plus PShape.thickness. INFERRED/PUBLIC common knowledge: LBP1/2 have
3 thick play layers. LBP2 also allows thin geometry (smaller thickness, offset z), and LBP3 adds more layers
(the lbp3-only fields isLBP2Shape, zBias, PEmitter.emitToNearestRearLayer, PPhysicsTweak.zPhase). Related fields:
PPhysicsTweak.zBehavior/allowInOut (movers that change layer), PEmitter.worldZ/zOffset/emitFrontZ/emitBackZ/theFuckingZOffset
(>=0x3ae), PSwitch/PControlinator.layerRange, PMaterialTweak.zSlice. Collision is INFERRED to happen between shapes whose
[z-thickness, z+thickness] ranges overlap: 2D polygon contact, gated by z-overlap.

### A5. RMaterial (physics material, B resources/RMaterial.java). For head>=0x27b all of these are present, in this order:
traction f32 (1.0), density f32 (20.0), restitution f32 (0.3), slidingFriction f32 (0.8), rollingFriction f32 (1.8),
soundEnum i32, grabbable bool, gravityMultiplier f32 (1.0), airResistanceMultiplier f32 (1.0), limb bool, shiftGrip bool,
collideType i32 (CollideType bits), collideIgnore i32, dissolvable, explodable, cuttableByExplosion bool, breakable bool,
breakMinVel, breakMinForce, explosionMinRadius, explosionMaxRadius, explosionMaxVel, explosionMaxAngVel, explosionMaxForce f32,
maxForce f32 (default NaN), bullet bool, circuitBoard bool, disableCSG bool. (`explosionType` is an Alear/custom extension; ignore it.)
CollideType (B enums/CollideType.java) is a bitmask: NONE 0, NORMAL 1, COLLECTABLE 2, PLAYER 4, MAGIC_WALL 8, PROJECTILE 0x10,
TEMPORARY 0x20. Use it to build the collision-group/filter masks (collideType = memberships, collideIgnore = filters out).
RMaterial has no "sticky" or "lethal" field. In LBP2 those live on PShape (lethalType, stickiness, grabbability) and
PMaterialTweak (B structs/things/parts/PMaterialTweak.java: restitution >0x2b6, frictionScale >0x30c, grabbability/stickiness
>0x342, grabFilter >0x3bc, noAutoDestruct, isProjectile, disablePhysicsAudio >0x356 ...). "Dissolve" = RMaterial.dissolvable
plus PWorld.dissolvingThings.

### A6. PJoint (B structs/things/parts/PJoint.java). LBP2 order:
a, b (Thing refs), aContact, bContact (v3 local anchors), length, angle, offsetTime f32, invertAngle bool,
settings (RJoint ref), boneIdx int[], boneLengths v4, `type i32`, strength f32 (0.027), stiff bool, slideDir v3 (>=0x18d),
animationPattern i32, animationRange, animationTime (60.0), animationPhase, animationSpeed, animationPause f32,
aAngleOffset, bAngleOffset, modStartFrames, modDeltaFrames, modScale f32, renderScale f32, jointSoundEnum i32,
tweakTargetMaxLength/MinLength f32 (>0x280), currentlyEditing bool, hideInPlayMode bool, behaviour i32 (>0x2c3).
LBP2 has no rail fields (those are lbp3 subVersion).
EJointType (comment in PJoint.java L86-98): LEGACY 0, ELASTIC 1, SPRING 2, CHAIN 3, PISTON 4, STRING 5, ROD 6;
the angular types start here: BOLT 7, SPRING_ANGULAR 8, MOTOR 9, QUANTIZED 10 (the class default `type = 10`).
EJointPattern (L81-84): WAVE 0, FORWARDS 1, FLIPPER 2. These are the piston/winch/motor animation modes. INFERRED UI mapping:
CHAIN = winch, PISTON = piston, STRING = string, ROD = rod, ELASTIC = elastic, SPRING = spring, BOLT = bolt,
SPRING_ANGULAR = sprung bolt, MOTOR = motor bolt, QUANTIZED = wobble bolt (wobble swings between angles; UNVERIFIED).
INFERRED: animationTime is in frames (60 = 2 s at 30 Hz), with length/tweakTargetMin/Max giving the piston/winch extents.
RJoint (B resources/RJoint.java, no version gates): allowExpand, allowContract, contractFreely bool; lengthElasticity,
lengthPlasticity f32; lengthFunc FCurve; angleDeviation, angleElasticity, anglePlasticity, angleVelocity f32; angleFunc FCurve;
normalizedForces, dontRotateA, dontRotateB bool; breakResistance f32; gfxMaterial, gfxWidth, eventNameAngle/Length, mesh.
-> RJoint is the per-type *template* (elasticity/plasticity = spring stiffness/permanent deformation, FCurves = animation
profile, breakResistance = breakable joints). Each joint type likely references a stock RJoint GUID (PC check).

### A7. PPhysicsTweak (B structs/things/parts/PPhysicsTweak.java). One part covers movers, anti-gravity, rotators,
followers, advanced mover, gyroscope etc., selected by `configuration`. LBP2 order:
tweakGravity f32, tweakBuoyancy f32 (>0x2ff), tweakDampening v3, input v3, middleVel v3, velRange f32, accelStrength f32,
decelStrength f32, localSpace bool, configuration i32, hideInPlayMode, colorIndex i32, name wstr, teamFilter u8,
behavior i32, allowInOut, allowUpDown bool, minRange, maxRange f32, followKey bool, angleRange f32 (>0x381), flee bool,
maximumMass f32, canPush bool, zBehavior s32, lastKnownActivation f32, waitingToMove bool (>0x36a);
recordingPlayer struct if >0x3b8 && configuration==0xD (move recording); a 11-field attractor block if configuration==0xE &&
>0x3e3 (attract-o-gel: the toolkit skips it with placeholder reads, so it is a GAP).
Quirk: on read with head<0x38a, the toolkit divides accel/decel by velRange (a normalisation change at 0x38a).
Values 0..0xC of the `configuration` enum are not named in cwlib -> PC check.
PWindTweak (B .../PWindTweak.java): windStrength 50, direction v3, angle, decay, currentInput, min/maxRadius, angleRange
(at subVersion 0 there are extra legacy u8/i32/i32/wstr reads). Probably absent from LBP2 levels, see A1.

### A8. PEmitter (B structs/things/parts/PEmitter.java): angVel; frequency, phase, lifetime as i32 (INFERRED: frames);
recycleEmittedObjects (>=0x2fe); plan (RPlan ref); maxEmitted, maxEmittedAtOnce (1000); speedScale (SwitchSignal);
worldRotationForEditorEmitters (>=0x38e); emitScale, linearVel; currentEmitted; emitFlip; parentRelativeOffset/Rotation;
worldZ, zOffset, emitFrontZ, emitBackZ; hideInPlayMode; behavior; effectCreate/Destroy (>=0x308);
ignoreParentsVelocity (>=0x32e); emittedObjectSource (>=0x340); editorEmitter (>=0x361); isLimboFlippedForGunEmitter (>=0x38d);
theFuckingZOffset (>=0x3ae). posVel was dropped at 0x368. PWorld#fixup sets emitter.lastUpdateFrame=frame when it is 0.

### A9. PWorld physics fields (B structs/things/parts/PWorld.java, serialize ~L350/L549)
things ArrayList<Thing>; maxVel f32 (100.0) and maxAVel f32 (1.0) = global velocity clamps; frame s32; simFrame s32 (>=0x2e2);
frameLevelStarted (>=0x377); thingUIDCounter; isPaused; gravity/fromGravity/targetGravity f32 (>=0x26f; a global multiplier
blended by global-settings things: currGlobalSettingsBlendFactors[12]); waterLevel/targetWaterLevel and wave magnitude
(buoyancy, LBP2 water); dissolvingThings[]; fixInvalidInOutMoverContacts (>=0x3dd). The `frame` and `simFrame` counters split
render frame from simulation frame (INFERRED: the sim can pause or differ from the presentation).

## B. Behaviour (public knowledge)
- PUBLIC: GDC 2009 "How Sackboy Learned to Love Physics" (David Smith, MM technical director), article
  https://www.gamedeveloper.com/game-platforms/gdc-how-sackboy-learned-to-love-physics (news article, copyrighted; facts only):
  custom in-house rigid-body physics, built and optimised for the worst case. There is NO sleeping and little frame-to-frame
  coherence, so performance is predictable for user-built contraptions. Objects far off-screen are not simulated until the player
  reaches them. Sackboy is a fully physical character in the same world. Air control is not physical. When the engine reaches an
  unsolvable paradox, it destroys offending objects rather than let it get worse (cf. LethalType.CRUSH, maxVel clamp).
  The article does not cover the solver type, timestep, layers or determinism.
- PUBLIC (SuperCheats guide, copyrighted, behaviour only; https://www.supercheats.com/guides/little-big-planet/material-types):
  cardboard and polystyrene are very light; sponge is light; wood is basic; metal and stone are heavy; glass is slippery;
  rubber is for wheels; dark matter is immovable (static, ignores gravity); peach floaty floats away (negative gravityMultiplier
  INFERRED); pink floaty stays where placed (gravityMultiplier 0 INFERRED). Grabbable: polystyrene, sponge, dissolve, floaties.
  Destructible by explosives: cardboard, polystyrene, sponge, dissolve, floaties. TCRF lists unused .mat files
  (Gold, Orrible, Chi, Fluid) but gives no values.
- INFERRED timing: LBP2 renders at 30 fps on PS3. The int frame counters (PWorld.frame/simFrame, emitter frequency/lifetime as
  ints, animationTime default 60) all point to a fixed tick of one sim step per 30 Hz frame. Internal substeps are unknown.
  Do not claim 60 Hz. I found no public source stating the tick rate, so it is UNVERIFIED and needs measuring.
- INFERRED dimensionality: the polygons are 2D (v2 when !requiresZ), angVel is a scalar, and z comes from PPos with layer
  transitions done by gameplay (Sackboy layer-shift, movers' zBehavior). So the model is 2D rigid bodies, each owning a z-slab,
  with collision filtered by slab overlap: "2.5D".
- No public MM paper on the solver (iterative impulse vs Verlet) was found. Alex Evans' talks (SIGGRAPH 2006, Umbra/"Fast
  Approximations") cover rendering, not physics.

## C. Rust options for Bevy 0.19.1 (crates.io API checked 2026-10-08)
| crate | latest | Bevy req | licence | notes |
|---|---|---|---|---|
| avian2d / avian3d | 0.7.0 (2026-06-20) | bevy ^0.19.0 | MIT OR Apache-2.0 | parry2d ^0.27; features: enhanced-determinism, f64, parallel, simd, serialize, xpbd_joints |
| bevy_rapier2d/3d | 0.36.0 (2026-08-08) | bevy ^0.19.0 | Apache-2.0 | pins rapier2d =0.35.0-glamx0.2 (a pre-release-style pin); features: enhanced-determinism, parallel, simd8 |
| rapier2d | 0.36.0 (2026-09-25) | n/a | Apache-2.0 | newer than bevy_rapier's pin; enhanced-determinism, block-solver |
| bevy | 0.19.1 (2026-08-13); 0.20.0-rc.2 is out | | MIT OR Apache-2.0 | expect a 0.20 port churn soon |
All of these licences are OK.
Avian 0.7 (read from the crate source, MIT/Apache): an impulse-based soft-step solver with substeps (`SubstepCount`, default 6),
running in `FixedPostUpdate` driven by Time<Fixed>. Joints: FixedJoint, DistanceJoint (DistanceLimit min/max + compliance), PrismaticJoint
(limits + LinearMotor), RevoluteJoint (AngleLimit + AngularMotor), SphericalJoint (3D). Motors use a MotorModel with a SpringDamper
{frequency, damping_ratio} variant and target_position/target_velocity/max_force. Also JointDamping, JointForces (to read forces,
e.g. for breakResistance), JointDisabled, CollisionLayers (32-bit membership/filter), CollisionHooks (custom pair filtering:
the z-slab overlap test fits here), Sensor, GravityScale, LinearDamping, LockedAxes. Colliders: `Collider::convex_decomposition(verts, [u32;2] idx)`
for 2D concave polylines (parry VHACD), plus polyline and trimesh. CI runs a cross-platform determinism hash test
(src/tests/determinism_2d.rs, after Box2D FallingHinges). `enhanced-determinism` uses libm.
Rapier: joints Fixed, Revolute, Prismatic (motors+limits), Rope, Spring, Generic, and multibody joints. It has a documented
cross-platform determinism feature (enhanced-determinism, IEEE-strict, no simd/parallel). Its convex decomposition comes from parry too.

LBP joint -> engine mapping (works on both engines):
- BOLT -> revolute, free. MOTOR -> revolute plus velocity motor (strength -> max_torque). SPRING_ANGULAR -> revolute plus position motor
  with SpringDamper (angleElasticity). QUANTIZED/wobble -> revolute plus position motor whose target follows animationPattern/Time/Range.
- PISTON -> prismatic along aContact->bContact, position motor animated between tweakTargetMin/Max; stiff -> also lock the angle
  (prismatic does this), non-stiff -> distance-style plus a free angle. CHAIN/winch -> distance-type joint with an animated rest length,
  free rotation. ROD -> distance joint with min=max (rigid length). STRING -> distance with min 0, max length (rope; Rapier RopeJoint).
  ELASTIC/SPRING -> distance joint with compliance/SpringDamper from RJoint.lengthElasticity, plus plasticity (rest length drifts)
  as custom per-tick code. breakResistance -> read JointForces each tick, then despawn.
- There is no engine support for: plasticity, FCurve-driven animation, LBP's "normalizedForces" (force scaled by mass), or the
  "destroy on paradox" policy. Write these as our own systems either way.

2D-per-layer vs 3D-locked-z:
- Recommended: ONE avian2d world. Each body has a component `ZSlab{z, half_thickness}`, and a CollisionHooks::filter_pairs rejects
  pairs whose slabs do not overlap. Layer moves (Sackboy, zBehavior movers) change ZSlab as gameplay, not as physics. Thin layers
  then work for free (any z/thickness). Joints across layers still work because joints ignore collision filtering.
  This keeps the solver 2D (cheap, stable stacking).
- Alternative, avian3d with LockedAxes (lock z translation and x/y rotation) and boxes extruded per slab: real 3D contacts at
  layer boundaries, but convex decomposition in 3D is costlier, stacking is less stable, and LBP's 2D contact semantics are lost.
  Not worth it.
- Alternative, separate 2D worlds per layer: avian has one world per App, so this breaks multi-layer objects and cross-layer joints. Reject.

RECOMMENDATION: avian2d 0.7.0 (bevy ^0.19), feature `enhanced-determinism` (+`f32`, `2d`, `parry-f32`; drop `parallel`/`simd` if bit-exact
replays matter). Fixed 30 Hz `Time<Fixed>` to match LBP frame counters, tune SubstepCount (start at 4-8). Set PWorld.maxVel/maxAVel as
clamps, plus a custom system that destroys or crushes bodies (LethalType.CRUSH) under large penetration.
Reasons: dual MIT/Apache licence (rapier is Apache-only, which is fine too), ECS-native and Bevy-version-aligned on the day of release (0.7 shipped
one day after Bevy 0.19), a CollisionHooks filter for z-slabs, motors with a spring model, joint force readout for breaking, and CI-tested
cross-platform determinism.
Risks: (1) avian has no rope/spring joint *types*. DistanceJoint with limits and compliance covers them, but tuning to match LBP feel takes
work. (2) Avian's API changes every release, and Bevy 0.20 is at rc.2, so pin exact versions. (3) The solver differs from MM's, so contraptions
tuned on PS3 (pistons fighting, stacked heavy stone) will behave differently: plan golden-scene comparison videos.
(4) VHACD decomposition of complex LBP shapes (holes via multi-loop polygons) can be slow and approximate. Cache the decomposition per
shape hash, and handle loops: outer loop + holes -> triangulate (earcut) -> merge to convex, rather than raw VHACD. (5) Mass: we must compute it
ourselves (area * massDepth * density) and override ColliderDensity/Mass so heavy and light materials match.
Fallback: bevy_rapier2d 0.36 (bevy ^0.19) if avian stability proves insufficient. It has native Rope/Spring joints and mature determinism,
but its rapier is pinned to a glamx pre-release and the ECS integration is less idiomatic. Writing our own solver: only if neither engine can reach the feel. Not now.

## D. PC-only checks (need the user's game data)
1. Histogram of PJoint.type (0..10) and animationPattern across all LBP2 levels/plans. Confirm the CHAIN=winch and QUANTIZED=wobble mappings
   by placing each tool in-game and diffing the saved PJoint.
2. Dump every RMaterial in the game's data (density, traction, slidingFriction, rollingFriction, restitution, gravityMultiplier,
   airResistance, collideType/Ignore, grabbable, dissolvable, breakable, maxForce) with material names. Check floaty gravity sign.
3. Dump the stock RJoint resources referenced by PJoint.settings per type (elasticity/plasticity/FCurves/breakResistance).
4. Do level things carry non-zero PBody.posVel/angVel and `frozen`? Distribution of `frozen` values.
5. Histograms of PShape.thickness, massDepth, and PPos world-z: derive layer z centres and spacing (thick vs thin layers).
6. Which LBP2 level/plan files have heads >0x3e2 and contain WIND_TWEAK (expect none). Values of PPhysicsTweak.configuration
   (name them by placing each tweaker), zBehavior values.
7. ShapeFlags and lethalType histograms; stickiness/grabbability/grabFilter value ranges (0/1/2?).
8. PWorld.maxVel/maxAVel/gravity values across levels; frame vs simFrame deltas (pause behaviour).
9. Measure the tick rate empirically (e.g. RPCS3 with frame advance): count frames for a 1 s timer, and the piston period vs animationTime.
10. Check whether the polygon loops>1 (holes) occur, plus the maximum vertex count per shape (affects the decomposition budget).
