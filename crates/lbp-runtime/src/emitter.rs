//! PEmitter (the emitter): when it emits and whether it may (D1134, docs/logic.md
//! Part 36).
//!
//! FOUND (01.33): each frame 0x9bee4 runs the emit clock. The clock's time runs at a
//! speed scale: 1.0 unwired; wired with behavior 2, the input's activation; wired with
//! behavior 0, 1.0 while the input's ternary is nonzero and 0.0 (no emits) otherwise.
//! When the scale changes the clock keeps the time it has made so far. It emits when the
//! clock crosses the next multiple of the frequency after the phase. Behavior 1 emits
//! only when the switch reaches it (0x11eac8 -> 0x9c3b0 -> 0x9baf8). Every emit then
//! passes 0x9baf8: with maxEmittedAtOnce set and no recycling, not while that many are
//! alive; then 0x93780: under maxEmitted (0 = no limit) and maxEmittedAtOnce nonzero.
//! An emit counts currentEmitted up (0x9a53c) and gives the object the lifetime.
//!
//! Not modelled: the thermometer budget (0x9baf8 loop over world+0x8d0), the flag at
//! +0xf6, the paused-world test 0x88fb1c, the group owner (0xf8e70) used when the input
//! carries no player, and placing the object. NOT FOUND: where a lifetime runs out and
//! whether recycling destroys the oldest.

use lbp_level::parts::Emitter;

use crate::switches::wired;
use crate::LevelWorld;

/// An emitter as stored, with its emit clock (PEmitter+0x40 state) and the count of its
/// objects alive (+0xec).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmitterState {
    pub part: Emitter,
    /// Clock time made before `base` beyond real time (+4).
    pub accum: f32,
    /// Frame the scale last changed (+8).
    pub base: f32,
    /// Current speed scale (+0xc).
    pub scale: f32,
    /// Player the input carried when the scale last changed (+0x14); -1 = none.
    pub player: i32,
    /// Emitted objects alive (+0xec); the host counts them down as they go.
    pub alive: u32,
}

impl EmitterState {
    pub fn new(part: Emitter) -> Self {
        Self {
            part,
            accum: 0.0,
            base: 0.0,
            scale: 1.0,
            player: -1,
            alive: 0,
        }
    }

    /// Clock time at frame `t` (0x9bee4 tail).
    fn clock(&self, t: f64) -> f32 {
        (t - self.base as f64) as f32 * self.scale + (self.base as f64 + self.accum as f64) as f32
    }

    /// Sets the speed scale at frame `t`, keeping the clock time made so far. Returns the
    /// clock at `t - 1` under the old scale.
    fn rescale(&mut self, t: u32, scale: f32, player: i32) -> f32 {
        let now = t as f64;
        let prev = (t.wrapping_sub(1) as f64 - self.base as f64) as f32 * self.scale
            + (self.accum as f64 + self.base as f64) as f32;
        if scale != self.scale {
            let run = (now - self.base as f64) as f32;
            self.accum = (self.accum as f64 + (self.scale * run - run) as f64) as f32;
            self.base = now as f32;
            self.scale = scale;
            self.player = player;
        }
        prev
    }

    /// Whether the clock crossed an emit between `prev` and `cur` (0x9bee4 at 0x9c128).
    fn crossed(&self, prev: f32, cur: f32) -> bool {
        let freq = self.part.frequency;
        if freq == 0 {
            return false;
        }
        let f = freq as f64;
        let phase = self.part.phase as f64;
        let cur = (cur as f64 + phase) as f32 as f64;
        let n = (((prev as f64 + phase) as f32 as f64) / f).floor();
        (n * f + f) as f32 as f64 <= cur
    }

    /// The gates of 0x9baf8 and 0x93780: whether an emit may happen now.
    pub fn may_emit(&self) -> bool {
        let p = &self.part;
        let at_once = p.max_emitted_at_once;
        if !p.recycle_emitted_objects && at_once != 0 && at_once as i64 <= self.alive as i64 {
            return false;
        }
        (p.max_emitted == 0 || p.current_emitted < p.max_emitted) && at_once != 0
    }
}

impl LevelWorld {
    /// Runs `thing`'s emit clock for frame `t` (0x9bee4) and, when it is time, emits if
    /// the gates allow it. Returns whether it emitted.
    ///
    /// INFERRED: the input is port 0, and its ternary is the on/off a behavior-0 emitter
    /// reads (output +0x1c).
    pub fn emitter_tick(&mut self, thing: u32, t: u32) -> bool {
        let input = wired(self, thing, 0).map(|(_, s)| s);
        let Some(th) = self.things.get_mut(thing as usize) else {
            return false;
        };
        if th.deactivated {
            return false;
        }
        let Some(e) = th.emitter.as_mut() else {
            return false;
        };
        let behavior = e.part.behavior;
        if behavior == 1 || (matches!(behavior, 0 | 2) && e.part.frequency == 0) {
            return false;
        }
        let (scale, player) = match input {
            None => (1.0, -1),
            Some(s) if behavior == 2 => (s.activation, s.player),
            Some(s) if behavior == 0 && s.ternary == 0 => (0.0, s.player),
            Some(s) => (1.0, s.player),
        };
        let prev = e.rescale(t, scale, player);
        if input.is_some() && scale == 0.0 {
            return false;
        }
        let cur = e.clock(t as f64);
        e.crossed(prev, cur) && emit(e)
    }

