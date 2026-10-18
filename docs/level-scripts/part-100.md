# Level scripts, part 100: the Story screen without the game's scripts (2026-10-18, D353)

The shipped app needs only the updated USA ISO and the RPCS3 update (owner rule,
2026-10-18), and the game's 426 scripts exist only in the encrypted patch.edat (D133). With
the disc and update alone the Pod's search state (`PodStateSearch`) never runs, so the
Story search never opened and the friends exe kept its Story button off. This part adds
the first native stand-in for a game script, the search state's Story entry, and a golden
trace recorder that tests it against the scripted run.

### What was found

- Golden trace (new `--level-scripts-trace PATH`, a JSON line per tick: the search state,
  the camera a script wrote, the pod's change-over value, the scripts' drawn menu frame and
  every Thing whose display values changed). Scripted run: `--level-scripts-in-use-at 600
  --level-scripts-choose-at 640` over the decrypted overlay. The trace is derived data and
  stays in ignored `local-analysis/d353/`; no script code is copied.
- MEASURED, the search state on the choice (tick 640): planet 1 selected, cursor mode 1,
  the search details asked for once, the info bubble's page counting from that frame, the
  default result the default story group's. On tick 641: cursor mode 4, browse mode 1,
  Earth has control, the cursor badge published once. Nothing more changes after that.
- MEASURED, the Story frame (tick 641): named frame 1 under high bits 2, 4800 x 1250,
  columns -0.5 / 1500 / -0.5, a style 15 bubble with "STORY" (style 16, font scale 2.5) and
  two images. Its matrix follows the world Thing in until tick 659 and then stands.
- MEASURED, what other scripts leave for the Story view: pod.bin opens the pod (change-over
  value 0.05 a tick from 601, open at 620, the pod Thing 2 of the shell level swapped to
  mesh 0x23959); the planets world script holds the in-use camera from tick 620 (eye (0,
  -299.48, 902.39), direction (0, 0.315, -0.949), 1 rad, near 10, far 100,000), rests the
  world Thing at tick 659 and places the Moon (Thing 4) on tick 1; Earth (Thing 1 and its
  two parts) has turned to the default group by tick 660.

### What changed

- `lbp-runtime/src/story_native.rs`: `PodStateSearchStory`, the native Story entry
  (`open` on the choice, `update` each tick after), and `LevelWorld::place_thing` for
  native stand-ins. It writes the trace's search state and builds the
  Story frame. It moves to the shared `NativeScript` trait when the Popit lane's lands.
- Viewer `--story`: with the scripts it is the scripted path above (in use 600, choice
  640); with none loaded (`story_fallback.rs`) the first tick opens the pod, puts the
  planets where the scripts leave them, draws with the in-use camera and runs the native
  Story entry. The pod, planets and camera values are STAND-INS for pod.bin and the planets
  world script, which the Popit lane owns; they are replaced by its native scripts.
- `--level-scripts-none` (development): drop the scripts from a decrypted snapshot, the
  shipped case.
- The friends exe's "Story screen" mode is on (the Pod's args plus `--story`).

### Left out

- The moves: the pod's change-over, the world Thing's move in, Earth's turn and the Story
  frame's slide are not played; the view starts where the scripts leave it.
- Leaving Story (back to the main menu) and any other search; only the default group.

### Verification

No scripts (`--level-scripts-none --story`, held at tick 120) against the scripted run
held at tick 900: mean abs difference 0.36 levels over the 1280 x 720 frame, 0.05 over its
top 80% (the rest is Sackboy's pose at the controller). Tests:
`d353_story_entry_matches_the_scripted_trace`, `d353_story_frame_is_drawn`,
`d353_no_default_group_no_default_result`. `cargo fmt --all --check`; clippy on
lbp-runtime and lbp-bevy-viewer with `-D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
