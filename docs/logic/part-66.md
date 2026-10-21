# Logic, Part 66

See docs/logic.md for Parts 1-49.

## Part 66: configuration 5 leads the player (D1165)

Part 65 wired the lead for tags on moving bodies. The player was left out because the
host passed no Sackboy velocity.

FOUND (Part 65, 0x95fb20, EBOOT 01.33): the lead comes from the target Thing's body
(+0x64), whatever that Thing is. A keyless aimer's target from 0x1c3e04 is the player's
Thing, so he is led the same way.

lbp-sim:
- `drive_spins` now takes the player as `(position, velocity)`. The velocity is
  Sackboy's `controller.vx, vy`, in game units per tick (the controller moves him by
  that delta each tick).
- The player's lead is that velocity alone. INFERRED: his body does not turn, so the
  perp(r) × ω term is 0, and his u16 +0xe is 0.

Test: `d1165_an_aimer_leads_the_player_by_his_velocity`.

Census (local-analysis/logic/d1165/census1.json, `chip.aim_player`): 67 configuration 5
aimers have no key and aim at the player. They are in 23 levels, none in the Rookie
Test. The route is unchanged and parity passes 5/0.
