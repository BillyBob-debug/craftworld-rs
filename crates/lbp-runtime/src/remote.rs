//! The key remote switch (switch type 54) (docs/logic.md Part 22).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 entry 54 → 0x11cf18 →
//! 0x10fc88, then the store 0x11bf18):
//! - The object is the switch Thing's parent (Thing+0x5c), or the Thing itself.
//! - The tags of the switch's key (PSwitch+0x80) come from the key's tag map (0xceb8c),
//!   each read as the tag sensor reads it (0x12f6f4, [`LevelWorld::tag_signal`]).
//! - A tag is skipped when its activation is 0.0 or when its Thing's parent, or the Thing
//!   itself, is the object.
//! - A tag signal without a player (-1) takes one: for a creature's Thing (Thing+0x78 &
//!   0x100) the creature's player number (0x3c594 → 0x1d2d30).
//! - The strongest tag wins: one is kept when its |activation| is above the best so far,
//!   which starts at 0.0, so the first in x order keeps a tie. The output is the kept
//!   tag's {activation, ternary, player}; with none it is {0.0, 0, -1}.
//! - Inverted through the standard tail (table 0x11bfd4 entry 54 is 0x11c16c). No
//!   end-of-update step (types above 0x27 go to the epilogue 0x124940 at 0x124938).
//!
//! INFERRED: the creature is the player whose Sackboy Thing is the tag's Thing. Not
//! modelled: the player of a tag owned by something else (+0x10 / +0x50 → +0x30 →
//! 0xf8e70); the skip of tags whose Thing+0x78 & 2 is set while a global (+0x180) is 1;
//! and the override after 0x10fc88 (0x2a1388, 0x2a7d38: a value kept per Thing UID when
//! the object's +8 & 0x4000 is set, taken when stronger), INFERRED to be network or
//! replay state.
//!
//! [`LevelWorld::tag_signal`]: crate::LevelWorld::tag_signal
use crate::switches::SwitchContext;
use lbp_script_vm::natives::SwitchInput;

/// The key remote switch (see the module).
pub(crate) fn key_remote(cx: &SwitchContext<'_>) -> SwitchInput {
    let w = cx.world;
    let owner = |t: u32| w.things.get(t as usize).and_then(|s| s.parent).unwrap_or(t);
    let object = owner(cx.thing);
    let mut best = SwitchInput {
        activation: 0.,
        ternary: 0,
        player: -1,
    };
    for r in w.tags.get(&cx.switch.key).into_iter().flatten() {
        let mut signal = w.tag_signal(r.thing);
        if signal.activation == 0. || owner(r.thing) == object {
            continue;
        }
        if signal.player == -1 {
            if let Some(p) = w.players.iter().find(|p| p.thing == r.thing) {
                signal.player = p.number;
            }
        }
        if signal.activation.abs() > best.activation.abs() {
            best = signal;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use crate::switches::SIGNAL_OFF;
    use crate::{LevelWorld, SwitchOutputState, SwitchState, TagKey, ThingState};
    use lbp_script_vm::natives::SwitchInput;

    const KEY: TagKey = TagKey { colour: 4, name: 9 };

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    fn at(x: f32) -> Option<[f32; 16]> {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15], m[12]) = (1., 1., 1., 1., x);
        Some(m)
    }

    fn output(targets: &[(u32, i32)]) -> Option<SwitchOutputState> {
        Some(SwitchOutputState {
            signal: SIGNAL_OFF,
            stored: SIGNAL_OFF,
            targets: targets.iter().map(|&(t, p)| (Some(t), p)).collect(),
            random_on: false,
        })
    }

    fn tag(x: f32, parent: Option<u32>) -> ThingState {
        ThingState {
            world_matrix: at(x),
            switch_key: Some(KEY),
            parent,
            ..Default::default()
        }
    }

    /// A battery holding `value`, wired to port 0 of `target`.
    fn battery(value: SwitchInput, target: u32) -> ThingState {
        let mut s = SwitchState {
            switch_type: 28,
            outputs: vec![output(&[(target, 0)])],
            ..Default::default()
        };
        s.memory.manual = value;
        ThingState {
            switch: Some(s),
            ..Default::default()
        }
    }

    /// Thing 0: a box. Thing 1: a key remote of [`KEY`] on it. Then `rest`.
    fn world(rest: Vec<ThingState>) -> LevelWorld {
        let remote = ThingState {
            parent: Some(0),
            switch: Some(SwitchState {
                switch_type: 54,
                key: KEY,
                outputs: vec![output(&[])],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut things = vec![ThingState::default(), remote];
        things.extend(rest);
        let mut w = LevelWorld {
            things,
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn out(w: &mut LevelWorld) -> SwitchInput {
        // Two passes, so wired tags read their batteries' outputs of this tick.
        w.update_switches();
        w.update_switches();
        w.things[1].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    #[test]
    fn key_remote_takes_the_strongest_tag_of_its_key() {
        // Things 2 and 3: tags at x 10 and -10, wired to batteries 4 and 5.
        let mut w = world(vec![
            tag(10., None),
            tag(-10., None),
            battery(sig(0.5, 1, 3), 2),
            battery(sig(-0.5, -1, 2), 3),
        ]);
        // A tie: the first in x order (Thing 3, at -10) is kept.
        assert_eq!(out(&mut w), sig(-0.5, -1, 2));
        w.things[4].switch.as_mut().unwrap().memory.manual = sig(-0.75, -1, 1);
        assert_eq!(out(&mut w), sig(-0.75, -1, 1));
        // A tag on the remote's own object is not seen.
        w.things[2].parent = Some(0);
        assert_eq!(out(&mut w), sig(-0.5, -1, 2));
        w.things[3].parent = Some(0);
        assert_eq!(out(&mut w), sig(0., 0, -1));
        // Nor are tags at 0.0: nothing kept is {0.0, 0, -1}.
        w.things[2].parent = None;
        w.things[4].switch.as_mut().unwrap().memory.manual = SIGNAL_OFF;
        assert_eq!(out(&mut w), sig(0., 0, -1));
    }

    #[test]
    fn key_remote_players_unwired_tags_and_inversion() {
        // Thing 2: an unwired tag, on. Thing 3: a tag that is deactivated.
        let mut w = world(vec![tag(0., None), tag(5., None)]);
        w.things[3].deactivated = true;
        assert_eq!(out(&mut w), sig(1., 1, -1));
        // A tag on a player's Sackboy carries his number.
        w.add_player([0.; 3], (0., 0.));
        w.add_player([0.; 3], (0., 0.));
        let sackboy = w.players[1].thing;
        let t = &mut w.things[sackboy as usize];
        (t.switch_key, t.world_matrix) = (Some(KEY), at(-5.));
        assert_eq!(out(&mut w), sig(1., 1, 1));
        // A different key is not seen.
        w.things[sackboy as usize].switch_key = Some(TagKey { colour: 1, name: 9 });
        assert_eq!(out(&mut w), sig(1., 1, -1));
        // Inverted: the standard tail.
        w.things[1].switch.as_mut().unwrap().inverted = true;
        assert_eq!(out(&mut w), sig(0., 0, -1));
        w.things[2].deactivated = true;
        assert_eq!(out(&mut w), sig(1., 1, -1));
    }
}
