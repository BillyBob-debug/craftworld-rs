//! PSwitchInput (part 0x20): the adapter a switch output drives on a Thing that is not a
//! logic part, such as a destroyer, a material made lethal or a sackbot (D1132,
//! docs/logic.md Part 34).
//!
//! FOUND (01.33): the part is not polled each frame. When an output's on state or player
//! changes, the switch pass (0x1244a4) calls 0x11ddf0 for each target, which jumps on the
//! target Thing's object type (+0x7e). Object types 68, 69 and 70 reach the lethal,
//! destroy and sackbot-behaviour handlers; no other updateType has one, and nothing reads
//! dataSource or the activation. Here the handler is chosen by the part's updateType
//! (INFERRED: object types 68/69/70 are the Things whose updateType is 2/1/10).

use crate::switches::wired;
use crate::{LevelWorld, SwitchInput};

/// ESwitchInputUpdateType DESTROY.
pub const UPDATE_DESTROY: i32 = 1;
/// ESwitchInputUpdateType LETHAL.
pub const UPDATE_LETHAL: i32 = 2;
/// ESwitchInputUpdateType SACKBOT_BEHAVIOR.
pub const UPDATE_SACKBOT_BEHAVIOR: i32 = 10;

/// A PSwitchInput at run time: the stored settings and the lethal frame it keeps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputAdapter {
    /// updateType (part +0x28).
    pub update_type: i32,
    /// lethalType (part +0x34): what the Thing becomes while the input is on.
    pub lethal_type: i32,
    /// lethalInverted (part +0x38): the lethal input is on while its signal is off.
    pub lethal_inverted: bool,
    /// The latch mode (part +0x30, INFERRED to be the stored `behavior`): 1 takes the
    /// frame of each rising edge and keeps it.
    pub latch: bool,
    /// includeRigidConnectors (part +0x3b): a destroy takes the rigidly joined Things too.
    pub include_rigid_connectors: bool,
    /// lethalActivationFrame (part +0x1c): the frame the lethal input came on, 0 while off.
    pub lethal_frame: i32,
}

/// A destroy the DESTROY input asked for ([`LevelWorld::destroy_requests`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyRequest {
    /// The Thing that owns the input; the game destroys its group (0x12f354).
    pub thing: u32,
    /// The signal's player, credited with the destroy.
    pub player: i32,
    pub include_rigid_connectors: bool,
    /// `PWorld.Frame` when the edge came.
    pub frame: i32,
}

/// Most destroy requests [`LevelWorld::destroy_requests`] keeps.
pub const MAX_DESTROY_REQUESTS: usize = 4096;

impl LevelWorld {
    /// A switch output reached `target`'s port with `on` (0x11ddf0's edge). Returns
    /// whether `target` has a PSwitchInput, which then takes the signal.
    ///
    /// FOUND: LETHAL (0x11ecf4) recomputes the frame from the Thing's own port 0 on every
    /// visit in the default mode: the current frame while (ternary != 0) XOR
    /// lethalInverted, else 0, and 0 when the Thing is deactivated or the port is not
    /// wired (0x12e3a8). In the latch mode it takes the current frame on `on` only.
    /// DESTROY (0x11ec6c) asks for the destroy on `on` only, credited to the signal's
    /// player. SACKBOT_BEHAVIOR (0x11ebf8) is not modelled.
    pub(crate) fn switch_input_signal(
        &mut self,
        target: u32,
        on: bool,
        signal: SwitchInput,
    ) -> bool {
        let i = target as usize;
        let Some(a) = self.things.get(i).and_then(|t| t.switch_input) else {
            return false;
        };
        let now = self.pworld.frame;
        match a.update_type {
            UPDATE_LETHAL if a.latch => {
                if on {
                    self.set_lethal_frame(i, now);
                }
            }
            UPDATE_LETHAL => {
                let port = wired(self, target, 0).map(|(_, s)| s);
                let active = port.is_some_and(|s| (s.ternary != 0) != a.lethal_inverted);
                let frame = if active && !self.things[i].deactivated {
                    now
                } else {
                    0
                };
                self.set_lethal_frame(i, frame);
            }
            UPDATE_DESTROY if on => {
                if self.destroy_requests.len() < MAX_DESTROY_REQUESTS {
                    self.destroy_requests.push(DestroyRequest {
                        thing: target,
                        player: signal.player,
                        include_rigid_connectors: a.include_rigid_connectors,
                        frame: now,
                    });
                }
            }
            // 0x12eccc on the owner's parent and its behaviour object: not modelled.
            UPDATE_SACKBOT_BEHAVIOR => {}
            _ => {}
        }
        true
    }

    fn set_lethal_frame(&mut self, i: usize, frame: i32) {
        if let Some(a) = self.things[i].switch_input.as_mut() {
            a.lethal_frame = frame;
        }
    }

    /// The Thing at the top of `thing`'s parent chain.
    fn group_root(&self, thing: u32) -> u32 {
        let mut t = thing;
        for _ in 0..self.things.len() {
            match self.things.get(t as usize).and_then(|s| s.parent) {
                Some(p) => t = p,
                None => break,
            }
        }
        t
    }

