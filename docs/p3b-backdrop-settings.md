# P3b: Pod lighting from the backdrop plan's PLevelSettings

Evidence: probes #104/#105 (`settings_holders_probe`) found that the live Pod values
(sun x 0.743, ambient, rims) equal preset 0 of the PLevelSettings holder inside backdrop
plan 0001d054 (3 presets), not the pod_planets level's own holder (1 preset). CONFIRMED
for the Pod; INFERRED for ordinary levels (needs a Moon-level memory read).

Change: `LevelScene.backdrop_presets` holds the plan holder's presets (read in
`load_backdrop`). With `--lbp-lighting` in Pod mode (`--pod`), preset n of that list drives
the planets scene and the shell instead of the level's own holder. Without a backdrop plan
or outside the Pod, the old preset logic is unchanged.

Not done: fog start/slope still come from the fitted constants (#103 moves them to data;
the Pod script FogBias stays the only open/closed difference).
