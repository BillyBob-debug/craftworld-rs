# Sackboy research: rig, skinning, animation, costumes, instancing (LBP2)

Date 2026-10-08. This is read-only research. Evidence is Toolkit/cwlib (MIT) at pinned commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Links use the base
`TK = https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/`. Paths are
relative to that base. `C` = lib/cwlib/src/main/java/cwlib. The default label is
**CONFIRMED_FROM_COMMUNITY_CODE (CFCC)**. Other labels: **INFERRED** (my reading, not verified),
**UNKNOWN**, and **CFCC-DATA** (decoded from a binary blob in the Toolkit repo; the blob is probably
game-derived, so never ship or vendor it). LBP2 band: head 0x341..0x3f8, branch 0, subVersion 0. Every
`subVersion > 0` gate (LBP3) is ignored below. Local disc ANMb heads are 888..951 (0x378..0x3b7), per
docs/animation-parser.md.

## 1. Sackboy skeleton: 43 bones, 3ds Max Biped naming (CFCC-DATA)

Source: `lib/cwlib/src/main/resources/binary/template.anim`. It is an ANMb resource at rev 0x132 with one
zlib chunk, 4468 bytes expanded. Toolkit loads it as the Sackboy template in `C/io/imports/AnimationImporter.java`
(`SACKBOY`, `getSackboyAnimBones`). I decoded it with the pre-0x378 path (BE `i32` count, then `{u32 hash, s32 parent,
s32 firstChild, s32 nextSibling}`). Every hash matched a name under `RAnimation.calculateAnimationHash`, so the
hash algorithm is verified. The template has 13 frames at 30 fps and **morphCount = 29**.

| idx | bone | parent | idx | bone | parent | idx | bone | parent |
|---|---|---|---|---|---|---|---|---|
|0|Dummy01 (root/locator)|-1|15|Bip01 R Clavicle|9|30|Bip01 L Hand|25|
|1|Bip01|0|16|Bip01 R Foot|10|31|Bip01 R Hand|26|
|2|Bip01 Footsteps|1|17|bone_zip_pull02|11|32|bone_tongue02|29|
|3|Bip01 Pelvis|1|18|Bip01 L Toe0|14|33-37|Bip01 L Finger0..4|30|
|4|Bip01 Spine|3|19|Bip01 L UpperArm|13|38-42|Bip01 R Finger0..4|31|
|5|Bip01 L Thigh|4|20|Bip01 R Toe0|16| | | |
|6|Bip01 R Thigh|4|21|Bip01 R UpperArm|15| | | |
|7|Bip01 Spine1|4|22|Bone_jaw|12| | | |
|8|Bip01 L Calf|5|23|bone_eye_pos_left|12| | | |
|9|Bip01 Neck|7|24|bone_eye_pos_right|12| | | |
|10|Bip01 R Calf|6|25|Bip01 L Forearm|19| | | |
|11|bone_zip_pull01|7|26|Bip01 R Forearm|21| | | |
|12|Bip01 Head|9|27|bone_eye_left|23| | | |
|13|Bip01 L Clavicle|9|28|bone_eye_right|24| | | |
|14|Bip01 L Foot|8|29|bone_tongue01|22| | | |

- `characters.boneset` (BSTb, Toolkit-custom `RBoneSet` → `Skeleton{bones, mirror, mirrorType, cullBones}`;
  `C/resources/custom/RBoneSet.java`, `C/structs/custom/Skeleton.java`) holds one skeleton per `SkeletonType`
  (`C/enums/SkeletonType.java`: SACKBOY 0, GIANT 1, DWARF 2, BIRD 3, QUAD 4; the extra types are probably LBP3
  creatures). Its strings also show `Bip01 R ForeTwist` and `Bip01 L/R Toe1..3` in other skeletons.
- **Anim hash** (`C/resources/RAnimation.java` `calculateAnimationHash`): if the name contains `" R "`, remove it and use
  offset 3. Otherwise, if it contains `" L "`, remove it and use offset 2. Then `h = h*0x1003F + c` over the
  chars (64-bit). The result is `(u32)(h*4) + offset`. So L/R pairs differ only in the low bits (0x...24/0x...25 style).
  `animHash == 0` maps to bone 0 (`getBoneIndex`).
