# D107 independent RLevel / Thing reader (L3, L4, L5 subset, Lw)

2026-10-08 America/Chicago. New UI-independent crate `crates/lbp-level` reads a whole
LVLb level on the LBP2 band and returns every Thing with its UID, parent/group/emitter
links, part set and the render-relevant part data. Maps to the items of
[level-research.md](level-research.md) section 6:

| Item | Where | State |
|---|---|---|
| L3 serializer core | `src/serial.rs` | Done: compressed ints (ULEB/zigzag), u64, m44 mask, intvector planes, guid, resource descriptors, reference tag table, depth/tag/Thing/array budgets |
| L4 Thing header + part flags | `src/thing.rs` | Done for heads 0x341..0x3f8, branch 0 |
| L5 render parts | `src/parts.rs` | POS, RENDER_MESH, SHAPE (+Polygon, ContactCache), GENERATED_MESH, GROUP, REF, BODY parsed with summaries |
| L6..Ln remaining parts | `src/parts.rs` | 38 part readers in total; every part used by a local in-band level is covered |
| Lw RLevel + PWorld | `src/level.rs`, `parts.rs` `world_tail` | Done: cross-play hashes, world Thing, full PWorld tail, PlayerRecord, tutorial inventory |

No GUI, physics or rendering. Parsing code depends only on `lbp-resource` (wrapper and
zlib via `binary::payload`) and `serde`. Plans share the Thing layout but are not wired
yet (RPlan `thingData` is `array(Thing)` with its own serializer; next item).

## Provenance

Every rule is CONFIRMED_FROM_COMMUNITY_CODE from ennuo/toolkit
`d6a2ef80b5a09999f08f23a803b516096055e5c9` (local clean checkout under `references/`),
written as independent specification notes in the module comments next to the code; no
Toolkit code was copied, adapted or translated. Classes read: `resources/RLevel.java`,
`structs/things/Thing.java`, `enums/Part.java`, `enums/PartHistory.java`,
`enums/CompressionFlags.java`, `io/serializer/Serializer.java`,
`io/streams/MemoryInputStream.java`, `structs/things/parts/P*.java` for each part below,
and the component structs they call (Polygon, ContactCache, Contact, Decal,
PaintControlPoint, EyetoyData, ColorCorrection, Decoration, CameraNode, LevelSettings,
RecordingPlayer, SwitchOutput, SwitchSignal, SwitchTarget, ScriptInstance,
InstanceLayout, FieldLayoutDetails, ScriptObjectUID, RegionOverride, Primitive,
CompactComponent, EmittedObjectSource, NpcBehavior, ActingData, InputRecording,
NpcJumpSolver, Note, CostumePiece, WhipSim, EggLink, SlotID, KeyLink, EditorSelection,
GlobalSettings, CameraSettings, CutsceneCameraManager, GlobalAudioSettings, MoveCursor,
PlayerRecord, NetworkPlayerID, NetworkOnlineID, InventoryItem, InventoryItemDetails,
CreationHistory, UserCreatedDetails, InventoryItemPhotoData, PhotoMetadata, PhotoUser,
CachedInventoryData). Field lists per part are in the `parts.rs` module comment.

CONFIRMED_FROM_LOCAL_DATA (this milestone): every rule below is exercised by the 236
in-band levels on 01.33 that parse to the last byte and agree with the D106 Toolkit
oracle on every compared field (see Parity). Gates that only differ for heads
0x341..0x37a are not exercised: no local in-band level is below 0x37b.

## Key rules

| Rule | Evidence | Applicability |
|---|---|---|
| Compression flag 1: i32/u32 are ULEB128, s32 is 32-bit zigzag, u64 ULEB; flag clear: fixed BE | COMMUNITY_CODE MemoryInputStream i32/s32/u64; LOCAL_DATA | flags 0 and 7 (only values seen) |
| f32/v2/v3/v4 always plain BE floats; flag 2 changes only intvector/longvector | COMMUNITY_CODE Serializer v3/v4/intvector | all |
| m44: flag 4 -> u16 mask, one f32 per set bit, identity otherwise; translation in elements 12..14 | COMMUNITY_CODE MemoryInputStream m44; LOCAL_DATA (world matrices bit-equal to oracle) | all |
| intvector compressed: count, then if nonzero a byte width and width planes of count bytes | COMMUNITY_CODE Serializer intvector | all |
| resource: flags i32 when head > 0x22e and not a descriptor call; kind byte bit 2 GUID, bit 1 SHA1 | COMMUNITY_CODE Serializer resource | head >= 0x191 |
| reference: i32 tag, 0 null, seen tag = same object, new tag = body inline; one table across classes (incl. ScriptObjectUID) | COMMUNITY_CODE Serializer reference, ScriptObjectUID | all |
| Thing: 0xAA, UID, parent, groupHead, oldEmitter, createdBy, changedBy, planGUID, flags i8, partsRevision s32, part flags u64 | COMMUNITY_CODE Thing.serialize | 0x341..0x3f8, branch 0 |
| Part present if partsRevision >= PartHistory and flag bit set; at partsRevision >= 0x32 indices > 0x28 shift by one, bit 0x29 = deprecated CREATOR_ANIM (i32 must be 0) | COMMUNITY_CODE Part.hasPart/serialize | LBP2 (subVersion 0) |
| RLevel: cross-play SHA1 array only when head > 0x3e6 (strict) | COMMUNITY_CODE RLevel.serialize; LOCAL_DATA counts equal oracle | 0x3e7..0x3f8 |

