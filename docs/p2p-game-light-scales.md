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

## P2r: output tint and gamma, one light set for the shell (2026-10-09)

The PC read the same capture again (local-analysis/results/2026-10-09-pod-renderdoc-tint-fog.md,
frame 5247, Pod closed). All 161 lit draws (Earth, Moon, pod.bin shell) share one constant set:
tintcol (4,4,4,1), fogccparams (0.240216,0,0,0), fogcol (0.646,0.409,0.228), and the preset 0
sun/ambient/rim values above. The Earth diffuse is SRGB DXT1 (decoded to linear on sample),
and the render target is plain RGBA8, so `sqrt(colour × tintcol)` is the only output gamma.

Change (`--lbp-lighting` only):
- The shader now ends `v = sqrt(diffuse × light² × 4)`, clamped to 1, and returns the linear
  value whose sRGB encoding is v, because Bevy encodes our output.
- The Pod shell is lit with the planets' preset 0 values. Its own light direction is kept.

| Rule | Label |
|---|---|
| tintcol (4,4,4) and the sqrt output into RGBA8 (this frame, all lit draws) | CONFIRMED_FROM_LOCAL_DATA |
| fc slot names (tintcol, fogccparams, fogcol), matched to brdf.cg | STRONG_INFERENCE |
| Shell draws use the planets' constants | CONFIRMED_FROM_LOCAL_DATA (eid 748) |
| tintcol 4 is constant, not level- or menu-driven | INFERRED (one frame) |

Not done: the luminance fog step (`fogcc = (lum + fogccparams.y) × fogccparams.x + 1`, lerp
to fogcol by the vertex fog factor), because the vertex fog factor is unknown. Bevy distance
fog stays as before. The shell's light direction is still its own level's, not checked.
