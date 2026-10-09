# P2p: game light scales in `--lbp-lighting`

Date: 2026-10-09 (Central). Evidence: the PC's first RenderDoc capture of the Pod in RPCS3
(local-analysis/results/2026-10-09-rsx-renderdoc-pod.md, not committed).

## Finding

The Pod shader constants are exactly pod_planets' preset 0 colours (rim below/above,
ambient, sun) in .rgb, each with a scale in .w: rim 0.730, ambient 0.547, sun 0.372.
With preset 0's multiplier 1.0206 and exposure 1.2165 these fit, within 0.001:

| Constant | .w | Fit |
|---|---|---|
| sun | 0.372 | 0.3 x multiplier x exposure = 0.3725 |
| ambient | 0.547 | 0.45 x exposure = 0.5474 |
| rim (both) | 0.730 | 0.6 x exposure = 0.7299 |

## Change

`PreviewLight::lbp_shaped` multiplies the stored colours by these scales
(`GAME_SUN_SCALE` 0.3, `GAME_AMBIENT_SCALE` 0.45, `GAME_RIM_SCALE` 0.6, each x exposure;
the sun also x multiplier, as before). `--preview-lighting` is unchanged.

| Rule | Label |
|---|---|
| Constant .rgb = preset 0 colours, .w = 0.730/0.547/0.372 (this frame) | CONFIRMED_FROM_LOCAL_DATA |
| Split into fixed constant x exposure (x multiplier) | INFERRED (one level, one frame; exact to 3 digits) |
| The shader applies .w to .rgb | INFERRED (consistent with the brdf.cg reading; check the decoded Pod shader) |

## Open

- Light position: the game's x is 222887 against ours 272370 (preset 0 sunPos x 0.908 x
  sunPositionScale 3e5); y and z match. 222887 / 3e5 = 0.743. Not explained; unchanged here.
- Ghidra (PC: local-analysis/ghidra/project) can confirm the scale constants and the sun
  x by finding where suncol/ambcol/rimcol are uploaded.
- Not tested here: needs a PC capture against RPCS3 (D188 baseline: room 1.9x too bright,
  globe 0.7x too dark).