- Bone flags (`C/enums/BoneFlag.java`): CAMERA 0x1, DUMMY 0x2. Mirroring (`C/enums/FlipType.java`): MAX, BIP_ROOT,
  BIP_PELVIS, BIP_BONE, PARENT_WAS_BIP_BONE, GRANDPARENT_WAS_BIP_BONE, COPY. These are used with
  `RMesh.mirrorBones`/`mirrorMorphs` and are presumably for the game's L/R mirroring of anims (INFERRED).

## 2. RMesh skinning layout (`C/resources/RMesh.java`, `C/structs/mesh/*`)

**Bone serialization.** `C/structs/mesh/Bone.java` `serialize` reads the fields in this order:
1. `str(name, 32)` fixed, NUL-padded.
2. `i32 flags`.
3. AnimBone (`C/structs/animation/AnimBone.java`): `i32 animHash`, `s32 parent`, `s32 firstChild`, `s32 nextSibling`.
   - With compressed ints these are varints, and `s32` is zigzag. That matches what `skip_bone` already consumes.
4. `m44 skinPoseMatrix` and `m44 invSkinPoseMatrix`.
5. `v4 obbMin`, `v4 obbMax`.
6. `MeshShapeVertex[]` and `MeshShapeInfo[]`:
   - `MeshShapeVertex` is `{v4 localPos, v4 localNormal, i32 boneIndex}`.
   - `MeshShapeInfo` is `{i32 numVerts, intbool isPointCloud}`.
7. `f32 shapeMinZ`, `f32 shapeMaxZ`.
8. `v4 boundBoxMin`, `v4 boundBoxMax`, `v4 boundSphere`.

**m44 format.** `C/io/streams/MemoryInputStream.java` `m44`: if `USE_COMPRESSED_MATRICES` is set, there is first a u16
bitmask. Only the set entries are stored as f32, and the rest default to identity. The 16 floats go into JOML
`Matrix4f.set(float[])`, which is **column-major**, so translation is at [12..14]. That maps 1:1 to Bevy
`Mat4::from_cols_array`.

**Bind pose semantics.** `skinPoseMatrix` is the bone's model-space bind transform, and `invSkinPoseMatrix` is its
inverse (the glTF inverse-bind matrix). The local transform is `parent.inv * self`
(`Bone.getLocalTransform`). LBP stores **no local TRS for bones**. Toolkit decomposes the matrices
(`lib/cwlib-gl/.../exporter/SceneExporter.java` `registerSkeletonChildrenInternal`), and first
`normalize3x3()`s each skinPose (`RMesh.fixupSkinForExport`). So skin poses may carry scale; the INFERRED fix is to
strip scale before decomposing, as Toolkit does.

**Locator roots and submeshes.** Bones with `parent == -1` are "locators". `RMesh.findAllMeshes` makes one submesh
per root. A primitive belongs to the root reached from the joint of its first vertex, and the submesh is skinned iff
`root.firstChild != -1`.

**Vertex streams.** Each vertex stream is `numVerts*0x10` bytes, and streams are BE.
- **Stream 0** (`STREAM_POS_BONEINDICES`): `v3 pos` + 4 bytes. `getBlendIndices` comments that the 4 bytes are
  "always 0x000000FF … deprecated", while the softbody code reads `[0xC]` as cluster*2 and `[0xF]` as softbody weight.
- **Stream 1** (`STREAM_BONEWEIGHTS_NORM_TANGENT_SMOOTH_NORM`) has 4 groups of 4 bytes.
  - Byte order: `[w2? w1 w0 J0][normal24 J1][tangent24 J2][smoothNormal24 J3]`.
  - `getJoints` reads joint bytes **3, 7, 11, 15** as `u8` indices into `RMesh.bones`.
  - `getWeights` builds `(b[2], b[1], b[0], 255-b0-b1-b2)/255` and special-cases `b[2]==0xFF` → `(1,0,0,0)`.
  - Normals are 24-bit: x is 12 bits (11 + sign), y is 11 bits (10 + sign), and z is reconstructed from the sign at
    bit 23 (`C/util/Bytes.java` `unpackNormal24`).
