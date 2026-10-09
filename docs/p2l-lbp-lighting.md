# P2l: opt-in LBP-shaped preview lighting (`--lbp-lighting`)

Date: 2026-10-09 (Central). Research basis: project research note `pod-fidelity/README.md`, method 3.

## What it does

`--lbp-lighting` turns preview lighting on (like `--preview-lighting`) and swaps the
D121 Lambert factor for a factor shaped like the LBP2 surface lighting. The default
(no flag) is unchanged: unlit.

Per fragment, with `n` the facing normal (D121 double-sided rule) and `v` the
direction from the surface to the eye:

```
f = ambient * (n.y * 0.15 + 0.5)
  + sun * max(n . L, 0)
  + lerp(rim, rim2, (n.y + 1) / 2) * clamp(n . -v + 0.9, 0, 1)^2
colour = texture * f^2
```

- `ambient`, `sun` (sun colour x sun multiplier), `rim`, `rim2` are the level's stored
  `CLevelSettings` values, **unfitted** (D121's fit to a target brightness is skipped).
- `L` is the stored sun direction, turned by the Pod planets root rotation so it stays
  in the level's frame while the camera rig turns the level (re-applied during the dolly).
- `f^2` is our reading of the game's square-then-sqrt combine in a linear pipeline.

Code: `lighting::PreviewLight::{lbp_shaped, lbp_factor}`, shader branch
`light_dir.w = 2` in `dual.rs` (rims at bindings 112/113), wiring in `apply_lighting`.
Test: `p2l_lbp_shape_uses_raw_values_hemisphere_and_grazing_rim`.

## Evidence

| Rule | Source | Label |
|---|---|---|
| Hemisphere ambient `(N.y*0.15+0.5)*ambcol`, sun `saturate(-N.L)*suncol`, rim `pow(saturate(N.V+0.9),2)*lerp(rimcol,rimcol2,(N.y+1)/2)` | Toolkit `lib/cwlib/src/main/resources/shaders/templates/cg/brdf.cg` (community reconstruction), described in our research notes; implemented independently from that description, no code copied | CONFIRMED_FROM_COMMUNITY_CODE (shape only) |
| Square-then-sqrt combine read as `texture * f^2` in a linear pipeline | Same file's combine (`light^2`, `sqrt` of the result) | INFERRED |
| Stored settings feed the uniforms directly (sun x multiplier, ambient, rims) | None: the CPU mapping from PLevelSettings (preset blend, exposure, darkness) to shader constants is unknown | INFERRED |
| Directional sun instead of the game's point sun at sunPosition x sunPositionScale | Simplification | INFERRED |
| Rim `rimcol` is hard-coded in the reconstruction; we use the stored `rimColor` | Research note | UNKNOWN which LBP2 uses |

## Not done / limits

- No specular, AO, shadows, sprite lights, fog-by-luminance, tint, DoF or colour grading.
- Pod shell layer materials are lit only if they are in the scene list (unchanged from D121).
- Values must be checked against an RPCS3 RenderDoc capture (`fc[n]` per draw), method 1.
- Not tested on the PC against game data from this cloud session; only unit tests and
  the WGSL parse test ran here.
