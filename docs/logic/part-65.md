# Logic, Part 65

See docs/logic.md for Parts 1-49.

## Part 65: configuration 5 leads a target on a moving body (D1164)

Part 50 found the lead of 0x960924 and Part 51 the shaper's lead rate. Both are now
wired into the turn.

FOUND, the configuration 5 branch of 0x95fb20 (EBOOT 01.33):
- 0x1c3e04 gives the target Thing. Its point is 0x3c66c(target) when its u16 +0x78
  bit 0x100 is clear, else 0x3becc(target)+0x30.
- The aim is that point minus the tweak's own position (state+0x20).
- When the target has a body (+0x64) whose u16 +0xe is 0, the led aim adds that body's
  point velocity, vel (+0x10) + perp(point − centre (+0x20)) × ω (+0x30).
- The angle error comes from the aim without the lead (the first angle block, slot
  +0xd0). The lead rate passed to 0x1c21f4 is the wrapped angle of the led aim minus
  that angle (0x960790, Part 51).

lbp-sim:
- `SpinTweak::target_led` and `turn_led` take the lead, per tick in game units. Their
  rate is wrap(angle(aim + lead) − angle(aim)).
- `target_along` and `turn_along` pass no lead.
- `drive_spins` takes a `motion` lookup: a body's velocity and turn speed per tick and
  its centre (INFERRED: the body's position). A tag on a body leads by
  `SpinTweak::lead`.
- Not modelled:
  - The player target takes no lead, because the host passes no Sackboy velocity.
  - The +0xe test is taken as always 0.

Test: `d1164_an_aimer_leads_a_moving_target_by_its_turn_per_tick`.

Census (local-analysis/logic/d1164/census1.json, `chip.aim_lead`): 277 keyed
configuration 5 aimers have a tag of their key on a dynamic body. They are in 39 levels,
none in the Rookie Test. The route is unchanged and parity passes 5/0.
