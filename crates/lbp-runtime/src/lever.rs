//! The lever switch (switch type 1) (docs/logic.md Part 28).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58, table 0x11be00 entry 1 →
//! 0x11cad0 → 0x11a6a4, then the store 0x11bf18):
//! - The handle is the Thing at PSwitch+0x90, the serialized referenceThing (read right
//!   after the type). Without one the signal is {0.0, 0, -1}.
//! - The handle's position less the switch Thing's (PSwitch+8) position, both from
//!   0xf13a0, is taken into the switch Thing's frame by the inverse of its world matrix
//!   ((Thing+0xc)+0x40); x is its first component.
//! - The activation is (x + 100) / 200 (r2-0x3f6c, r2-0x3eac) kept to [0, 1]. The ternary
//!   is the default rule 0x11873c on output 0 (0x118874): 0 when |a| is at most 0.25
//!   (r2-0x3f10), 1 at or above 0.75, -1 at or below -0.75, in between what output 0
//!   holds. The player is -1. Every output gets the same signal.
//! - Inverted through the standard tail (table 0x11bfd4 entry 1 is 0x11c16c).
//!
//! Not modelled: 0xf13a0 applies the world matrix to a local point (+0x50 of the Thing's
//! object at +0x30) when there is one; here the position is the matrix's translation.
//! Across the 74 levers the ternary band still matches the stored one on 73 (the
//! activations differ by at most 0.064). Without a world matrix on either Thing the
//! signal is off here.
use crate::switches::{held_ternary, SwitchContext, SIGNAL_OFF};
use crate::LevelWorld;
use lbp_script_vm::natives::SwitchInput;

/// Half the handle's travel (r2-0x3f6c).
pub const LEVER_HALF_TRAVEL: f32 = 100.;

impl LevelWorld {
    /// The x offset of switch `thing`'s referenceThing in the switch Thing's frame (see
    /// the module), or None without a referenceThing or a world matrix.
    pub fn reference_offset(&self, thing: u32) -> Option<f32> {
        let t = self.things.get(thing as usize)?;
        let r = t.switch.as_ref()?.reference_thing?;
        let m = t.world_matrix?;
        let p = self.things.get(r as usize)?.world_matrix?;
        let d = [p[12] - m[12], p[13] - m[13], p[14] - m[14]];
        // Row 0 of the inverse of the 3×3 part is (Y × Z) / det.
        let (y, z) = ([m[4], m[5], m[6]], [m[8], m[9], m[10]]);
        let yz = [
            y[1] * z[2] - y[2] * z[1],
            y[2] * z[0] - y[0] * z[2],
            y[0] * z[1] - y[1] * z[0],
        ];
        let det = m[0] * yz[0] + m[1] * yz[1] + m[2] * yz[2];
        if det == 0. {
            return None;
        }
        Some((yz[0] * d[0] + yz[1] * d[1] + yz[2] * d[2]) / det)
    }
}

/// The ternary of 0x11873c's default rule for `activation` (see the module).
pub(crate) fn default_ternary(cx: &SwitchContext<'_>, activation: f32) -> i32 {
    if activation.abs() <= 0.25 {
        0
    } else if activation >= 0.75 {
        1
    } else if activation <= -0.75 {
        -1
    } else {
        held_ternary(cx)
    }
}

/// The lever (see the module).
pub(crate) fn lever(cx: &SwitchContext<'_>) -> SwitchInput {
    let Some(x) = cx.world.reference_offset(cx.thing) else {
        return SIGNAL_OFF;
    };
    let activation = ((x + LEVER_HALF_TRAVEL) / (2. * LEVER_HALF_TRAVEL)).clamp(0., 1.);
    SwitchInput {
        activation,
        ternary: default_ternary(cx, activation),
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

    /// A world matrix turned `deg` about z and moved to (`x`, `y`).
    fn at(deg: f32, x: f32, y: f32) -> Option<[f32; 16]> {
        let (s, c) = deg.to_radians().sin_cos();
        Some([c, s, 0., 0., -s, c, 0., 0., 0., 0., 1., 0., x, y, 0., 1.])
    }

    /// Thing 0: the handle at (`hx`, `hy`). Thing 1: a lever at (1000, 50) turned `deg`,
    /// its file storing (0.5, 1), with the handle as its referenceThing if `reference`.
    fn world(deg: f32, hx: f32, hy: f32, reference: bool) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: sig(0.5, 1),
            stored: sig(0.5, 1),
            targets: vec![],
            random_on: false,
        });
        let lever = ThingState {
            world_matrix: at(deg, 1000., 50.),
            parts: vec!["POS", "SWITCH"],
            switch: Some(SwitchState {
                switch_type: 1,
                reference_thing: reference.then_some(0),
                outputs: vec![output; 2],
                ..Default::default()
            }),
            ..Default::default()
        };
        let handle = ThingState {
            world_matrix: at(0., hx, hy),
            parts: vec!["POS"],
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![handle, lever],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn outs(w: &mut LevelWorld) -> Vec<SwitchInput> {
        w.update_switches();
        let s = w.things[1].switch.as_ref().unwrap();
        s.outputs
            .iter()
            .map(|o| o.as_ref().unwrap().signal)
            .collect()
    }

    #[test]
    fn lever_reads_its_handle_offset() {
        assert_eq!(outs(&mut world(0., 1100., 50., false)), vec![SIGNAL_OFF; 2]);
        // Right end, left end, past the right end, centre.
        assert_eq!(outs(&mut world(0., 1100., 50., true)), vec![sig(1., 1); 2]);
        assert_eq!(outs(&mut world(0., 900., 50., true)), vec![sig(0., 0); 2]);
        assert_eq!(outs(&mut world(0., 1300., 80., true)), vec![sig(1., 1); 2]);
        assert_eq!(outs(&mut world(0., 1000., 50., true)), vec![sig(0.5, 1); 2]);
        // In the lever's frame: turned 90°, a handle above it is at x = +50.
        assert_eq!(
            outs(&mut world(90., 1000., 100., true)),
            vec![sig(0.75, 1); 2]
        );
    }

    #[test]
    fn lever_holds_its_ternary_between_the_bands_and_inverts() {
        // 0.5 is between 0.25 and 0.75: output 0's ternary (1 as stored) is kept.
        let mut w = world(0., 1000., 50., true);
        assert_eq!(outs(&mut w), vec![sig(0.5, 1); 2]);
        // 0.2 is in the dead band.
        let mut w = world(0., 940., 50., true);
        assert_eq!(outs(&mut w)[0].ternary, 0);
        let mut w = world(0., 1100., 50., true);
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert_eq!(outs(&mut w), vec![sig(0., 0); 2]);
    }
}
