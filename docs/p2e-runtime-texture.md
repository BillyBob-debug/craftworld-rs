# P2e: opt-in runtime texture for an unrendered material slot (2026-10-09)

## Question
The Pod shell mesh pod.mol (mesh GUID 00002f6c) primitive 2 draws untextured grey.
On the PC its material resolves as a Solid route (diffuse 0.118 grey, 1,822 triangles)
with "Other required texture slots unrendered: [0]" (CONFIRMED_FROM_LOCAL_DATA, PC
viewer label). The material's sampler for slot 0 is reachable from an output, but the
slot is not filled from the material file, so the game fills it at runtime. The likely
texture is one of the Pod display textures 000027fb or 00004277 (the swirly side-panel
art). That is STRONG_INFERENCE from the PC; which one (if either) is UNKNOWN.

## Change
New viewer flag `--runtime-texture MESH:SLOT:TEXTURE` (repeatable, at most 16 rules):
- MESH and TEXTURE are eight-digit hexadecimal GUIDs (nonzero); SLOT is 0-7.
- Two rules giving different textures for the same mesh slot are refused.
- The flag needs a level (`--level-guid` or `--level-path`). It applies to the level
  and to the `--pod-shell` level, which is loaded by a second
  `load_level_scene_with_options` call with the same `LevelSceneOptions`.

Matching rule (`lbp_preview::runtime_texture::RuntimeTexture::matches`): a primitive
group with a non-empty triangle range is replaced when
1. its model mesh was named by GUID and the GUID equals MESH,
2. its material loaded as a Solid route, and
3. that material lists SLOT among its unrendered required texture slots.

A group that matches 1 and 3 but already has a texture route is left alone and counted
("group already textured"). Groups that share one material share one replacement.

Where: `lbp-preview/src/runtime_texture.rs` (parsing, matching, replacement) and the
model-mesh loader in `level_scene::load_level_scene_with_options` (before the D147
joint skin copies, so they inherit it). The replacement is an ordinary texture-route
`Asset`, so the viewer's `prepare`/`prepare_buckets` draw it unchanged. The texture is
decoded with `lbp_texture::decode_reader` through the same resolve and read path as the
other material textures. The mesh bytes are re-read for UV0 only when a group matches.
Retained bytes are charged to the level's material budget.

Display policy (INFERRED, not game parity):
- UV channel: mesh UV0, untransformed (identity scale/offset, no material sampler
  transform, since the Solid route did not select the slot-0 sampler).
- Addressing: Repeat on both axes, linear filtering.
- Tint: none. The texture replaces the solid colour (white base colour), and the
  replaced colour is kept only in the group label. Multiplying by 0.118 grey would
  make the panel near black. Where the game uses the slot (diffuse, emissive or a mask)
  is UNKNOWN.

Status lines (one per rule, shown with the scene counts and in `--level-report`):
`Runtime texture 000027fb on mesh 00002f6c slot 0: N groups (opt-in; UV0, Repeat,
untinted: inferred; game binding unconfirmed)`, then `not applied K groups: <reason>`
lines. Counting unit: primitive groups of distinct loaded meshes; a mesh drawn by
several Things counts once. With `--pod-shell`, the shell plan's lines replace the
level's, so the level's rule lines are added again with a `Main level:` prefix. The
group label reads `Runtime texture 000027fb on mesh 00002f6c slot 0 (opt-in
--runtime-texture; game binding unconfirmed) • UV0 untransformed, Repeat, untinted
(inferred) • W×H format • replaces solid diffuse [...]`.

Failures stay explicit. If the texture GUID does not resolve or decode, or UV0 is
missing or does not match the vertex count, the groups stay Solid and the status line
gives the reason (for example `texture 000027fb: Unresolved: GUID`). Without the flag
nothing changes: `runtime_textures` is empty and no lines are added.

## Evidence and limits
- CONFIRMED_FROM_LOCAL_DATA: slot 0 is an unrendered required slot of the Solid route
  of 00002f6c primitive 2 (PC label).
- STRONG_INFERENCE: the slot holds a Pod display texture (000027fb or 00004277).
- INFERRED: UV0, identity transform, Repeat, untinted replacement.
- UNKNOWN: which texture, the game's sampler transform and wrap for the runtime slot,
  and how the material combines it.
- Not covered: D119 backdrop meshes and SHA-1-only mesh references (rules match mesh
  GUIDs of model meshes), the single-mesh view, and the legacy `lbp-viewer`.
- Not tested here: no game data in this environment, so no capture; the PC check below
  is still open.
- Tests (synthetic only): `runtime_texture::tests` covers parsing, conflicts and the
  rule cap, the matching rule, shared-binding replacement and empty groups, and failure
  reasons. The viewer test `runtime_texture_flag_is_repeatable_and_refuses_conflicts`
  covers repeating the flag.

## PC check
`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --runtime-texture 00002f6c:0:000027fb --qa-hide-ui --qa-capture local-analysis/NEW.png`

`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --runtime-texture 00002f6c:0:00004277 --qa-hide-ui --qa-capture local-analysis/NEW2.png`

- In the log or panel, expect `Runtime texture ... on mesh 00002f6c slot 0: 1 groups`
  (or more if other primitives share the material) and no `not applied` line.
- Compare both captures with the RPCS3 Pod reference to choose the texture. If the art
  is mis-scaled or mirrored, the UV0/identity choice is wrong. Record the outcome before
  making either binding a default.
