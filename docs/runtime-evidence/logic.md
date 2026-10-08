# LBP2 logic and scripting: research notes (2026-10-08)
Scope: LBP2 band only (head 0x341..0x3f8, branch 0, subVersion 0). Labels follow
docs/formats.md: CONFIRMED_FROM_COMMUNITY_CODE (CCC), CONFIRMED_FROM_DOCUMENTATION (DOC),
STRONG_INFERENCE (SI), UNKNOWN. Toolkit = ennuo/toolkit, MIT, pinned
d6a2ef80b5a09999f08f23a803b516096055e5c9. URL prefix
`T = https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/`.
Toolkit code is used only as a format specification. Nothing below copies its code.

## A. Data layer (Toolkit, CCC unless marked)

### A1. Part ids (T+enums/Part.java, PartHistory.java)
SWITCH 0x19, SWITCH_KEY 0x1a, GAMEPLAY_DATA 0x1b, SWITCH_INPUT 0x20, MICROCHIP 0x21,
SEQUENCER 0x25, CONTROLINATOR 0x26, POCKET_ITEM 0x28, TAG_SYNCHRONIZER 0x2f,
TRIGGER 0x5, SCRIPT_NAME 0xC, CHECKPOINT 0xE, SCRIPT 0x11, EFFECTOR 0x13, EMITTER 0x14,
PHYSICS_TWEAK 0x1e. These are the gadget parts that send or receive signals.

### A2. PSwitch (T+structs/things/parts/PSwitch.java, `serialize`)
Every logic source is a Thing with a PSwitch, including sensors, gates, the microchip,
timers, counters, the randomiser, the selector, batteries and so on. `type` (SwitchType)
picks the behaviour. Read order in the LBP2 band (version v):
inverted b, radius f32, minRadius f32 (v>=0x382), colorIndex s32, name wstr,
crappyOldLbp1Switch b (v>=0x38f; reader forces true if v<0x309), behaviorOld s32,
**outputs = array(SwitchOutput)** (v>=0x329, LBP2 band always), [stickerPlan omitted in
v>=0x398 here], hideInPlayMode b, **type enum32**, referenceThing (thing ref),
manualActivation SwitchSignal, stickerPlan (v>=0x398 and type STICKER),
platformVisualFactor f32 (v<0x368), activationHoldTime s32, requireAll b,
angleRange f32, includeTouching s32 (+stickerPlan if v>=0x398, MICROCHIP and
includeTouching==1), bulletsRequired s32, bulletsDetected s32, bulletPlayerNumber
(v<0x398), bulletRefreshTime s32, resetWhenFull b, **logicType enum32 (v<0x398 only)**,
updateFrame s32 (v<0x369 only), inputList thingarray (deprecated, still serialized),
includeRigidConnectors b, timerCount f32, teamFilter s32 (subVersion<0x100),
**behavior enum32 SwitchBehavior**, randomBehavior, randomPattern, randomOnTimeMin/Max,
randomOffTimeMin/Max (i32), randomPhaseOn u8 + randomPhaseTime s32 (v<0x3ad),
retardedOldJoint b, keySensorMode i32, userDefinedColour i32, wiresVisible b (v>0x34c),
bulletTypes i8 (v>0x34f), detectUnspawnedPlayers b (v>0x390), playSwitchAudio b
(v>0x3a4), playerMode i8 (v>0x3ec), DATA_SAMPLER value block (v>0x3ee).
Split points inside the band: 0x368, 0x369, 0x382, 0x38f, 0x390, 0x398, 0x3a4, 0x3ac,
0x3ec, 0x3ee. Before 0x398 the gate kind is in `logicType` as well as `type`. After it,
only `type` (AND 14, OR 21, XOR 22, NOT 23, NOP 24) is serialized (SI: the engine derives the gate kind).
Other runtime state is also serialized: timerCount (timer/counter value),
bulletsDetected, output activations. Level saves therefore hold live logic state.

