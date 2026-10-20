//! PSequencer (the sequencer): how its play head moves and what wires do to it (D1135,
//! docs/logic.md Part 37).
//!
//! FOUND (01.33): each frame 0xf5008 adds a step (0xf45e4) to the play head (+0x48) and
//! keeps it inside 0..=length, the length of the sequencer's circuit board (0xf454c).
//! The step is tempo * 7/960 per frame, signed and scaled by the behavior (+0x68) and
//! the input on port 0: unwired and not on another sequencer, it runs while playing;
//! unwired on another sequencer, the input counts as 1 while the Thing is not
//! deactivated and 0 when it is. Running past the end loops to 0 when loop is set,
//! otherwise stops there; running below 0 loops to the end, otherwise stops at 0. A
//! rising edge on port 0 starts a behavior 2 sequencer from 0 and a behavior 3 one from
//! the end (0xf675c, not when already playing); one on port 1 stops it at 0 (0xf52b8).
//! When the head moves, every Thing on the board has its deactivated flag worked out
//! again (0xf4f20 -> 0x409dc), which switches the board's contents by the head.
//!
//! Not modelled: the music itself (0x2cebcc, 0xf6db8), the "music pending" byte +0x4d
//! and the check 0x900730 on a wired input with ternary 0. INFERRED: the frame count
//! +0x50 that holds a music sequencer for its first 30 frames counts this module's ticks.

use lbp_level::parts::{ChipComponent, Sequencer};
use lbp_level::thing::ThingRecord;

use crate::switches::wired;
use crate::LevelWorld;

/// Play-head units per frame for each tempo unit (FOUND, 0xd9593c).
pub const STEP_PER_TEMPO: f32 = 7.0 / 960.0;

/// Play-head units per column of the board (FOUND, 0xd92e94 and 0xd949e8: 52.5).
pub const UNITS_PER_COLUMN: f32 = 52.5;

/// A sequencer as stored, with the runtime-only frame count (+0x50).
#[derive(Debug, Clone, PartialEq)]
pub struct SequencerState {
    pub part: Sequencer,
    /// The board length (the circuit board's microchip +0x3c, 0xf454c).
    pub length: f32,
    /// Frames run (+0x50).
    pub frames: u32,
    /// The columns each Thing on the board plays in, (Thing, first, last), from the
    /// microchip's component list (0xae53c).
    pub ranges: Vec<(u32, i32, i32)>,
}

impl SequencerState {
    pub fn new(part: Sequencer, length: f32) -> Self {
        Self {
            part,
            length,
            frames: 0,
            ranges: Vec::new(),
        }
    }

    /// Whether the Thing on the board is inside the play head (0x409dc): its columns
    /// (s, e] hold the head's column. None when the board does not list it.
    pub fn plays(&self, thing: u32) -> Option<bool> {
        let h = self.part.play_head / UNITS_PER_COLUMN;
        let &(_, s, e) = self.ranges.iter().find(|r| r.0 == thing)?;
        Some((s as f32) < h && h <= e as f32)
    }

    /// The play-head step for one frame (0xf45e4). `input` is port 0's (activation,
    /// ternary) as the step sees it, `None` when unwired and not on another sequencer.
    pub fn step(&self, input: Option<(f32, i32)>) -> f32 {
        let p = &self.part;
        if p.music_sequencer && self.frames <= 29 {
            return 0.0;
        }
        let base = p.tempo * STEP_PER_TEMPO;
        let playing = if p.is_playing { 1.0 } else { 0.0 };
        let Some((act, tern)) = input else {
            return base * playing;
        };
        match p.behavior {
            0 | 6 => base * tern as f32,
            1 if tern == 0 => -base,
            1 => base * tern as f32,
            2 => base * playing,
            3 => -base * playing,
            4 => base * act,
            5 => act.abs() * self.length - p.play_head,
            _ => 0.0,
        }
    }

    /// Moves the head by `step` (0xf5008). `stops` is set for behaviors 2 and 3 and for
    /// an unwired sequencer that is not on another sequencer: those keep their playing
    /// flag and only stop at an end; the rest play while the head is off 0. Returns the
    /// new (head, playing).
    pub fn advance(&self, step: f32, stops: bool) -> (f32, bool) {
        let p = &self.part;
        let len = self.length;
        let mut head = p.play_head + step;
        if head < 0.0 {
            if !p.looped {
                return (0.0, false);
            }
            head = len;
        } else if len <= head {
            if p.looped {
                head = 0.0;
            } else {
                return (len, !stops && len != 0.0);
            }
        }
        if stops {
            (head, p.is_playing)
        } else {
            (head, head != 0.0)
        }
    }
}

