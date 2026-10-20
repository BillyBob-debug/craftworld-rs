//! The control pad switch (switch type 38, the controlinator's 18 outputs) (docs/logic.md
//! Part 21) and the movinator pad (switch type 44, 25 outputs, Part 32).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 entry 38 → 0x11d378):
//! - The pad is the driver's (the object at +0x98 reached from the switch's Thing,
//!   0x11d378). Each output's activation comes from 0x6a1004 and its player from
//!   0x6a0728; the code after them only sets up a button prompt (0x3dae04) and stores
//!   the same signal (0x11bf18).
//! - No driver, or a driver whose pad is not live (pad+0x1c & 0x1001): every output 0.0
//!   and the player -1 (0x6a13f0, 0x6a0728).
//! - The outputs (0x6a1004, tables 0x6a15c0): 0 the constant 1.0; 1 square; 2 circle;
//!   3 triangle; 4 cross or d-pad right; 5 L1; 6 L2's pressure over 255 while L2 is down;
//!   7 L3; 8 R1 or d-pad up; 9 R2's pressure as L2's; 10 d-pad right less left; 11 d-pad
//!   up less down; 12 and 13 the left stick x and y (pad+0x30, +0x34); 14 and 15 the
//!   right stick x and y (0x38b604); 16 the tilt pad+0xf8 over 90, 17 the tilt pad+0x100
//!   over -90, both kept to [-1, 1]. A button reads 1.0 when down, else 0.0.
//! - The player is the driver's number (*(slot)+0xc).
//! - The ternary (0x11873c, the jump at 0x118830): outputs 6 and 9 are 1 above 0, else
//!   0; the others 0 within the dead zone 0.25 (r2-0x3f10), 1 at or above 0.75, -1 at or
//!   below -0.75, between them what the output holds (its slot+0x1c, 0x1188c0).
//! - Inverted (PSwitch+0xc5): outputs 10 to 17 negated (table 0x11bfd4 entry 38 is
//!   0x11c228), the others the standard tail. No end-of-update step (table 0x124b0c
//!   entry 38 is the epilogue 0x124940).
//! - The movinator pad (table 0x11be00 entry 44) runs the same handler 0x11d378 and the
//!   same ternary jump (0x118830); inverted, its outputs 10 to 24 are negated (table
//!   0x11bfd4 entry 44 is 0x11c24c). Its outputs past 17 read 0.0 here (not modelled:
//!   0x6a1004's values for them); the button prompts set after it (0x11d448) are not
//!   modelled. All 50 placed store the no-driver signal on all 25 outputs.
//!
//! INFERRED: the driver is the player whose `piloting` is the switch's Thing; circle and
//! cross are not swapped (0x6a0ea8 reads 0, as for a pad whose enter button is cross);
//! the mirror sign of outputs 10, 12, 14 and 16 (a determinant, 0x6a1140) is +1; the
//! held ternary of an inverted output is turned back as the inversion turns it. Not
//! modelled: recorded values (an entry at +0x54 → +0x60 whose +0x18 is not 0, 0x6a122c),
//! the mask 0xff0fff7f that drops square and the d-pad (a byte +0x50), the second table
//! 0x6a163c (an object whose +0x7e is 7), the right stick's response curve (0x38b3ec) and
//! the d-pad and tilt fallbacks (0x386c78, 0x386fa8).
use crate::switches::SwitchContext;
use lbp_script_vm::natives::SwitchInput;

/// The PS3 pad as the controlinator reads it (pad+0x50 and the analogue values).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadState {
    /// Whether the pad is live (pad+0x1c & 0x1001).
    pub live: bool,
    /// The buttons (pad+0x50), the `BUTTON_*` bits.
    pub buttons: u32,
    /// L2 and R2 pressure, 0 to 255 (pad+0x130 + 2 * 0xa and 0xb).
    pub l2: u8,
    pub r2: u8,
    /// Left stick x and y (pad+0x30, +0x34), -1 to 1.
    pub left: [f32; 2],
    /// Right stick x and y (0x38b604), -1 to 1.
    pub right: [f32; 2],
    /// Tilt in degrees (pad+0xf8, +0x100).
    pub tilt: [f32; 2],
}

