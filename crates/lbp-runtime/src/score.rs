//! The score switch (switch type 30) (docs/logic.md Part 31).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 entry 30 → 0x11c834 →
//! 0x11ab74, then the shared tail 0x11c2c4):
//! - The score is taken over the game state's player records (count +0x1e8, by index
//!   0x156b74), counting only records whose number (+0xdc) passes the team filter
//!   (PSwitch+0x60, 0xf9a64). By the PWorld's game mode (the chain 0xf9a64 reads too):
//!   mode 0 the sum of their scores (+0x30); mode 1 (versus) the highest score above
//!   0.0, the first record keeping a tie, with that record's player; any other mode 0.0.
//! - With a target (PSwitch+0xcc, the field the counters read as bulletsRequired) of 0
//!   the signal is {1.0, 1, player}. Otherwise the activation is the score over the
//!   target, not clamped, the ternary 1 at or above 1.0, -1 at or below -1.0, otherwise
//!   0 (0x11873c, type 30 at 0x1187e8), and the player -1.
//! - Inverted through the standard tail (table 0x11bfd4 entry 30 is 0x11c16c).
//!
//! Not modelled: the switch Thing's owner (Thing+0x50) the team filter is given; as for
//! the button it is nobody.
use crate::sensors::{team_filter, NO_OWNER};
use crate::switches::SwitchContext;
use lbp_script_vm::natives::SwitchInput;

/// The score switch (see the module).
pub(crate) fn score_switch(cx: &SwitchContext<'_>) -> SwitchInput {
    let w = cx.world;
    let s = cx.switch;
    let counted = w
        .player_records
        .iter()
        .filter(|r| team_filter(w, r.number, s.team_filter, NO_OWNER));
    let (mut score, mut player) = (0., -1);
    match w.pworld.game_mode {
        0 => score = counted.map(|r| r.score).sum(),
        1 => {
            for r in counted {
                if r.score > score {
                    (score, player) = (r.score, r.number);
                }
            }
        }
        _ => {}
    }
    if s.bullets_required == 0 {
        return SwitchInput {
            activation: 1.,
            ternary: 1,
            player,
        };
    }
    let activation = score / s.bullets_required as f32;
    let ternary = if activation.abs() < 1. {
        0
    } else if activation < 0. {
        -1
    } else {
        1
    };
    SwitchInput {
        activation,
        ternary,
        player: -1,
    }
}

#[cfg(test)]
mod tests {
    use crate::switches::SIGNAL_OFF;
    use crate::{LevelWorld, SwitchOutputState, SwitchState, ThingState};
    use lbp_script_vm::natives::{PlayerRecord, SwitchInput};

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    /// Output 0 of a score switch with `target`, `filter`, inverted or not, in game
    /// `mode`, with one record per (number, team, score).
    fn run(
        mode: i32,
        target: i32,
        filter: i32,
        inverted: bool,
        rs: &[(i32, i32, f32)],
    ) -> SwitchInput {
        let output = Some(SwitchOutputState {
            signal: SIGNAL_OFF,
            stored: SIGNAL_OFF,
            targets: vec![],
            random_on: false,
        });
        let thing = ThingState {
            switch: Some(SwitchState {
                switch_type: 30,
                bullets_required: target,
                team_filter: filter,
                inverted,
                outputs: vec![output],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![thing],
            ..Default::default()
        };
        w.pworld.game_mode = mode;
        w.player_records = rs
            .iter()
            .map(|&(number, team, score)| PlayerRecord {
                number,
                team,
                score,
                ..Default::default()
            })
            .collect();
        w.wire_inputs();
        w.update_switches();
        let s = w.things[0].switch.as_ref().unwrap();
        s.outputs[0].as_ref().unwrap().signal
    }

    #[test]
    fn score_switch_sums_or_takes_the_best_score() {
        let rs = [(0, 0, 300.), (1, 1, 900.)];
        assert_eq!(run(0, 1000, 0, false, &rs), sig(1.2, 1, -1));
        assert_eq!(run(0, 2000, 0, false, &rs), sig(0.6, 0, -1));
        // Versus: the best score; its player only without a target.
        assert_eq!(run(1, 1000, 0, false, &rs), sig(0.9, 0, -1));
        assert_eq!(run(1, 0, 0, false, &rs), sig(1., 1, 1));
        // Teams and the other modes count nothing.
        assert_eq!(run(5, 1000, 0, false, &rs), sig(0., 0, -1));
        assert_eq!(run(0, 0, 0, false, &[]), sig(1., 1, -1));
        // Versus filter 1 passes only the owner, which is nobody.
        assert_eq!(run(1, 1000, 1, false, &rs), sig(0., 0, -1));
    }

    #[test]
    fn score_switch_is_not_clamped_and_inverts() {
        assert_eq!(run(0, -100, 0, false, &[(0, 0, 250.)]), sig(-2.5, -1, -1));
        assert_eq!(run(0, 1000, 0, true, &[(0, 0, 1000.)]), sig(0., 0, -1));
        assert_eq!(run(0, 1000, 0, true, &[(0, 0, 0.)]), sig(1., 1, -1));
    }
}
