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
- shader: `ao = sample(UV1).r * scale + bias` multiplies the ambient and sun terms inside the
  squared light factor; the rim is not occluded.

INFERRED, unconfirmed: ao scaling ambient and sun (not the rim or gloss), the lightmap being the
raw byte (remap 0xaae4) in the red channel, clamp addressing, the preset whose scale/bias is used
(the same one that lights the level; the selection rule is still unknown), baked shadow strength
(bakedShadowAmount/Blur, not drawn). Not rendered in the cloud (no GPU, no game data).

PC verification wanted: fmt/test/clippy on 1.99; `--level-report` backdrop errors empty for
Bravery Test (0001e795) and tutorial_23 (0001db4f); gameplay-distance before/after (`--no-baked-ao`)
against the RPCS3 frame: if the backdrop is too dark, try AO on the sun term only.
