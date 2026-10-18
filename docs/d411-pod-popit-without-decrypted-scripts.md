# D411: the Pod and its Popit without the decrypted patch.edat (Popit part 10)

Before: run with only the disc and the installed update (no decrypted overlay), the Pod
loaded 0 scripts and drew a black frame: the planets level's script writes the camera, and
the shell layer is drawn only under that camera. The Popit card itself already fell back
to `fallback_cards()`, but nothing of it could be seen. After: with no script loaded, the
Pod draws with the camera the planets script comes to rest at, so Sackboy, the pod
computer and the Popit card show where they do with the scripts.

## Why
- All 426 compiled scripts (`FSHb`) are in the encrypted `USRDIR/patches/patch.edat`; the
  disc FARCs have none (D133). Decrypting it at run time needs Sony EDAT keys, which the
  app may not carry (owner rule, 2026-10-18), so every scripted mode needs a built-in
  fallback when the scripts are absent.

## D411: resting camera fallback
- `level_play::POD_REST_CAMERA`: eye (0, -250, 1130), direction (0, 0.132, -0.991), up +Y,
  1 rad vertical field of view, near 10, far 100,000 (level units). MEASURED: the script
  camera's "at rest" line from tick 452 with the decrypted scripts.
- Used only when the level scripts loaded 0 scripts and the Pod's shell layer is on
  (`--pod-camera N --pod-shell HEX`); it stands in for the script's camera in
  `script_camera`, as `CameraSource::Script`. With the scripts loaded nothing changes.
- One line says so at load: `Level scripts: none loaded (patch.edat not decrypted) ...`.

## MEASURED (viewer only)
- Command: the D409 MEASURED command with `--snapshot d79-inventory-0133.json` (disc +
  update, no `patch_edat.farc`) and `--game-dir "<disc>;<update>"`.
- Before: black frame, only the "Close Popit" pill. After: the tent, pod computer, Sackboy
  and the five-button play card with its "Customize Character" tooltip, in the same place
  as the run with the decrypted overlay.
- The Popit card log reads `fallback constants (no Poppet script ...)`; its buttons,
  order and focus match the script's card.

## Open
- Without the scripts the planets (Earth, Moon) are not placed and the shell's outside
  shows other pictures; the camera's dolly in is not played (the fallback is the rest
  pose from the camera's first frame).
- Other scripted modes (Story screen, Rookie Test) need their own fallbacks.
