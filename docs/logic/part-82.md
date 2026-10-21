# Logic, Part 82

See docs/logic.md for Parts 1-49.

## Part 82: a piston driven by its band (D1181)

Part 81 decoded the piston's linear row 0x96178c: its `delta` and `err` edges, the
strength cap on the `delta` edge and the k cap k x max(|err|, 10) on the 1.5 x `err`
edge. Until now the piston was a slider held at its rule's length every tick.

What we do: `PistonDrive::slide_to` (crates/lbp-sim/src/lib.rs) gives the slider motor
its target rate and push from the row's band, as `band_to` does for motors (Part 80).
`delta` is the length rule over the next frame of the clock, and `err` is the next
length less the measured anchor distance along the slider. `delta` is cut to between 0
and `err`, except for a piston wired in mode 1 or 4. Below the band it drives to the low
edge and above it to the high edge, each with that edge's cap. Inside the band it does
not push. `drive_pistons` measures the ends along the slider as avian does (anchor
separation dotted with body 1's slider axis) and sets limits to the rule's range, base
to base + range (`SceneJoint::range`).

INFERRED: only a piston that is not stiff and has a strength slides (`slides`). A stiff
piston, or one with strength 0, is held at its length as before. The stiff flag's use in
the solver is not decoded. A first run that let every piston slide inside its saved
length (the spawn limits, min = max) pinned the pistons whose rule differs from the saved
length. It also left the strength-0 ones with no push. Sackboy then landed 150 units
higher at tick 990 (y 5,338.4 instead of 5,188.6).

Route (Part 57) unchanged: x -2,870.3, y 5,188.6 at ticks 1,000 and 1,400; x -2,440.6,
y 5,400.9 at ticks 1,600 and 2,007. Parity 5 pass.

Open: where the solver reads the stiff flag, and whether a strength-0 piston is rigid in
the game.

Tests: `d1181_a_piston_drives_its_slider_within_its_band`,
`d1181_only_a_springy_piston_with_a_strength_slides`.
