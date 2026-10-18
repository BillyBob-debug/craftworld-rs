# Handoff: level1 lane (Rookie Test played with scripts), part 4 (2026-10-17)

## Read first
- docs/level1-play.md, section "Part 4" (the route right of the spawn, line 3)
- crates/lbp-bevy-viewer/src/sackboy_play.rs `parse_script` (the `--play-sackboy-input` keys)

## Facts
- PR 133 (branch level1-4, D503): `--level-scripts-close-at` takes a comma list of ticks.
  Next D number: D504.
- Route run: `--level-guid 0001e786 --level-scripts --play-sackboy --qa-hide-ui
  --level-scripts-close-at 120,600,1150 --play-sackboy-input
  "-:620,r:160,rj:20,r:330,f:15,-:5,r:400,rj:20,r:400" --level-scripts-hold-at 2000
  --qa-capture local-analysis/<new>.png`, run from the canonical repo folder (the capture
  path must be under its local-analysis). Sackboy positions print every tick ("sackboy tick N").
- Line 3 opens on tick 882 at bubble 1485. Sackboy stops at x -938 (a jump clears it) and at
  x -385; with no input after tick 1965 he falls from x -385.
- Line 2's bubble 1054 has no SWITCH part, so its end pulse feeds nothing.
- There is no per-Thing wiring dump; local-analysis/level1/p2-switch-debug.patch (throwaway
  LBP_DBG_SW) is the only one. level1/dump.txt has trigger positions.
- Toolchain env: local-analysis/toolchain (RUSTUP_HOME, CARGO_HOME, cargo\bin and the gnu
  self-contained bin on PATH); CARGO_TARGET_DIR E:\Projects\wt-level1\target. A background
  cargo build may still hold the target lock; build in the foreground before running.

- Standing rule (owner, 2026-10-18): the app must never need a decrypted copy of anything;
  the only requirement is the fully updated USA LBP2 ISO plus the RPCS3-installed update
  folder, and no keys go in the repo. The route run above uses
  local-analysis/d133-edat-overlay (decrypted EDAT files): fine for research, but whatever
  Rookie Test needs from encrypted files must get a built-in fallback before it ships.

## Next
1. The route past x -385: get a game reference (RPCS3 F12 frames past line 2, background
   only) or try jumps and layer moves there.
2. The camera at x -938 (a frame at tick 1600 looks through a front wall).
3. Under the no-decrypted-copy rule: all 426 game scripts live only in patch.edat (Popit
   lane finding), so without the overlay Rookie Test loads no scripts. It needs a built-in
   fallback for the script behaviour it relies on (Da Vinci's lines, the bubble switches).
   Approved strategy (docs/project-boost.md): native Rust per script, one script per task,
   with the RScript VM kept as a dev oracle that records golden traces to test it.
4. Sackboy pushed into a wall turns to the camera and plays `stand`, not a push.