Strictness conflicts with the community reader (all explicit outcomes, none hit on local
data): ULEB longer than 5 bytes / above 32 bits is Invalid (Toolkit truncates); intvector
byte width above 4 is Unsupported (Toolkit would overflow its shift); a reference tag
reused across classes is Invalid (Toolkit would fail later with a cast); a null
InstanceLayout with the layout flag set is Invalid (Toolkit NPE); unknown script machine
or object types stop as Unsupported (Toolkit throws). Branch 0x4c44 (LEERDAMMER) and
0x4431 (DOUBLE11) are refused: they add branch gates (PCreature head piece, PWorld water,
Vita fields) that are not implemented. Toolkit's post-load `Thing.onFinishSave` drops an
empty SCRIPT_NAME; this reader keeps the serialized part and records `(bytes, name_len)`.

Budgets (`Limits`): depth 256 inline bodies, 2,000,000 tags, 500,000 Things, 4,000,000
elements per array; every count is also checked against remaining bytes times the
minimum element size before allocating. Stored input 32 MiB, expanded 128 MiB. The
reader recurses natively, so callers need a large stack for the default depth (examples
use a 512 MiB worker thread; deepest local level: 162). Cancellation is checked per Thing.

## Outputs

`lbp_level::read(bytes, limits, cancel) -> LevelReport`: revision, expanded/consumed
bytes, `trailing_bytes` after a complete read, cross-play hash count, tags, max depth,
`world_things_walked`, detached shapes, `stop: Option<Stop>` (Unsupported,
UnsupportedPart{part, head, parts_revision, offset}, Invalid, Budget, Cancelled) and
`things: Vec<ThingRecord>` (tag order). ThingRecord: tag, offsets, UID, parent / group
head / old emitter (Thing indices), createdBy/changedBy, planGUID, flags,
partsRevision, raw part flags, unmapped flag bits, present parts, per-part read state
(parsed/null/shared/deprecated_zero) and `data`: PPos (bone-of, anim hash, world m44),
PRenderMesh (mesh/anim descriptors, bone Things, anim pos/speed/loop, editor colour,
shadows, visibility, poppet scale), PShape (vertices, requiresZ, loops, material, old
material, thickness, mass depth, colour, brightness, bevel size, COM, behavior, flags,
contacts), PGeneratedMesh (gfx material, bevel, uv offset, plan GUID, play visibility),
PGroup, PRef plan, Decorations (render mesh + offset), PWorld things list and UID counter,
script name lengths. Back-references to a PShape/PRenderMesh body resolve to the same data.

## Census (01.33)

`level_census` over every magic-confirmed level MAP row of
`local-analysis/d68-inventory-0133-verified.json`, unit distinct level content hash:

| Outcome | Levels |
|---|---:|
| complete (byte-exact, 0 trailing bytes) | 236 |
| out of band: head below 0x341 (LBP1-era layouts, 0x15e..0x338) | 122 |
| out of band: branch 0x4c44 (heads below 0x341) | 20 |
| wrapper refused: DOUBLE11 (Vita) payload | 20 |
| total | 398 |

