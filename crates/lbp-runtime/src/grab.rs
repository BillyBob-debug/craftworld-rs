//! The grab switch (switch type 6) (docs/logic.md Part 20).
//!
//! FOUND in the 01.33 executable (compute_output 0x11bd58 entry 6 → 0x11cb50 → 0x10f508,
//! then the shared tail 0x11c2c4):
//! - The object is the switch Thing's parent (Thing+0x5c), or the Thing itself when it
//!   has no parent (0x10f580). With no object the output is off.
//! - The mode is PSwitch+0x8c (the IncludeTouching slot, 0x10f6e4): 1 and 2 (0x10fb64,
//!   0x10f7ac) look for a player whose grab target is the object; 3 (0x10fae4) for one
//!   whose held object (*(sub+0x140)+4) is it; any other value gives off.
//! - A player counts only while grabbing (player+0x1c & 0x1001 and the byte +0x180).
//!   With PlayerMode 0 only the local player counts (0x228818, 0x240a88); otherwise any.
//! - With the tag filter (PSwitch+0xc9, read as RequireAll) the grabbing player must also
//!   carry a tag of the switch's key (0x1d2540).
//! - The signal: {1.0, 1, the grabber's player number (sub+0x230)} when one counts, else
//!   off. The ternary is set directly, not from a threshold. Nothing is stored and there
//!   is no end-of-update step (table 0x124b0c entry 6 is the epilogue 0x124940).
//!   Inverted through the shared tail (table 0x11bfd4 entry 6 is the standard 0x11c16c).
//!
//! - No cone and no layer test (D1142, docs/logic.md Part 44): AngleRange (PSwitch+0x7c)
//!   is never read and none of the cone or layer helpers is called. Mode 2's extra test
//!   0x1d31c0 compares the tag filter of the Thing the player grabs with the other
//!   Thing's id, an identity check, not a spatial one.
//!
//! INFERRED: modes 2 and 3 read as mode 1 (mode 2's identity test is not modelled;
//! mode 3's held object is taken to be the grab target); the local player is the
//! first player record's, as the leader of the player sensor; a player carries a tag when
//! his Sackboy's Thing or a child of it has the key. Not modelled: creatures that grab
//! (the scan of world+0x114 for creatures of kind 0xc), the joint walk when PSwitch+0xcc
//! is set, and the branch for a Thing under an ancestor of object kind 0x96 (0x10f350).
use crate::switches::{SwitchContext, SIGNAL_OFF};
use lbp_script_vm::natives::SwitchInput;

/// The grab switch (see the module).
pub(crate) fn grab_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let Some(thing) = w.things.get(cx.thing as usize) else {
        return SIGNAL_OFF;
    };
    let target = thing.parent.unwrap_or(cx.thing);
    if w.things.get(target as usize).is_none() || !(1..=3).contains(&s.include_touching) {
        return SIGNAL_OFF;
    }
    let leader = w.player_records.first().map(|r| r.number);
    let tagged = |t: u32| {
        w.things
            .iter()
            .enumerate()
            .any(|(i, c)| (i as u32 == t || c.parent == Some(t)) && c.switch_key == Some(s.key))
    };
    let grabber = w.players.iter().find(|p| {
        p.grabbing == Some(target)
            && (s.player_mode != 0 || Some(p.number) == leader)
            && (!s.require_all || tagged(p.thing))
    });
    match grabber {
        Some(p) => SwitchInput {
            activation: 1.,
            ternary: 1,
            player: p.number,
        },
        None => SIGNAL_OFF,
    }
}

#[cfg(test)]
mod tests {
    use crate::switches::SIGNAL_OFF;
    use crate::{LevelWorld, SwitchOutputState, SwitchState, TagKey, ThingState};
    use lbp_script_vm::natives::SwitchInput;

    /// Thing 0: a box. Thing 1: a grab switch of mode 1 on it. Two players (numbers 0 and
    /// 1, the first the leader) at Things 2 and 3, grabbing nothing.
    fn world() -> LevelWorld {
        let sensor = ThingState {
            parent: Some(0),
            switch: Some(SwitchState {
                switch_type: 6,
                include_touching: 1,
                player_mode: 1,
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
            things: vec![ThingState::default(), sensor],
            ..Default::default()
        };
        w.add_player([0.; 3], (0., 0.));
        w.add_player([0.; 3], (0., 0.));
        w.wire_inputs();
        w
    }

    fn set(w: &mut LevelWorld, f: impl FnOnce(&mut SwitchState)) {
        f(w.things[1].switch.as_mut().unwrap());
    }

    fn out(w: &mut LevelWorld) -> SwitchInput {
        w.update_switches();
        w.things[1].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    fn on(player: i32) -> SwitchInput {
        SwitchInput {
            activation: 1.,
            ternary: 1,
            player,
        }
    }

    #[test]
    fn grab_switch_sees_the_player_grabbing_its_object() {
        let mut w = world();
        assert_eq!(w.players[1].number, 1);
        // Nobody grabbing, or grabbing something else: off.
        assert_eq!(out(&mut w), SIGNAL_OFF);
        w.players[1].grabbing = Some(1);
        assert_eq!(out(&mut w), SIGNAL_OFF);
        // Player 1 grabs the parent: on with his number.
        w.players[1].grabbing = Some(0);
        assert_eq!(out(&mut w), on(1));
        // Both grab it: the first in the list.
        w.players[0].grabbing = Some(0);
        assert_eq!(out(&mut w), on(0));
        // PlayerMode 0: only the leader counts.
        w.players[0].grabbing = None;
        set(&mut w, |s| s.player_mode = 0);
        assert_eq!(out(&mut w), SIGNAL_OFF);
        w.players[0].grabbing = Some(0);
        assert_eq!(out(&mut w), on(0));
        // Inverted: the standard tail.
        set(&mut w, |s| s.inverted = true);
        assert_eq!(
            out(&mut w),
            SwitchInput {
                activation: 0.,
                ternary: 0,
                player: 0
            }
        );
    }

    #[test]
    fn grab_switch_modes_parent_and_tags() {
        let mut w = world();
        w.players[0].grabbing = Some(0);
        for (mode, grabbed) in [(0, false), (1, true), (2, true), (3, true), (4, false)] {
            set(&mut w, |s| s.include_touching = mode);
            assert_eq!(out(&mut w) == on(0), grabbed, "mode {mode}");
        }
        // With no parent the switch's own Thing is the object.
        set(&mut w, |s| s.include_touching = 1);
        w.things[1].parent = None;
        assert_eq!(out(&mut w), SIGNAL_OFF);
        w.players[0].grabbing = Some(1);
        assert_eq!(out(&mut w), on(0));
        // The tag filter: the grabber must carry the key, on his Thing or a child.
        let key = TagKey { colour: 3, name: 7 };
        set(&mut w, |s| (s.require_all, s.key) = (true, key));
        assert_eq!(out(&mut w), SIGNAL_OFF);
        let player = w.players[0].thing;
        w.things.push(ThingState {
            parent: Some(player),
            switch_key: Some(key),
            ..Default::default()
        });
        assert_eq!(out(&mut w), on(0));
    }

    #[test]
    fn grab_switch_has_no_cone() {
        // D1142: AngleRange is never read; a grab from any side counts.
        let mut w = world();
        set(&mut w, |s| s.angle_range = 5.);
        w.players[0].grabbing = Some(0);
        assert_eq!(out(&mut w), on(0));
    }
}
