//! Cutscene cameras and the camera finished switch (docs/logic.md Part 16).
//!
//! FOUND in the 01.33 executable (the cutscene manager's update 0x369130, its start
//! 0x368c40 and switch 0x368680, the camera finished output 0x11c854 → 0x10ee78):
//! - A camera tweak of cameraType 3 is a cutscene camera. Its time-since-used byte
//!   (PCameraTweak+0x5d) is reset to 0 while its input is on or while it is the
//!   cutscene's camera, and otherwise counts up by one a tick while below 10 (0x369130's
//!   first loop; 0xff never counts).
//! - With no cutscene running, a cutscene camera whose time is above 9 and whose input
//!   is on starts one (0x368c40): it becomes the cutscene's camera, the tick count and
//!   the ending count go to 0 and its time to 0 (0x368680, 0x549f8).
//! - While the ending count is not above 0 the tick count goes up by one a tick, and once
//!   the hold time (cutSceneHoldTime, in ticks; below 0 for ever) is below the tick count
//!   the ending count is set to 3 (0x369130). Each later tick takes one off; at 0 the
//!   cutscene ends and no camera is the cutscene's.
//! - Camera finished (switch type 32): off unless the switch's Thing has a cutscene
//!   camera; off while it is the cutscene's camera with the ending count not above 0;
//!   on ({1.0, 1, -1}) while its time is at most 9; else off. So it fires as the cutscene
//!   ends and for 9 ticks after.
//!
//! INFERRED: the input is the signal wired to the camera's port 0, on when its ternary
//! is not 0 (the game reads a byte of the Thing's input object, 0x57108); the step runs
//! before the switch pass. Not modelled: the trigger box of an unwired camera
//! (0x56808), the transition in (the tick count waits for it) and out, a hold time from
//! the camera's audio (0x55160), skipping, chained cameras (0x365470), single use
//! (PCameraTweak+0x72, 0x54a10) and the game-paused checks.
use crate::switches::{wired, SwitchContext, SIGNAL_OFF};
use crate::LevelWorld;
use lbp_script_vm::natives::SwitchInput;

/// The camera type of a cutscene camera (PCameraTweak+0x30).
pub const CUTSCENE_CAMERA: i32 = 3;

/// A camera tweak at run time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CameraTweakState {
    pub camera_type: i32,
    /// cutSceneHoldTime, in ticks; below 0 the cutscene holds until it is ended otherwise.
    pub hold_time: i32,
    /// cutSceneTimeSinceUsed: ticks since the camera was last used, kept to 10 (0xff:
    /// never used).
    pub time_since_used: u8,
}

/// The running cutscene (the manager's state).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cutscene {
    /// The cutscene's camera Thing (manager+4).
    pub camera: u32,
    /// Ticks shown so far (manager+0x14).
    pub ticks: i32,
    /// Ticks left before it ends, once the hold is over (manager+0x18).
    pub ending: i32,
}

impl LevelWorld {
    /// Whether the cutscene camera `thing` has its input on (INFERRED, see the module).
    fn camera_input(&self, thing: usize) -> bool {
        let deactivated = self.things.get(thing).is_some_and(|t| t.deactivated);
        let on = u32::try_from(thing)
            .ok()
            .and_then(|t| wired(self, t, 0))
            .is_some_and(|(_, s)| s.ternary != 0);
        !deactivated && on
    }

    /// One tick of the cutscene manager (0x369130): the cameras' time since used, the
    /// running cutscene's hold and end, then a start.
    pub fn cutscene_step(&mut self) {
        let current = self.cutscene.map(|c| c.camera as usize);
        for i in 0..self.things.len() {
            let Some(c) = self.things[i].camera_tweak else {
                continue;
            };
            if c.camera_type != CUTSCENE_CAMERA || c.time_since_used >= 10 {
                continue;
            }
            let time = if self.camera_input(i) || current == Some(i) {
                0
            } else {
                c.time_since_used + 1
            };
            if let Some(c) = self.things[i].camera_tweak.as_mut() {
                c.time_since_used = time;
            }
        }
        if let Some(mut cut) = self.cutscene {
            let hold = self
                .things
                .get(cut.camera as usize)
                .and_then(|t| t.camera_tweak);
            if cut.ending < 1 {
                let hold = hold.map_or(-1, |c| c.hold_time);
                if hold >= 0 && hold < cut.ticks {
                    cut.ending = 3;
                }
                cut.ticks += 1;
                self.cutscene = Some(cut);
            } else {
                cut.ending -= 1;
                self.cutscene = (cut.ending != 0).then_some(cut);
            }
            return;
        }
        let start = (0..self.things.len()).find(|&i| {
            self.things[i]
                .camera_tweak
                .is_some_and(|c| c.camera_type == CUTSCENE_CAMERA && c.time_since_used > 9)
                && self.camera_input(i)
        });
        if let Some(i) = start {
            if let Some(c) = self.things[i].camera_tweak.as_mut() {
                c.time_since_used = 0;
            }
            self.cutscene = Some(Cutscene {
                camera: i as u32,
                ..Default::default()
            });
        }
    }
}

