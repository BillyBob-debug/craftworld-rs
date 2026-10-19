//! The velocity sensors: linear velocity (switch type 42) and angular velocity (type 43)
//! (docs/logic.md Part 19).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 sends both to 0x11d28c; linear
//! 0x11d1d0, angular 0x11d2a8):
//! - The body is the one of the switch Thing's parent (Thing+0x5c), or of the Thing
//!   itself when it has no parent. With no body, or a body whose +0x140 is set, the
//!   output is off (0x11bf08); type 42 is also off when that Thing has no position
//!   (Thing+0x2c).
//! - The scale is angleRange (PSwitch+0x7c, the float after minRadius; the angle switch
//!   0x11cd3c reads it too) and the mode is bulletsRequired (PSwitch+0xcc). The activation
//!   is the value over the scale, kept to [-1, 1]. The ternary is 1 at or above T, -1 at
//!   or below -T, else 0, with T 1.0 for type 42 (r2-0x4174) and 0.999 for type 43
//!   (r2-0x3e10). The player is -1. Inverted through the shared tail (table 0x11bfd4
//!   entries 42 and 43 are the standard 0x11c16c).
//! - Linear (0x11d1d0, 0x11db10): 0 when the body's halfword +0xe is set; else the
//!   velocity of the Thing's point: the body's linear velocity plus its angular
//!   velocity s times (r.y, -r.x), r the Thing's position less the body's centre. With
//!   resetWhenFull (PSwitch+0xe0, 0x11da58) the velocity is turned into the frame of
//!   0x1141f4's matrix: x' = x m.x + y m.y, y' = x m.y - y m.x. Modes: 1 y, 2 x, 3 the
//!   downward part max(-y, 0), 4 the upward max(y, 0), 5 the leftward max(-x, 0), 6 the
//!   rightward max(x, 0), 0 and above 6 the speed.
//! - Angular (0x11d2a8): the body's angular velocity ω. Modes: 1 max(ω, 0), 2
//!   max(-ω, 0), 0 and above 2 ω.
//!
//! INFERRED: the frame's m is the x axis of the body Thing's world matrix, made unit
//! length; whether 0x1141f4 also turns it to the nearest 90 degrees is not established.
//! A NaN activation (a scale of 0 and no motion) reads as 0. Not modelled: the branch
//! for a Thing under an ancestor of object kind 0x96 (0x3c594), which reads a joint's
//! angle (0x1d3700) or motion (0x1d315c) instead.
use crate::switches::{SwitchContext, SIGNAL_OFF};
use lbp_script_vm::natives::SwitchInput;

/// The motion of a Thing's physics body, as the host's physics reports it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyMotion {
    /// Linear velocity (body+0x110 → +0x10), units a second.
    pub linear: [f32; 2],
    /// Centre of mass (body+0x110 → +0x20).
    pub centre: [f32; 2],
    /// Angular velocity (body+0x110 → +0x30), radians a second.
    pub angular: f32,
    /// The halfword body+0xe: the linear sensor reads 0.
    pub still: bool,
    /// body+0x140: both sensors are off.
    pub inactive: bool,
}

/// The ternary threshold of the linear velocity sensor (r2-0x4174).
pub const LINEAR_THRESHOLD: f32 = 1.0;
/// The ternary threshold of the angular velocity sensor (r2-0x3e10).
pub const ANGULAR_THRESHOLD: f32 = 0.999;

