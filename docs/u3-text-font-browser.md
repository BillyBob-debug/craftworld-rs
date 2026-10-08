# U3: strings and fonts browser (`--text-browser`), OS font fallback for UI text

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U3; inputs U1, U2, U6). **Not run against disc
data**; the Integrator assigns the D-number, runs the PC checks and updates STATUS/MODLOG.

## Question
Can the explorer browse every `.trans` table (search by text, tag or key) and show each
present font face's glyphs, with counts in the app? Also: D131 QA saw a tofu box in a
"DC Comics" title because Bevy's built-in font is a small subset.

## Evidence and rules
| Rule | Source | Label |
|---|---|---|
| Table format, LAMS key | U1 (`docs/u1-translation.md`) | as recorded there |
| Tables have no magic: selected by a `.trans` MAP path, grouped by content hash | U1 / D109 | CONFIRMED_BY_LOCAL_EXPERIMENT (D109: 40 tables, 20 real) |
| g54800 English base, g69910 English patch; other tables' languages | Toolkit `RestlessPopulator` | CONFIRMED_FROM_COMMUNITY_CODE; others UNKNOWN |
| Glyph decode, atlas layout | U2 (`docs/u2-font-glyphs.md`) | as recorded there |
| Sheet colours: compressed = white with alpha = coverage; uncompressed = bytes as green/blue like Toolkit `getGlyph` | U2 | alpha meaning STRONG_INFERENCE; uncompressed meaning UNKNOWN |

## Implementation
- `crates/lbp-preview/src/text.rs` (no UI dependency): `load_text(source, catalog,
  control)` reads every `.trans` row and every magic-confirmed `fontface` row (unreadable
  rows are kept with their error, e.g. fonts inside the encrypted `patch.edat`);
  `search(table, query)` matches a case-insensitive substring of text or tag, an 8-digit
  hex key, or the LAMS key of the query as a tag (as typed and upper-cased);
  `font_sheet` composes an RGBA glyph sheet (1024 wide, 4096 x 4096 cap); `count`,
  `count_lines`. Units: distinct table content hash; distinct LAMS keys per table
  (summed); distinct font content hash; glyph record.
- `crates/lbp-bevy-viewer/src/text_browser.rs`: tabs for tables (English base/patch
  first) and fonts; 24 rows per page; typed search (Backspace deletes; switching tabs
  clears it); font tab shows the sheet (up to 2x) and glyph records.
  `--text-browser-report` prints the counts, every table and every font, and exits.
- `gooey_ui.rs`: optional shared text box (`SharedText`): typed characters go to the
  screen and Space types instead of activating (Enter still activates).
- **UI text font**: gooey text now uses `FontSource::SansSerif` with Bevy's
  `system_font_discovery` feature (parley/fontique), so characters missing from Bevy's
  built-in FiraMono subset are drawn from the user's OS fonts. Nothing is bundled.
  `Cargo.lock` gains fontique's platform crates (on Linux, `yeslogic-fontconfig-sys`
  links libfontconfig; on Windows it uses DirectWrite).

## Tests (synthetic only)
`crates/lbp-preview/tests/text.rs`: `u3_search_matches_text_tag_and_hex_key`,
`u3_font_sheets_and_counts` (uncompressed and zlib fonts, broken font, counts). Viewer
unit test `u3_search_pages_and_tab_switch_clears_query`.

Cloud (Linux, Rust 1.99.0): `cargo fmt --all --check` ok; `cargo +1.99.0 clippy -q -p
lbp-preview -p lbp-bevy-viewer -p lbp-gooey --all-targets -- -D warnings` clean;
`cargo test -q -p lbp-preview --test text` 2 pass; `cargo test -q -p lbp-bevy-viewer` 25
pass. The viewer with `system_font_discovery` builds on Linux.

## PC verification (Integrator)
1. Build on Windows (GNU): `cargo build -q -p lbp-bevy-viewer` (new fontique deps).
2. Headless: `target/debug/lbp-bevy-viewer.exe --text-browser --text-browser-report --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133"`
   Expect 40 rows / 20 real tables as D109 and 16 font rows with 1 present (D110 U0).
3. Visual QA on the second monitor (`--qa-capture local-analysis/u3-text-browser-0133.png`):
   search `SLOT_NAME`-style tags and a word in the English base; open the font tab
   (icon_patch_font sheet as D112).
4. Re-run the D131 popit capture: the "DC Comics" title should now render without a
   tofu box.

## Expected effect / not tested
- Viewable/Complete: +0 (UI data). New in-app counts as above.
- Not tested: real tables/fonts in this browser, the OS font fallback on Windows (built
  on Linux only), languages of the 18 non-English tables, game fonts used for UI text.
- Next: U9 (Pod scene) needs U0 pod candidates and level work; U11 waits on patch.edat.
