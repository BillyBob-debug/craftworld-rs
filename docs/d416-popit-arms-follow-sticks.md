# D416: Sackboy's arms follow the sticks while the Popit cursor is shown (Popit part 12)

Before: with the Popit cursor shown, only his head moved (D415); his arms kept the stand
pose. After: both arms take the arm-acting pose, each swung round by a stick: the left
stick (x mirrored) drives arm 0, the right stick drives arm 1.

## The rule (FOUND, EBOOT 0x2097e0 cursor branch 0x209ce0 -> 0x20936c)
- Same cursor test as D415. The arms do NOT aim at the cursor's world place: 0x20936c is
  called for arm 1 with the smoothed stick anim+0xad0 (mirrored) and for arm 0 with
  anim+0xac0, x negated, each at weight 1.0, without the L1 / R1 arm buttons.
- Sticks (0x1fb0e4): each smoothed vector eases 0.3 a tick (0xd994cc) toward the raw stick.
- One arm: m = |x| + |y|; if m > 0.2 (0xd994f0) target = atan2(y, x) in (0, 2pi], else no
  change. The target is moved by a whole turn into [cur - pi, cur + pi) (short way round),
  then angle += clamp(m, 0, 1) (target - angle); stored unwrapped at anim+0xa40.
- Frame = angle / 2pi x 32 of act_neutral_arm_small (0x3ef0) and act_neutral_arm_big
  (0x3eef), 33 frames each; pose = lerp(small, big, clamp(m, 0, 1)), added as its
  difference from frame 0 of the base stand clip (0x2022b8). Arm 1 mirrors both the pose and
  the reference (0x1ebb48), so one authored clip serves both arms.
- Mood variants of the arm clips (0x4e4.. / 0x4f8..) are left out: neutral only.

## Port
- `SackboyAnim::set_arm_sticks` takes the raw (left, right) sticks; `Blend::MixAdditive`
  layers (arm 1 first, as the game calls them) carry the mix and the base clip;
  `arm_angle_step` / `arm_frame` hold the math. The viewer adds the mixed, optionally
  mirrored per-bone transforms (`BlendPose::add_trs`); a clip missing from the bank adds
  nothing.
- `popit_input` writes the sticks while the cursor is shown (left stick also WASD).
- Wrapping the frame into one turn (frame 32 = frame 0) and the layer weight 1 - air ease
  (the 0x208dec merge, exact when standing) are INFERRED. Stick axes x right, y up INFERRED.

## Check
- `cargo test -p lbp-sim d416` (dead zone, ease, short way round, frame mapping, layers).
- Viewer as in D415 with `--popit-cursor`: hold either stick and the matching arm swings;
  `--popit-arm-sticks LX,LY,RX,RY` (x right, y up) fixes the sticks for a QA capture.

## Open
- Whether the stick vectors are zero while the Popit owns the pad (pad+0x1c & 0x27 at
  0x1fb0e4); if so the arms rest in act_neutral_arm_small at their last angle. Needs an
  RPCS3 memory read of anim+0xac0 / 0xad0 / 0xa40 with the cursor shown.
- Stick y sign and which stick is pad+0x30; which bones the act clips move.
- The no-cursor arm drift of 0x2097e0 and the first part of 0x208dec (arms_opp clips).