- **Streams 2..** are morph targets (`morphCount = streamCount - 2`, max 32). Per vertex: `v3 offset` + `u32 packed normal`
  (`unpackNormal32` 10/10/9 + signs). `getMorphs`, and `SceneExporter` exports them as glTF targets with
  `extras.targetNames = morphNames`. Morph names are 32 fixed `str(0x10)` entries in the header.
- **UV attributes** are a separate blob: `attributeCount` channels × 8 bytes per vertex, interleaved per vertex as
  `(attributeCount*8*i + 8*ch)`.

**Remaining tail fields of `serialize`**, in order:
- `primitives`: material, textureAlternatives ≥0x179, then `minVert`, `maxVert`, `firstIndex`, `numIndices`,
  **`region`**.
- `bones`, `mirrorBones short[]`, `mirrorBoneFlipTypes`, `mirrorMorphs short[]`, `primitiveType u8`.
- Softbody: `cluster`, `springs`, `equivs`, `mass`, `implicitEllipsoids`, cluster matrices, inside ellipsoids,
  `planes`, `softPhysSettings`, spring ranges, `springyTriIndices`, `springTrisStripped`, softbody bounds.
- **`cullBones`** (`{m44 invSkinPose, v4 bbMin, v4 bbMax}`).
- **`regionIDsToHide int[]`**, **`costumeCategoriesUsed i32`**, **`hairMorphs` enum32** (≥0x141: HAT 0, HELMET 1,
  FRINGE 2), `bevelVertexCount`, `implicitBevelSprings`.
- `vertexColors` is DOUBLE11 (Vita) only.
- `skeletonType` is not read in the main path for LBP2. It is a Toolkit field defaulting to SACKBOY.
- `triangleAdjacencyInfos` is subVersion≥FUZZ, so it is LBP3-only.

## 3. RAnimation (ANMb) keyframe format (`C/resources/RAnimation.java` `serialize`)

**Old format (version < 0x378).** Plain arrays: `AnimBone[]` (`i32/s32 ×4`), `i32 numFrames`, `i32 fps`,
`i32 morphCount`, four `i16` index arrays (rot/pos/scale/morph animated), then `v4[] packedRotation`, `pos`,
`scale`, `f32[] morph`. Locators are a TODO. This path is not needed for local LBP2 data.

**New format (≥0x378, every local file).**
1. Header: `i16 numFrames`, `i16 fps`, `i16 loopStart`, `i8 morphCount`, then `u8 boneCount`, `u8 rotAnims`,
   `u8 posAnims`, `u8 scaleAnims`, `u8 morphAnims`, `u16 locatorKeys`.
2. Version-gated header fields: `>0x38b` adds `v4 posOffset`, `v4 posScale`; `>0x3b1` adds `i8 fat`.
3. `bytearray animData`, then `Locator[]` (`{v3 position, str name, i8 looping, i8 type}`).

Inside `animData`, which is **big-endian**:

| order | content | encoding |
|---|---|---|
|1|bones × `{u32 animHash, u8 parent, u8 firstChild, u8 nextSibling, u8 pad}`|parent 0xFF = -1 (read as i8)|
|2|`locatorKeys × 4` bytes|skipped (structure UNKNOWN: "short index?, byte, pad")|
|3|rotAnimated[rotAnims], posAnimated[], scaleAnimated[], morphAnimated[]|u8 bone/morph indices|
|4|pad to 2-byte alignment||
|5|packedRotation: `boneCount + rotAnims*(numFrames-1)` entries|3×i16: x,y,z /0x7FFF; **LSB of x = sign of w**; `w = sqrt(1-x²-y²-z²)`|
|6|packedPosition: `boneCount + posAnims*(numFrames-1)`|3×f16 (half)|
|7|packedScale: `boneCount + scaleAnims*(numFrames-1)`|3×f16|
|8|packedMorph: `morphCount + morphAnims*(numFrames-1)`|f16|
|9|pad to 16 (writer)||