/// The linear (42) or angular (43) velocity sensor (see the module).
pub(crate) fn velocity_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let Some(thing) = w.things.get(cx.thing as usize) else {
        return SIGNAL_OFF;
    };
    let obj = thing
        .parent
        .and_then(|p| w.things.get(p as usize))
        .unwrap_or(thing);
    let Some(body) = obj.body.filter(|b| !b.inactive) else {
        return SIGNAL_OFF;
    };
    let (value, threshold) = if s.switch_type == 42 {
        let Some(m) = obj.world_matrix else {
            return SIGNAL_OFF;
        };
        let [mut x, mut y] = if body.still {
            [0., 0.]
        } else {
            let r = [m[12] - body.centre[0], m[13] - body.centre[1]];
            [
                body.linear[0] + body.angular * r[1],
                body.linear[1] - body.angular * r[0],
            ]
        };
        if s.reset_when_full {
            let len = m[0].hypot(m[1]);
            let (mx, my) = if len > 0. {
                (m[0] / len, m[1] / len)
            } else {
                (1., 0.)
            };
            (x, y) = (x * mx + y * my, x * my - y * mx);
        }
        let v = match s.bullets_required {
            1 => y,
            2 => x,
            3 => (-y).max(0.),
            4 => y.max(0.),
            5 => (-x).max(0.),
            6 => x.max(0.),
            _ => x.hypot(y),
        };
        (v, LINEAR_THRESHOLD)
    } else {
        let w = body.angular;
        let v = match s.bullets_required {
            1 => w.max(0.),
            2 => (-w).max(0.),
            _ => w,
        };
        (v, ANGULAR_THRESHOLD)
    };
    let a = value / s.angle_range;
    let activation = if a.is_nan() { 0. } else { a.clamp(-1., 1.) };
    let ternary = if activation >= threshold {
        1
    } else if activation <= -threshold {
        -1
    } else {
        0
    };
    SwitchInput {
        activation,
        ternary,
        player: -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LevelWorld, SwitchOutputState, SwitchState, ThingState};

    /// Thing 0: a body moving at `linear` with its centre at the origin. Thing 1: a
    /// velocity switch of type `ty` on it, at (10, 0), with scale 100.
    fn world(ty: i32, linear: [f32; 2], angular: f32) -> LevelWorld {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
        m[12] = 10.;
        let body = ThingState {
            world_matrix: Some(m),
            body: Some(BodyMotion {
                linear,
                angular,
                ..Default::default()
            }),
            ..Default::default()
        };
        let sensor = ThingState {
            parent: Some(0),
            switch: Some(SwitchState {
                switch_type: ty,
                angle_range: 100.,
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
            things: vec![body, sensor],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn set(w: &mut LevelWorld, f: impl FnOnce(&mut SwitchState)) {
        f(w.things[1].switch.as_mut().unwrap());
    }

    fn out(w: &mut LevelWorld) -> (f32, i32, i32) {
        w.update_switches();
        let s = w.things[1].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal;
        (s.activation, s.ternary, s.player)
    }

    #[test]
    fn linear_velocity_modes_scale_and_threshold() {
        let mut w = world(42, [30., -40.], 0.);
        // Mode 0: the speed, 50 / 100.
        assert_eq!(out(&mut w), (0.5, 0, -1));
        let modes = [
            (1, -0.4),
            (2, 0.3),
            (3, 0.4),
            (4, 0.),
            (5, 0.),
            (6, 0.3),
            (7, 0.5),
        ];
        for (mode, a) in modes {
            set(&mut w, |s| s.bullets_required = mode);
            assert_eq!(out(&mut w).0, a, "mode {mode}");
        }
        // Full scale: on at 1.0, -1 below; kept to [-1, 1].
        set(&mut w, |s| (s.bullets_required, s.angle_range) = (1, 40.));
        assert_eq!(out(&mut w), (-1., -1, -1));
        set(&mut w, |s| (s.bullets_required, s.angle_range) = (2, 10.));
        assert_eq!(out(&mut w), (1., 1, -1));
        // Inverted: the standard tail, 1 - 0.5 and the ternary 0 turned to 1.
        set(&mut w, |s| {
            (s.bullets_required, s.angle_range, s.inverted) = (0, 100., true)
        });
        assert_eq!(out(&mut w), (0.5, 1, -1));
        // A still body reads 0; an inactive body, no body or no position, off.
        let mut w = world(42, [30., -40.], 0.);
        w.things[0].body.as_mut().unwrap().still = true;
        assert_eq!(out(&mut w), (0., 0, -1));
        w.things[0].body.as_mut().unwrap().inactive = true;
        assert_eq!(out(&mut w), (0., 0, -1));
        set(&mut w, |s| s.inverted = true);
        assert_eq!(out(&mut w), (1., 1, -1));
        let mut w = world(42, [30., -40.], 0.);
        w.things[0].world_matrix = None;
        set(&mut w, |s| s.inverted = true);
        assert_eq!(out(&mut w), (1., 1, -1));
    }

    #[test]
    fn linear_velocity_spin_and_local_frame() {
        // Spinning at 2 rad/s about the origin: the point at (10, 0) moves at (0, -20).
        let mut w = world(42, [0., 0.], 2.);
        set(&mut w, |s| s.bullets_required = 1);
        assert_eq!(out(&mut w).0, -0.2);
        // A Thing with no parent uses its own body.
        w.things[1].parent = None;
        assert_eq!(out(&mut w), (0., 0, -1));
        // The local frame: turned a quarter, x' = y and y' = x.
        let mut w = world(42, [30., -40.], 0.);
        let m = w.things[0].world_matrix.as_mut().unwrap();
        (m[0], m[1], m[4], m[5]) = (0., 2., -2., 0.);
        set(&mut w, |s| {
            (s.reset_when_full, s.bullets_required) = (true, 2)
        });
        assert_eq!(out(&mut w).0, -0.4);
        set(&mut w, |s| s.bullets_required = 1);
        assert_eq!(out(&mut w).0, 0.3);
    }

    #[test]
    fn angular_velocity_modes_and_threshold() {
        let mut w = world(43, [0., 0.], -50.);
        assert_eq!(out(&mut w), (-0.5, 0, -1));
        set(&mut w, |s| s.bullets_required = 1);
        assert_eq!(out(&mut w).0, 0.);
        set(&mut w, |s| s.bullets_required = 2);
        assert_eq!(out(&mut w).0, 0.5);
        // The threshold is 0.999: just under full scale is already on.
        set(&mut w, |s| (s.bullets_required, s.angle_range) = (0, 50.04));
        assert_eq!(out(&mut w).1, -1);
        set(&mut w, |s| s.angle_range = 51.);
        assert_eq!(out(&mut w).1, 0);
        // An angular sensor needs no position.
        w.things[0].world_matrix = None;
        assert_eq!(out(&mut w).1, 0);
        set(&mut w, |s| s.angle_range = 10.);
        assert_eq!(out(&mut w), (-1., -1, -1));
    }
}
