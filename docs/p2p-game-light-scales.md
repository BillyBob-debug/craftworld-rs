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

## P2s: every draw lit in LBP mode (2026-10-09)

The PC re-measure of P2r (PR #82 comment): planets 0.79-1.01 of RPCS3, but the room stayed at
139 against 75, the same value as D188 before any lighting change. A region that no lighting
change moves is most likely not lit at all. Draws whose generated smooth normals fail
`adapter::normal_trust` were kept unlit (D121). In the game every Pod draw is lit (all 161 in
the capture share the lit constants), so under `--lbp-lighting` those draws are now lit with
their face normal (flat), mode `light_dir.w = 3`. `--preview-lighting` is unchanged. The
lighting summary counts them as "lit flat by their face normals".

Label: INFERRED (the room's draws being the untrusted ones is a hypothesis from the
unchanged 139; the level report's count line confirms or refutes it).

## P2t: the game's fog, in output space (2026-10-09)

The PC traced the room's brightness to fog, not lighting (RenderDoc frame 14790, eid 2388; the
shell walls were already lit, "0 of 124 unlit", so P2s changes nothing in this view). We ran
Bevy DistanceFog after the output step, in linear space, toward the preset fog colour. The
game fogs in its sqrt output space, before display:

`v = mix(v0, (dot(v0, (0.3, 0.6, 0.1)) × 0.240216 + 1) × fogcol, saturate((depth - start) / 52648))`

with fogcol (0.646, 0.409, 0.228), the slope from vc16, and start 25810 (Pod closed) or 5810
(open). Under `--lbp-lighting` the shader now does exactly this, with depth the view-space
distance converted to level units by each layer's display scale. Start is interpolated by
the Puter opening progress. Bevy DistanceFog is dropped in that mode.

| Rule | Label |
|---|---|
| Fog formula, colour, CC 0.240216, slope 1/52648, starts 25810/5810 | CONFIRMED_FROM_LOCAL_DATA (frames 5247, 14790) |
| Start interpolated by Puter progress between the two frames | INFERRED |
| depth = view-space z, level units via each layer's normalization | INFERRED (planets also sit under the World transform's scale) |
| fogcol source (not a clean function of preset 0's fog colour) | UNKNOWN |

The PC's equivalent test patch measured the room at 1.02-1.10x of the game's main pass
(1.12x of the RPCS3 final frame; the rest is the game's vignette/DoF pass) and moved the
planets by at most 6%.
