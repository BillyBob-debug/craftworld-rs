# P2f: the game's Pod camera preset, and Sackboy's hold spot (2026-10-08)

## Question
D164 runs PlanetsWorld.UpdatePlanetsView for 300 ticks without faults. With the Pod
computer not in use the script sets the camera from (0, -250, 1130) to (0, 100, -1500),
up (0, 1, 0) (CONFIRMED_FROM_LOCAL_DATA, PC VM trace; the four float arguments of
Cam.SetCameraFromToUp are not reported yet). Which level's coordinates is that in, and
does it replace the fitted shell eye?

PC run of D163 (2026-10-08), closed view: Sackboy 0.46-0.54 w / 0.56-0.84 h, window
0.33-0.67 x 0.21-0.64 (RPCS3 0.35-0.67 x 0.28-0.69). Open: only the top of Sackboy's
head shows (centre 1.12 h, RPCS3 0.90 h) and the tent arch is still visible (RPCS3: none).

## Change
- `PodRig` shell eyes may turn (`shell_rot`); `with_game_camera(from, to, bounds)` sets
  the closed shell eye from a from/to/up(+Y) preset and resets `scale` from the shell's
  depth along that view. The root becomes `X -> P + scale R^-1 (X - S)`, which keeps
  the shell's projection from its own eye (test `p2f_game_camera_turns_the_shell...`).
- `--pod-shell-camera FX,FY,FZ,TX,TY,TZ` (shell level units) applies it. Testing the
  preset against the shell is a HYPOTHESIS: the camera may belong to the planets view
  only, since PlanetsWorld also moves Things (Thing.SetWorldPosition).
- The status prints both shell eyes in shell level units, so the fitted eye can be
  compared with the preset even without the flag.
- Sackboy holding the controller now has his own spot: `fit_figure_open` puts his centre
  at `SACKBOY_OPEN` (0.50/0.90) as far in front of the open eye as he stands in front of
  the closed one (same distance INFERRED). Previously he kept the standing spot.

## Not changed
- The open eye is still fitted to the DualShock (0.50/0.86, width 0.17); the PC measured
  0.20 width, and the tent shows. The "in use" camera preset (GetPodComputerInUse true)
  should replace it once the PC has it.

## PC check
`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f --play-animation 0001a697 --pod-hold-animation 000042b2 --animation-frame 0 --qa-hide-ui --qa-capture local-analysis/NEW.png`
- Repeat with `--pod-open`, and both again with `--pod-shell-camera 0,-250,1130,0,100,-1500`.
- Report the "Shell eyes" and "Figure fitted" lines and the window/pad/Sackboy fractions.