/// The columns a component covers (0xae53c): `x` its position along the board and `w`
/// its width. `m` is 2 for the wide kinds (4, 0x48, 0x49) and 1 otherwise; the kinds
/// 0x2f, 0x31 and 0x32 (`by_width`) reach back by their width instead. Rounds half up
/// (0x88f6cc taken as floor, INFERRED).
pub fn column_range(x: f32, w: f32, m: f32, by_width: bool) -> (i32, i32) {
    let xr = x / UNITS_PER_COLUMN;
    let w = w.max(1.0);
    let (back, m) = if by_width { (w, 1.0) } else { (m, m) };
    (
        (xr - back + 0.5).floor() as i32,
        (w * m + xr + 0.5).floor() as i32,
    )
}

/// The columns a component of a sequencer's board covers, from its list entry (0xae53c).
/// FOUND: the kinds (0x3ed2c): 4 is a music sequencer, 0x2f/0x31/0x32 are switch types 39
/// (control pad), 15 (AND) and 16 (countdown). INFERRED: the entry's Thing pointer takes 3
/// words, so the stored f32 #0 is word 3 (x) and #3 / #4 are words 6 and 7 (the width).
pub fn component_columns(thing: &ThingRecord, c: &ChipComponent) -> (i32, i32) {
    let v = c.values;
    let w = if v[4] != 0.0 { v[3] / v[4] } else { 1.0 };
    let switch = thing.data.switch.as_ref().map(|s| s.switch_type);
    let by_width = matches!(switch, Some(39 | 15 | 16));
    let music = thing.data.sequencer.is_some_and(|q| q.music_sequencer);
    column_range(v[0], w, if music { 2.0 } else { 1.0 }, by_width)
}

impl LevelWorld {
    /// Runs `thing`'s sequencer for one frame (0xf5008). Returns whether the head moved;
    /// the Things on its board then have their flags worked out again.
    pub fn sequencer_tick(&mut self, thing: u32) -> bool {
        let wire = wired(self, thing, 0).map(|(_, s)| (s.activation, s.ternary));
        let nested = self.on_sequencer(thing);
        let deactivated = self
            .things
            .get(thing as usize)
            .is_some_and(|t| t.deactivated);
        let input = match wire {
            Some(w) => Some(w),
            None if nested && deactivated => Some((0.0, 0)),
            None if nested => Some((1.0, 1)),
            None => None,
        };
        let Some(q) = self
            .things
            .get_mut(thing as usize)
            .and_then(|t| t.sequencer.as_mut())
        else {
            return false;
        };
        let stops = matches!(q.part.behavior, 2 | 3) || input.is_none();
        let (head, playing) = q.advance(q.step(input), stops);
        q.frames = q.frames.saturating_add(1);
        self.set_play_head(thing, head, playing)
    }

    /// A switch output reached `thing`'s sequencer on `port` (0x11f3a0). Only rising
    /// edges act: port 0 starts a behavior 2 sequencer from 0 and a behavior 3 one from
    /// the end, unless it is already playing, and records the player; port 1 stops it
    /// at 0. Returns whether the head moved.
    pub fn sequencer_input(&mut self, thing: u32, port: usize, on: bool, player: i32) -> bool {
        let Some(q) = self
            .things
            .get_mut(thing as usize)
            .and_then(|t| t.sequencer.as_mut())
        else {
            return false;
        };
        if !on {
            return false;
        }
        match (port, q.part.behavior) {
            (0, 2 | 3) if !q.part.is_playing => {
                let head = if q.part.behavior == 2 { 0.0 } else { q.length };
                q.part.trigger_player = player;
                self.set_play_head(thing, head, true)
            }
            (1, _) => {
                if q.part.is_playing {
                    q.part.trigger_player = -1;
                }
                self.set_play_head(thing, 0.0, false)
            }
            _ => false,
        }
    }