pub const BUTTON_L2: u32 = 0x1;
pub const BUTTON_R2: u32 = 0x2;
pub const BUTTON_L1: u32 = 0x4;
pub const BUTTON_R1: u32 = 0x8;
pub const BUTTON_TRIANGLE: u32 = 0x10;
pub const BUTTON_CIRCLE: u32 = 0x20;
pub const BUTTON_CROSS: u32 = 0x40;
pub const BUTTON_SQUARE: u32 = 0x80;
pub const BUTTON_L3: u32 = 0x2_0000;
pub const BUTTON_UP: u32 = 0x10_0000;
pub const BUTTON_RIGHT: u32 = 0x20_0000;
pub const BUTTON_DOWN: u32 = 0x40_0000;
pub const BUTTON_LEFT: u32 = 0x80_0000;

/// The ternary dead zone of the pad's outputs (r2-0x3f10).
pub const DEAD_ZONE: f32 = 0.25;

/// The value of output `output` of a live pad (0x6a1004).
fn pad_value(pad: &PadState, output: usize) -> f32 {
    let down = |mask: u32| f32::from(u8::from(pad.buttons & mask != 0));
    let pressure = |mask: u32, p: u8| {
        if pad.buttons & mask != 0 {
            f32::from(p) / 255.
        } else {
            0.
        }
    };
    match output {
        0 => 1.,
        1 => down(BUTTON_SQUARE),
        2 => down(BUTTON_CIRCLE),
        3 => down(BUTTON_TRIANGLE),
        4 => down(BUTTON_CROSS | BUTTON_RIGHT),
        5 => down(BUTTON_L1),
        6 => pressure(BUTTON_L2, pad.l2),
        7 => down(BUTTON_L3),
        8 => down(BUTTON_R1 | BUTTON_UP),
        9 => pressure(BUTTON_R2, pad.r2),
        10 => down(BUTTON_RIGHT) - down(BUTTON_LEFT),
        11 => down(BUTTON_UP) - down(BUTTON_DOWN),
        12 => pad.left[0],
        13 => pad.left[1],
        14 => pad.right[0],
        15 => pad.right[1],
        16 => (pad.tilt[0] / 90.).clamp(-1., 1.),
        17 => (pad.tilt[1] / -90.).clamp(-1., 1.),
        _ => 0.,
    }
}

/// Whether output `output` of a pad of `switch_type` is negated when inverted: 10 to 17
/// of the control pad (0x11c228), 10 to 24 of the movinator pad (0x11c24c).
pub(crate) fn negated(switch_type: i32, output: usize) -> bool {
    let last = if switch_type == 44 { 24 } else { 17 };
    (10..=last).contains(&output)
}

/// The ternary of output `output` (0x11873c for types 38 and 44, the jump at 0x118830).
fn pad_ternary(cx: &SwitchContext<'_>, output: usize, a: f32) -> i32 {
    if output == 6 || output == 9 {
        return i32::from(a > 0.);
    }
    if a.abs() <= DEAD_ZONE {
        0
    } else if a >= 1. - DEAD_ZONE {
        1
    } else if a <= DEAD_ZONE - 1. {
        -1
    } else {
        let held = cx.current(output).ternary;
        match (cx.switch.inverted, negated(cx.switch.switch_type, output)) {
            (false, _) => held,
            (true, true) => -held,
            (true, false) => i32::from(held == 0),
        }
    }
}

