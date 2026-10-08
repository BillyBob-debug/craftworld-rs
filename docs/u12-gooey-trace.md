# U11 + U12: UI natives and a headless Gooey trace (`ui_trace`)

Date 2026-10-08 (America/Chicago). Cloud implementer, UI research plan
([ui-research.md](ui-research.md), items U11 and U12). **Not run against game data**; the
Integrator assigns the D-number and runs the PC checks below.

## U11: what the UI scripts call (PC census D133/D134, names and counts only)

Source: PC export `/mnt/project-files/lbp2/research/g10-ui-natives.md` (PC copy
`local-analysis/d133-g10-ui-natives.md`), from the 426 scripts in the user's decrypted
01.33 `patch.edat`.

- 61 UI-named classes (one script each) call 802 distinct natives, 4,776 calls in total.
- Largest: PodStateSearchList (147 natives, 522 calls), PodStateStream (129/407),
  PodStateTweakPlayer (98/385), PodStateTweakLevel (114/339), PodStateStore (98/260),
  Poppet (100/201), PodStateMainMenu (61/200).
- 7 classes call no natives (HUD, PodComputerPlanets, PoppetTutorial, two ScoreBoard
  classes, Start_Menu_Rope, Start_Menu_Support).
- The top 30 natives are almost all `Gooey` frame calls: `SetFrameSizing__ff` (326 calls in
  39 of the 61 classes), `EndFrame__` (254), `SetFrameLayout__ii` (149), `DoBreak__` (136),
  `SetFrameBorders__ff`/`__ffff` (128 each), plus `LAMS.Translate__i` (261), `DoText` (77),
  `DoSpaceButtonNamed` (72), `StartFrame__Q23Gooey.EGooeyBubbleStyle` (30),
  `Resource.LoadTextureByFilename__g` (32), `System.Is4to3__` (41) and data getters
  (`GetSlotID`, `GetType`, `GetOnlineID`).

Conclusion: menus are immediate-mode frame trees built by script calls, matching the U6
model. About 25 Gooey natives plus `LAMS.Translate` cover most UI calls. The other ~775
natives are data and server queries (slots, comments, photos, store), which can stay
stubbed while showing the static screens.

## U12: headless trace (this change)

| Rule | Source | Label |
|---|---|---|
| Native owner class + mangled name `Base__argcodes` | U11 census names | CONFIRMED_FROM_LOCAL_DATA (names only) |
| `StartFrame*` opens a frame, `EndFrame*` closes the innermost; `SetFrame<Prop>` sets a property of the open frame; `DoText*` text, `Do*Button*` button, `Do*Break*` break | native names only | UNKNOWN (meaning guessed from the name); every call is kept in the trace |
| `LAMS.Translate(i)` returns the string for a LAMS key (U1 tables) | name + U1 key format | STRONG_INFERENCE |
| Native returning nothing for a typed `CALL` gets the default of the instruction's return machine type | VM stub behaviour | not a format rule (host design) |

- New crate `crates/lbp-ui-script` (runtime side; depends on `lbp-script-vm`, never the
  other way). `GooeyHost` implements the VM `Host`: records a capped trace (100,000 calls)
  and per-native counts, builds a `Frame`/`Node` tree, translates through supplied tables,
  stubs everything else. Member reads return `Void`, writes are dropped (both counted).
- `functions(script)`, `default_args` (receiver first for member functions, defaults by
  argument machine type), `run(..)` (fresh instance, budget, `RunReport`), `outline(..)`.
- `lbp-script-vm`: when the host returns `Void` for a typed call, the VM now stores
  `Value::default_for(type)`, so stubbed natives don't fault later on type checks.
- Example `ui_trace` (see usage in the file): lists a class's functions, or runs one
  function or `all` of them and writes the trace and tree to a new JSON file.
- Not done: binding to `lbp-gooey` (drawing the tree in the viewer), real input, Thing
  members, server data. Script GUIDs are not known in directory mode, so cross-script
  calls resolve by class name.

## Tests (synthetic only)
`crates/lbp-ui-script/tests/gooey_host.rs` `u12_gooey_calls_build_a_frame_tree_and_trace`:
frame open/close, frame property, translated text, button label, stub bool return,
unbalanced `EndFrame` counted, outline. Cloud (Linux, Rust 1.99.0): `cargo fmt --all
--check` ok; `cargo test -q -p lbp-ui-script -p lbp-script-vm` pass;
`cargo +1.99.0 clippy -q -p lbp-ui-script -p lbp-script-vm --all-targets -- -D warnings`
clean.

## PC verification (Integrator)
Use the same extracted `.ff` directory as the D133 `script_check` run (DIR below), and the
English `.trans` tables g54800 and g69910 extracted to local-analysis (base first).
1. `cargo run -q -p lbp-ui-script --example ui_trace -- DIR PodStateMainMenu` and report the
   function list.
2. `cargo run -q -p lbp-ui-script --example ui_trace -- DIR PodStateMainMenu all local-analysis/<new>-u12-mainmenu.json BASE.trans PATCH.trans`
   and report the printed summary lines (one per function, plus the totals line).
3. For the function with the most Gooey calls, rerun with that function name instead of
   `all` and paste the outline (40 lines). Check that the text lines are readable English
   menu labels.
4. Repeat step 2 for `Gooey` and `PodStateStoriesMenu`.

## Expected effect / not tested
- Viewable/Complete: +0 (UI scripts, not mesh assets). New counts are per function: faults,
  native calls, translated strings and frame balance.
- Not tested: real scripts (entry points, arguments, whether stub returns keep loops
  bounded; the 5,000,000 instruction budget per function stops runaway loops).
- Next: U12b, map the Gooey frame natives onto `lbp-gooey::Builder` so one menu script
  draws in the viewer (`--ui-script CLASS`), once the PC trace shows which function draws.
