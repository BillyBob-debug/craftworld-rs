# Lane state: popit (2026-10-18)

## Read first
1. `docs/d417-native-pod-script.md` (native script registry, Pod port, Open).
2. `crates/lbp-runtime/src/native_script.rs` (the `NativeScript` trait other lanes use).
3. `docs/d416-popit-arms-follow-sticks.md` (arm rule, Open).

## State
- PR for D417 (branch `popit-15`, worktree `E:\Projects\wt-popit`): native script
  registry + native `Pod` / `Trigger_Pod` + dev trace recorder; coordinator merges once
  `rust` + `python` are green. PR #150 (D416 arms) is merged.
- Lane paused at a clean point (one-thread-at-a-time rule from 2026-10-18 06:00 UTC).
  Nothing running, RPCS3 not used.
- Split agreed with the Story lane: Popit owns the trait, pod.bin and the planets world
  script; Story owns PodStateSearch's Story entry (`story_fallback.rs`) and moves its
  stand-ins onto the trait.
- Decision numbers: D416-D499, next free D418.

## Facts
- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"`.
- No-decrypted run: `--snapshot ...\local-analysis\d79-inventory-0133.json --game-dir
  "...\game-data\a1-disc;...\game-data\a1-update-0133"`.
- Script GUIDs: Pod g00004905, Trigger_Pod g00002d07. `--native-scripts` forces the ports
  (viewer and the `level_scripts` example); example trace flag `--trace=FILE[@A-B,..]`.
- Pod engage check (disc + update only): D409 args with `--level-scripts-press-at 40
  --level-scripts-flags 2`: tick 40 RequestHostUsePod(true), tick 60 change-over 1.00.
- VM vs native: 30 ticks of pod.bin, 890 identical events; fixture
  `crates/lbp-runtime/tests/fixtures/d417_pod_vm_trace.rs` (JSON in .rs: .gitignore only
  admits .rs under crates/).
- Arms (D416): angle atan2(y, x), dead zone 0.2, clips 0x3ef0 / 0x3eef, QA flag
  `--popit-arm-sticks LX,LY,RX,RY`. `--qa-capture` opens unfocused; run from canonical.
- Planets world camera (Story's VM trace): eye (0,-299.48,902.39), dir
  (0,0.31499,-0.94910), fov 1 rad, near 10, far 100000, steady from tick 620.

## Next
1. Tell the Story lane the D417 PR number so it moves onto the trait.
2. Native planets world script (PlanetsWorld / Earth / OrbitingMoon), camera as above;
   then Pod's parent `BaseWorld` and `Pod.Render__` (D417 Open).
3. RPCS3 memory read of anim+0xac0/0xad0/0xa40 with the cursor shown (D416 Open).
4. Which Popit screens are cursor-active; D414 lean vs the Moon create frame.
