//! The button switch (switch type 0) (docs/logic.md Part 23).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 entry 0 → 0x11caac →
//! 0x11a998 with mode 1, then the shared tail 0x11c2c4):
//! - The press force (0x100da4) is the sum, over this tick's contacts of the button's
//!   body with another body, of the contact's two impulse magnitudes (the pair record's
//!   vectors +0x10 and +0x20, [`Contact::impulse`]), counting only contacts whose other
//!   body's player passes the team filter (PSwitch+0x60, 0xf9a64).
//! - The presser's player: the counted contacts' players in order, each one with a
//!   player replacing the one kept, except that once the kept player is output 0's
//!   player it stays; -1 when none has one.
//! - The activation is the force over 10000 (r2-0x3e6c) kept to [0, 1]; the ternary is
//!   0 at or below 0.25 (r2-0x3f10), 1 at or above 0.75, in between what output 0 holds
//!   (0x11873c, the default rule).
//! - The release hold: while the hold count (PSwitch+0x24) is 3 or less and the button is
//!   not pressed (ternary 0), the output keeps output 0's signal (taken back through the
//!   inversion first, so the standard tail gives it back).
//! - At the end of the update (table 0x124b0c entry 0 → 0x124bb8), while output 0 is on
//!   before the inversion, the press is worked out again without the hold (mode 0): a
//!   press sets the hold count to 0, no press adds 1. So a released button stays on for
//!   four more ticks.
//! - Inverted through the standard tail (table 0x11bfd4 entry 0 is 0x11c16c).
//!
//! INFERRED: the button's body (PSwitch+0x90, whose bodies +0x58 and +0x5a are walked)
//! is the switch's own Thing; a body's player is the player whose Sackboy Thing it is;
//! a contact of the body with itself (0x101a44) is one whose two sides are that Thing.
//! Where the hold count starts was not found; here at 0, so a button never pressed keeps
//! its stored signal until it is. Not modelled: the team filter's owner (as for the tag
//! sensor, nobody); the glow step 0x10ffd4 (PSwitch+0x20 eased toward the output and
//! written to the mesh), which changes no signal.
//!
//! [`Contact::impulse`]: crate::Contact::impulse
use crate::sensors::{team_filter, NO_OWNER};
use crate::switches::{held_ternary, invert_standard, SwitchContext};
use crate::LevelWorld;
use lbp_script_vm::natives::SwitchInput;

/// The force that fully presses a button (r2-0x3e6c).
pub const FULL_PRESS: f32 = 10000.;

/// Ticks a released button keeps its signal for, less one (PSwitch+0x24 <= 3).
pub const RELEASE_HOLD: i32 = 3;

/// The press of the button from this tick's contacts (0x11a998 up to the hold).
fn press(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let body = cx.thing;
    let held = cx.current(0).player;
    let (mut force, mut player) = (0., -1);
    for c in &w.contacts {
        let other = match (c.a == body, c.b == body) {
            (true, false) => c.b,
            (false, true) => c.a,
            _ => continue,
        };
        let p = w
            .players
            .iter()
            .find(|p| p.thing == other)
            .map_or(-1, |p| p.number);
        if !team_filter(w, p, s.team_filter, NO_OWNER) {
            continue;
        }
        force += c.impulse;
        if p != -1 && !(player != -1 && player == held) {
            player = p;
        }
    }
    let activation = (force / FULL_PRESS).clamp(0., 1.);
    let ternary = if activation <= 0.25 {
        0
    } else if activation >= 0.75 {
        1
    } else {
        held_ternary(cx)
    };
    SwitchInput {
        activation,
        ternary,
        player,
    }
}

/// The button switch before the inversion (see the module).
pub(crate) fn button(cx: &SwitchContext<'_>) -> SwitchInput {
    let pressed = press(cx);
    if pressed.ternary != 0 || cx.switch.memory.phase > RELEASE_HOLD {
        return pressed;
    }
    let held = cx.current(0);
    if cx.switch.inverted {
        invert_standard(held)
    } else {
        held
    }
}

impl LevelWorld {
    /// The button's step at the end of its update (FOUND, 0x124bb8; see the module).
    pub(crate) fn button_step(&mut self, thing: u32) {
        let Some(s) = self
            .things
            .get(thing as usize)
            .and_then(|t| t.switch.as_ref())
        else {
            return;
        };
        let cx = SwitchContext {
            world: self,
            thing,
            switch: s,
        };
        if (cx.current(0).ternary != 0) == s.inverted {
            return;
        }
        let pressed = press(&cx).ternary != 0;
        let s = self.things[thing as usize].switch.as_mut().unwrap();
        s.memory.phase = if pressed { 0 } else { s.memory.phase + 1 };
    }
}

