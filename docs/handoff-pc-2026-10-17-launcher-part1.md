# Handoff: friends' launcher lane, part 1 (PC, 2026-10-17/18)

## Read first
1. docs/d600-launcher.md
2. crates/lbp-bevy-viewer/src/launcher.rs (MODES table, start_setup, launch)
3. crates/lbp-bevy-viewer/src/launcher/iso.rs
4. STATUS-CURRENT.md

## Facts
- Owner rule (2026-10-18, standing): the app needs only the user's fully updated USA ISO
  (BCUS98245) plus the 01.33 update RPCS3 installed. Never require a decrypted copy, never
  add keys; encrypted-only data (patch.edat scripts) gets built-in fallbacks.
- PR 139 (branch `launcher`, worktree E:\Projects\wt-launcher): green, handed to the
  coordinator for merge. D-numbers D600-D699; D600 used.
- Launcher: no args or `--launcher` opens it; `--launcher-setup ISO UPDATE` is headless.
  Data in %LOCALAPPDATA%/craftworld-rs (disc/ copy 1.2 GB, inventory.json, launcher.txt).
- Modes open: Pod, Popit (D411 fallback camera from the Popit lane; planets not placed),
  Rookie Test (no --level-scripts), Moon, explorer. Story is `later: true` until the Story
  lane has a script-free fallback; then set its args (Pod args + the Story QA flags or a
  menu route) and `later: false`.
- Zip: E:\Projects\LBP2 Rust Rewrite\local-analysis\friends-build\craftworld-rs-preview-
  2026-10-18.zip (38 MB, stripped via CARGO_PROFILE_RELEASE_STRIP=true). Not published:
  a GitHub release waits for the owner's word in the thread.
- Window captures without focus: scratchpad winshot.ps1 used PrintWindow on the launcher
  (works with --no-activate); viewer modes use --qa-hide-ui --qa-capture under local-analysis.

## Next
- Story button when its fallback lands; rebuild and re-zip.
- Optional: a GitHub release only after the owner says so.
