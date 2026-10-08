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
| `Get*Width` / `Get*Height` natives return 1280 / 720 (D136 showed `Sizing=NaN` from a zero `GetPuterWidth`) | none | UNKNOWN |

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
