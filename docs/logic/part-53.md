# Logic, Part 53

See docs/logic.md for Parts 1-49.

## Part 53: the followers' up axis, local space and in/out (D1151)

Part 52 left three branches of the follower packer 0x957df4 unread. The serializer
0xb7834c names the bytes: +0x5c localSpace, +0xe4 allowInOut, +0xe5 allowUpDown, +0xf5
flee.

FOUND, allowInOut: a configuration 11 tweak with allowInOut set is not packed by
0x957df4; its caller sends it to 0x1c8250 instead. The lvsl at 0x9585ac does not use
+0xe5 (rA = 0); it loads from the stack at 0x70.

FOUND, the up axis: Y is the body's world-matrix row +0x50, turned by theta = the float
at tweak+0x220 (not negated) to a = (Y.x cos + Y.y sin, Y.y cos - Y.x sin, Y.z). With
allowUpDown off (0x9587ac) the error loses its part along a, normalised:
e -= a x (a.e) / (a.x^2 + a.y^2), and e = 0 when that length is 0; the code rejoins the
main path at 0x9585dc. INFERRED: that this keeps the follower moving in the plane across
its up axis.

FOUND, localSpace (0x9582b4, 0x9582c4): 0xcc484 builds c = u cos t + (u.y, -u.x) sin t
with t = tweak+0x220, negated when the body's basis determinant is negative (mirrored);
u = 0x113fb0(body), the local Y, unit in xy. The wanted velocity becomes
(c.x v.y + c.y v.x, c.y v.y - c.x v.x). This applies to configurations 6, 7, 8 and 11.
INFERRED: that 0x113fb0 untilts the axis.

FOUND, the rows (already modelled): the main row's limits are +accel x vr and
-decel x vr, both +-vr x accel for configuration 11 (0x95839c); a second row (0x1c80b0)
along (d.y, -d.x) with limits +-decel x vr is added only when decel x vr is not 0. Each
row's rate is k = 1/m with m the float at (body+0x64 -> +0x110)+0x38 (0 when m is 0),
and the limits are scaled by k x the activation. INFERRED: that m is the mass.

Model: `MoveTweak` gains `angle` (0 until a host fills it); `rows` turns the up axis by
it, drops the error along that axis when allowUpDown is off, and turns a local follower's
wanted velocity as configuration 8 already did; `build_scene` drops allowInOut
followers. The test `d1151_followers_turn_their_up_axis_drop_in_out_and_keep_local_space`
checks each.

Census (census-all-40, followers by flag):

| allowInOut | localSpace, allowUpDown on | localSpace, allowUpDown off |
|---|---|---|
| 224 | 0 local, 1,895 world | 0 local, 175 world |

Not modelled: a host filling the +0x220 angle; the mirror negation; the allowInOut path
0x1c8250; the 1/mass rate.