### A3. Wires: SwitchOutput / SwitchSignal / SwitchTarget
(T+structs/things/components/switches/*.java)
- SwitchOutput = { activation: SwitchSignal, targetList: array(SwitchTarget),
  userDefinedName wstr (v>=0x34d) }. There is one entry per **output port** of the gadget.
  The array index is the output port number (SI).
- SwitchSignal = { activation f32, ternary s32 (v>=0x310), player i32 (v>=0x2a3, default -1) }.
  So one wire carries an analog float, a ternary sign/state (-1/0/+1, SI) and the id of
  the player who caused it. Both analog and digital values travel in the same struct.
  The digital view is derived from them (SI).
- SwitchTarget = { thing: Thing ref, port i32 (v>0x326) }. **Wires are stored on the
  source only**: output k -> list of (destination Thing, destination input port).
  No wire list is stored on the receiver. Toolkit's `PWorld.getSwitchInput(target)` finds
  the inputs of a target by scanning every PSwitch's outputs (T+structs/things/parts/PWorld.java).
  A runtime has to build that reverse index itself.
- Before 0x329 there was one output with an inline signal plus a target list (LBP1
  and early LBP2 dev builds). This is out of band.
- Revisions.java: DYNAMIC_SWITCHES 0x1a5 ("switches now carry activation"),
  SWITCH_BEHAVIOR 0x2c4 ("Randomiser added, SwitchInput no longer carries signals"),
  SIGNAL_TERNARY 0x310, COMPACT_MICROCHIP 0x34d (T+enums/Revisions.java).

### A4. Enums (T+enums/)
- SwitchType: INVALID -1, BUTTON 0, LEVER 1, TRINARY 2, PROXIMITY 3, KEY 4, STICKER 5,
  GRAB 6, PRESSURE 7, PAINT 8, CONTROLLER_BINARY 9, CONTROLLER_ANALOG 10,
  SACKBOT_KEY 11, SACKBOT_PLAYER 12, SACKBOT_BOT 13, AND 14, COUNTDOWN 15, TIMER 16,
  TOGGLE 17, IMPACT 18, RANDOM 19, DIRECTION 20, OR 21, XOR 22, NOT 23, NOP 24,
  MOISTURE 25, INACTIVE 26, SIGN_SPLIT 27, ALWAYS_ON 28 (battery), ANIMATIC 29, SCORE 30,
  DEATH 31, CUTSCENE_CAM_FINISHED 32, CUTSCENE_CAM_ACTIVE 33, MAGIC_MOUTH 34,
  SELECTOR 35, MICROCHIP 36, CIRCUIT_BOARD 37, CONTROL_PAD 38, PROJECTILE 39,
  CIRCUIT_NODE 40, ANGLE 41, VELOCITY_LINEAR 42, VELOCITY_ANGULAR 43, MOVINATOR_PAD 44,
  MOTION_RECORDER 45, FILTER 46, COLOUR_GATE 47, WAVE_GENERATOR 48, EMITTEE 49,
  POCKET_ITEM 50, POSE 51, ADVANCED_COUNTDOWN 52, STATE_SENSOR 53, KEY_REMOTE 54,
  ... 55-86 are Vita/LBP3 additions (VITA_PAD, QUEST, GRID_MOVER, DATA_SAMPLER 65, ...).
  The LBP2 band presumably uses about 0..54 (SI; check with the histogram in section D).
- SwitchBehavior (output behaviour on receivers): OFF_ON 0, SPEED_SCALE 1, DIRECTION 2, ONE_SHOT 3.
- SwitchLogicType: AND 0, OR 1, XOR 2, NOT 3, NOP 4.
- SwitchKeyType (deprecated <0x2dc): MAGNETIC 0, IMPACT 1.
- TriggerType: RADIUS 0, RECT 1, SWITCH 2, TOUCH 3, RADIUS_3D 4.
- GameplayPartType: UNDEFINED 0, LEVEL_KEY 1, PRIZE_BUBBLE 2, SCORE_BUBBLE 3,
  COLLECTABUBBLE 4, POCKET_ITEM_BUBBLE 5, TREASURE_ITEM 6, OBJECT_SAVER 7, RUMBLER 8.

### A5. Receivers: PSwitchInput (T+structs/things/parts/PSwitchInput.java)
In the band: dataSource Thing, updateType i32, lethalType i32, includeRigidConnectors b,
lethalActivationFrame i32, lethalInverted b, hideInPlayMode b, disableLethalAudio b
(v>0x38f), sackbotBehavior ref(NpcBehavior), sackbotObjectColorIndex s32,
behavior i32, effectDestroy i32 (v>0x308), playerMode i8 (v>0x3ec).
The source comments give the updateType values (ESwitchInputUpdateType): ACTIVATION 0,
DESTROY 1, LETHAL 2, REGISTER_COUNT 3, PHYSICS_TWEAK_X 4, _Y 5, _ROTATION 6,
CIRCUITBOARD 7, SIGNED_ACTIVATION 8, TIMER_ACTIVATION 9, SACKBOT_BEHAVIOR 10,
SACKBOT_WAYPOINT 11, _LOOKAT 12, _JUMP 13, _GRAB 14, _LAYERCHANGE 15.
SI: this is the "input adapter" on non-logic targets such as destroyers, materials made
lethal and sackbot inputs. dataSource is the Thing whose PSwitch output feeds it.

### A6. Other parts
- PSwitchKey (tag/key): colorIndex s32, name wstr, hideInPlayMode. At v>=0x3ed this
  becomes a flags byte (0x80 hide, 0x40 isDummy). Sensors match tags by colour plus name (SI).
- PTagSynchroniser: type s32, colorIndex s32, name wstr, radius, minRadius, scaleX,
  scaleY f32, useLayers b, visible b. It looks like a tag/zone volume (SI on meaning).
- PMicrochip: circuitBoardThing, hideInPlayMode, wiresVisible, lastTouched s32, offset v4,
  name wstr, components array(CompactComponent{thing, x, y, angle, scaleX, scaleY, flipped}),
  circuitBoardSizeX/Y f32 (v>=0x34d). Logic inside a chip is ordinary Things parented to
  the board. `components` stores their board-space layout. The chip has its own PSwitch
  of type MICROCHIP whose input acts as the enable (SI, matches in-game "chip activation").
  broadcastMicroChipEntries in PWorld is subVersion>=0x2f, so it is LBP3 only. Not in band.
- PSequencer: tempo, swing, echoFeedback, echoTime, echoMix f32, reverbSettings i32
  (v>0x370), loop b (v>0x36e), startPoint f32, numChannels i32, volume f32[6],
  playHead f32 and isPlaying b (v>0x369), musicSequencer b (v>0x36c), behavior i32
  (v>0x371), triggerPlayer i32 and previewThing (v>0x3a0). The board x position of a
  component is its time slot. Toolkit's tools/sequencerdump derives MIDI from instrument
  children's relative x (https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/tools/sequencerdump/src/main/java/SequencerDump.java).
- PTrigger: triggerType u8, inThings thingarray (runtime occupants), radiusMultiplier
  (default 600), allZLayers, hysteresisMultiplier, enabled, zOffset (v>=0x322).
- PEffector (wind, water and gravity zones): posVel v3, angVel, viscosity, density, gravity
  v3 (default (0,-2.7,0)), pushBack, swimmable, viscosityCheap, modScale.
- PEmitter: frequency/phase/lifetime ints (frames, SI), plan, maxEmitted, maxEmittedAtOnce,
  **speedScale: SwitchSignal** (the emitter keeps its own input signal), behavior i32,
  emitScale, linearVel, currentEmitted, parent-relative offset/rotation, z fields, effectCreate/Destroy.
- PCheckpoint: activeFlags (bool before subVersion 0x102), activationFrame i32, spawnsLeft,
  maxSpawnsLeft, instanceInfiniteSpawns, spawningList thingarray, spawningDelay,
  lifeMultiplier, teamFilter. The entrance/start point is the same part (SI).
  PWorld has `useEvenNewerCheckpointCode`.
- PGameplayData: fluffCost s32, eggLink ref(EggLink), keyLink ref(SlotID), gameplayType
  enum32 (v>=0x2d1, after keyLink while subVersion<0xef).
- PWorld: frame s32, simFrame s32, frameLevelStarted, scoreLocked, trophy thresholds
  (scoreRequiredForBronze/Silver/Gold).

### A7. PScript and ScriptInstance (T+structs/things/parts/PScript.java,
components/script/ScriptInstance.java, InstanceLayout.java, FieldLayoutDetails.java,
ScriptObjectUID.java)
- PScript = ScriptInstance { script: ResourceDescriptor(SCRIPT), hasLayout b,
  layout: reference(InstanceLayout), reflectDivergent b, then the member values }.
- InstanceLayout = array(FieldLayoutDetails) + instanceSize s32.
- FieldLayoutDetails: name str, then **for v>=0x3d9: modifiers i16, machineType u8,
  fishType u8, dimensionCount i8, arrayBaseMachineType u8, instanceOffset s32**; before
  0x3d9 each of these is a 32-bit enum/int. This packing switch falls **inside the LBP2 band**.
- Member values follow in layout order. DIVERGENT fields are skipped unless
  reflectDivergent is set. Encoding by machineType: BOOL b, CHAR i16, S32 i32 (fishType GUID
  means a resource GUID), F32, V4, M44, OBJECT_REF -> ScriptObjectUID, SAFE_PTR -> Thing ref.
- ScriptObjectUID: type enum32 (ScriptObjectType: NULL 0, ARRAY_BOOL 1, ..., ARRAY_S32 3,
  ARRAY_F32 4, ARRAY_VECTOR4 5, ARRAY_SAFE_PTR 10, ARRAY_OBJECT_REF 11, RESOURCE 12,
  INSTANCE 13, STRINGW 14, AUDIOHANDLE 15, STRINGA 16, POPPET 17, ...), a reference id i32
  (shared objects are deduplicated through a visited table), then the payload.
- MachineType: VOID 0, BOOL 1, CHAR 2, S32 3, F32 4, V4 5, M44 6, (STRING 7 dep), RAW_PTR 8,
  REF_PTR 9, SAFE_PTR 0xa, OBJECT_REF 0xb, S64 0xc, F64 0xd.
  BuiltinType (fish types): VOID 0, BOOL 1, CHAR 2, S32 3, F32 4, V2 5, V3 6, V4 7, M44 8,
  GUID 9, S64 0xa, F64 0xb.
  ModifierType bits: STATIC 0, NATIVE 1, EPHEMERAL 2, PINNED 3, CONST 4, PUBLIC 5,
  PROTECTED 6, PRIVATE 7, PROPERTY 8, ABSTRACT 9, plus DIVERGENT and others (see enum).
- The instance layout is stored inline, so a level is self-describing for script *state*
  without the RScript (also noted in docs/level-research.md:98).

### A8. RScript ("FishFingers" .ff) (T+resources/RScript.java `serialize`)
Order: [v<=0x360: upToDateScript descriptor], className str, superClassScript
descriptor(SCRIPT), modifiers (**i16 if v>=0x3d9 else i32**), typeReferences,
fieldReferences, functionReferences, fieldDefinitions, propertyDefinitions,
functionDefinitions, sharedArguments (TypeOffset{typeRefIdx, offset}),
sharedBytecode (i32 count + u64 per instruction), sharedLineNos (i32 count + i16 each),
sharedLocalVariables, stringA table (index intvector + byte blob, NUL-separated ASCII),
stringW table (index intvector + i32 char count + UTF-16BE blob), constantTableS64
(v>=0x30c), constantTable f32 (i32 count + f32s), dependingGUIDs (v>=0x1ec, i32 + GUIDs).
Format notes: v<=0x1eb ("inline script format") is not supported. SHARED_SCRIPT_DATA is 0x1ec.
- Rows (T+structs/script/): TypeReferenceRow {machineType u8, fishType u8,
  dimensionCount i8, arrayBaseMachineType u8, script descriptor, typeNameStringIdx};
  FieldReferenceRow / FunctionReferenceRow {typeReferenceIdx, nameStringIdx} (these resolve
  across scripts by name); FieldDefinitionRow {modifiers, typeRefIdx, nameIdx};
  PropertyDefinitionRow {+getFunctionIdx, setFunctionIdx}; FunctionDefinitionRow
  {modifiers, typeRefIdx, nameIdx, arguments/bytecode/lineNos/locals [begin,end) ranges
  (**u16 if v>=0x3d9 else i32**), stackSize i32}; LocalVariableDefinitionRow
  {modifiers, typeRefIdx, nameIdx, offset}.
- Instruction word (u64, T+structs/script/instructions/*): bits 0-7 opcode (InstructionType),
  bits 8-15 machine type (CALL, ARG, GET/SET_MEMBER), bits 16-31 dest/source register,
  bits 32-47 operand a / arg index / call index / base, bits 48-63 operand b / field index.
  Branch: offset s32 in bits 32-63, source register in bits 16-31. LoadConst: dest in bits
  16-31, constant/string index u32 in bits 32-63. SI: registers are byte offsets into a
  per-call frame of `stackSize` bytes (register VM, not a stack VM).
- Opcodes 0x00..0xCA (203; T+enums/InstructionType.java). Groups: NOP; LCb/c/i/f/sw/
  NULLsp/v4/NULLo/sa/s64 (load const); MOV*/NEG/ABS/SQRT/SIN/COS/TAN/INC/DEC/LOG_NEG
  (unary); ADD/SUB/MUL/DIV/MOD/MIN/MAX/shifts/bitops/compare for i, f, c, b, v4, s64,
  DOT2/3/4, CROSS3, MULm44 (binary); GET/SET_V4_{X,Y,Z,W,LEN*}, GET/SET_M44_* (built-in
  members); GET/SET_SP_MEMBER, GET/SET_RP_MEMBER, GET_SP_NATIVE_MEMBER, GET/SET_OBJ_MEMBER;
  GET/SET_ELEMENT, NEW_ARRAY, ARRAY_INSERT/APPEND/ERASE/FIND/CLEAR/RESIZE/RESERVE,
  GET_ARRAY_LEN; WRITE (debug print), ARG, CALL, CALLVo, CALLVsp (virtual), RETURN;
  B, BEZ, BNEZ; CASTsp, CASTo, INT*/FLOAT*/BOOL* conversions; NEW_OBJECT; ASSERT.
  `InstructionType.getInstructionClass` maps opcodes to 19 decoder classes.
- Toolkit has **no disassembler or interpreter**. It only (de)serializes.

### A9. Which gadgets are scripted (T+resources/RLevel.java `onStartSave`, `RLevel()`)
- When Toolkit down-converts to v<=0x2c3 it *re-attaches* these scripts: switchBase 42511,
  tweakJoint 19749, checkpoint 11757, triggerCollect 11538, tweakEgg 27432, enemy 31617,
  triggerParent 17789, tweakParent 39027, cameraZone 26580, tweakKey 52759,
  triggerCollectKey 17022, enemyWard 43463, soundObject 31319, emitter 27150,
  spriteLight 46946. The source comment reads "...when they overhauled the switch system,
  LBP2 already removes them". Reading (SI, strong): **in LBP1 switches, checkpoints,
  emitters, keys and enemies were partly fish-scripted. In LBP2 (0x2c4+) this switch and logic
  behaviour is native C++.** Their PScript parts are absent in LBP2 levels.
- Scripts that LBP2 levels still carry (Toolkit edits them, so they exist at >0x2c3):
  scoreboard 11599, gun 66090 (Plan, PlanIcon fields), speech bubble / magic mouth 18420
  (DialogueListFile, DialogueListGibberishFile), trigger music 18256 (needs PTrigger),
  default world-thing script 19744 (RLevel constructor), and "switchBase" 42511 still
  exists in LBP3 (the BasicIcons field). UNKNOWN: the full list for LBP2 story levels.
  Measure it on PC (see D).

## B. Behaviour sources (web)
- Fish naming: source `.cod`, compiler "fishmonger", compiled `.ff` "FishFingers" (BeyondLBP
  tweet, https://x.com/BeyondLBP/status/704157524312899584. DOC-ish, secondary; robots
  blocked the fetch, so it is quoted from the search snippet).
- **zishzingers** (Beyley, Zig, https://github.com/Beyley/zishzingers, HEAD
  8b5d641201167b13dd97bcac60eb04041aec157c, archived 2025-05-25, **NO LICENCE, so behaviour
  reference only. Do not copy or translate it.** It has a disassembler (`disasm`), a library
  generator from extracted assets plus a MAP (`generate_library`) and an "A#" compiler with
  LBP2/Deploy/Vita targets. The samples show the binding model: a class `extends Thing`, with
  engine callbacks such as `OnCreate()`, and engine services called as natives
  (e.g. `GameMessagingManager.Messaging_NewMessage(...)`, `CreateInventoryCollection`).
  `TweakEgg(g27432) extends TriggerCollectItem` shows that the script GUID is the class identity
  and that inheritance crosses resources (superClassScript). Its MMTypes.zig describes
  GET_SP_NATIVE_MEMBER as reading a native member of the Thing in a register. The "extended
  runtime" codegen emits raw PPC native invokes (TOC index). That confirms NATIVE functions
  are engine C++ bound by name. SI: any interpreter needs a large native API surface
  (Thing, Poppet, Inventory, LAMS, GameMessaging, Audio, ...).
- Logic semantics (fan docs. GameFAQs "Detailed logic summary", LBP2 board
  https://gamefaqs.gamespot.com/boards/954843-littlebigplanet-2/58078244 and LBP3 board
  70701406. User posts with no stated licence. Facts only, DOC-secondary):
  - Signal range is -100%..+100%. Wires can carry analog values while looking unlit.
  - Digital: AND = all on, OR = any on, XOR = exactly one on, NOT inverts. Every gate has
    "Invert Output" (NAND/NOR/XNOR). This matches PSwitch.inverted.
  - Analog: AND outputs **min** of its inputs, OR outputs **max**. XOR is "max plus binary inputs"
    (details unclear, UNKNOWN). NOT passes the value through (inverted if set), and its digital
    output is on only when the analog input is at max.
  - Toggle, Randomiser and Selector take only 0/100% (digital). Counter takes digital count
    input and outputs current/target as a ratio. Timer takes signed speed (SPEED_SCALE) and
    outputs elapsed/target.
  - Combiner outputs the magnitude of (input+ - input-). Sign split routes |x| to the +
    or - output, and the output is always positive. Battery is a constant -100..+100 (ALWAYS_ON).
  - Player and tag sensors give distance-based analog closeness. Score, projectile and paint
    sensors give completion %. Grab, impact, water and sticker sensors are digital only.
- Mirror wikis (littlebigplanet.miraheze.org AND_Gate/OR_Gate, fandom "Analog_value",
  "Logic"; fandom content is CC BY-SA) returned 402/403 to the fetcher. Read them manually.
- **Not found in public sources: per-gate frame delay, evaluation order, multi-wire
  fan-in rules.** All UNKNOWN. SI hints from the format: `PSwitch.updateFrame` (removed
  0x369) and `PSwitchInput.updateFrame` (removed 0x2c4) are per-node "last evaluated
  frame" stamps. That suggests lazy, pull-based evaluation memoised per frame, where a cycle
  sees the previous frame's value. Treat this as a hypothesis to measure.
- Tick rate: PWorld.frame/simFrame are integer frame counters, and timers, randomiser and
  hold times are ints. Commonly cited as 30 Hz sim (SI, confirm on PC from known timer
  durations in story levels, e.g. timerCount versus the UI seconds).

## C. Rust/Bevy 0.19 design (proposal)
C1. Graph model (independent design, no game code)
- `LogicNode` component on each Thing with a PSwitch: `kind: SwitchType`, `params`
  (inverted, behavior, requireAll, random*, timer targets...), `state` (timerCount,
  counter, toggle bit, rng state), `outputs: SmallVec<[Signal; N]>`.
- `Signal { analog: f32, ternary: i8, player: i8 }`. This mirrors SwitchSignal, so loaded
  saves round-trip.
- `WireGraph` resource built at level load from SwitchOutput.targetList: edges
  (src Entity, out_port) -> (dst Entity, in_port). Also build a reverse index (the receiver
  never stores its inputs). Rebuild only on edit/spawn/despawn and mark it dirty through
  change detection.
- `SignalBuffers` resource: two `Vec<Signal>` indexed by a dense slot id (prev, cur),
  swapped each tick. This is double-buffered: every node reads `prev` for back edges and
  `cur` for forward edges.
- Order: topologically sort the DAG part (Tarjan SCCs). Inside an SCC read `prev`, giving a
  one-frame delay on feedback only. Ties are broken by stable Thing UID, which makes the
  run deterministic and independent of ECS iteration order. Make "one frame of delay per
  gate" or "zero-delay combinational" a config flag until it is measured (see D7).
- Schedule: run in `FixedUpdate` with `Time::<Fixed>::from_hz(30.0)` (pending D7). Set
  order: Sensors (read physics/players from last step) -> LogicEval (single-threaded
  exclusive system over the sorted order, no par_iter, for determinism) -> Actuators
  (emitters, movers, PSwitchInput adapters, sackbot inputs, checkpoints) -> Physics.
  Seed one RNG per randomiser from level seed plus UID.
- Microchips: flatten them. A chip is an enable gate on its child subgraph (a disabled chip
  outputs 0 and freezes child state, SI). Keep `components` only for editor layout.
- Sequencer: map board x to time using tempo and playHead. Logic sequencers emit timed
  pulses, and music sequencers drive the audio crate.
- Tags: spatial query on PSwitchKey (colour plus name) within radius/angle for tag sensors.
C2. Scripts: options (**user decision. Not decided here**)
- (a) **Bytecode interpreter** running the user's own RScript resources at runtime.
  Pros: fidelity for scoreboard, magic mouth, gun, music and the world script; story-level
  custom scripts work unchanged. Cons: the bytecode is copyrighted game code, *executed*
  rather than shipped. That is different from parsing assets, and AGENTS.md says "product
  contains no original LBP code" (it does not cover runtime execution of user data). Every
  NATIVE function (engine API) still has to be reimplemented by name, which is the larger
  job. Register VM + u64 decode + 203 opcodes is moderate work. Cross-script resolution goes
  through Field/FunctionReferenceRow names. Needs a sandbox: instruction budget per tick,
  bounds checks, no host pointers.
- (b) **Native Rust per gadget**: reimplement behaviour (scoreboard, speech bubble, music
  trigger, gun...) and use PScript member data (inline layout) only as *configuration*
  read by name. Pros: no game code executed and clearer legal ground. Cons: per-script effort,
  and custom or unknown scripts fall back to "unsupported".
- Hybrid: (b) for the core gadgets plus an optional, off-by-default (a) behind a feature flag.
  Either way, parse PScript instance data now (needed for both).
- Legal flag for the user: executing user-supplied original bytecode is not the same act as
  distributing it. Get a decision or advice before building (a).

## D. PC-only checks (need game data, write to local-analysis/)
D1. Count RScript (.ff) resources, revision histogram, and className list per GUID.
D2. Opcode histogram over all LBP2 scripts (which of 0x00..0xCA occur. CALLV*, ASSERT,
    s64 ops) and count of NATIVE functionDefinitions and distinct native names (API size).
D3. Story levels: PScript GUID histogram (which scripts are referenced) and member field names
    per GUID. Confirm whether 42511/11757/27150 etc. are absent in LBP2 story levels.
D4. SwitchType histogram over story levels plus the creator-mode plans (gadgets in popit).
    Count logicType/type mismatches before 0x398.
D5. Revision histogram of levels against split points 0x368/0x369/0x382/0x38f/0x398/0x3a4/
    0x3ad/0x3d9/0x3ec/0x3ee. Make sure PSwitch and FieldLayoutDetails parse cleanly.
D6. Wire stats: outputs per switch, max port index, targets lacking a PSwitch/PSwitchInput,
    cycles (SCC count and size), microchip nesting depth.
D7. Timing ground truth: compare timer/countdown targets and randomiser frame ints with
    in-game seconds (tick rate). Test chains and feedback loops in-game (or via RPCS3
    recording) for per-gate delay.
D8. Ternary values seen in serialized SwitchSignal (confirm -1/0/1) and the player id range.
D9. Which updateType values occur in PSwitchInput across story levels.
