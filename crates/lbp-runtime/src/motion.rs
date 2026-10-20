//! The motion recorder switch (switch type 45, its three outputs) (docs/logic.md Part 24).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58, table 0x11be00 entry 45 →
//! 0x11d4d0, then the store 0x11bf18):
//! - Each output is read from one word of the switch, PSwitch+0xcc (the slot the other
//!   types call bulletsRequired), with the player -1:
//!   - output 0 (0x11d8bc): the low four bits over 15 (r2-0x4100), the ternary 1 when
//!     they are not 0, else 0;
//!   - output 1 (0x11d914): bit 4 as 0.0 or 1.0, the ternary the same bit;
//!   - output 2 (0x11d4e8): bit 5, the same way;
//!   - any other output off (0x11bf08).
//! - Inverted through the standard tail (table 0x11bfd4 entry 45 is 0x11c16c). No
//!   end-of-update step (table 0x124b0c stops at 0x27).
//!
//! Not modelled: what writes the word while the game runs (the recorder's own update was
//! not found), so here it keeps the value the level file stores. In the 378 levels every
//! recorder stores 0 or 1, and only output 1 is ever wired (6 of 142).
use crate::switches::{SwitchContext, SIGNAL_OFF};
use lbp_script_vm::natives::SwitchInput;

/// Output 0's divisor (r2-0x4100).
pub const LEVELS: f32 = 15.;

/// The motion recorder's output `output` (see the module).
pub(crate) fn motion_recorder(cx: &SwitchContext<'_>, output: usize) -> SwitchInput {
    let word = cx.switch.bullets_required as u32;
    let (activation, ternary) = match output {
        0 => ((word & 0xf) as f32 / LEVELS, i32::from(word & 0xf != 0)),
        1 | 2 => {
            let bit = (word >> (output + 3)) & 1;
            (bit as f32, bit as i32)
        }
        _ => return SIGNAL_OFF,
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
    use lbp_script_vm::natives::SwitchInput;

    fn sig(activation: f32, ternary: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player: -1,
        }
    }

    /// Thing 0: a motion recorder with its three outputs and the word `word`.
    fn world(word: i32) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: SIGNAL_OFF,
            stored: SIGNAL_OFF,
            targets: vec![],
            random_on: false,
        });
        let recorder = ThingState {
            switch: Some(SwitchState {
                switch_type: 45,
                bullets_required: word,
                outputs: vec![output; 3],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![recorder],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn outs(w: &mut LevelWorld) -> Vec<SwitchInput> {
        w.update_switches();
        let s = w.things[0].switch.as_ref().unwrap();
        s.outputs
            .iter()
            .map(|o| o.as_ref().unwrap().signal)
            .collect()
    }

    #[test]
    fn motion_recorder_reads_its_word() {
        assert_eq!(outs(&mut world(0)), vec![sig(0., 0); 3]);
        // The stored 1 most recorders carry: output 0 at 1/15, on.
        assert_eq!(outs(&mut world(1))[0], sig(1. / 15., 1));
        assert_eq!(outs(&mut world(1))[1..], [sig(0., 0); 2]);
        let o = outs(&mut world(0x1f));
        assert_eq!(o, vec![sig(1., 1), sig(1., 1), sig(0., 0)]);
        let o = outs(&mut world(0x2a));
        assert_eq!(o, vec![sig(10. / 15., 1), sig(0., 0), sig(1., 1)]);
        // Bits above 5 are not read.
        assert_eq!(outs(&mut world(0x40)), vec![sig(0., 0); 3]);
    }

    #[test]
    fn motion_recorder_inverts_through_the_standard_tail() {
        let mut w = world(0x13);
        w.things[0].switch.as_mut().unwrap().inverted = true;
        // Output 0 at 3/15 (on) gives 1 - 0.2 and off; bit 4 on gives off; bit 5 off on.
        assert_eq!(
            outs(&mut w),
            vec![sig(1. - 3. / 15., 0), sig(0., 0), sig(1., 1)]
        );
    }
}
