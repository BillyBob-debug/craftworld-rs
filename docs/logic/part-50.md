# Logic, Part 50

See docs/logic.md for Parts 1-49.

## Part 50: the aim tweaks' stick, flip and lead (D1148)

What the game does (EBOOT 01.33, packer 0x95fb20):
- Stick: 0xcb02c returns the f32 at entry +0x18 of word 5 of the input block
  *(*(tweak+8)+0x60) when its count (word 0) is at least 2; 0xcb074 the same for word 6
  with a count of at least 3; a missing slot is 0. Slot 0 is the main input (0xcaf9c).
  INFERRED: the slots are the Thing's wired inputs PHYSICS_TWEAK_X (4) and _Y (5) of
  ESwitchInputUpdateType, raw signed analog, not read from the pad.
- Flip: r25 (0x95fd14) is byte tweak+0x224 XOR the body basis determinant below 0
  (0x95fcf8). +0x224 is not saved: 0xcd2e0 rebuilds it as the mirror parity up the
  Thing+0x50 parent chain. The flip negates 4's x (0x960108) and picks f13 = ±1
  (0xd97fb0, 0xd98188).
- +0x220 (built by 0xcd2e0): the sum of the angles up the parent chain (atan2 of each
  basis plus π/2, DAT_00d953a0, or a link's stored f32), negated across mirrored links.
  It is not angleRange. 0x960560 adds f13 × +0x220 to the second atan2 term; the wrap
  at 0x96059c-0x960604 is INFERRED.
- Lead: 5's target comes from 0x1c3e04 (called at 0x96001c), aim = target point minus
  own position (state+0x20). When the target Thing has a body (+0x64) whose u16 +0xe is
  0, 0x960924 adds its point velocity: vel (+0x10) + perp(P − centre (+0x20)) × ω
  (+0x30), perp(x, y) = (−y, x). No scale, no dt, and the tweak's own velocity is
  unused.

lbp-sim: `SpinTweak.stick` (0 until a host wires the inputs) and `flip` (the tweak's
world basis, or its body's when it has none, mirrored; INFERRED to carry the parent
chain's parity); `aim` negates 4's x by the flip; `SpinTweak::lead(vel, ω, r)` gives the
point velocity for a host to add to 5's aim. Not modelled: +0x220, which is 0 for a
body with no parent chain. D1164 (Part 65) adds the lead in `drive_spins` for tags on
bodies, and D1165 (Part 66) for the player.

Census (census-all-36.json): mirrored aim tweaks are configuration 3 ×13 of 213 and
5 ×31 of 577; none of the 80 configuration 4 tweaks is mirrored.

Test: `d1148_aim_tweaks_take_the_stick_inputs_mirror_and_lead_the_target`.