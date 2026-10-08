# U7: story slot browser (`--slot-browser`)

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U7; inputs U1, U4, U6). **Not run against disc
data**; the Integrator assigns the D-number, runs the PC checks below and updates
STATUS/MODLOG.

## Question
Can the explorer list every slot in the disc slot lists with its translated name, its
badge icon, lock and group, and show measured counts in the app?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| Name = `translate(tag + "_NAME")` when the tag is non-empty, else the tag; no tag gives the stored name, else "Unnamed Level" | `structs/slot/Slot.java` `toString` | CONFIRMED_FROM_COMMUNITY_CODE |
| Base table g54800 (`E_AMERICAN_TRANS`), patch table g69910 patched over it (patch wins per key) | `tools/unlockall/src/main/java/RestlessPopulator.java`, `resources/RTranslationTable.java` `patch` | CONFIRMED_FROM_COMMUNITY_CODE; file paths UNKNOWN (rows must have a `.trans` path) |
| Description = `translate(tag + "_DESC")`, else the stored description | naming convention only (ui-research.md) | STRONG_INFERENCE; counted separately |
| Group SlotID (DEVELOPER, 0) = no group | `SlotID` default constructor | STRONG_INFERENCE |
| Lock shown = `initiallyLocked` (disc default; live state is profile data) | `Slot.java` | CONFIRMED_FROM_COMMUNITY_CODE (field); profile state not read |
| Slot list layout | U4 (`docs/u4-slot-pin-palette.md`) | as recorded there |

## Implementation
- `crates/lbp-preview/src/story.rs` (no UI dependency): `load_story(source, catalog,
  control)` reads every magic-confirmed `slot_list` row (grouped by content hash, all MAP
  paths kept), the two English tables, and decodes each distinct icon reference through
  `lbp_texture::decode_reader` (budgets: 4096 icons, 512 MiB RGBA). `build_story`,
  `display_name`, `display_description`, `count`, `count_lines` are pure and tested.
- Counts (`StoryCounts`, unit: slot entry in a distinct slot-list content hash): lists
  read, slots, distinct slot ids, slots by type, names by source (Translated / TagOnly /
  Stored / Unnamed), descriptions translated, icons shown / with icon / resolved, distinct
  icons decoded, root levels resolved, locked at start, grouped, group not found.
- `crates/lbp-bevy-viewer/src/slot_browser.rs`: one tab per list, 24 badges per page
  (icon + name, "[locked]" when initially locked), details for the focused slot (type and
  number, list path, tag, group name, sub-level, on-planet, level type, root and icon
  references with resolve/decode state, `location`, description). Count lines on screen.
- `gooey_ui.rs`: `run_gooey(title, screen, images)` now hosts any screen; game images are
  uploaded once and drawn for `ImageRef::Guid`/`Hash` (grey placeholder when absent).
  `--ui-demo` uses the same path. `lbp-gooey::ImageRef` derives `Hash`.
- Not done: planet view (badges at `Slot.location` on the planet mesh), game fonts (U2),
  profile lock state, playing a level from its badge.

## Tests (synthetic only)
`crates/lbp-preview/tests/story.rs`: `u7_names_follow_toolkit_slot_to_string_with_patch_over_base`,
`u7_story_counts_lists_slots_icons_and_groups`. Viewer unit test
`u7_browser_pages_and_focus_details` (paging, focus details, idle frames equal).

Cloud (Linux, Rust 1.99.0): `cargo fmt --all --check` ok; `cargo +1.99.0 clippy -q -p
lbp-preview -p lbp-bevy-viewer -p lbp-gooey --all-targets -- -D warnings` clean;
`cargo test -q -p lbp-preview --test story` 2 pass; `cargo test -q -p lbp-bevy-viewer`
23 pass.

## PC verification (Integrator)
1. Headless counts:
   `target/debug/lbp-bevy-viewer.exe --slot-browser --slot-browser-report --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133"`
   Report every printed line: lists read, slots, names by source, icons shown/with
   icon, roots resolved, both table states, per-list status, icon failure reasons.
2. Tables: both g54800 and g69910 should be `Loaded`. If either is `Missing`/`Failed`,
   record the detail line; names then fall back to tags (TagOnly count).
3. Visual QA on the second monitor: `--slot-browser ... --qa-capture local-analysis/u7-slot-browser-0133.png`.
   Check one story list: names readable English, icons are badges (not garbage), a
   locked slot shows "[locked]", a grouped slot's details name its group.
4. Spot-check 3 slot names against Toolkit (Slot list view) or the game.

## Expected effect / not tested
- Viewable/Complete: +0 (slots are UI data, not mesh assets). New in-app counts for the
  slot browser as above; icons shown is the measured badge count.
- Not tested: real slot lists and tables, icon formats (any non-2D or over-budget icon is
  listed with its reason), the `_DESC` rule, the window itself (built, not run here).
- Next: U8 (Popit-style inventory browser from U5 item details).
