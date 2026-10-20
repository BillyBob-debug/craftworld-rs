//! The pocket item pedestal switch (switch type 65) (docs/logic.md Part 27).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58, table 0x11be00 entry 65 →
//! 0x11d164, then the store 0x11bf18):
//! - FUN_0010b0d8 gives the switch's value block (PSwitch+0xe4, 0x40 bytes, allocated on
//!   first use). Output `o` reads activation = floats[o] (vector at +0x28, count +0x2c)
//!   and ternary = s8s[o] (vector at +0x34, count +0x38), each 0 past its count; the
//!   player -1.
//! - Inverted through the standard tail (0x11bfac sends types above 0x37 to 0x11c16c).
//!
//! The name is INFERRED (the switch-type name table 0xd6cb58). FUN_0010e4f0 fills both
//! vectors from a registry entry keyed by the block's id (+0) and name (+4) (entry stride
//! 0x30: float vector +0x18, s8 vector +0x24), or empties them on a miss. It runs when the
//! key is set (FUN_0010e714) and at level setup (FUN_001ab508). What fills the registry
//! is not found (INFERRED: pocket item or profile state).
//!
//! Not modelled: the registry, so [`SwitchState::pedestal_activations`] and
//! [`SwitchState::pedestal_ternaries`] stay empty and every output reads off, not the
//! signal the level file stores.
//!
//! [`SwitchState::pedestal_activations`]: crate::SwitchState::pedestal_activations
//! [`SwitchState::pedestal_ternaries`]: crate::SwitchState::pedestal_ternaries
use crate::switches::SwitchContext;
use lbp_script_vm::natives::SwitchInput;

/// The pedestal's output `output` (see the module).
pub(crate) fn pedestal_sensor(cx: &SwitchContext<'_>, output: usize) -> SwitchInput {
    let s = cx.switch;
    SwitchInput {
        activation: s.pedestal_activations.get(output).copied().unwrap_or(0.),
        ternary: s
            .pedestal_ternaries
            .get(output)
            .copied()
            .map_or(0, i32::from),
        player: -1,
    }
}

#[cfg(test)]
mod tests {
    use crate::{LevelWorld, SwitchOutputState, SwitchState, ThingState};
    use lbp_script_vm::natives::SwitchInput;

    fn sig(activation: f32, ternary: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player: -1,
        }
    }

    /// Thing 0: a pedestal switch with three outputs, the file storing on, and the value
    /// arrays `acts` and `terns`.
    fn world(acts: &[f32], terns: &[i8], inverted: bool) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: sig(1., 1),
            stored: sig(1., 1),
            targets: vec![],
            random_on: false,
        });
        let switch = ThingState {
            parts: vec!["POS", "SWITCH"],
            switch: Some(SwitchState {
                switch_type: 65,
                inverted,
                outputs: vec![output; 3],
                pedestal_activations: acts.to_vec(),
                pedestal_ternaries: terns.to_vec(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![switch],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn outs(mut w: LevelWorld) -> Vec<SwitchInput> {
        w.update_switches();
        let s = w.things[0].switch.as_ref().unwrap();
        s.outputs
            .iter()
            .map(|o| o.as_ref().unwrap().signal)
            .collect()
    }

    #[test]
    fn pedestal_reads_its_value_arrays_per_output() {
        // Empty arrays (the registry is not modelled): off, not the stored on.
        assert_eq!(outs(world(&[], &[], false)), vec![sig(0., 0); 3]);
        // Each array is read up to its own count.
        assert_eq!(
            outs(world(&[0.25, 1.], &[1, -1, 1], false)),
            vec![sig(0.25, 1), sig(1., -1), sig(0., 1)]
        );
    }

    #[test]
    fn pedestal_inverts_through_the_standard_tail() {
        assert_eq!(outs(world(&[], &[], true)), vec![sig(1., 1); 3]);
        assert_eq!(outs(world(&[1.], &[1], true))[0], sig(0., 0));
    }
}