All 236 in-band levels (29 heads, 0x37b..0x3f7) read to the end: 542,452 Things, all
complete; 318,657 with PPos, 68,618 with PRenderMesh, 166,801 with PShape; max inline
depth 162. No first-unsupported part remains. Every in-band level has unique UIDs and
max UID within PWorld thingUIDCounter. 37 part types occur (Things/levels): POS
318,657/236, SHAPE 166,801/221 (flags), SWITCH 141,572/219, GENERATED_MESH 121,382/220,
GROUP 86,982/235, BODY 82,355/236, RENDER_MESH 68,618/236, PHYSICS_TWEAK, STICKERS,
MICROCHIP, DECORATIONS, TRIGGER, AUDIO_WORLD, SWITCH_KEY, GAMEPLAY_DATA, SWITCH_INPUT,
JOINT, SCRIPT, SPRITE_LIGHT, MATERIAL_TWEAK, CAMERA_TWEAK, EMITTER, ENEMY, INSTRUMENT,
SEQUENCER, COSTUME, CREATURE, NPC, CHECKPOINT, CONTROLINATOR, EFFECTOR, LEVEL_SETTINGS,
WORLD, SCRIPT_NAME, TAG_SYNCHRONIZER, WORMHOLE, MATERIAL_OVERRIDE. REF has a reader but
no local level uses it (backdrop plans arrive through PWorld plan descriptors).
Repeat run byte-identical (464,087 B).

## Parity with the D106 Toolkit oracle

`level_parity` against `d106-level-oracle-all.json` and `d106-level-scene-run1.json`
(pin checked). Summary scope: oracle `parsed_exact` levels at head 0x341..0x3f8,
branch 0 = 236 levels (162 out of scope: older heads, LEERDAMMER, DOUBLE11 which the
oracle partly parses). Result: 236/236 levels equal on thing count, consumed / expanded /
trailing bytes, cross-play hash count, reference tag count, world things listed, part
histogram, POS/RENDER_MESH/SHAPE/GENERATED_MESH counts and pairings, vertex totals,
render-mesh / shape-material / gfx-material / bevel / ref-plan descriptor tallies,
bone-count and polygon-size histograms. Scene scope: 5 oracle scene levels, 4,180
Things matched by reference tag: 4,180/4,180 equal on UID, parent/group/emitter tags,
flags, plan GUID, part list, PPos (anim hash, bone-of, world m44 bit-exact f32),
PRenderMesh (mesh, anim, bones, editor colour, visibility), PShape (vertices bit-exact,
loops, material, old material, thickness, bevel, brightness, colour, flags, behavior),
PGeneratedMesh (gfx material, bevel, plan GUID, uv offset).

Two oracle normalizations are applied on our side only in the parity example: the part
histogram/list counts non-null part objects (null references and deprecated slots are
empty Java slots), and drops SCRIPT_NAME with an empty name (Toolkit onFinishSave). A
first parity run without shared-body resolution found 862 scene Things whose PShape was
first read through another shape's ContactCache; the reader now resolves such
back-references to the stored body (fixed before the recorded run).

## Verification

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check
cargo test -q -p lbp-level                                   # 18 tests (8 L3, 10 L4/L5/RLevel)
cargo clippy -q -p lbp-level --all-targets -- -D warnings
cargo build -q --release -p lbp-level --example level_census --example level_parity
target/release/examples/level_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d107-level-census-0133.json
target/release/examples/level_parity local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d106-level-oracle-all.json local-analysis/d106-level-scene-run1.json local-analysis/d107-level-parity-0133.json
```

Synthetic tests (authored recipes, no game bytes): integers fixed/ULEB/zigzag with
overflow and truncation, m44 masked/full, intvector planar/plain/width/count bounds,
count budgets, descriptors with/without flags word and the 0x22e gate, tag
null/new/seen/class conflict/budget, depth budget and cancel; part-flag mapping with the
CREATOR_ANIM shift and history gates; a full synthetic level in flags 0 and 7 (world
Thing, inline parent, back-references, POS/RENDER_MESH/SHAPE, deprecated part, full
PWorld tail with an EditorSelection, RLevel tail with one modern InventoryItem) read
byte-exact; every truncation of it is Invalid without panic; unsupported part reports
name/head/partsRevision/offset; null/shared parts; contact-first shared PShape; script
name length; header violations; inline parent chain hits the depth budget; Thing budget.

## Counts and limits

Counting unit: distinct level content hash on the 01.33 overlay. Levels fully parsed:
0 -> 236 of 398 (baseline: no level reader). Assets: +0 newly viewable, +0 newly Complete
(no rendering this milestone). Not tested: heads 0x341..0x37a (no local level), branches
other than 0, plans (`thingData`), multi-bone placement, any visual correctness; part
readers that exist only for completeness are bounded by the same parity run only where
local levels use them (REF unused). Script field values are skipped, not interpreted.

Next: wire the Thing reader to RPlan `thingData` (plans share the layout), then feed the
Bevy level-scene item (L2/Lb) from `LevelReport` instead of the oracle JSON.
