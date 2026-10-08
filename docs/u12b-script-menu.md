# U12b: draw a UI script's menu in the viewer (`--ui-script`)

Date 2026-10-08 (America/Chicago). Cloud implementer, follows U12 (D136,
[u12-gooey-trace.md](u12-gooey-trace.md)). **Not run against game data**; the Integrator
assigns the D-number and runs the PC checks below.

## Question
Can the viewer show the menu a Pod script builds (PodStateMainMenu, D136: Story, Community,
Recent Activity, Store, Me) on screen instead of as a text outline?

## Rules
| Rule | Source | Label |
|---|---|---|
| Frame tree, labels and nesting come from the script run (U12 host) | D136 PC run | CONFIRMED_FROM_LOCAL_DATA (labels); host mapping UNKNOWN as in U12 |
| Layout: every frame a column; gap = first `DefaultChildSpacing` number, padding = first `Borders` number (0..64), framed when it has properties | none (placeholder) | UNKNOWN; not the game's layout |
| Screen natives answer 1280 x 720: `Get*XRes`/`Get*YRes` (s32), `Get*Width`/`Get*Height`/`GetAspectRatio` (f32), `GetScreenEdgeCrop` 0. The Pod scripts' `GetPuterWidth(w)` divides `w` by `HUD.GetXRes`, so the default 0 gave `Sizing=NaN` (PC listings, first #39 run) | PC run d141 | values UNKNOWN |

## Implementation
- `lbp-ui-script`: `load::{from_catalog, load}` (moved from the `ui_trace` example; every
  `script` MAP row under its GUIDs), `tree::ui_tree(root, uid, header, input)` (frame tree to
  `lbp_gooey::UiTree`), screen-size answers in `GooeyHost` (`screen` field).
- `lbp-bevy-viewer`: `ui_script.rs`, flag `--ui-script CLASS` (default function
  `UpdateUI__Q5Gooey`, `--ui-script-function NAME`), `--ui-script-report` prints the counts,
  table states and a 60-line outline and exits. The window shows the count lines (result
  or fault with its site, native calls, strings translated, frame balance) above the tree.
- Not done: running the script every frame with input (buttons don't act), Puter/state
  objects the scripts read (D136 faults: NullObject in `PartyManagement.GatherFriendData`,
  `PodComputerState.SetupFrameMatrix`, and the final `Puter` read in `UpdateUI`; waits on
  `Vm::construct`, PR #38), real frame styles and fonts.

## Tests
`crates/lbp-ui-script/tests/gooey_host.rs` `u12b_frame_tree_maps_to_a_gooey_tree_and_screen_size_answers`.
Cloud (Linux, Rust 1.99.0): `cargo fmt --all --check` ok; `cargo test -q -p lbp-ui-script` 2
pass; `cargo test -q -p lbp-bevy-viewer` 25 pass; `cargo +1.99.0 clippy -q -p lbp-ui-script
-p lbp-bevy-viewer --all-targets -- -D warnings` clean; viewer builds.

## PC verification (Integrator)
1. `target/debug/lbp-bevy-viewer.exe --ui-script PodStateMainMenu --ui-script-report --snapshot SNAPSHOT --game-dir GAME-DIR`
   (the D136 inputs): report the printed lines; the outline should list Story, Community,
   Recent Activity, Store and Me, with no `Sizing=NaN`.
2. Window on the second monitor with `--qa-capture local-analysis/<new>-u12b-mainmenu.png`:
   the five labels are readable buttons and keyboard focus moves between them.
3. Same report for `PodStateStoriesMenu`.

## Expected effect / not tested
- Viewable/Complete: +0 (UI, not mesh assets). New in-app counts as above.
- Not tested: the window itself, real script output (cloud has no game data).

## U12c: construct objects and supply null script fields (after D141)
D140 runs stopped on NullObject in `UpdateUI+888` (the final `Puter` read),
`PartyManagement.GatherFriendData+2` and `PodComputerState.SetupFrameMatrix+3`.
- `run` now builds `this` and script-class arguments with `Vm::construct` (D141: inherited
  fields plus `.init__`) and then supplies every null `object_ref` field whose declared type
  is a loaded script class with a constructed instance of that class, one level deep
  (`make_object`). In the game the engine sets objects like the Pod's `Puter`; this is a
  stand-in (INFERRED), and each supplied field is listed (`Class.field: Type`) in the
  report, the `ui_trace` output and the viewer header.
- VM (additive): `Vm::set_field`, `Vm::null_script_fields`.
- Test: `u12c_null_script_class_fields_are_supplied`.
- PC check: rerun step 1 and step 3 above; report the "Supplied objects" line and any fault
  with its site.

## U12d: frame sizes from the script (after D146)
D146: the main-menu and Stories scripts run with no faults. Frame `Sizing` values seen:
outer 2400 x 1250, then -1 x 500, 0 x 0 (three), 1000 x 300 (main menu).
- `lbp-gooey`: `Widget::Frame` has `size: [Option<f32>; 2]` (fractions of the screen;
  `None` fits the content) and `Builder::set_frame_size` for the open frame; the viewer
  maps it to `Vw`/`Vh`.
- `tree::ui_tree`: the first frame with a positive `Sizing` is the full screen (D146:
  `GetPuterWidth(2400)` returns 2400, so script units are screen units at 2400 wide,
  INFERRED); each frame's positive `Sizing` is divided by it; zero or negative axes fit the
  content (meaning UNKNOWN).
- Test: `u12d_sizing_is_relative_to_the_first_sized_frame`.
- PC check: `--ui-script PodStateMainMenu` capture on the second monitor; the outer frame
  should fill the window and the 1000 x 300 frame should be about 42% x 24% of it. The count
  header sits above the tree, so the outer frame overflows by the header height (known).
