# P3e: PWorld lighting fields (which backdrop preset does a level use?)

Findings so far (PC, #105/#110): the Pod's live light is preset 0 of backdrop plan 0001d054's
PLevelSettings; the Moon "Create!" level's live block is preset 1 of plan 0001a2fe
(env_tutorial_plain_background) with each stored colour x0.3*mult*exposure (sun),
x0.6*exposure (rims), ambient x0.45*exposure (Pod proves ambient does not take mult; Moon's
mult 1.5 makes the two coincide), fog colour unscaled. The rule that selects the preset is
UNKNOWN (Pod 0 of 3, Moon create 1 of 2).

Question for this change: does the level itself say which? The PWorld tail stores
`lighting`, `colourCorrection`, `fog`, `fogTint`, `darkness` f32 and `target`/`from`
GlobalSettings (5 f32 each) plus a blend factor. They were skipped; `World` now keeps them
(`lighting`, `gs_target`, `gs_from`, `gs_blend`; field meaning and order are UNVERIFIED
community layout). `settings_holders_probe` prints them with the backdrop plan GUIDs.

Run on the Pod level, the Moon blank level, Bravery Test and tutorial_23, then compare the
values with the preset the game was seen to use (Pod 0, Moon 1).