**Decoding rules.**
- **Frame 0 is the base pose for all bones** (first `boneCount` entries). For frame f ≥ 1, track k of a channel is at
  `boneCount + (f-1)*nAnimated + k` and applies to bone `animated[k]`. Non-animated bones keep the frame-0 value
  (`getAnimDataForFrame`).
- Morphs work the same way with a `morphCount` base.
  - Toolkit's `getMorphFrames` indexes with the morph index instead of the track index. This looks like a bug, so
    use the `getAnimDataForFrame` logic.
  - `getBaseTransform` uses `sx.y` for z scale, also a bug.
- **Looping** (`getLoopedFrame`): frames wrap in `[loopStart, numFrames)`. Non-looped animations clamp.
- **Sampling**: Toolkit lerps pos/scale and slerps rot between integer frames (`getBlendedFrameMatrix`). The exporter
  uses timestep `1/fps` and LINEAR samplers.
- **Bone matching**: animation bones are matched to mesh bones **by animHash**, not by index
  (`getBoneIndex`, `Bone.getByHash`).
- **UNKNOWN**: `posOffset`/`posScale` (>0x38b) are read but **never applied** by Toolkit. They may be a dequant affine
  for the f16 positions. `fat` (>0x3b1) is ignored, and the source comment guesses "4 bytes vs 2 bytes". Both need a
  local check (section 8).

## 4. Thing parts for a Sackboy, Sackbots, and checkpoints (LBP2-gated fields only)

A Thing's parts come from `C/enums/Part.java`. A player Sackboy Thing is expected to carry: YELLOWHEAD 0x6, CREATURE
0xD, COSTUME 0x17, RENDER_MESH 0x3, POS 0x4, BODY 0x0, plus STICKERS 0xF / MATERIAL_OVERRIDE as needed. A Sackbot adds
NPC 0x1F and lacks YELLOWHEAD. This combination is INFERRED; Toolkit has no factory for it.

**PYellowHead** (`C/structs/things/parts/PYellowHead.java`) is the player link. Fields in order:
1. `head` (Thing).
2. `legacyToolTetherJoint` (>0x1fc).
3. `legacyToolTetherWidth f32`.
4. **`playerNumber s32`**.
5. `poppet` (Poppet struct).
6. `sensorHistory v4[]` + `newestSense` (<0x3da, so present in LBP2).
7. `requestedSuicide`.
8. `legacyJetpack`.
9. `onScreenCounter`, `onScreenStatus`, `editJetpack`, `recordee` (>0x272).
10. `lastTimeSlappedAPlayer` (>0x359).

`animSetKey`, `creatureToSpawnAs` and `monstrousHeadScale` are LBP3-only.

**PCreature** (`.../PCreature.java`) holds the movement state. The LBP2 fields fall into these groups:
- **Config and jump:** `config` → **SETTINGS_CHARACTER resource (`.cha`, type 12) — Toolkit has NO parser for it.**
  It is the likely home of speeds and jump constants. Then `jumpFrame`.
- **Ground and grab:** `groundDistance`, `groundNormal`, **`grabJoint` (Thing)**, `jumpingOff`.
- **State:** **`state i32`, `stateTimer`** (`subState` is LBP3). There is no enum for the state values in Toolkit, so
  they are UNKNOWN.
- **Modifiers and mode:** `speedModifier`, `jumpModifier`, `strengthModifier`, `zMode` (layer), `playerAwareness`,
  `moveDirection`.
- **Crush and air:** `forceThatSmashedCreature`, `crushFrames`, `awarenessRadius`, `airTime`. Then
  `bouncepadThingUIDs` / `grabbedThingUIDs` (≥0x354) and `haveNotTouchedGroundSinceUsingJetpack`.
- **Creature brain lists:** `legList`, `lifeSourceList`, `lifeCreature`, `aiCreature`, `jumpInterval/Phase`,
  `meshDirty`, `eyeList`, `brainAiList`, `brainLifeList`, `reactToLethal` (LethalType in `C/enums/LethalType.java`:
  FIRE 1, ELECTRIC 2, CRUSH 4, SPIKE 5, GAS 6-11, NO_STAND 12, BULLET 13, DROWNED 14).
