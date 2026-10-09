# P2q: material bump maps and gloss under --lbp-lighting

Materials were diffuse-only: bump (output port 3) and specular (port 2) were parsed and
never drawn (pod-fidelity README item 4). This milestone draws the two simplest shapes of
each under the LBP-shaped preview lighting (P2l/P2p).

## What is drawn

- **Bump:** output port 3 wired straight (port 0, unswizzled) from one static texture
  sampler. The sampler's own slot, UV channel, transform and wrap are used; the map is
  uploaded as linear data and sampled on UV1. Offset = (texel.yw - 0.502) × BumpLevel; the
  facing normal is tilted by that offset in a cotangent frame from screen-space position
  and UV1 derivatives.
- **Gloss:** output port 2 wired straight from one static, unwired COLOR. brdf.cg combine
  `diffuse·light² + 2·light.w·light²·spec`, spec = colour × pow(saturate(R·L),
  CosinePower), R the mirrored view ray. light.w sums the game's per-light scalars (the
  .w of suncol/ambcol/rimcol: 0.3·multiplier·exposure, 0.45·exposure, 0.6·exposure,
  Ghidra-confirmed on the PC) with the same hemisphere, N·L and rim weights as the colours;
  they now ride in the .w of the light uniforms.
- Only with `--lbp-lighting` (the shader branch is `light_dir.w = 2`); on by default
  there, `--no-bump-gloss` turns it off, `--bump-gloss` loads it without LBP lighting
  (no visible effect then, counts only).
- Not drawn and named per primitive (`P2q surface: ... not drawn (reason)`): swizzled or
  multi-wire ports, gloss from a texture or a graph, bump on a draw whose UV1 already
  carries a second diffuse texture, unreadable bump textures.
- The P2j wall-art override keeps window_glassy's own extras (tissue_paper_bump on
  slot 0, gloss 0.6, CosinePower 66).
- `--level-report` prints `P2q surface: B of N displayed draws bump-mapped, G glossy`.

## Evidence

| Rule | Label | Source |
| --- | --- | --- |
| Output ports 2 specular, 3 bump | CONFIRMED_FROM_COMMUNITY_CODE | Toolkit BrdfPort, docs/material-graph.md |
| BumpLevel / CosinePower prefix scalars | CONFIRMED_FROM_COMMUNITY_CODE | Toolkit RGfxMaterial, docs/material-prefix.md |
| Bump offset (.yw - 0.502) × BumpLevel; specular pow(saturate(-R·L), CosinePower), default 22 | CONFIRMED_FROM_LOCAL_DATA | D196 decode of the Pod gmats' own shaders (pod-fidelity README) |
| Which of .y/.w is the U and V tilt, and their signs | INFERRED | none; checked by eye only |
| Cotangent frame from derivatives instead of the game's stored tangents | INFERRED | stand-in; game tangents are decoded but not uploaded |
| Gloss combine `2·light.w·light²·spec` | CONFIRMED_FROM_COMMUNITY_CODE | Toolkit brdf.cg reconstruction (pod-fidelity/ghidra-and-lbp-engine.md); the Pod shader's own combine not yet compared |
| .w of each light = its scale (vec4 multiply) | CONFIRMED_FROM_LOCAL_DATA | PC Ghidra read of the 01.33 EBOOT (local-analysis/results/2026-10-09-ghidra-light-constants.md) |
| Shadow and AO factors in light.w | not applied | treated as 1 |
| Untrusted scalars: BumpLevel clamped 0..16, CosinePower 1..4096, non-finite → 0 / 22 | implementation guard | — |

## Tests

- `p2q_routes_take_one_bump_sampler_and_one_static_gloss_color` (lbp-preview).
- `p2q_untrusted_scalars_are_clamped_or_defaulted` (lbp-preview).
- `authored_shader_validates_with_every_branch_enabled` now covers the bump and gloss
  code (lbp-bevy-viewer).

## Not tested

No game data in the cloud session: no counts, no capture. For the PC: `--level-report`
counts on pod_planets and the Pod shell, and one Pod capture with `--lbp-lighting`
against `--no-bump-gloss`. If the bump looks inside out, swap or negate the .yw mapping.

## Next

Glass and alpha modes (window_glassy alpha_mode 4), then mipmaps (G6), then depth of field.
