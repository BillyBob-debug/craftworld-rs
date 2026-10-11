# D230: Moon movement, game vs ours (2026-10-11)

RPCS3 01.33, owner's blank Moon level, default camera, player 1 keyboard (focused input
granted for the job). Sackboy pos row at guest `0x34188580` (x, y); z word is not at +0x88
this boot. Per-tick reads at ~1 ms polling; the level tick word lags, so one distinct
position = one 30 Hz step. Data: `local-analysis/d230-moon/*.csv` (airrel-1, airrev-1,
groundrev-1, standjump-air-1; scripts `d211-moon/scripts/run2.py`, `d230-moon/seq.py`).
Ghidra (EBOOT 01.33): `local-analysis/results/2026-10-11-ghidra-sackboy-movement.md`.

| Quantity | Game | Ours before | Ours now |
| --- | --- | --- | --- |
| Gravity per tick^2 | 2.70 (every airborne step 2.69-2.71; Ghidra -2.7) | 2.693 | 2.7 |
| Walk accel / top (A key) | 3 / 30 (D key 29.34, keyboard quantisation) | 3 / 30 | unchanged |
| Ground reversal at 29.3 | -3.0 per tick straight through zero | same | unchanged |
| Air accel / cap | 2.4 per tick, cap 22.5; reversal at 29.2 also -2.4 per tick | same | unchanged |
| Stick released in air | speed kept, -0.009 per tick at 29 (ratio ~0.9997) | no drag | `AIR_DRAG` 0.9997 |
| Jump release | next vy = previous - 2.7 (24.83 -> 22.13) | same | unchanged |
| Landing, no stick | ground decel -3 per tick from touch-down | same | unchanged |
| Landing dip | -26.2, -35.0, -35.0, then -31.8 ... 0 over 15 ticks; same depth for 19 and 29 per tick impact | none | `LANDING_DIP` (render offset only) |
| Walk-speed analog, terminal fall, slopes, walls | Ghidra: world MaxVel 100/tick, slope limit ~55 deg; not measured in RPCS3 (blank Moon has no walls, fall from hover is ~400) | not modelled | unchanged |

The dip is applied to the figure's drawn position (`SackboyBody::controller.landing_dip()`),
not to the capsule, so collision and lanes behave as before. Impact-speed dependence below
18.9 per tick is untested.