    /// The lethal type the LETHAL inputs give `thing`'s group (0x12e114), 0 for none.
    ///
    /// FOUND: over the group's LETHAL inputs on Things that are not deactivated, the one
    /// with the largest nonzero frame wins. With none on, an input whose port 0 is not
    /// wired applies (it is always on). INFERRED: the group is the Things with the same
    /// root of the parent chain. The game writes the type to the material only when its
    /// current value is not 6 to 11; that write is not modelled.
    pub fn lethal_type(&self, thing: u32) -> i32 {
        let root = self.group_root(thing);
        let mut best: Option<(i32, i32)> = None;
        let mut always = None;
        for (i, t) in self.things.iter().enumerate() {
            let Some(a) = t.switch_input.filter(|a| a.update_type == UPDATE_LETHAL) else {
                continue;
            };
            let Ok(n) = u32::try_from(i) else { break };
            if t.deactivated || self.group_root(n) != root {
                continue;
            }
            if a.lethal_frame != 0 && best.is_none_or(|(f, _)| a.lethal_frame > f) {
                best = Some((a.lethal_frame, a.lethal_type));
            }
            if always.is_none() && t.inputs.first().copied().flatten().is_none() {
                always = Some(a.lethal_type);
            }
        }
        best.map(|b| b.1).or(always).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputWire, SwitchOutputState, SwitchState, ThingState};

    /// A battery on Thing 0 wired to port 0 of Thing 1, which has `adapter`.
    fn world(adapter: InputAdapter) -> LevelWorld {
        let battery = ThingState {
            switch: Some(SwitchState {
                switch_type: 25,
                outputs: vec![Some(SwitchOutputState {
                    targets: vec![(Some(1), 0)],
                    ..Default::default()
                })],
                ..Default::default()
            }),
            ..Default::default()
        };
        let input = ThingState {
            switch_input: Some(adapter),
            inputs: vec![Some(InputWire {
                switch: 0,
                output: 0,
            })],
            ..Default::default()
        };
        LevelWorld {
            things: vec![battery, input],
            ..Default::default()
        }
    }

    fn drive(w: &mut LevelWorld, ternary: i32, on: bool) {
        let s = SwitchInput {
            activation: ternary as f32,
            ternary,
            player: 2,
        };
        if let Some(o) = w.things[0]
            .switch
            .as_mut()
            .and_then(|s| s.outputs[0].as_mut())
        {
            o.signal = s;
        }
        assert!(w.switch_input_signal(1, on, s));
    }

    #[test]
    fn lethal_input_follows_its_port_and_inverts() {
        let lethal = InputAdapter {
            update_type: UPDATE_LETHAL,
            lethal_type: 1,
            ..Default::default()
        };
        let mut w = world(lethal);
        w.pworld.frame = 40;
        assert_eq!(w.lethal_type(1), 0);
        drive(&mut w, 1, true);
        assert_eq!(w.things[1].switch_input.map(|a| a.lethal_frame), Some(40));
        assert_eq!(w.lethal_type(1), 1);
        drive(&mut w, 0, false);
        assert_eq!(w.lethal_type(1), 0);
        let mut w = world(InputAdapter {
            lethal_inverted: true,
            ..lethal
        });
        w.pworld.frame = 7;
        drive(&mut w, 0, false);
        assert_eq!(w.lethal_type(1), 1);
        w.things[1].deactivated = true;
        drive(&mut w, 0, false);
        assert_eq!(w.lethal_type(1), 0);
    }

    #[test]
    fn latest_lethal_input_wins_and_unwired_is_always_on() {
        let mut w = world(InputAdapter {
            update_type: UPDATE_LETHAL,
            lethal_type: 1,
            latch: true,
            ..Default::default()
        });
        w.things.push(ThingState {
            parent: Some(1),
            switch_input: Some(InputAdapter {
                update_type: UPDATE_LETHAL,
                lethal_type: 4,
                lethal_frame: 30,
                ..Default::default()
            }),
            inputs: vec![Some(InputWire {
                switch: 0,
                output: 0,
            })],
            ..Default::default()
        });
        assert_eq!(w.lethal_type(1), 4);
        w.pworld.frame = 50;
        drive(&mut w, 0, false);
        assert_eq!(w.lethal_type(2), 4, "the latch takes edges only");
        drive(&mut w, 1, true);
        assert_eq!(w.lethal_type(2), 1);
        w.things[2].switch_input = w.things[2].switch_input.map(|a| InputAdapter {
            lethal_frame: 0,
            ..a
        });
        w.things[1].switch_input = w.things[1].switch_input.map(|a| InputAdapter {
            lethal_frame: 0,
            ..a
        });
        w.things[2].inputs.clear();
        assert_eq!(w.lethal_type(1), 4);
    }

    #[test]
    fn destroy_input_asks_on_the_edge_only() {
        let mut w = world(InputAdapter {
            update_type: UPDATE_DESTROY,
            include_rigid_connectors: true,
            ..Default::default()
        });
        w.pworld.frame = 9;
        drive(&mut w, 1, false);
        assert!(w.destroy_requests.is_empty());
        drive(&mut w, 1, true);
        let want = DestroyRequest {
            thing: 1,
            player: 2,
            include_rigid_connectors: true,
            frame: 9,
        };
        assert_eq!(w.destroy_requests, [want]);
        assert!(!w.switch_input_signal(0, true, SwitchInput::default()));
    }
}
