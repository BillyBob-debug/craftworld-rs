# D192: material-less primitives are hidden (game skips their draw)

Date: 2026-10-09 (America/Chicago). Follows D188 (docs/d188-absent-material-primitives.md),
which drew these groups neutral because the game's behaviour was `UNKNOWN`.

## Evidence

PC Ghidra pass on the decrypted 01.33 EBOOT (read only; project local-analysis/ghidra,
scripts and output under local-analysis/ghidra/nullmat/, not committed). Full note:
project oracle file `lbp2/oracles/results/2026-10-09-null-material-primitives.md`.

- Per-primitive mesh draw (function at 0x33c814, loop 0x33c978-0x33ce88, stride 0x20
  over the RMesh primitive view): the test at 0x33ca88 sends a null material straight to
  the loop increment. Neither the material bind (0x174aa0) nor the RSX index draw is
  reached. No default material GUID/hash is substituted in this path. **CONFIRMED**
  (executable investigation, draw path).
- A whole-mesh branch (caller 0x348580 passes -1 when its flags have bit 0 clear, e.g.
  0x489c / 0x401c) draws the full index buffer without per-primitive materials, so
  material-less triangles may still reach a depth or shadow pass. **INFERRED** (pass not
  identified; not drawn here).
- Not traced: whether the loader can substitute a material for a null descriptor before
  the draw loop. The explicit null tests suggest nulls reach the loop.
- Visual cross-check target: 0001d050 (cosmos_room_default_lbp2), placed in Pod planets
  00018fdb through backdrop plan 0001d054.

## Change

- `lbp-bevy-viewer` `prepare` drops the triangles of material-less groups
  (`drop_materialless`) before bucketing, in single-mesh preview and level scenes. Their
  details line reads `Hidden: Material-less ...`; they stay counted in
  `materialless_groups` and outside the diffuse denominator; Complete is still not
  confirmable while one is present (depth/shadow use inferred).
- `lbp-preview` level scenes: `grouped_neutral` ignores material-less groups, so a Thing
  whose only unbound group is material-less no longer counts as neutral.
  `MATERIALLESS_REASON` and census evidence text now say the group is not drawn.
- `--level-report` prints, under each mesh, `neutral (K groups): <reason>` for every
  unbound group, material-less ones included (`level_scene::neutral_reasons`).
- No flag restores neutral drawing; censuses are unchanged in counts
  (`--materialless-blocking` still gives the pre-D188 classification).

## Bravery Test (PC report at ea55c1e, before this change)

35 neutral Things, all carrying one material-less group: 00019e98 (22 items; also one
`folded sampler port inputs under a nested instance tint` group), 00018d2b (10 + 1
colour variant), 000150c3 (3), 0001ece6 (2), 000150d6, 000150fe, 000150dc
vin_crane_machine (1 each). Expected after D192: only the 00019e98 Things stay neutral
(folded-sampler group). Not measured in this container (no game data).

## Not tested

No game data here: synthetic test only
(`level_scene::tests::materialless_groups_are_hidden_not_neutral_d192`). The PC level
report and a close gameplay-camera capture verify it.