- **Animation offset:** `oldAnimMatrix m44`, `animOffset` (≥0x1a9).
- **Ground smoothing:** `groundNormalRaw`, `groundNormalSmooth`, `bodyAdjustApplied`.
- **Weapons and powerups:** the gun block (≥0x243..0x320: dir, fire frames, bullets, emitters, accuracy, gunThing),
  `airTimeLeft`, submerged amounts and scuba (≥0x2c9/0x289), `handPiece` (Plan), `head`, `toolTetherJoint`,
  `jetpack`, `wallJumpDir/Pos`, `bootContactForceList`, `gunType`, `wallJumpMat` (≥0x273).
- **Remote control and misc:** `directControlPrompt` (>0x2e4; older 0x29e..0x336 adds stick/anim frame/state),
  `responsiblePlayer`, `canDropPowerup`, `capeExtraMaxVelocityCap` (≥0x3f0), **`behavior` (≥0x35a)**,
  `effectDestroy`, and `whipSim` (≥0x3c0, grappling-hook sim).

All of this is runtime state. None of it is tuning data.

**PNpc** (`.../PNpc.java`) holds Sackbot AI:
- `jumpSolver` (`NpcJumpSolver`: `jumpData[]{a,b,c, min,max, flipped, NpcMoveCmd[], apex}`, `maxEffectiveJumpHeight`,
  `trained`, `standingJump`). The a/b/c fields are probably parabola coefficients (INFERRED).
- Sound/input recording buffers and `recordingBehavior` (`NpcBehavior` type: PATROL 0, FOLLOW 1, FLEE 2, IDLE 3, ACT 4,
  WAYPOINT 5; attrs CAN_CHANGE_HEIGHT 1, CAN_JUMP 2, CAN_CHANGE_LAYER 4, HOSTILE 0x10; `maxMoveSpeed`, `maxWaitTime`,
  `awarenessRadius`; in `.../components/npc/NpcBehavior.java`).
- `flags` (IS_JUMPING 1, IS_HOSTILE 2, IS_PAUSED 4), `behaviorThing`, `rootBehaviorThing`, `moveTarget`, `waitTime`,
  `playerNumber`, `actorName` (>0x338), throw/hit times, `costumeToCopy` (>0x391).
- The recording `Input` is `{alive, leftStick v3, rightStick v3, buttons, buttonsOld, sensor*, pressureData,
  playerNumber}` (`.../npc/Input.java`). `ActingData` and `InputRecording` hold the record/replay "Sackbot actor"
  data.
- Sackbot switch inputs are in `PSwitchInput` (SACKBOT_BEHAVIOR 10, WAYPOINT 11, LOOKAT 12, JUMP 13, GRAB 14,
  LAYERCHANGE 15). `PWorld.nextSackbotPlayerNumber = -2` (≥0x2b0) implies Sackbots get negative player numbers
  (INFERRED).

**PControlinator** (`.../PControlinator.java`) is LBP2 possession. Fields: `attachedPlayer`, `lastAttachedPlayer`,
`prompt`, `promptPlayer`, `colorIndex`, `radius`, `sideMode`, `remoteControlState`, `disablePoppetControls`,
`autoDock`, **`overrideSackbot`**, `padSwitch`, and `playerMode` (>0x3ec). The bone parenting fields are LBP3.

**PCheckpoint** (`.../PCheckpoint.java`) handles spawning. LBP2 fields: `activeFlags` (bool → 0xF), `activationFrame`,
`spawnsLeft`, `maxSpawnsLeft`, `instanceInfiniteSpawns`, **`spawningList Thing[]` + `spawningDelay`** (≥0x1f3),
`lifeMultiplier`, and `teamFilter` (≥0x2ae). `isStartPoint`, `name` and `creatureToSpawnAs` are LBP3. So in LBP2 the
"entrance" checkpoint is not flagged by a field; INFERRED: the game identifies it by the checkpoint plan/mesh.

