# Logic, Part 67

See docs/logic.md for Parts 1-49.

## Part 67: allowInOut followers change layer (D1166)

Part 53 found that a configuration 11 tweak with allowInOut (+0xe4) set runs a layer
move before its follower rows. That move was not modelled.

FOUND (0x1c7ed0, EBOOT 01.33, the block inlined at 0x1c8250-0x1c83b4): every tick,
when allowInOut is set, the tweak's Thing +0x7f is 0 and the Thing's input (+0x60) is
null, 0 or on, the root is the Thing's parent (+0x5c, else the Thing). Its body (+0x64)
must exist and not be held (*(body+0x140) == 0).
- The target is found with the follower search 0x1c3e04, using minRange, maxRange,
  angleRange, colour, followKey and slot. If the target has a parent (+0x5c), that
  parent is used.
- The target's z comes from 0x3becc (+0x38). The root's z is the translation
  `*(*(root+0x2c)+0xc)+0x70`, lane 2.
- Each z is turned into a layer: trunc((z + 679.0) / 200.0), with constants at
  0xd983e4 and 0xd98048.
- d is the target's layer minus the root's layer. While d != 0 the block calls
  0xcd770(d, 0.0, root), which is Part 47's glued-group move of d layers. It stops on
  success, stops when |d| < 2, and otherwise moves d one step towards 0.
- The follower rows (0x1c7fc0) then run as usual.

lbp-sim:
- `scene::layer_of(z)` gives the layer, and `LayerTweak::delta` now uses it.
- `MoveTweak.in_out` holds allowInOut.
- `MoveTweak::in_out_step(z, target_z, fits)` returns the group's z step (d × 200), the
  nearest that keeps the group in Part 47's z band and passes `fits`, or 0.
- INFERRED: the Thing input test and the held-body test are left to the host, as in
  Part 47.

Not hosted: the viewer still moves no glued groups (Part 47), so nothing calls
`in_out_step` yet.

Test: `d1166_in_out_followers_move_to_their_targets_layer_or_as_near_as_fits`.

Census (local-analysis/logic/d1166/census1.json, `chip.follow_in_out`): 149 followers
have allowInOut. They are in 29 levels, none in the Rookie Test. The route is
unchanged and parity passes 5/0.
