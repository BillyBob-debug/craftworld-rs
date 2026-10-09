# P2y: where do the Pod's live lighting presets come from?

2026-10-09 (America/Chicago). Question from the PC Ghidra read (oracles/results/2026-10-09-shading-ghidra.md):
the live game holds 3 presets for the Pod's planets (sun position (0.74296, 0.53306, 0.99904), FogNear 5804.64,
FogFar 58453.61, FogColor (0.6456, 0.4092, 0.2281)); our pod_planets.bin parse holds 1 preset (sun x 0.908,
fog (0.973, 0.764, 0.503) 15,083..84,227). D195 read the file to its end with 0 trailing bytes, so the parse
is consistent with the stored bytes.

Observations from the repo (no game data in the cloud):
- Only y and z of the sun position agree (0.533, 0.999); x, fog colour, FogNear and FogFar all differ, so the
  live container is not a re-read of the stored preset 0.
- `LevelSettings::find` takes the world Thing's holder, else the first holder; other holders are not read.
- pod_planets has no backdrop Thing (`Backdrop: none`), so a PLevelSettings in backdrop plan 0001d054 is only
  reachable if some other Thing instantiates it.
- The Pod script drives fog (SetDOFAndFog with FogBias) and view transitions, so the live values may be written by
  script natives after load, not by the file.

`cargo run -q -p lbp-preview --example settings_holders_probe -- SNAPSHOT GAME-DIRS LEVEL-PATH` lists every
holder (Thing index, stored preset count, main and preset values) and flags values matching the live ones.
Run on pod_planets and, if the plan is reachable, the plan's level path. Outcome decides: another holder
(read it), a script write (model it in the Pod script bindings), or a parse fault (fix the reader).
