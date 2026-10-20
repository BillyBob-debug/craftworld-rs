//! The angle sensor switch (switch type 41) (docs/logic.md Part 25).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58, table 0x11be00 entry 41 →
//! 0x11cd3c, then the store 0x11bf18):
//! - The object is the switch Thing's parent (Thing+0x5c), or the Thing itself. 0x1141f4
//!   takes its world matrix's Y axis (m[4..6]), turned by the shortest arc that takes its
//!   Z axis (m[8..10], made unit length) onto +z (0x113fb0), and divides it by the length
//!   of its x, y part: b. For an object in the level plane that is (m[4], m[5]) made unit
//!   length.
//! - b is turned by PSwitch+0xdc radians (the other way when the matrix is mirrored), and
//!   the angle is atan2(-b.x, b.y): the object's turn about z, counterclockwise positive.
//! - r is the angle over angleRange (PSwitch+0x7c) in radians (/ 180.0 × π, r2-0x416c and
//!   r2-0x4020). By the mode, bulletsRequired (PSwitch+0xcc): 1 (0x11d940) 1 + r when r is
//!   below 0, else 0; 2 (0x11d9c8) 1 - r when r is above 0, else 0; any other 1 - |r|.
//!   The activation is that kept to [0, 1], the ternary 1 when it is above 0, else 0, and
//!   the player -1. Every output gets the same signal (the output index is not read).
//! - Inverted through the standard tail (table 0x11bfd4 entry 41 is 0x11c16c). No
//!   end-of-update step (0x124934 sends types above 0x27 past table 0x124b0c).
//!
//! Not modelled: PSwitch+0xdc (not read from the level file here, so 0, and the mirror
//! test then changes nothing); a switch Thing that is a creature's (0x3c594), whose b is
//! (sin φ, cos φ) of the creature's facing (creature+0x140 → +0x84) instead. Without a
//! world matrix the output is off here.
use crate::switches::{SwitchContext, SIGNAL_OFF};
use lbp_script_vm::natives::SwitchInput;

/// b of 0x1141f4 for the world matrix `m` (see the module).
fn direction(m: &[f32; 16]) -> [f32; 2] {
    let len = (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt();
    let z = [m[8] / len, m[9] / len, m[10] / len];
    // The shortest-arc quaternion from z to +z: xyz = z × (0, 0, 1) / s, w = s / 2.
    let s = (2. * (1. + z[2])).sqrt();
    let q = [z[1] / s, -z[0] / s, 0.];
    let w = s / 2.;
    let p = [m[4], m[5], m[6]];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let t = cross(q, p);
    let t = [2. * t[0], 2. * t[1], 2. * t[2]];
    let qt = cross(q, t);
    let u = [p[0] + w * t[0] + qt[0], p[1] + w * t[1] + qt[1]];
    let len = (u[0] * u[0] + u[1] * u[1]).sqrt();
    [u[0] / len, u[1] / len]
}

/// The angle sensor (see the module).
pub(crate) fn angle_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let Some(thing) = w.things.get(cx.thing as usize) else {
        return SIGNAL_OFF;
    };
    let obj = thing
        .parent
        .and_then(|p| w.things.get(p as usize))
        .unwrap_or(thing);
    let Some(m) = obj.world_matrix else {
        return SIGNAL_OFF;
    };
    let b = direction(&m);
    let angle = (-b[0]).atan2(b[1]);
    let r = angle / (s.angle_range / 180. * std::f32::consts::PI);
    let a = match s.bullets_required {
        1 if r < 0. => 1. + r,
        2 if r > 0. => 1. - r,
        1 | 2 => 0.,
        _ => 1. - r.abs(),
    };
    let activation = a.clamp(0., 1.);
    SwitchInput {
        activation,
        ternary: i32::from(activation > 0.),
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

    /// A matrix turned `deg` degrees about z (counterclockwise), mirrored in x if `flip`.
    fn turned(deg: f32, flip: bool) -> Option<[f32; 16]> {
        let (sin, cos) = deg.to_radians().sin_cos();
        let f = if flip { -1. } else { 1. };
        let mut m = [0.; 16];
        (m[0], m[1], m[4], m[5], m[10], m[15]) = (f * cos, f * sin, -sin, cos, 1., 1.);
        Some(m)
    }

    /// Thing 0: a box turned `deg`. Thing 1: an angle sensor on it, range 45, `mode`.
    fn world(deg: f32, mode: i32) -> LevelWorld {
        let output = Some(SwitchOutputState {
            signal: SIGNAL_OFF,
            stored: SIGNAL_OFF,
            targets: vec![],
            random_on: false,
        });
        let sensor = ThingState {
            parent: Some(0),
            switch: Some(SwitchState {
                switch_type: 41,
                angle_range: 45.,
                bullets_required: mode,
                outputs: vec![output],
                ..Default::default()
            }),
            ..Default::default()
        };
        let object = ThingState {
            world_matrix: turned(deg, false),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![object, sensor],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn out(w: &mut LevelWorld) -> SwitchInput {
        w.update_switches();
        w.things[1].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    fn near(a: SwitchInput, b: SwitchInput) -> bool {
        (a.activation - b.activation).abs() < 1e-5 && a.ternary == b.ternary && a.player == -1
    }

    #[test]
    fn angle_sensor_reads_its_objects_turn() {
        assert!(near(out(&mut world(0., 0)), sig(1., 1)));
        // 22.5 of 45 degrees either way: half.
        assert!(near(out(&mut world(22.5, 0)), sig(0.5, 1)));
        assert!(near(out(&mut world(-22.5, 0)), sig(0.5, 1)));
        // At or past the range: off.
        assert!(near(out(&mut world(45., 0)), sig(0., 0)));
        assert!(near(out(&mut world(170., 0)), sig(0., 0)));
        // Mirrored, the Y axis is unchanged.
        let mut w = world(22.5, 0);
        w.things[0].world_matrix = turned(22.5, true);
        assert!(near(out(&mut w), sig(0.5, 1)));
        // Out of the level plane: the Y axis is turned back into it first.
        let mut w = world(0., 0);
        let m = w.things[0].world_matrix.as_mut().unwrap();
        let (sin, cos) = 30f32.to_radians().sin_cos();
        (m[5], m[6], m[9], m[10]) = (cos, sin, -sin, cos);
        assert!(near(out(&mut w), sig(1., 1)));
    }

    #[test]
    fn angle_sensor_modes_and_inversion() {
        // Mode 1 counts only clockwise turns (r below 0), mode 2 only counterclockwise.
        assert!(near(out(&mut world(-9., 1)), sig(0.8, 1)));
        assert!(near(out(&mut world(9., 1)), sig(0., 0)));
        assert!(near(out(&mut world(0., 1)), sig(0., 0)));
        assert!(near(out(&mut world(9., 2)), sig(0.8, 1)));
        assert!(near(out(&mut world(-9., 2)), sig(0., 0)));
        // Inverted: the standard tail.
        let mut w = world(9., 0);
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert!(near(out(&mut w), sig(0.2, 0)));
    }
}
