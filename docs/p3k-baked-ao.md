# P3k: baked AO from the backdrop lightmap

Question: where does LBP2 keep baked AO? Answer (PC, oracles/results/2026-10-10-baked-ao.md,
CONFIRMED from data and fpdec): there is no per-vertex AO on PS3. The backdrop static mesh
(SMHb) names a lightmap (`StaticMeshInfo.lightmap`, B8 GTF up to 4096 x 4096, 13 mips) that the
base fragment variants sample at TC7 (INFERRED = SMH UV1, the unique 0..1 unwrap) as
`ao = map * bakedAOScale + bakedAOBias`. The two scalars are PLevelSettings fields after
rimColor2 (file order: shadow amount, shadow blur, AO bias, AO scale, dynamic AO, DOF near);
Pod 51/97, 40/97 confirmed on a capture.

Change (only under `--lbp-lighting`, `--no-baked-ao` turns it off):
- `lbp-level`: `LevelSettings.baked_ao_bias/baked_ao_scale` parsed (main block and presets).
- `lbp-preview`: `load_backdrop` decodes lightmap 0 to one byte per texel (`Lightmap`, top mip,
  charged to the level budget) and every `ScenePayload::Backdrop` batch carries the same `Arc`;
  a decode failure is counted in the backdrop errors as `lightmap: ...`.
- viewer: R8 texture with a box-filtered mip chain (trilinear, clamped) on binding 129/130,
  `ao_params` (scale, bias, on) on 131; the batch's `uv1` (the unwrap) is written to UV_1 when no
  material feature already took it, else "P3k: baked AO not drawn (UV1 is taken)".
- shader (revised after the PC review of #117): fpdec v0/v3 arithmetic is `ao = aomap.w x scale
  + bias`; ambient x ao; sun x shadowmap.x x ao; rim x shadowmap.x (CONFIRMED). A B8 texel reads
  as A = one in RPCS3, so a B8 lightmap on `aomap` would make `ao = scale + bias` constant; the
  lightmap texel therefore enters as `shadowmap.x`. The first draft used the texel as `aomap.w`
  and darkened Bravery Test to x0.2-0.45 (PC changed-pixel table).

INFERRED, unconfirmed: the lightmap binding to `shadowmap.x` (u14) rather than `aomap` (u8) (the
bind is not decoded; needs an RSX capture), the red channel as the texel, clamp addressing, the
preset whose scale/bias is used (the one that lights the level), baked shadow strength
(bakedShadowAmount/Blur, not applied), the gloss term following the sun visibility. Not rendered in
the cloud (no GPU, no game data).

PC verification wanted: gameplay-distance before/after (`--no-baked-ao`) on Bravery Test and
tutorial_23 against the RPCS3 frame; an RSX bind check for which unit the lightmap is on.