/// The control pad switch's output `output` (see the module).
pub(crate) fn pad_sensor(cx: &SwitchContext<'_>, output: usize) -> SwitchInput {
    let driver = cx
        .world
        .players
        .iter()
        .find(|p| p.piloting == Some(cx.thing) && p.pad.live);
    let (activation, player) = match driver {
        Some(p) => (pad_value(&p.pad, output), p.number),
        None => (0., -1),
    };
    SwitchInput {
        activation,
        ternary: pad_ternary(cx, output, activation),
        player,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::switches::SIGNAL_OFF;
    use crate::{LevelWorld, SwitchOutputState, SwitchState, ThingState};

    /// Thing 0: a control pad switch with 18 outputs. Player 0 (Thing 1) drives it with
    /// a live pad and nothing pressed.
    fn world() -> LevelWorld {
        pad_world(38, 18)
    }

    /// As [`world`], a pad switch of `switch_type` with `outputs` outputs.
    fn pad_world(switch_type: i32, outputs: usize) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: SIGNAL_OFF,
            stored: SIGNAL_OFF,
            targets: vec![],
            random_on: false,
        });
        let pad = ThingState {
            switch: Some(SwitchState {
                switch_type,
                outputs: vec![output; outputs],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![pad],
            ..Default::default()
        };
        w.add_player([0.; 3], (0., 0.));
        w.players[0].piloting = Some(0);
        w.players[0].pad.live = true;
        w.wire_inputs();
        w
    }

    fn outs(w: &mut LevelWorld) -> Vec<(f32, i32, i32)> {
        w.update_switches();
        let s = w.things[0].switch.as_ref().unwrap();
        s.outputs
            .iter()
            .map(|o| {
                let s = o.as_ref().unwrap().signal;
                (s.activation, s.ternary, s.player)
            })
            .collect()
    }

    #[test]
    fn control_pad_maps_buttons_sticks_and_tilt() {
        let mut w = world();
        let o = outs(&mut w);
        assert_eq!(o[0], (1., 1, 0));
        assert!(o[1..].iter().all(|&s| s == (0., 0, 0)));
        let pad = &mut w.players[0].pad;
        pad.buttons = BUTTON_SQUARE | BUTTON_L2 | BUTTON_UP | BUTTON_LEFT | BUTTON_L3;
        (pad.l2, pad.r2) = (51, 255);
        (pad.left, pad.right, pad.tilt) = ([0.5, -1.], [0.8, 0.1], [45., 180.]);
        let o = outs(&mut w);
        let a: Vec<f32> = o.iter().map(|s| s.0).collect();
        let want = [
            1., 1., 0., 0., 0., 0., 0.2, 1., 1., 0., -1., 1., 0.5, -1., 0.8, 0.1, 0.5, -1.,
        ];
        assert_eq!(a, want);
        // L2 at 0.2 is on (above 0); R2 is up, so off whatever its pressure.
        assert_eq!((o[6].1, o[9].1), (1, 0));
        // Cross or d-pad right, R1 or d-pad up.
        w.players[0].pad.buttons = BUTTON_RIGHT;
        let o = outs(&mut w);
        assert_eq!((o[4].0, o[8].0, o[10].0), (1., 0., 1.));
        // No driver, or a pad that is not live: all 0.0, the player -1.
        w.players[0].pad.live = false;
        assert!(outs(&mut w).iter().all(|&s| s == (0., 0, -1)));
        w.players[0].pad.live = true;
        w.players[0].piloting = None;
        assert!(outs(&mut w).iter().all(|&s| s == (0., 0, -1)));
    }

    #[test]
    fn control_pad_dead_zone_hysteresis_and_inversion() {
        let mut w = world();
        let stick = |w: &mut LevelWorld, x: f32| {
            w.players[0].pad.left[0] = x;
            outs(w)[12]
        };
        // In the dead zone 0; at 0.75 on; between them what the output holds.
        assert_eq!(stick(&mut w, 0.25).1, 0);
        assert_eq!(stick(&mut w, 0.5).1, 0);
        assert_eq!(stick(&mut w, 0.75).1, 1);
        assert_eq!(stick(&mut w, 0.5).1, 1);
        assert_eq!(stick(&mut w, -0.5).1, 1);
        assert_eq!(stick(&mut w, -0.8).1, -1);
        assert_eq!(stick(&mut w, 0.2).1, 0);
        // Inverted: the signed outputs negated, the others the standard tail.
        w.things[0].switch.as_mut().unwrap().inverted = true;
        w.players[0].pad.buttons = BUTTON_SQUARE;
        let o = stick(&mut w, 0.8);
        assert_eq!(o, (-0.8, -1, 0));
        let o = outs(&mut w);
        assert_eq!((o[0], o[1], o[2]), ((0., 0, 0), (0., 0, 0), (1., 1, 0)));
        // The held ternary of an inverted signed output is turned back first.
        assert_eq!(stick(&mut w, 0.5), (-0.5, -1, 0));
    }

    #[test]
    fn movinator_pad_reads_the_pad_and_negates_to_24() {
        let mut w = pad_world(44, 25);
        w.players[0].pad.left[0] = 0.8;
        let o = outs(&mut w);
        assert_eq!((o[0], o[12]), ((1., 1, 0), (0.8, 1, 0)));
        // Past the pad's 18 outputs: 0.0 here.
        assert!(o[18..].iter().all(|&s| s == (0., 0, 0)));
        // Inverted: 10 to 24 negated, the others the standard tail.
        w.things[0].switch.as_mut().unwrap().inverted = true;
        let o = outs(&mut w);
        assert_eq!(
            (o[1], o[12], o[24]),
            ((1., 1, 0), (-0.8, -1, 0), (-0., 0, 0))
        );
        // No driver: all 0.0 and the player -1, as all 50 placed store.
        w.things[0].switch.as_mut().unwrap().inverted = false;
        w.players[0].piloting = None;
        assert!(outs(&mut w).iter().all(|&s| s == (0., 0, -1)));
    }
}
