# U5: InventoryItemDetails values and the RPlan details tail

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U5). **Not verified against disc data**; the
Integrator assigns the D-number, runs the PC checks below and updates STATUS/MODLOG.

## Question
Every Popit item card shows a plan's InventoryItemDetails: icon, name and description
keys, type (sticker, material, gadget...), Popit location/category, creator, photo or
EyeToy data. D33 (`lbp-resource::plan::inventory`) reads only the scalar prefix, and the
level reader walks the same layout without keeping values. Can one bounded reader return
every value for every plan on 01.33, with icons resolved in the catalog?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| Modern layout (head > 0x37c) field order, reference fields (CreationHistory, UserCreatedDetails, photo, EyeToy, NetworkPlayerID) | `structs/inventory/InventoryItemDetails.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Middle layout (0x233..=0x37c): fixed-width words, pad above 0x234, flags above 0x334, keys above 0x2ba else translation tag, structs for creator/user details/history, pad byte above 0x358 | same | CONFIRMED_FROM_COMMUNITY_CODE |
| CreationHistory: i32 count, then fixed 20-byte names above 0x37c, else wide strings | `CreationHistory.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Photo data (icon, sticker, PhotoMetadata, painting above 0x3c7); PhotoMetadata (photo, SlotID, level name, SHA1, s64, PhotoUser array); EyeToy (two resources with flags word, m44, ColorCorrection 6 f32, outline above 0x39f) | `InventoryItemPhotoData`, `PhotoMetadata`, `PhotoUser`, `EyetoyData`, `ColorCorrection` | CONFIRMED_FROM_COMMUNITY_CODE |
| NetworkPlayerID: NetworkOnlineID (16 + 1 + 3) then 8 + 8 bytes; i32 length prefixes below 0x234 | `structs/profile/NetworkPlayerID.java`, `NetworkOnlineID.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| RPlan: i32 plan revision (ignored), bytearray thingData, details from 0x197 (`PLAN_DETAILS`), then location/category u32 keys from 0x2bb (`LAMS_KEYS`) else two `str` tags | `resources/RPlan.java`, `enums/Revisions.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Type bits per game: LBP1 at head <= 0x332, else LBP2 (bit 12 PAINT/SEQUENCER, 17 FLOOD_FILL/EDIT_MODE_TOOL, 18 STICKER_TOOL/POD_TOOL_LBP2, 31 POD_TOOL_LBP1/EARTH_TOOL, ...) | `enums/InventoryObjectType.java`, `GameVersion.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| dateAdded kept as the stored s64; not interpreted as a date (D33 noted a sign conflict for high-bit values) | `docs/plan-inventory-prefix.md` | STRONG_INFERENCE |

## Implementation (`crates/lbp-level/src/item_details.rs`)
- `item_details(&mut Reader)` → `ItemDetails` for either layout. It is public, so the
  level reader can switch to it later. `parts.rs` is a level-batch file and was not
  touched.
- `read_plan_details(bytes, ...)` / `read_plan_details_payload` → `PlanDetailsReport
  { revision, result: Result<PlanDetails, Stop>, trailing_bytes }`. It skips thingData
  (D119 reads that), then reads the details and the location/category `Placement`.
- Reference fields are `Ref::{Null, Value, Shared}`, where Shared is a back-reference
  to an object read earlier.
- `type_names(bits, head)` decodes the type bits with Toolkit's game-version filter.
- Bounds:
  - Counts go through `Reader::count`.
  - Negative sizes and lengths are Invalid. NetworkPlayerID length prefixes must match.
  - Reference nesting goes through `enter`/`leave`.
  - Input is capped at 16 MiB and the expanded payload at 64 MiB.
  - Heads below 0x197 (no plan details), branches other than 0, LBP3 heads and heads above `LBP2_MAX` are Unsupported.

## Tests (synthetic recipes only, `crates/lbp-level/tests/item_details.rs`)
- `u5_modern_details_values_and_plan_keys` (flags 0 and 7, every reference present)
- `u5_middle_layout_fixed_fields` (head 0x350)
- `u5_type_bits_follow_the_game_version`
- `u5_untrusted_input_stops_explicitly`: every truncation is Invalid; Unsupported heads; no details below 0x197; a tag reused across classes is Invalid.

Cloud results (Linux, Rust 1.99.0):
- `cargo fmt --all --check`: ok.
- `cargo +1.99.0 clippy -q -p lbp-level --all-targets -- -D warnings`: clean.
- `cargo test -q -p lbp-level`: all pass, 4 of them new.

## PC verification (Integrator)
1. Build: `cargo build -q --release -p lbp-level --example plan_details_census`
2. Run it:
   `target/release/examples/plan_details_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/u5-plan-details-census-0133.json`
3. Report these figures from the output:
   - plans and status counts (complete / stopped / no_details), with heads and stop labels;
   - `detail_counts`, especially `with_icon` vs `icon_resolved_in_catalog`;
   - `type_counts`.
4. Cross-check against D33: for every plan both read, the scalar prefix should be equal (date bits, SlotID, sound, colour, type, subType, keys).
5. Toolkit spot-check on 3 plans (one material, one sticker, one with a creator): icon, title key, type, location and category should be equal.
6. Not runnable yet: plans inside the encrypted `patch.edat` (D126 found the 01.33 scripts there). Count them only once decryption is approved.

## Expected effect / not tested
- Viewable/Complete: +0 (data only).
- New count for the Popit track: plans with full details read and icon resolved. Unit: distinct content hash. Scope: 01.33 overlay outside `patch.edat`.
- Not tested: real plans, Vita/LBP3.
- Next: U8 (Popit-style inventory browser) needs U5 plus U1/U6. U7 (story slot browser) needs U1, U4 and U6, which are all merged.

## U5b follow-up (2026-10-08): early layout and wrapper errors
D127 on 01.33: 11,936 of 12,245 plans complete, 225 stopped, 84 wrapper_error. The PC
split them (D127 verification): 106 stopped plans are branch 0 at heads 0x19f..0x232
(early layout), 119 are Leerdammer (branch 0x4c44) at head 0x272, refused by the band
check; all 84 wrapper errors are `Unsupported("DOUBLE11 branch payload")` (Vita branch,
excluded by design).

- Early layout, 0x197 <= head < 0x233, from `InventoryItemDetails.serialize` (`head <
  0x233` branch), CONFIRMED_FROM_COMMUNITY_CODE: `str` translation tag; fixed i32
  location, category, primary indices and a pad word; fixed type bits, subType,
  toolType; icon descriptor; fixed numUses then lastUsed above 0x1c0; fixed highlight
  sound GUID; fixed colour; EyeToy reference; photo reference; SlotID with compressed
  type and fixed number; copyright bool and NetworkPlayerID struct (length-prefixed runs
  below 0x234); UserCreatedDetails above 0x1aa; CreationHistory (i32 count, wide names)
  above 0x1b0; allowEmit above 0x204; fixed s64 dateAdded above 0x221; shareable above
  0x222. The branches Toolkit keeps for heads at or below 0x196 (wide tags, a str sound
  path) are not implemented: plans carry no details there.
- Leerdammer: branch 0x4c44 is accepted only at `LD_HEAD` 0x272 and reads the middle
  layout. Title/description keys (and RPlan placement keys) replace the tag when the
  branch revision is at least `LD_LAMS_KEYS` 0x8 (`InventoryItemDetails` line 351,
  `RPlan.serialize`, `Revision.has`), CONFIRMED_FROM_COMMUNITY_CODE. The other LD
  revisions change CThing/profile/world data, which this reader skips.
  Test: `u5b_leerdammer_branch_keys_follow_its_revision` (branch revisions 0x7, 0x8,
  0x17; flags 0/7; another head on the branch is Unsupported).
- Test: `u5b_early_layout_fixed_fields_and_tags` (heads 0x19f, 0x1b0, 0x1fe, 0x232; flags
  0/1/3; every truncation Invalid). The old "0x232 is refused" assertion now only checks
  that mismatched bytes do not panic.
- The census prints and writes `wrapper_error_counts` (first 4 bytes as ASCII + error)
  so later wrapper errors are classified in the output.

PC check: rerun step 2 with a new output name
(`local-analysis/u5b-plan-details-census-0133.json`); expected stopped 225 -> 0 (or a
labelled remainder) and wrapper_error 84 all DOUBLE11. Toolkit spot-check one plan at
head 0x1fe (tag, icon, type, location/category tags) and one Leerdammer plan (keys).
