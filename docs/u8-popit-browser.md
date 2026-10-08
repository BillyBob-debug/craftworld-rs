# U8: inventory browser (`--popit`)

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U8; inputs U1, U5, U6, U7's gooey host). **Not
run against disc data**; the Integrator assigns the D-number, runs the PC checks and
updates STATUS/MODLOG. Stacked on PR #28 (U7): it reuses `run_gooey` and the story
module's table and icon helpers.

## Question
Can the explorer list every disc plan as an inventory card (icon + translated title),
grouped like Popit pages, with measured counts in the app?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| A non-empty translation tag sets title/description keys to `lams(tag + "_NAME")` / `lams(tag + "_DESC")`; non-zero keys are looked up in the LAMS table | `structs/inventory/InventoryItemDetails.java` `updateTranslations` | CONFIRMED_FROM_COMMUNITY_CODE |
| Base table g54800 with patch g69910 over it | as U7 (`RestlessPopulator`, `RTranslationTable.patch`) | CONFIRMED_FROM_COMMUNITY_CODE |
| Untranslated fallbacks: user-created name, then tag, then MAP path stem | browser choice | not a game rule |
| Pages = first `InventoryObjectType` name of the type bits (U5 `type_names`) | browser choice over `enums/InventoryObjectType.java` | grouping is not the game's Popit layout (script-driven, UNKNOWN) |
| RPlan location/category u32 values are LAMS keys of page/category names | `resources/RPlan.java` stores them as keys; profiles resolve indices via a string table (`types/save/BigSave.java`) | STRONG_INFERENCE; counted (`placement_translated`) |

Side note for U7: `updateTranslations` confirms the `_DESC` suffix for items; for slots it
remains an inference by analogy.

## Implementation
- `crates/lbp-preview/src/popit.rs` (no UI dependency): `load_popit(source, catalog,
  control)` reads every magic-confirmed `plan` row (grouped by content hash, budgets
  50,000 plans / 2 GiB stored bytes) through `item_details::read_plan_details`;
  `item_text`, `build_popit`, `count_lines` are pure and tested.
- `story.rs`: `Strings::get(key)`, and `load_strings`, `resolve_key`, `decode_icon` made
  public for reuse.
- Counts (`PopitCounts`, unit: distinct plan content hash): plans, items, not read by
  reason, items per page, titles by source (Translated / UserCreated / TagOnly /
  KeyUntranslated / Path), descriptions, icons with/resolved, placement keys translated.
- `crates/lbp-bevy-viewer/src/popit_browser.rs`: one tab per page, 24 cards per page, a
  details panel (types, subType, head, location, category, creator, icon state, plan
  path, description). Icons are decoded when a card is shown: `gooey_ui` gained an
  optional `ImageLoader` that loads at most 8 missing images per frame and respawns the
  tree when one arrives (failures are not retried).
- `--popit-report` prints the counts and exits; with `--popit-decode-icons` it decodes
  every distinct resolved icon once and prints the decoded count and failure reasons
  (the measured icon count).

## Tests (synthetic only)
`crates/lbp-preview/tests/popit.rs`: `u8_titles_follow_update_translations_then_fallbacks`,
`u8_pages_group_by_type_and_count`. Viewer unit test
`u8_tabs_pages_and_icons_only_for_resolved_items`.

Cloud (Linux, Rust 1.99.0): `cargo fmt --all --check` ok; `cargo +1.99.0 clippy -q -p
lbp-preview -p lbp-bevy-viewer -p lbp-gooey --all-targets -- -D warnings` clean;
`cargo test -q -p lbp-preview --test popit --test story` 4 pass; `cargo test -q -p
lbp-bevy-viewer` 24 pass.

## PC verification (Integrator)
1. Headless counts with icon decoding:
   `target/debug/lbp-bevy-viewer.exe --popit --popit-report --popit-decode-icons --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133"`
   Report every printed line (items/plans, not-read reasons, titles by source, pages,
   icons decoded / distinct resolved, placement keys translated, table states).
   Expect items close to D127's complete count (11,936; more once U5b #29 merges).
2. Visual QA on the second monitor:
   `--popit ... --qa-capture local-analysis/u8-popit-browser-0133.png` (first tab, first
   page). Check titles are English, icons are item icons, the details panel matches.
3. Toolkit spot-check 3 items (a material, a sticker, an object): title and icon.

## Expected effect / not tested
- Viewable/Complete: +0 (plans are not mesh assets). New in-app counts as above.
- Not tested: real plans, icon formats, the location/category key rule, the window
  itself (built, not run here), browsing speed with 10k+ items.
- Next: U9+ per ui-research.md; U11 native census waits on patch.edat decryption.
