# Lane state: popit (2026-10-18)

## Read first
1. `docs/d416-popit-arms-follow-sticks.md` (arm rule, port, Open).
2. `docs/project-boost.md` "Decide first" item 1 (native script fallback strategy).
3. Research notes (canonical, ignored): `local-analysis/d416/arms-cursor.md`.

## State
- PR #150 (branch `popit-14`, worktree `E:\Projects\wt-popit`): D416 arms follow the
  sticks while the Popit cursor is shown; coordinator merges once `rust` + `python` green.
- Branch `popit-15` (same worktree): D417 native script registry
  (`crates/lbp-runtime/src/native_script.rs`, trait `NativeScript`) + native `Pod` and
  `Trigger_Pod`; in progress, no PR yet.
- Split agreed with the Story lane: Popit owns the trait, pod.bin (Pod, Trigger_Pod,
  pod-open change-over) and the planets world script; Story owns PodStateSearch's Story
  entry and its `story_fallback.rs`, then moves onto the trait.
- Decision numbers: D416-D499, next free D418.

## Facts
- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"`.
- No-decrypted run: `--snapshot ...\local-analysis\d79-inventory-0133.json --game-dir
  "...\game-data\a1-disc;...\game-data\a1-update-0133"`; blank level
  `--level-path gamedata/levels/palettes/borders/blank_level_large.bin --play-sackboy`.
- Arms (0x2097e0 cursor branch 0x209ce0 -> 0x20936c x2, weight 1.0): angle atan2(y, x),
  dead zone |x|+|y| <= 0.2, ease w = clamp(m, 0, 1), frame = angle / 2pi * 32; clips
  act_neutral_arm_small 0x3ef0 / big 0x3eef; arm 0 left stick x-negated, arm 1 right
  stick mirrored; sticks ease 0.3 a tick. QA flag `--popit-arm-sticks LX,LY,RX,RY`.
- `--qa-capture` opens the window unfocused (SW_SHOWNOACTIVATE); run it from the
  canonical folder (capture path must be under its local-analysis).
- Planets world camera (Story's VM trace): eye (0,-299.48,902.39), dir
  (0,0.31499,-0.94910), fov 1 rad, near 10, far 100000, steady from tick 620.

## Next
1. Finish D417 (registry + native Pod / Trigger_Pod), PR, tell Story the PR number.
2. Native planets world script (PlanetsWorld / Earth / OrbitingMoon placement).
3. RPCS3 memory read of anim+0xac0/0xad0/0xa40 with the cursor shown (D416 Open).
4. Which Popit screens are cursor-active; D414 lean vs the Moon create frame.