/// The camera finished switch (type 32; 0x11c854 → 0x10ee78, see the module).
pub(crate) fn camera_finished(cx: &SwitchContext<'_>) -> SwitchInput {
    let w = cx.world;
    let Some(c) = w.things.get(cx.thing as usize).and_then(|t| t.camera_tweak) else {
        return SIGNAL_OFF;
    };
    if c.camera_type != CUTSCENE_CAMERA {
        return SIGNAL_OFF;
    }
    if w.cutscene
        .is_some_and(|cut| cut.camera == cx.thing && cut.ending <= 0)
    {
        return SIGNAL_OFF;
    }
    if c.time_since_used <= 9 {
        SwitchInput {
            activation: 1.,
            ternary: 1,
            player: -1,
        }
    } else {
        SIGNAL_OFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SwitchOutputState, SwitchState, ThingState};

    fn output(signal: SwitchInput, targets: &[(u32, i32)]) -> Option<SwitchOutputState> {
        Some(SwitchOutputState {
            signal,
            stored: signal,
            targets: targets.iter().map(|&(t, p)| (Some(t), p)).collect(),
            random_on: false,
        })
    }

    fn on() -> SwitchInput {
        SwitchInput {
            activation: 1.,
            ternary: 1,
            player: -1,
        }
    }

    /// Thing 0: a battery wired to Thing 1's port 0. Thing 1: a cutscene camera with a
    /// camera finished switch, holding `hold` ticks, never used.
    fn world(hold: i32) -> LevelWorld {
        let battery = ThingState {
            switch: Some(SwitchState {
                switch_type: 28,
                outputs: vec![output(SIGNAL_OFF, &[(1, 0)])],
                ..Default::default()
            }),
            ..Default::default()
        };
        let camera = ThingState {
            switch: Some(SwitchState {
                switch_type: 32,
                outputs: vec![output(SIGNAL_OFF, &[])],
                ..Default::default()
            }),
            camera_tweak: Some(CameraTweakState {
                camera_type: CUTSCENE_CAMERA,
                hold_time: hold,
                time_since_used: 0xff,
            }),
            ..Default::default()
        };
        let mut w = LevelWorld {
            things: vec![battery, camera],
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn set_battery(w: &mut LevelWorld, signal: SwitchInput) {
        let s = w.things[0].switch.as_mut().unwrap();
        s.memory.manual = signal;
    }

    /// One tick (the cutscene step, then the switch pass): camera finished's output.
    fn tick(w: &mut LevelWorld) -> SwitchInput {
        w.update_switches();
        w.things[1].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    fn time(w: &LevelWorld) -> u8 {
        w.things[1].camera_tweak.unwrap().time_since_used
    }

    #[test]
    fn camera_finished_fires_as_the_cutscene_ends() {
        let mut w = world(4);
        // Never used and not triggered: off, no cutscene.
        assert_eq!(tick(&mut w), SIGNAL_OFF);
        assert_eq!(w.cutscene, None);
        // The battery comes on (seen by the step one tick later): the cutscene starts.
        set_battery(&mut w, on());
        assert_eq!(tick(&mut w), SIGNAL_OFF);
        assert_eq!(tick(&mut w), SIGNAL_OFF);
        assert_eq!(w.cutscene.map(|c| c.camera), Some(1));
        set_battery(&mut w, SIGNAL_OFF);
        // Held for ticks 0..=4, then the ending count 3; off until the ending starts.
        let mut outs = Vec::new();
        for _ in 0..20 {
            outs.push(tick(&mut w) == on());
        }
        let first = outs.iter().position(|&o| o).unwrap();
        let ons = outs.iter().filter(|&&o| o).count();
        assert_eq!(first, 5);
        // On for the 4 ticks of the ending (3, 2, 1 and the tick it ends), then the 9
        // ticks of counting.
        assert_eq!(ons, 4 + 9);
        assert!(outs[first..first + ons].iter().all(|&o| o));
        assert_eq!(w.cutscene, None);
        assert_eq!(time(&w), 10);
    }

    #[test]
    fn time_since_used_and_other_camera_types() {
        let mut w = world(-1);
        // A hold below 0 never ends: the camera stays the cutscene's, output off.
        set_battery(&mut w, on());
        for _ in 0..30 {
            assert_eq!(tick(&mut w), SIGNAL_OFF);
        }
        assert_eq!(w.cutscene.map(|c| (c.camera, c.ending)), Some((1, 0)));
        assert_eq!(time(&w), 0);
        // Not the cutscene's camera: on while the time is at most 9, counting up.
        w.cutscene = None;
        set_battery(&mut w, SIGNAL_OFF);
        assert_eq!(tick(&mut w), on());
        assert_eq!(time(&w), 0);
        for t in 1..=9 {
            assert_eq!(tick(&mut w), on());
            assert_eq!(time(&w), t);
        }
        assert_eq!(tick(&mut w), SIGNAL_OFF);
        assert_eq!(time(&w), 10);
        // Inverted: the shared tail inverts the off.
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert_eq!(tick(&mut w).ternary, 1);
        // Another camera type, or no camera tweak: off, and the time does not count.
        let mut w = world(4);
        w.things[1].camera_tweak.as_mut().unwrap().camera_type = 0;
        w.things[1].camera_tweak.as_mut().unwrap().time_since_used = 0;
        assert_eq!(tick(&mut w), SIGNAL_OFF);
        assert_eq!(time(&w), 0);
        w.things[1].camera_tweak = None;
        assert_eq!(tick(&mut w), SIGNAL_OFF);
    }
}