**PRenderMesh** (`.../PRenderMesh.java`) holds `mesh`, **`boneThings Thing[]`** (one Thing per bone, with PPos),
`anim`, `animPos`, `animSpeed`, `animLoop`, `loopStart/End`, `editorColor`, `castShadows`, `RTTEnable`,
`visibilityFlags`, `poppetRenderScale`, and `parentDistanceFront/Side` (0x1f5..0x34d).
- For saved Sackboys, the per-bone pose lives in the boneThings' PPos.
- `SceneExporter.registerThing` sets each joint node matrix to `boneThing.PPos.localPosition * bone.invSkinPose`.

**PAnimation** holds `{anim, velocity, position}`. **PAnimationTweak** is wholly subVersion-gated, so it is LBP3 and
not relevant.

## 5. Costumes

**PCostume** (`.../PCostume.java`, LBP2) fields: `mesh` (base body MESH), `material` (GFX_MATERIAL),
`materialPlan` (≥0x19a), **`meshPartsHidden int[]`**, `primitives Primitive[]`, `costumePieces CostumePiece[]`, and
`temporaryCostumePiece[]` (≥0x2c5).

**CostumePiece** (`.../components/CostumePiece.java`) fields: `mesh`, `categoriesUsed` (bitmask of
`CostumePieceCategory`: BEARD 0, FEET 1, EYES 2, GLASSES 3, MOUTH 4, MOUSTACHE 5, NOSE 6, HAIR 7, HEAD 8, NECK 9,
TORSO 10, LEGS 11, HANDS 12, WAIST 13 — so 14 slots), **`morphParamRemap`** (count + i32 each in LBP2, since
subVersion < 0x105; default -1 ×32), `primitives`, and `plan` (≥0x19a).