    /// Sets the head and the playing flag (0xf4f20); when the head changed, works out
    /// the flag of every Thing on the sequencer's board again.
    fn set_play_head(&mut self, thing: u32, head: f32, playing: bool) -> bool {
        let Some(t) = self.things.get_mut(thing as usize) else {
            return false;
        };
        let board = t.circuit_board;
        let Some(q) = t.sequencer.as_mut() else {
            return false;
        };
        q.part.is_playing = playing;
        if q.part.play_head == head {
            return false;
        }
        q.part.play_head = head;
        let mut child = board.and_then(|b| self.things.get(b as usize)?.first_child);
        let mut left = self.things.len();
        while let Some(c) = child.filter(|_| left > 0) {
            left -= 1;
            self.recompute_deactivated(c);
            child = self.things.get(c as usize).and_then(|t| t.next_sibling);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SwitchState, ThingState};

    fn part(behavior: i32, looped: bool) -> Sequencer {
        Sequencer {
            tempo: 120.0,
            behavior,
            looped,
            is_playing: true,
            ..Default::default()
        }
    }

    /// Thing 0 the sequencer, Thing 1 its board, Things 2 and 3 on the board; Thing 3
    /// has a wired input.
    fn world(q: Sequencer) -> LevelWorld {
        let board = |t: i32| ThingState {
            switch: Some(SwitchState {
                switch_type: t,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut s = SequencerState::new(q, 4.0 * UNITS_PER_COLUMN);
        s.ranges = vec![(2, 0, 1), (3, 2, 3)];
        let seq = ThingState {
            parts: vec!["SEQUENCER"],
            sequencer: Some(s),
            circuit_board: Some(1),
            first_child: Some(1),
            ..board(36)
        };
        let b = ThingState {
            parent: Some(0),
            first_child: Some(2),
            ..board(37)
        };
        let c = ThingState {
            parent: Some(1),
            next_sibling: Some(3),
            ..Default::default()
        };
        let d = ThingState {
            parent: Some(1),
            inputs: vec![Some(crate::InputWire {
                switch: 0,
                output: 0,
            })],
            ..Default::default()
        };
        LevelWorld {
            things: vec![seq, b, c, d],
            ..Default::default()
        }
    }

    fn head(w: &LevelWorld) -> (f32, bool) {
        let q = &w.things[0].sequencer.as_ref().unwrap().part;
        (q.play_head, q.is_playing)
    }

    #[test]
    fn unwired_head_runs_at_tempo_and_stops_or_loops_at_the_end() {
        let mut w = world(part(0, false));
        assert!(w.sequencer_tick(0));
        assert_eq!(head(&w), (120.0 * STEP_PER_TEMPO, true));
        for _ in 0..300 {
            w.sequencer_tick(0);
        }
        assert_eq!(
            head(&w),
            (4.0 * UNITS_PER_COLUMN, false),
            "stops at the end"
        );
        let mut w = world(part(0, true));
        for _ in 0..241 {
            w.sequencer_tick(0);
        }
        assert!(head(&w).0 < 1.0, "looped back to 0: {:?}", head(&w));
        let mut w = world(Sequencer {
            is_playing: false,
            ..part(0, false)
        });
        assert!(!w.sequencer_tick(0), "not playing: no step");
        let mut w = world(Sequencer {
            music_sequencer: true,
            ..part(0, false)
        });
        for _ in 0..30 {
            assert!(!w.sequencer_tick(0), "music waits 30 frames");
        }
        assert!(w.sequencer_tick(0));
    }

    #[test]
    fn port_zero_starts_and_port_one_stops() {
        let mut w = world(Sequencer {
            is_playing: false,
            play_head: 50.0,
            ..part(2, false)
        });
        assert!(
            !w.sequencer_input(0, 0, false, 0),
            "falling edge does nothing"
        );
        assert!(w.sequencer_input(0, 0, true, 1));
        assert_eq!(head(&w), (0.0, true));
        assert!(!w.sequencer_input(0, 0, true, 2), "already playing");
        assert_eq!(
            w.things[0].sequencer.as_ref().unwrap().part.trigger_player,
            1
        );
        w.sequencer_tick(0);
        assert!(w.sequencer_input(0, 1, true, 0));
        assert_eq!(head(&w), (0.0, false));
        let mut w = world(Sequencer {
            is_playing: false,
            ..part(3, false)
        });
        assert!(w.sequencer_input(0, 0, true, 0));
        assert_eq!(
            head(&w),
            (4.0 * UNITS_PER_COLUMN, true),
            "behavior 3 from the end"
        );
    }

    #[test]
    fn board_things_play_while_the_head_is_in_their_columns() {
        let mut w = world(part(0, false));
        w.sequencer_tick(0);
        assert!(!w.things[2].deactivated, "head in column 0..1");
        assert!(!w.things[3].deactivated, "wired: not gated");
        w.things[0].sequencer.as_mut().unwrap().part.play_head = 2.5 * UNITS_PER_COLUMN;
        w.sequencer_tick(0);
        assert!(w.things[2].deactivated, "head past its columns");
        assert_eq!(column_range(105.0, 1.0, 1.0, false), (1, 3));
        assert_eq!(column_range(105.0, 3.0, 2.0, false), (0, 8));
        assert_eq!(column_range(105.0, 3.0, 1.0, true), (-1, 5));
    }
}
