# Lane state: friends' launcher (2026-10-19)

## Read first
1. docs/d600-launcher.md (D600, D601)
2. crates/lbp-bevy-viewer/src/launcher.rs (MODES, disc_input, archive_problem, start_setup)
3. README.md "Play it" and "If something goes wrong"

## State
- PR 139 (D600) merged; Story button on (PR 153).
- D601 (branch `launcher-first-run-help`, cloud thread): plain-words setup errors, disc
  folder accepted, FARC-tag check for encrypted dumps, "didn't load" window in game modes,
  README guide for non-programmers. Handed to the coordinator for merge.
- Decision numbers D600-D699; next free D602.

## Facts
- Owner rule (2026-10-18, standing): the app needs only the user's fully updated USA ISO
  (BCUS98245) plus the 01.33 update RPCS3 installed. Never require a decrypted copy, never
  add keys; encrypted-only data (patch.edat scripts) gets built-in fallbacks.
- Launcher: no args or `--launcher` opens it; `--launcher-setup ISO UPDATE` is headless.
  Data in %LOCALAPPDATA%/craftworld-rs (disc/ copy 1.2 GB, inventory.json, launcher.txt).
  A disc folder (holding PS3_GAME) is read in place, not copied.
- Every README error-table row matches a launcher/viewer message; change both together.
- Zip: local-analysis/friends-build/craftworld-rs-preview-2026-10-18.zip predates D601.
  No GitHub release yet: the README links the Releases page for when one is posted.
  Rebuild, re-zip and any release wait for the owner's word.

## Next
1. Owner's go: rebuild the zip with D601 and (if asked) publish a release.
2. On the PC: run Set up against the real ISO once to confirm the FARC check passes.
