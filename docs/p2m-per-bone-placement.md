# P2m: per-bone placement, LBP lighting defaults to preset 0, rim facing the eye

Date: 2026-10-09 (Central). Evidence: PC runs D185 (`pod_globe_extent`) and D201
(captures with `--lbp-lighting`, main block vs preset 0), local-analysis only.

## Per-bone placement (`--per-bone-placement`, opt-in)

POS model Things whose RENDER_MESH lists two or more boneThings get a per-item mesh
copy with each vertex at POS(boneThings[j]) x inverse bind(j) for its bone j (linear
blend over its weights, the D147 `skin_span_positions` rule), expressed in the item's
frame so the item matrix and every placement built on it stay. Bones without a placed
Thing keep the item matrix; items whose bones all match it are left unchanged.
Code: `level_scene::skin_bone_things`, `MeshSource::BoneSkin`, counts `per_bone_*`
(one QA line). Test: `p2m_bone_things_place_each_vertex_by_its_bone`.

| Rule | Evidence | Label |
|---|---|---|
| The game draws pod_planets' Earth per bone: per-bone max radius 1679.3 vs the RPCS3 world radius 1681.8 (at scale 1), bone-0 rule 1848.4 (+10%) | D185 probe vs RPCS3 memory read | CONFIRMED_FROM_LOCAL_DATA (this mesh) |
| The same rule holds for other levels' boneThings models | none yet | INFERRED |

Default off until the PC measures the corpus change (items placed per bone, levels
affected). Under it the Earth's sea/land bones scale (1.352, 1.352, 1.165) instead of a
uniform 1.350. This explains about 10% of the 1.26x globe oversize; the rest is open.

## `--lbp-lighting` defaults to preset 0

When a level stores presets and `--lighting-preset` is not given, `--lbp-lighting` lights
from preset 0 (`--lighting-preset main` forces the main block). D201: with preset 0 the
globe core is L134 vs RPCS3 L124; with the main block L192 and the 1000-4000 fog washes the
frame sky-blue. INFERRED as the game's behaviour (the PWorld blend that selects it is
unknown).

## Rim term faces the eye

The rim term is now `sat(N.V + 0.9)^2` with V toward the eye: full on surfaces facing the
camera, 0.81 at grazing. D199/D201: RPCS3's globe edge is darker than its core (L113 vs
L124) while ours was brighter (L143 vs L134) with V away from the eye. The direction of V
in the game's shader (D196 decode: `(rim + rim_round)^2`, rim_round 0.9) is not yet
confirmed: INFERRED.

## Not tested here

Cloud session: unit tests and workspace clippy only. Needs a PC capture with
`--pod-game-camera --pod-open --window-16-9 --lbp-lighting --per-bone-placement` against
the D156 RPCS3 frame.

## P2n (2026-10-09): rim change reverted

D203 (PC, /mnt/project-files/2026-10-09-pod-per-bone-capture.md): with the eye-facing rim the
room measured L213 vs RPCS3 L75 and the Moon L140 vs L67 (the grazing rim of D201 gave
139 / 64). The rim term is back to V from the eye (0 facing, 0.81 at grazing). The darker
RPCS3 globe edge is left unexplained until the RSX capture gives the real constants.
Per-bone placement: Earth r 0.247 h vs RPCS3 0.260 (was 0.290); Moon 0.127 vs 0.118.

## P2o (2026-10-09): on by default under `--pod-game-camera`

PC corpus check (/mnt/project-files/2026-10-09-per-bone-level-report.md): 18,892 items placed
per bone and 1,113 unchanged across all 236 levels. bravery_test and crazy_inferno looked identical to
the default at the opening view. Per-bone placement is now on by default only under
`--pod-game-camera`, where the Pod Earth confirms it; `--no-per-bone-placement` turns it off
there. Elsewhere it stays opt-in: it copies a mesh per item (memory/load cost not measured)
and shows no visible gain.
