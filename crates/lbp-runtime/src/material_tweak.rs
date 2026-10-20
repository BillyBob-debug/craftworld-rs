//! PMaterialTweak (the material tweaker): what the tweaks of a group give its physics
//! body (D1133, docs/logic.md Part 35).
//!
//! FOUND (01.33): a switch output reaching a Thing with a tweak (kind 0x4d) only makes the
//! game recompute the body (0x11e394 -> 0xadd84 -> 0xadc64 -> 0xfb258); the signal value is
//! not used there. The body takes the tweaks that are on (0xadbac) across its rigid group
//! (0x3dae4): restitution and friction scale are averaged, grabbability, grab filter and
//! stickiness take the largest, noAutoDestruct, disablePhysicsAudio and hasShadow are
//! OR'd. Here the result is computed when asked, which gives the same answer.

use crate::switches::wired;
use crate::LevelWorld;

/// The stickiness value that makes the tweak its own kind (0x9f, 0x3ed2c), handled apart
/// from the group average and not modelled here.
pub const STICKINESS_SEPARATE: u8 = 3;

/// What the group's tweaks that are on give its body (0xfb258).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyMaterial {
    /// The tweaks that count.
    pub tweaks: u32,
    /// Mean restitution (body+0x74, shape byte +0x78 = v * 255).
    pub restitution: f32,
    /// Mean friction scale (body+0x78, shape byte +0x79 = v * 255).
    pub friction_scale: f32,
    /// Largest grabbability (body+0xc8).
    pub grabbability: i8,
    /// Largest grab filter (body+0xc7).
    pub grab_filter: i8,
    /// Largest stickiness, low 7 bits (body+0xc9).
    pub stickiness: u8,
    /// Any noAutoDestruct (body+0xc0 flag 0x40).
    pub no_auto_destruct: bool,
    /// Any disablePhysicsAudio (body+0xc0 flag 0x100).
    pub disable_physics_audio: bool,
    /// Any hasShadow.
    pub has_shadow: bool,
}

/// The stored stickiness without the flag in bit 7 (0xadb84).
fn stickiness(raw: i8) -> u8 {
    raw as u8 & 0x7f
}

impl LevelWorld {
    /// Whether `thing`'s tweak is on (0xadbac).
    ///
    /// FOUND: off on a deactivated Thing; on with no wired input; otherwise on while the
    /// wired output's value is nonzero. INFERRED: the input is port 0 and the value is the
    /// activation.
    pub fn tweak_on(&self, thing: u32) -> bool {
        let Some(t) = self.things.get(thing as usize) else {
            return false;
        };
        if t.material_tweak.is_none() || t.deactivated {
            return false;
        }
        wired(self, thing, 0).is_none_or(|(_, s)| s.activation != 0.0)
    }

    /// What the tweaks of `thing`'s group give its body; None when no tweak in it is on,
    /// and the body keeps its material's values (friction scale 1.0).
    ///
    /// INFERRED: the rigid group is the Things with the same root of the parent chain.
    /// Tweaks with stickiness 3 do not count (kind 0x9f; the game writes their friction
    /// and restitution times 100 to a child's body instead).
    pub fn body_material(&self, thing: u32) -> Option<BodyMaterial> {
        let root = self.group_root(thing);
        let mut m = BodyMaterial::default();
        for (i, t) in self.things.iter().enumerate() {
            let Some(tw) = t.material_tweak else {
                continue;
            };
            let Ok(n) = u32::try_from(i) else { break };
            let sticky = stickiness(tw.stickiness);
            if sticky == STICKINESS_SEPARATE || self.group_root(n) != root || !self.tweak_on(n) {
                continue;
            }
            m.tweaks += 1;
            m.restitution += tw.restitution;
            m.friction_scale += tw.friction_scale;
            m.grabbability = m.grabbability.max(tw.grabbability);
            m.grab_filter = m.grab_filter.max(tw.grab_filter);
            m.stickiness = m.stickiness.max(sticky);
            m.no_auto_destruct |= tw.no_auto_destruct;
            m.disable_physics_audio |= tw.disable_physics_audio;
            m.has_shadow |= tw.has_shadow.unwrap_or(true);
        }
        if m.tweaks == 0 {
            return None;
        }
        m.restitution /= m.tweaks as f32;
        m.friction_scale /= m.tweaks as f32;
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputWire, SwitchOutputState, SwitchState, ThingState};
    use lbp_level::parts::MaterialTweak;

    fn tweak(restitution: f32, friction_scale: f32, stickiness: i8) -> MaterialTweak {
        MaterialTweak {
            restitution,
            friction_scale,
            stickiness,
            grabbability: stickiness,
            has_shadow: Some(true),
            ..Default::default()
        }
    }

    /// Thing 0 a battery; Things 1 and 2 a group (2's parent is 1) with tweaks; Thing 2's
    /// port 0 is wired to the battery.
    fn world() -> LevelWorld {
        let battery = ThingState {
            switch: Some(SwitchState {
                switch_type: 25,
                outputs: vec![Some(SwitchOutputState::default())],
                ..Default::default()
            }),
            ..Default::default()
        };
        let a = ThingState {
            material_tweak: Some(tweak(0.2, 1.0, 1)),
            ..Default::default()
        };
        let b = ThingState {
            parent: Some(1),
            material_tweak: Some(tweak(0.6, 3.0, 2)),
            inputs: vec![Some(InputWire {
                switch: 0,
                output: 0,
            })],
            ..Default::default()
        };
        LevelWorld {
            things: vec![battery, a, b],
            ..Default::default()
        }
    }

    fn set(w: &mut LevelWorld, activation: f32) {
        if let Some(o) = w.things[0]
            .switch
            .as_mut()
            .and_then(|s| s.outputs[0].as_mut())
        {
            o.signal.activation = activation;
        }
    }

    #[test]
    fn group_tweaks_average_and_take_the_largest() {
        let mut w = world();
        set(&mut w, 1.0);
        let m = w.body_material(2).unwrap();
        assert_eq!(m.tweaks, 2);
        assert!((m.restitution - 0.4).abs() < 1e-6);
        assert!((m.friction_scale - 2.0).abs() < 1e-6);
        assert_eq!((m.grabbability, m.stickiness), (2, 2));
        assert!(m.has_shadow && !m.no_auto_destruct);
        assert_eq!(w.body_material(1), Some(m));
        assert_eq!(w.body_material(0), None);
    }

    #[test]
    fn wired_tweak_follows_its_input_and_deactivation() {
        let mut w = world();
        assert!(!w.tweak_on(2));
        assert!(w.tweak_on(1), "an unwired tweak is always on");
        assert_eq!(w.body_material(2).map(|m| m.tweaks), Some(1));
        set(&mut w, 0.5);
        assert!(w.tweak_on(2));
        w.things[1].deactivated = true;
        w.things[2].deactivated = true;
        assert_eq!(w.body_material(1), None);
    }

    #[test]
    fn stickiness_three_is_kept_apart_and_bit_7_is_dropped() {
        let mut w = world();
        set(&mut w, 1.0);
        w.things[2].material_tweak = Some(tweak(0.6, 3.0, 3));
        assert_eq!(w.body_material(1).map(|m| m.tweaks), Some(1));
        w.things[2].material_tweak = Some(tweak(0.6, 3.0, -126));
        assert_eq!(w.body_material(1).map(|m| m.stickiness), Some(2));
    }
}
