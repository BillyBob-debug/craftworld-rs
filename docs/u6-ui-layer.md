# U6: UI layer (lbp-gooey model + Bevy adapter + `--ui-demo`)

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U6). **No game data involved; needs PC QA of the
demo window.** The Integrator assigns the D-number and updates STATUS/MODLOG.

## Question
Front-end screens (slot browser U7, Popit-style inventory browser U8, Pod computer U9)
need one way to describe UI. LBP2 uses a native immediate-mode widget system driven by
scripts (research §2). Can we have a renderer-independent immediate-mode model in that
shape, with pad/keyboard focus navigation, drawn through bevy_ui?

## Evidence and provenance
No format rules. The shape (frames with layout/style, text, buttons, images, spacers,
64-bit widget ids, per-widget input results) is behaviour evidence from ennuo/alear
(LBP1 engine, **no licence**): vocabulary only, nothing copied, names and code are our
own (STRONG_INFERENCE that LBP2 works the same way; research §2). Bevy 0.19 `bevy_ui`,
`bevy_text`, `bevy_ui_render`, `default_font` features (MIT/Apache, already the project's
engine).

## Implementation
- New crate `crates/lbp-gooey` (no dependencies): `Builder` records a `UiTree` per frame
  (`frame`, `text`, `button`/`button_with`, `image`, `spacer`, stable `auto_id` for
  non-interactive widgets, duplicate-id reporting); `FrameInput` answers
  `Response { activated, focused }`; `navigate` picks the next focusable widget in a
  direction from laid-out rectangles (nearest centre, sideways distance weighted double);
  `settle_focus` keeps focus valid after a rebuild. Added to the CI crate list.
- `lbp-bevy-viewer/src/gooey_ui.rs`: rebuilds bevy_ui entities when the tree changes
  (full respawn; menus are small and focus is kept by id), arrows move focus, Enter/Space
  activate, mouse hover/click via `Interaction`, focused button gets a gold border.
  Swatch images draw as colour; game textures by GUID/hash draw neutral grey until a later
  item resolves them. Text uses Bevy's default font (game fonts via the U2 atlas later).
- `lbp-bevy-viewer --ui-demo`: authored synthetic screen (four tabs, a wrapping grid of 12
  items with one disabled, a status line). Same hidden-until-second-monitor window rule.
- Gamepad not wired (`bevy_gilrs` not enabled); Cargo.lock grows by the bevy_ui/bevy_text
  dependency tree (parley etc.).

## Tests
`crates/lbp-gooey/tests/gooey.rs` (6): tree order/parents, input matching, stable auto
ids, duplicate ids, grid navigation, focus settling. `gooey_ui::tests::
u6_demo_screen_ids_unique_and_tabs_switch_items`.
Cloud (Linux, toolchain 1.97): `cargo fmt --all --check` ok; `cargo test -q -p lbp-gooey`
6 pass; `cargo test -q -p lbp-bevy-viewer u6_` 1 pass; `cargo clippy -q -p lbp-bevy-viewer
-p lbp-gooey --all-targets -- -D warnings` clean; `cargo check -p lbp-bevy-viewer` ok.
The window could not be opened in the cloud container (no display libraries or GPU).

## PC QA (Integrator)
1. Build both viewers (Windows GNU toolchain; new bevy_ui/bevy_text dependencies).
2. `target/debug/lbp-bevy-viewer.exe --ui-demo` on the second monitor: arrows move the gold
   focus border along rows/columns and between tabs and grid, Enter switches tabs / sets
   "Picked", the disabled item never focuses, mouse hover/click works, no warnings about
   duplicate ids, the rebuild counter only rises on changes.
3. Normal viewer run unchanged (egui panel and 3D view still draw over/with bevy_ui).

## Expected effect / not tested
- Viewable/Complete: +0 (no assets). Not tested: the window itself, gamepad, game fonts and
  textures in widgets, layout at other window sizes.
