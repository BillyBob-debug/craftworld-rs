# U4: slot list, pins, palette and packs readers

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U4). **Not verified against disc data**; the
Integrator assigns the D-number, runs the PC checks below and updates STATUS/MODLOG.

## Question
The Pod, the story planets, the moon and the Popit show level badges, pins and item
collections. These live in four resource types: SLOT_LIST (`SLTb`, type 29), PINS
(`PINb`, 47), PALETTE (`PALb`, 43) and PACKS (`PCKb`, 27). Can bounded readers on the
L3 serializer core (`lbp_level::serial`) read them for the LBP2 band?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

All rows are CONFIRMED_FROM_COMMUNITY_CODE (read at the pin; no code copied). None is
CONFIRMED_FROM_LOCAL_DATA until the PC census below runs.

| Rule | Source |
|---|---|
| Type ids, magics `SLT`/`PIN`/`PAL`/`PCK` + `b` | `enums/ResourceType.java` |
| RSlotList: array(Slot), then `fromProductionBuild` bool at head >= 0x3b6 | `resources/RSlotList.java`, `Revisions.PRODUCTION_BUILD` |
| Slot field order and head gates (0x13b author name, 0x183 translation tag, 0x134 group, > 0x237 shareable + background GUID, >= 0x333 planet decorations, < 0x188 pad byte, > 0x1de level type else SideMission bool, 0x1ae..0x1b8 pad byte, 0x1b9..0x36b progression state, stop at <= 0x2c3, >= 0x33c labels, >= 0x2ea / 0x2f4 collectabubbles, >= 0x352 sub-level, stop below 0x3d0, then min/max players, move, >= 0x3e9 cross-compatible, >= 0x3d1 show on planet, >= 0x3d2 lives override) | `structs/slot/Slot.java` `serialize` |
| SlotID = enum32 type + u32 number; Label = i32 key + i32 order; Collectabubble = plan descriptor + i32 count | `structs/slot/{SlotID,Label,Collectabubble}.java` |
| NetworkOnlineID: 16 bytes, i8, 3 bytes; below head 0x234 each byte run has an i32 length prefix (16, 3) | `structs/profile/NetworkOnlineID.java` |
| Pin: u32 id, progressType, category, title key, description key; icon descriptor; u32 initial, target; i8 trophy; fixed i16 flags; i8 LBP1 trophy at >= 0x3f7 | `structs/profile/Pin.java` |
| RPalette: i32 count of plan resources **with** flags word, i32 location, i32 description, then at >= 0x323 i32 count of converted plan **descriptors** (no flags word) | `resources/RPalette.java`, `Revisions.PALETTE_CONVERTED_PLANS` |
| Pack: enum32 contents type, mesh descriptor, Slot, str content id, s64 timestamp (zigzag when compressed); Toolkit sorts packs by unsigned timestamp after reading | `structs/slot/Pack.java`, `resources/RPacks.java` |
| `str` = s32 byte length + bytes (Java default charset), `wstr` = s32 unit count + UTF-16BE; NULs removed | `io/streams/MemoryInputStream.java` |
| enum32 reads i32, not zigzag; an unknown value becomes null | `MemoryInputStream.enum32` |
| Enum names: SlotType 0..17, LevelType 0..8, ContentsType 0,1,2,3,5 | `enums/{SlotType,LevelType,ContentsType}.java` |

Vita (DOUBLE11) and LBP3 add Slot fields behind branch and sub-revision gates
(`ADVENTURE` 0x145, `SLOT_ENFORCE_MINMAX` 0x215, D1_* gates) and Pack's
`crossBuyCompatible`. Those are refused as Unsupported here, not guessed.

## Implementation (`crates/lbp-level/src/slots.rs`)
- `slots::read(bytes, limits, cancel)` picks the kind from the magic, expands the payload
  with `lbp_resource::binary::payload` and calls `read_payload(kind, data, rev, ...)`.
- `UiDataReport { revision, result: Result<UiData, Stop>, checks }`, where `UiData` is
  `SlotList`, `Pins`, `Palette` or `Packs`. Fields absent at the file's head are `None`.
- Bounds: every count goes through `Reader::count` (array budget and remaining bytes),
  negative string lengths are Invalid, input 64 MiB and expanded 256 MiB budgets, cancel
  checks per slot/pack. Branch != 0, LBP3 heads, heads above `LBP2_MAX` and below 0x191
  are Unsupported.
- Counted, not fatal: unknown SlotType/LevelType/ContentsType values, invalid UTF-16
  (replaced with U+FFFD like Java), trailing payload bytes.
- Packs are kept in file order; the Toolkit sort is presentation.
- Not wired into the viewer yet; U3 (string browser) and the Pod/Popit screens use it.

## Tests (synthetic recipes only, `crates/lbp-level/tests/slots.rs`)
`u4_slot_list_reads_lbp2_slot_fields` (flags 0 and 7, head 0x3f8),
`u4_slot_gates_stop_early_at_older_heads` (head 0x2c3), `u4_pins_palette_and_packs`,
`u4_untrusted_input_stops_explicitly`.

Cloud (Linux): `cargo fmt --all --check` ok; `cargo clippy -q -p lbp-level
--all-targets -- -D warnings` clean; `cargo test -q -p lbp-level` all pass (4 new).

## PC verification (Integrator)
1. `cargo build -q --release -p lbp-level --example ui_data_census`, then
   `target/release/examples/ui_data_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/u4-ui-data-census-0133.json`
   (same snapshot and game-dir pair as the D107 level census). Report files per
   type, status counts, stop counts, heads, slot totals (named, with translation tag)
   and SlotType counts.
2. Spot-check 3 slot lists (one story planet, the Pod list if present, one DLC) and 1 pins
   file against Toolkit's JSON export of the same resource: slot names, types, root GUIDs,
   pin ids and title keys equal.
3. Any `stopped` row: record the stop label and head; do not patch around it without the
   Toolkit field that explains it.

## Follow-up after D123 (2026-10-08)
The PC census read 15 of 16 files complete. The 16th, `gamedata/scratch/testrnp.slt`
(GUID 000056dc), is a slot list in Toolkit's text serialization (`SLT` type, marker
not `b`): a developer scratch file. `read` now reports it as `Unsupported("text
serialization (not binary)")` and the census counts it as `excluded_text_form`, not as a
failure. Text-form resources stay out of scope.

## Expected effect / not tested
- Viewable/Complete: +0 (data only, no assets). New count for the UI track: slot lists,
  pins, palettes and packs read complete, unit distinct content hash, scope 01.33 overlay.
- Not tested: real game files, heads below 0x2c4 except the synthetic 0x2c3 recipe, Vita
  and LBP3 files (refused).
