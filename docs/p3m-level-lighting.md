# P3m: a level's lighting is its stored presets blended by PWorld `lighting`

2026-10-10 (America/Chicago). Follows P2w (level default) and P3k (baked AO). Evidence: live RPCS3
reads of the lighting block at guest 0xe3a7f0 (tutorial_23, Bravery Test, the blank Moon level, the Pod),
local-analysis 2026-10-10-live-level-lighting.md, and the stored values read by `settings_holders_probe`.

Before: a level was lit from its main PLevelSettings block (Lambert preview) or from preset 0
(`--lbp-lighting`). Preset 0 was wrong for Bravery Test (cyan cast, PR #102 "NOT OK"), baked AO took
preset 0's scale/bias, and the main block is the defaults block (fog (0.3, 0.5, 0.7) from 1000 to 4000
units): under `--preview-lighting` that fogs any level larger than 4000 units to flat blue.

After: the effective settings are
- `n` stored presets spread evenly over PWorld `lighting` (0..1): position `lighting x (n - 1)`, linear
  blend of the two neighbours (colours, multiplier, exposure, fog colour and range, sun position, AO
  scale and bias); one preset is used as is, no presets keep the main block;
- PWorld `darkness`: exposure and fog colour x `(1 - darkness)`.
The existing LBP mapping (sun = H x multiplier x 0.3 x e, ambient 0.45 e, rims 0.6 e, .w likewise) then
applies unchanged, and the baked-AO tweak follows the same settings. `--lighting-preset N|main` still
force one block; the Pod keeps its backdrop-plan preset 0.

## Evidence (offline prediction vs live block, 4 digits on every component)
| level | PWorld lighting / darkness | presets | position | live check |
|---|---|---|---|---|
| tutorial_23 | 1.0 / 0 | 3 | 2.0 (preset 2) | sun, amb, rim, rim2, fog, aotweak all match |
| Bravery Test (0001e795) | 0.6 / 0.19 | 3 | 1.2 (80% preset 1, 20% preset 2) | exposure 1.0417 x 0.81 = 0.8437 (live amb.w 0.3797 = 0.45 e), sun .402, fog x 0.81, aotweak (1.0, 0.1979) |
| blank Moon level | 0 / 0 | 2 | 0 (preset 0) | sun, amb, rim, rim2 match; fog differs (see below) |

Check script: `local-analysis/lighting-presets/verify_live.py` (not committed).

## Not settled
- Spacing of 4 and 5 presets (35 and 17 readable levels) is the same rule, INFERRED; 3-preset levels are
  the only ones measured.
- Which part of `darkness` goes to exposure and which to fog colour is fitted on one level (Bravery).
- PWorld `colourCorrection`, `fog`, `fogTint` are read but not applied; the live Moon and Pod fog colours
  differ from the preset's (script or create-mode driven), so fog colour there is not predicted.
- One readable level stores no presets (england_garden_cc_skateboard); it uses the main block, INFERRED.
- Level-time changes (scripts moving `lighting`) are not modelled; the value is the file's.
