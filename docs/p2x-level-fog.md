# P2x: a level's own FogNear/FogFar and FogColor in the game fog

2026-10-09 (America/Chicago). Plan: `lbp2/research/shading-lighting.md` item 3. Evidence: PC
Ghidra read, `lbp2/oracles/results/2026-10-09-shading-ghidra.md` (relayed 2026-10-09 evening).

Before: under `--lbp-lighting` every scene used the Pod's fitted fog constants (start 25,810
closed / 5,810 open, span 52,648, colour (0.646, 0.409, 0.228)), whatever the level's own settings.

After: a level scene uses its own settings: fog start = FogNear, span = FogFar - FogNear,
colour = FogColor divided by its own w when w is non-zero. Pod scenes (shell layer,
`--pod-game-camera`, `--pod-camera`) keep the live-game constants.

Why the Pod is special (PC Ghidra read): the vertex shader's fog coordinate is
`(forward depth - (FogNear + bias)) / (FogFar - FogNear)`; the live Pod preset 0 has FogNear
5,804.64, FogFar 58,453.61 and FogColor (0.6456, 0.4092, 0.2281), matching the capture to 4-5
digits. The closed/open start difference of 20,000 is the Pod script's SetDOFAndFog FogBias x 2,
not engine code. pod_planets.bin as we parse it holds 1 preset (fog 0.973, 0.764, 0.503, 15,083-
84,227) while the live game holds 3, so the stored values and the live ones differ on the data
side. Until that is explained the Pod keeps its fitted constants.

Not done: script-set FogBias and the open/closed interpolation for levels (a level script that
calls SetDOFAndFog would shift the start); the luma CC (0.240216) is still the Pod's constant.
INFERRED for a Moon level until the capture or memory read.
