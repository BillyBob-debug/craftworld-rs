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

## Fix after the first PC run (2026-10-08)
PC result: PodStateMainMenu `all` ran 15 functions, 7 faulted with Type faults on
`GET_OBJ_MEMBER`/`SET_OBJ_MEMBER`/`SET_V4_X` within the first 11 instructions;
`UpdateUI__Q5Gooey` stopped after 48 of 1,201 instructions with 1 frame open and no text.
Gooey: 45 run, 9 faulted. PodStateStoriesMenu: 13 run, 5 faulted.

Two causes, both fixed here (rules INFERRED, the trace reports them):
- Receiver placement. The first version put `this` in the first argument row, so for
  `UpdateUI__Q5Gooey` it replaced the Gooey argument and register 0 stayed empty.
  `entry_registers` now puts `this` at register 0, inside the rows only when a row sits
  at register 0, and reports which (`receiver`, `argument_offsets` per run). An object
  argument whose type names a loaded script class (e.g. `Gooey`) gets a fresh instance
  instead of null.
- Fresh registers. `SET_V4_*` / `SET_M44_*` on a never-written register now start from a
  zero vector/matrix (VM change; registers behave like zeroed storage).
- New VM entry `Vm::start_registers` stages registers directly; a run that yields is
  reported as `<yielded>`.

## Second PC run (86684dd) and fix
Faults dropped to 3 of 15 (receiver `ImplicitRegister0` everywhere): `OnActivateImpl` and
`OnDeactivateImpl` stop on a missing object field, `PerFrameUpdate` on register 65535, and
`UpdateUI` still returns after 48 instructions. Cause of the first two (and likely of the
early return): directory mode loads scripts without GUIDs, and the VM finds superclasses
by GUID, so a subclass's inherited fields and functions were missing; calls to missing
script functions fell through to the host as natives and returned defaults. `ui_trace` now
also takes the catalog input `script_census` uses (SNAPSHOT GAME-DIR), loading each script
under its MAP GUIDs, and prints how many superclasses are still not loaded. Register 65535
is unexplained (needs the listing lines at that pc).

## Third PC run (83652c6) and fix
GUIDs: 426 loaded, superclass not loaded 0. Remaining: two register-65535 faults in callees
(the "no register" operand; fixed in the VM by PR #37, which also reports the fault site,
now printed by `ui_trace` as `at Class.Function+pc`), one NullObject in `Draw`. `UpdateUI`
still stopped after 48 instructions: the PC listing shows it calls the Gooey script's
`StartFrameNamed__i`, which forwards to the native `StartFrameNamed__iQ23Gooey.EGooeyBubbleStyle`,
and skips the whole frame body when the result is false. The stub returned the bool
default (false). `Gooey.StartFrame*` now returns true (frame open; guessed, UNKNOWN).

## Tests (synthetic only)
`crates/lbp-ui-script/tests/gooey_host.rs` `u12_gooey_calls_build_a_frame_tree_and_trace`:
frame open/close, frame property, translated text, button label, stub bool return,
unbalanced `EndFrame` counted, outline, receiver at register 0 with a member read,
script-class argument instance, `SET_V4_X` on a fresh register. Cloud (Linux, Rust 1.99.0): `cargo fmt --all
--check` ok; `cargo test -q -p lbp-ui-script -p lbp-script-vm` pass;
`cargo +1.99.0 clippy -q -p lbp-ui-script -p lbp-script-vm --all-targets -- -D warnings`
clean.

## PC verification (Integrator)
Use the D133 `script_census` inputs (SNAPSHOT and GAME-DIR; written `SNAPSHOT GAME-DIR` where
DIR appears below), and the
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