    /// The switch reached `thing`'s emitter (0x11eac8): a behavior-1 emitter emits once
    /// if the gates allow it. Returns whether it emitted.
    pub fn emitter_trigger(&mut self, thing: u32) -> bool {
        let Some(th) = self.things.get_mut(thing as usize) else {
            return false;
        };
        match th.emitter.as_mut() {
            Some(e) if !th.deactivated && e.part.behavior == 1 => emit(e),
            _ => false,
        }
    }
}

/// One emit after the gates (0x9baf8 -> 0x9a53c).
fn emit(e: &mut EmitterState) -> bool {
    if !e.may_emit() {
        return false;
    }
    e.part.current_emitted += 1;
    e.alive += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputWire, SwitchOutputState, SwitchState, ThingState};

    fn part(behavior: i32, frequency: u32, phase: u32) -> Emitter {
        Emitter {
            behavior,
            frequency,
            phase,
            max_emitted_at_once: 2,
            ..Default::default()
        }
    }

    /// Thing 0 a battery; Thing 1 an emitter, its port 0 wired to the battery if `wire`.
    fn world(e: Emitter, wire: bool) -> LevelWorld {
        let battery = ThingState {
            switch: Some(SwitchState {
                switch_type: 25,
                outputs: vec![Some(SwitchOutputState::default())],
                ..Default::default()
            }),
            ..Default::default()
        };
        let em = ThingState {
            emitter: Some(EmitterState::new(e)),
            inputs: if wire {
                vec![Some(InputWire {
                    switch: 0,
                    output: 0,
                })]
            } else {
                vec![]
            },
            ..Default::default()
        };
        LevelWorld {
            things: vec![battery, em],
            ..Default::default()
        }
    }

    fn set(w: &mut LevelWorld, activation: f32, ternary: i32) {
        if let Some(o) = w.things[0]
            .switch
            .as_mut()
            .and_then(|s| s.outputs[0].as_mut())
        {
            o.signal.activation = activation;
            o.signal.ternary = ternary;
        }
    }

    fn emits(w: &mut LevelWorld, frames: std::ops::Range<u32>) -> Vec<u32> {
        frames.filter(|&t| w.emitter_tick(1, t)).collect()
    }

    #[test]
    fn unwired_emitter_emits_every_frequency_frames_after_the_phase() {
        let mut w = world(
            Emitter {
                max_emitted_at_once: 0,
                recycle_emitted_objects: true,
                ..part(0, 30, 10)
            },
            false,
        );
        assert_eq!(
            emits(&mut w, 1..100),
            vec![],
            "maxEmittedAtOnce 0 never emits"
        );
        let mut w = world(part(0, 30, 10), false);
        w.things[1]
            .emitter
            .as_mut()
            .unwrap()
            .part
            .max_emitted_at_once = 10;
        assert_eq!(emits(&mut w, 1..100), vec![20, 50, 80]);
        assert_eq!(w.things[1].emitter.unwrap().part.current_emitted, 3);
    }

    #[test]
    fn at_once_and_total_limits_gate_emits() {
        let mut w = world(part(0, 10, 0), false);
        assert_eq!(emits(&mut w, 1..60), vec![10, 20], "two alive at once");
        w.things[1].emitter.as_mut().unwrap().alive = 0;
        w.things[1].emitter.as_mut().unwrap().part.max_emitted = 3;
        assert_eq!(emits(&mut w, 60..100), vec![60]);
        let e = w.things[1].emitter.as_mut().unwrap();
        e.part = Emitter {
            recycle_emitted_objects: true,
            current_emitted: 0,
            ..e.part
        };
        assert_eq!(
            emits(&mut w, 100..121),
            vec![100, 110, 120],
            "recycling ignores alive"
        );
    }

    #[test]
    fn wired_input_sets_the_speed_and_behavior_one_waits_for_the_switch() {
        let mut w = world(part(0, 10, 0), true);
        w.things[1]
            .emitter
            .as_mut()
            .unwrap()
            .part
            .max_emitted_at_once = 100;
        assert_eq!(emits(&mut w, 1..30), vec![], "off input: no emits");
        set(&mut w, 1.0, 1);
        assert_eq!(
            emits(&mut w, 30..55),
            vec![39, 49],
            "the clock keeps its time while off"
        );
        let mut w = world(part(2, 10, 0), true);
        w.things[1]
            .emitter
            .as_mut()
            .unwrap()
            .part
            .max_emitted_at_once = 100;
        set(&mut w, 0.5, 0);
        assert_eq!(
            emits(&mut w, 1..45),
            vec![19, 39],
            "half speed from frame 1"
        );
        let mut w = world(part(1, 10, 0), true);
        set(&mut w, 1.0, 1);
        assert_eq!(emits(&mut w, 1..30), vec![]);
        assert!(w.emitter_trigger(1));
        w.things[1].deactivated = true;
        assert!(!w.emitter_trigger(1));
    }
}
