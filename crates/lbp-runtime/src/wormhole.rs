//! The wormhole switch (switch type 57) (docs/logic.md Part 26).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58, table 0x11be00 entry 57 →
//! 0x11d104, then the store 0x11bf18):
//! - The Thing is the switch's own (PSwitch+8), not its parent. Off (0x11bf08) unless bit
//!   49 of its 64-bit part mask (Thing+0x10) is set and part slot 49 of its part block
//!   (Thing+0x34 → +0xc4) is not null.
//! - On (1.0, ternary 1) while byte +0x16 of that part is set, else 0.0 and 0; the player
//!   -1. Every output gets the same signal.
//! - Inverted through the standard tail (0x11bfac sends types above 0x37 to 0x11c16c). No
//!   end-of-update step (0x124934 sends types above 0x27 past table 0x124b0c).
//!
//! Slot 49 is the WORMHOLE part (INFERRED: the runtime part enum shifts indices above 0x28
//! by one, so serialized 0x30 is slot 49; all 123 type-57 Things in the 378 levels have
//! it). PWormhole (ctor 0x137944) sets +0x16 to 0; its update (0x138040) sets it for one
//! tick when the wormhole stops being active (+0x17), and clears it on the next update.
//!
//! Not modelled: the wormhole's own update, so [`ThingState::wormhole_ended`] stays as
//! loaded (false) and a wired wormhole switch reads off here.
use crate::switches::{SwitchContext, SIGNAL_OFF};
use crate::ThingState;
use lbp_script_vm::natives::SwitchInput;

/// The part the switch reads.
pub const WORMHOLE_PART: &str = "WORMHOLE";

/// The wormhole switch (see the module).
pub(crate) fn wormhole_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let Some(thing) = cx.world.things.get(cx.thing as usize) else {
        return SIGNAL_OFF;
    };
    if !has_wormhole(thing) {
        return SIGNAL_OFF;
    }
    let on = thing.wormhole_ended;
    SwitchInput {
        activation: f32::from(u8::from(on)),
        ternary: i32::from(on),
        player: -1,
    }
}

fn has_wormhole(thing: &ThingState) -> bool {
    thing.parts.contains(&WORMHOLE_PART)
}

#[cfg(test)]
mod tests {
    use super::WORMHOLE_PART;
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

    /// Thing 0: a box. Thing 1: a wormhole switch on it with two outputs, the file storing
    /// on, the WORMHOLE part if `part`, its ended byte `ended`.
    fn world(part: bool, ended: bool) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: sig(1., 1),
            stored: sig(1., 1),
            targets: vec![],
            random_on: false,
        });
        let mut parts = vec!["POS", "SWITCH"];
        if part {
            parts.push(WORMHOLE_PART);
        }
        let switch = ThingState {
            parent: Some(0),
            parts,
            wormhole_ended: ended,
            switch: Some(SwitchState {
                switch_type: 57,
                outputs: vec![output; 2],
                ..Default::default()
            }),
            ..Default::default()
        };
        // The parent has the part and the byte; the switch reads its own Thing only.
        let object = ThingState {
            parts: vec!["POS", WORMHOLE_PART],
            wormhole_ended: true,
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![object, switch],
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
    fn wormhole_switch_reads_its_ended_byte() {
        // Without the part: off, not the stored on and not the parent's part.
        assert_eq!(outs(&mut world(false, true)), vec![SIGNAL_OFF; 2]);
        assert_eq!(outs(&mut world(true, false)), vec![sig(0., 0); 2]);
        assert_eq!(outs(&mut world(true, true)), vec![sig(1., 1); 2]);
    }

    #[test]
    fn wormhole_switch_inverts_through_the_standard_tail() {
        let mut w = world(true, false);
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert_eq!(outs(&mut w), vec![sig(1., 1); 2]);
        let mut w = world(true, true);
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert_eq!(outs(&mut w), vec![sig(0., 0); 2]);
    }
}