**How Toolkit assembles a costumed Sackboy** (`lib/cwlib-gl/.../SceneExporter.java` `registerThing`; the INFERRED
rule is that this matches the game's intent):
1. Render `PRenderMesh.mesh` (the body) with skeleton S.
2. Hide every primitive whose `region` is in `PCostume.meshPartsHidden`. In the game this set is presumably the union
   of the pieces' `RMesh.regionIDsToHide`.
3. Replace material **GUID 9698** (Sackboy's default fabric) with `PCostume.material`.
4. For each costume piece, skip mesh GUIDs **9876 / 9877** (default/empty placeholder pieces), then render the piece's
   mesh **on the body skeleton S**, with stickers from `PStickers.costumeDecals[i]`.
   - Piece joint bytes are treated as indices into S's bone list. Toolkit's `remap` array is never filled (WIP), so
     this binding is unreliable. Bind by animHash instead.
5. Region hiding and material overrides are also used for `PMaterialOverride.overrides` (RegionOverride).

**Facial expressions and morphs.** The body mesh carries up to 32 named morphs. The template anim has 29 morph
channels. `CostumePiece.morphParamRemap[32]` maps body morph params to the piece's own morphs (-1 = none), so a mask or
beard follows the face morphs. INFERRED: expressions (happy/sad/angry/scared × 3 levels) are driven as morph weights
plus eye/jaw/tongue bones. The emote animations' morph tracks supply them. `HairMorph` (HAT/HELMET/FRINGE) chooses how
hair is squashed under hats, presumably via specific body or hair morphs.

**ROutfitList** (`C/resources/ROutfitList.java`) is `i32 count` + `Outfit{guidlist components, resource(PLAN) outfit}`
(`C/structs/inventory/Outfit.java`). These are saved outfits.

**RCachedCostumeData** is an `arraylist` of `{guid plan, titleKey, descriptionKey, type, subType, colour, guid icon,
value}`. It is an inventory/menu cache; there is no geometry.

**RSyncedProfile.sackboyAvatarWorld** (Thing) holds the profile's avatar. This is where the player's costume lives
(`C/resources/RSyncedProfile.java`).

## 6. What Toolkit does NOT tell us

| Topic | Status | Where it probably lives |
|---|---|---|
|Walk/run speed, accel, friction, jump impulse/arc, gravity, air control|UNKNOWN|`.cha` SETTINGS_CHARACTER (unparsed), plus code|
|Creature `state` enum (walk/jump/grab/fall/dead…), transitions|UNKNOWN|game code|
|Which RAnimation plays per state, blend times, layering (upper body grab vs legs), anim GUID table|UNKNOWN|code / settings (no `.cha` parser; no anim name list in Toolkit)|
|Grab logic (reach, `grabJoint` creation, strength), slap, emote button mapping|UNKNOWN|code (`PJoint` created at runtime)|
|IK (foot plant on slopes: `groundNormalSmooth`, `bodyAdjustApplied` hint at body tilt), look-at/eyes|UNKNOWN beyond field names|code|
|Physics capsule/shape for collision|UNKNOWN; `Bone.shapeVerts/shapeInfos/obb`, `shapeMinZ/MaxZ` look like per-bone collision hulls (INFERRED); `PShape.collidableSackboy`|mesh + code|
|Sackbot AI path/jump solver math|only serialized fields|code|
|Softbody (Sackboy is softbody: springs/clusters in RMesh, `.sph` SoftPhys settings)|layout only|code|
|Morph name list for the Sackboy mesh, expression → weight mapping|UNKNOWN (needs local mesh)|data|

## 7. Public sources (licence flags)

- **ennuo/toolkit** (MIT). The primary source.
  - `characters.boneset` / `template.anim` are game-derived blobs. Use them as research evidence only and never ship
    them. Their licensing is unclear even under the MIT repo.
- **LittleBigWiki** (littlebigplanet.miraheze.org, wiki CC BY-SA typical; verify) and the **LBP Fandom wiki**
  (CC BY-SA). Gameplay and descriptive behaviour only (power-ups, Sackbot behaviours, Controlinator UX). No numbers.
- **LBPUnion / ProjectLighthouse** (AGPL server). No character behaviour; ignore.
- **uhwot/toolkit** is a fork of Toolkit (check licence = MIT inherited).
- **Zhaxxy/lbptoolspy** (GPL-3.0). Saves/archives only; not relevant.
- **Hidden Palace LBP2 debug/prototype builds** (2010, 2011, 2013 listings). These are leaked proprietary binaries,
  possibly with symbols. Do **not** use them as a source.
  - Any decompilation (including of the user's own disc in RPCS3 for local research) may only yield behaviour
    observations such as speeds and state names. Never copy code or tables verbatim.
- Searches found **no open-source LBP gameplay reimplementation**, and no public Sackboy animation-name list or Sackboy
  GUID list (as of 2026-10-08). Gameplay constants must be measured: RPCS3 frame-stepping plus memory watch of the
  PCreature fields above, or video timing. Treat the results as OBSERVED_LOCAL.

## 8. Implementation plan against the existing Rust crates

**(1) Decode bones (`crates/lbp-mesh/src/geometry.rs`).** Replace `skip_bone` with a parse that reads:
- `name[32]`, `flags`, `animHash`, then zigzag/BE parent/firstChild/nextSibling (the same integer reader).
- Two m44, each with an optional u16 mask, filling missing entries with identity, column-major.
- OBB, shapeVerts (`v4, v4, i32`), shapeInfos, minZ/maxZ, and 3 × v4 bounds.

Then also keep `mirrorBones`, `mirrorBoneFlipTypes`, `mirrorMorphs` (currently skipped), and parse the tail through
`cullBones`, `regionIDsToHide`, `costumeCategoriesUsed`, `hairMorphs`. Per vertex, read stream 1 joints (bytes 3/7/11/15)
and weights (b2, b1, b0, remainder; special case 0xFF). Read morph streams 2.. as `v3 + u32 normal`, and morph names.
Validate this on a local Sackboy mesh: parents must form a tree, and `skinPose * invSkinPose ≈ I`.

**(2) Decode ANMb keyframes (`crates/lbp-resource/src/animation.rs`).**
- Parse `animData` per the table in section 3: bones (u8 parent), skip `locatorKeys*4`, the four index lists, align 2,
  then `rot i16x3` with the w sign in x's LSB, half-float pos/scale/morph.
- Expand to a dense `[frame][bone]` TRS by copying the base and overriding animated tracks.
- Test that the consumed length equals `animData` length minus 0..15 pad. This is the strongest self-check.
- Decide whether positions need `posOffset + p*posScale` by comparing frame-0 positions against the bind pose
  local translations of the matching mesh bones.

**(3) Skin in Bevy 0.19** (confirm exact API names against the 0.19 docs; this is from 0.15–0.18 knowledge).
- One joint entity per bone with `Transform` = local bind (`parent.inv*self`, scale-normalized), and `Name` = bone name.
- `SkinnedMesh { inverse_bindposes: Handle<SkinnedMeshInverseBindposes>(vec of invSkinPose), joints: Vec<Entity> }`.
- Mesh attributes `ATTRIBUTE_JOINT_INDEX` (Uint16x4) and `ATTRIBUTE_JOINT_WEIGHT` (Float32x4, normalized).
- Morphs: `MorphTargetImage` / `Mesh::set_morph_target_names`, `MorphWeights` on the parent and `MeshMorphWeights` on the
  mesh entity.
- Clips: `AnimationClip` with per-bone curves (`AnimatableCurve` + `animated_field!(Transform::rotation/translation/scale)`)
  using `AnimationTargetId::from_names(path)`, and a morph weights curve. Sample times are `f/fps`, linear. Looping
  from `loopStart` needs either a split clip (intro + loop) or a custom driver.
- `AnimationGraph` + `AnimationPlayer` on the root, with target components on the joints.
- Match clip bones to joints **by animHash**, not index.
- Each root locator (parent -1) is a separate submesh node.

**(4) Costumed Sackboy.**
1. Body mesh = `PCostume.mesh` (or `PRenderMesh.mesh`). Hide primitives with `region ∈ meshPartsHidden`.
2. Override material 9698 → `PCostume.material`.
3. Spawn each `costumePieces[i].mesh` (skip 9876/9877) skinned to the **same joint entities**, remapping each piece
   bone to the body joint with the same animHash.
4. Apply face/expression morph weights to the body, and through `morphParamRemap` to each piece.
5. Pose source: `PRenderMesh.boneThings[i].PPos` for saved poses, or an RAnimation clip.

## 9. Questions needing the user's game data (PC-only checks)

1. Sackboy body mesh: GUID and path (likely under `characters/sackboy`). Check that the bone count is 43 with the same
   names, parents and hash order as section 1, and whether any skinPose has non-unit scale.
2. Morphs on the body mesh: count (≤32), the 16-char names, and `mirrorMorphs`. Is it 29 like the template anim?
3. Are GUIDs 9698 (fabric gmat) and 9876/9877 present in the LBP2 MAP, and what are their paths?
4. Animation inventory: number of ANMb files whose bone sets equal the 43 Sackboy hashes, and their names/paths (walk,
   run, jump, grab, emotes, expressions). Also their fps, loopStart and morph track counts.
5. ANMb `fat` byte values across heads >0x3b1: always 0? And `posOffset/posScale` values: identity (0,1)? Does
   `animData` length match the f16/i16 computed size exactly (fat = 0), or 2× (fat = 1 → 32-bit)?
6. `locatorKeys != 0` samples: record their 4-byte records and locator names (probably footstep/sound/effect events).
7. Costume piece meshes: are bone counts and order identical to the body, or only hash-matched? Check
   `regionIDsToHide`, `costumeCategoriesUsed` and `hairMorphs` values, and the region IDs present on body primitives.
8. Is there a SETTINGS_CHARACTER (`.cha`, type 12) resource in the MAP? Report size and revision; this is the likely
   source of movement constants. The format is unparsed anywhere public.
9. Does `PCostume.meshPartsHidden` in a saved level/profile equal the union of the pieces' `regionIDsToHide`?
10. In a level with a Sackbot: `PNpc.jumpSolver.jumpData` a/b/c/apex values. Do they fit a parabola, which would give
    an implied gravity/jump arc?
