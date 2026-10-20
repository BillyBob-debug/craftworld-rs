# Logic, Part 55

See docs/logic.md for Parts 1-49.

## Part 55: touching layers do not collide (D1153)

After Part 54 the route stopped at x -2,467.6: lift 2643 (follower 2652) never left. Its
shapes sit at z 0 with half depth 90 and rest in xy against Thing 1330, a thin shape
stored at z 99.99994 with half depth 10. Their depth ranges only touch at 90 (100 less
float rounding), but `ZSlab::overlaps` used a strict |dz| < sum, which 99.99994 < 100
passes, so the lift collided with a shape one layer behind it. Gyroscope 1320 (body 1319,
z -0.00012) overlaps Thing 1330 in xy the same way.

NOT FOUND, the game's pairwise depth test. The PPU readers of PShape thickness (+0x7c,
182 of them) hold no pairwise test; the nearest is 0x3632b4, a default-thickness check
`fabs(thickness - const) <= const`. The broad phase likely runs on the SPU: the EBOOT
carries 28 SPU ELFs at file offsets 0xcb1080 to 0xd43780 and Ghidra has no SPU language.

Model (INFERRED): slabs collide only when their depth ranges overlap by more than
`SLAB_TOUCH_EPSILON` (0.01), so touching slabs stay apart through z rounding. This keeps
the existing INFERRED rule of docs/sim-physics.md ("slabs that only touch do not
collide") and only adds the tolerance. Tests: crates/lbp-sim/tests/slab.rs.

Census (`level_physics --all`, local-analysis/logic/d1153/census-all.json): collider
pairs on different bodies whose xy boxes overlap and whose depth ranges are within the
tolerance of touching, so they collided before and do not now.

| levels read | levels with such pairs | pairs | pairs with a dynamic body |
|---|---|---|---|
| 358 | 218 | 20,398 | 19,739 |

Rookie Test alone has 69 (67 with a dynamic body). Most per level: level_2b 531,
minigame_2 432, introduction 413.

Route: the x -2,467.6 stop is gone. Sackboy now stands at x -1,092.7, y 5,238.4 on
ticks 2,006 to 2,008, inside the xy range of body 1951 (Things 2472, 2473 and 2669,
x -1,140.6 to -745.0, z 0, half depth 90), which carries 3 movers and 3 spins. Not looked
into yet.

Not modelled: the game's exact depth test (SPU); a tolerance relative to the slab size.