#[cfg(test)]
mod tests {
    use crate::switches::SIGNAL_OFF;
    use crate::{Contact, LevelWorld, SwitchOutputState, SwitchState, ThingState};
    use lbp_script_vm::natives::SwitchInput;

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    /// Thing 0: a button. Thing 1: a box. Players 0 and 1 after them.
    fn world() -> LevelWorld {
        let button = ThingState {
            switch: Some(SwitchState {
                switch_type: 0,
                outputs: vec![Some(SwitchOutputState {
                    signal: SIGNAL_OFF,
                    stored: SIGNAL_OFF,
                    targets: vec![],
                    random_on: false,
                })],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![button, ThingState::default()],
            ..Default::default()
        };
        w.add_player([0.; 3], (0., 0.));
        w.add_player([0.; 3], (0., 0.));
        w.wire_inputs();
        w
    }

    fn push(w: &mut LevelWorld, other: u32, impulse: f32) {
        w.contacts.push(Contact {
            a: other,
            b: 0,
            force: impulse != 0.,
            impulse,
        });
    }

    fn out(w: &mut LevelWorld) -> SwitchInput {
        w.update_switches();
        w.things[0].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    #[test]
    fn button_force_threshold_and_presser() {
        let mut w = world();
        let (p0, p1) = (w.players[0].thing, w.players[1].thing);
        assert_eq!(out(&mut w), sig(0., 0, -1));
        // The hold count at 0 keeps output 0's signal while not pressed.
        push(&mut w, 1, 2000.);
        assert_eq!(out(&mut w), sig(0., 0, -1));
        // Past the hold, a box pressing with 2000: 0.2, below the dead zone, no player.
        w.things[0].switch.as_mut().unwrap().memory.phase = 4;
        assert_eq!(out(&mut w), sig(0.2, 0, -1));
        // Player 1 adds 6000: 0.8, on, his number.
        push(&mut w, p1, 6000.);
        assert_eq!(out(&mut w), sig(0.8, 1, 1));
        // Between 0.25 and 0.75 the ternary held; the force kept to 1.0.
        w.contacts.pop();
        push(&mut w, p1, 3000.);
        assert_eq!(out(&mut w), sig(0.5, 1, 1));
        push(&mut w, p0, 9000.);
        // Player 1 holds the output, so he stays the presser over player 0.
        assert_eq!(out(&mut w), sig(1., 1, 1));
        // With output 0's player not among them, the last with a player.
        w.contacts.retain(|c| c.a != p1);
        push(&mut w, p1, 0.);
        w.things[0].switch.as_mut().unwrap().outputs[0]
            .as_mut()
            .unwrap()
            .signal = sig(1., 1, -1);
        assert_eq!(out(&mut w).player, 1);
        // A contact of the button with itself does not count.
        w.contacts = vec![Contact {
            a: 0,
            b: 0,
            force: true,
            impulse: 9000.,
        }];
        w.things[0].switch.as_mut().unwrap().memory.phase = 4;
        assert_eq!(out(&mut w), sig(0., 0, -1));
    }

    #[test]
    fn button_holds_four_ticks_after_release_and_inverts() {
        let mut w = world();
        let p0 = w.players[0].thing;
        push(&mut w, p0, 10000.);
        assert_eq!(out(&mut w), sig(1., 1, 0));
        w.contacts.clear();
        for _ in 0..4 {
            assert_eq!(out(&mut w), sig(1., 1, 0));
        }
        assert_eq!(out(&mut w), sig(0., 0, -1));
        // Off, the hold count does not move: a new press is on at once.
        assert_eq!(out(&mut w), sig(0., 0, -1));
        push(&mut w, p0, 10000.);
        assert_eq!(out(&mut w), sig(1., 1, 0));
        // Inverted: off while pressed, then the release hold keeps the inverted signal.
        w.things[0].switch.as_mut().unwrap().inverted = true;
        assert_eq!(out(&mut w), sig(0., 0, 0));
        w.contacts.clear();
        for _ in 0..4 {
            assert_eq!(out(&mut w), sig(0., 0, 0));
        }
        assert_eq!(out(&mut w), sig(1., 1, -1));
    }
}
