//! PControlinator (the controlinator): which controlinator a control pad belongs to and
//! whose pad it reads (D1136, docs/logic.md Part 38).
//!
//! FOUND (01.33): the part sits in Thing slot +0x98 (part bit 0x26, kinds 6 to 8 at
//! 0x3ed2c). The reader 0xb73008 drops the stored +0x4c and keeps the runtime state
//! (0 at load, 0x6a0844); 1 and 2 are the two ways of being in it. The control pad
//! (0x11d378) takes the controlinator of its Thing's parent's parent. The driver a pad
//! reads (0x6a0134): in mode 8 with state 2, the attached player (+0x34); otherwise the
//! driver (+0xc) of the controlinator on the link Thing (+0x28). With none, every output
//! but 0 reads 0.0 (0x6a1004). Group occupancy (0x69ffec): another controlinator in the
//! same mode, on a Thing not deactivated, in state 1, whose port 0 is not on, with a
//! driver.
//!
//! Getting in and out (D1138, Part 40), FOUND in the update 0x6a3264: the gate 0x6a096c
//! shuts while port 0 is on, and then the seat drops its driver. With no driver, a
//! creature in an alive state within the radius +0x44 (0x6a2938) can sit; with +0x51 set
//! it sits at once (0x6a2fc4), else a player's button decides. Sitting puts the creature
//! in state 4 and makes it the driver (0x6a0b78); the seat lets go once the driver is
//! no longer in state 4 (0x89950c).
//!
//! Not modelled: the gate's world and Thing flags (world+0x804, +0x820, T+0x7f,
//! 0x550e70), the button test itself, the brain path (brain+0x58), the recorded inputs
//! read first from the +0x54 Thing's input table, the Thing type 8 path of 0x6a1004 and
//! the mirror sign from the link Thing's group matrix.

use lbp_level::parts::Controlinator;

use crate::LevelWorld;

/// The creature state of a seated driver.
pub const SEATED: u8 = 4;

/// The creature states that may sit (0x6a2938).
const MAY_SIT: [u8; 11] = [0, 1, 2, 3, 7, 8, 9, 10, 11, 12, 16];

/// A controlinator as stored, with its runtime state (+0x4c).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlinatorState {
    pub part: Controlinator,
    /// 0 empty; 1 and 2 driven (2 with mode 8 is the attached player's).
    pub state: i32,
}

impl ControlinatorState {
    pub fn new(part: Controlinator) -> Self {
        Self { part, state: 0 }
    }
}

impl LevelWorld {
    fn controlinator(&self, thing: Option<usize>) -> Option<&ControlinatorState> {
        self.things.get(thing?)?.controlinator.as_ref()
    }

    /// The Thing whose controlinator the control pad on `thing` reads (0x11d378): the
    /// Thing's parent's parent, when that has one.
    pub fn pad_controlinator(&self, thing: u32) -> Option<u32> {
        let parent = self.things.get(thing as usize)?.parent?;
        let g = self.things.get(parent as usize)?.parent?;
        self.controlinator(Some(g as usize)).map(|_| g)
    }

    /// The Thing driving `ctl`'s pad (0x6a0134), or None.
    pub fn controlinator_driver(&self, ctl: u32) -> Option<u32> {
        let c = self.controlinator(Some(ctl as usize))?;
        if c.state == 2 && c.part.mode == 8 && c.part.attached.is_some() {
            return c.part.attached.map(|t| t as u32);
        }
        self.controlinator(c.part.link)?
            .part
            .driver
            .map(|t| t as u32)
    }

    /// Whether the seat is open (0x6a096c): not while port 0 is on.
    pub fn controlinator_open(&self, ctl: u32) -> bool {
        self.controlinator(Some(ctl as usize)).is_some()
            && !crate::switches::wired(self, ctl, 0).is_some_and(|(_, s)| s.ternary != 0)
    }

    fn creature_state(&self, thing: u32) -> u8 {
        self.things
            .get(thing as usize)
            .and_then(|t| t.creature)
            .map_or(0, |c| c.state)
    }

    /// Whether `who`, `distance` away, may sit in `ctl` (0x6a2938): the seat open and
    /// empty, the creature in an alive state and closer than the radius (+0x44).
    pub fn controlinator_in_reach(&self, ctl: u32, who: u32, distance: f32) -> bool {
        let Some(c) = self.controlinator(Some(ctl as usize)) else {
            return false;
        };
        c.part.driver.is_none()
            && self.controlinator_open(ctl)
            && MAY_SIT.contains(&self.creature_state(who))
            && distance < c.part.input_scale
    }

    /// Whether `ctl` seats a creature in reach without a button (+0x51).
    pub fn controlinator_auto(&self, ctl: u32) -> bool {
        self.controlinator(Some(ctl as usize))
            .is_some_and(|c| c.part.flags_50[1] != 0)
    }

    /// Seats `who` (0x6a2fc4, 0x6a0b78): it drives `ctl` and its creature goes to state 4.
    pub fn controlinator_enter(&mut self, ctl: u32, who: u32) {
        let auto = self.controlinator_auto(ctl);
        let Some(c) = self
            .things
            .get_mut(ctl as usize)
            .and_then(|t| t.controlinator.as_mut())
        else {
            return;
        };
        c.part.driver = Some(who as usize);
        if auto {
            c.part.ref_18 = Some(who as usize);
        }
        if let Some(cr) = self
            .things
            .get_mut(who as usize)
            .and_then(|t| t.creature.as_mut())
        {
            cr.state = SEATED;
            cr.frames = 0;
            cr.seat = Some(ctl);
        }
    }

    /// The host destroyed `thing`: every creature seated in it loses its seat (+0xecc),
    /// so its next [`crate::CreatureState::seated_tick`] puts it in state 5.
    pub fn seat_destroyed(&mut self, thing: u32) {
        for cr in self.things.iter_mut().filter_map(|t| t.creature.as_mut()) {
            if cr.seat == Some(thing) {
                cr.seat = None;
            }
        }
    }

    /// One frame of a seat with a driver: when the seat shuts the driver is put out of
    /// state 4, and once the driver is not in state 4 the seat lets go (0x89950c,
    /// 0x6a0b78). Returns the driver let go.
    pub fn controlinator_seat_tick(&mut self, ctl: u32) -> Option<u32> {
        let driver = self.controlinator(Some(ctl as usize))?.part.driver? as u32;
        if self.controlinator_open(ctl) && self.creature_state(driver) == SEATED {
            return None;
        }
        if let Some(cr) = self
            .things
            .get_mut(driver as usize)
            .and_then(|t| t.creature.as_mut())
            .filter(|c| c.state == SEATED)
        {
            cr.state = 0;
            cr.frames = 0;
            return None;
        }
        self.things[ctl as usize]
            .controlinator
            .as_mut()?
            .part
            .driver = None;
        Some(driver)
    }

    /// Whether `other` is a driven controlinator in `ctl`'s mode (0x69ffec).
    ///
    /// INFERRED: the input it reads (Thing+0x60 entry 0, +0x1c) is port 0's ternary.
    pub fn controlinator_holds_group(&self, ctl: u32, other: u32) -> bool {
        let (Some(c), Some(th)) = (
            self.controlinator(Some(ctl as usize)),
            self.things.get(other as usize),
        ) else {
            return false;
        };
        let Some(o) = th.controlinator.as_ref() else {
            return false;
        };
        let on = crate::switches::wired(self, other, 0).is_some_and(|(_, s)| s.ternary != 0);
        !th.deactivated
            && o.part.mode == c.part.mode
            && o.state == 1
            && !on
            && o.part.driver.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThingState;

    fn ctl(driver: Option<usize>, link: Option<usize>) -> ThingState {
        ThingState {
            controlinator: Some(ControlinatorState::new(Controlinator {
                driver,
                link,
                input_scale: 1.0,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    /// 0 the controlinator (link to itself, driven by 3); 1 a part of it; 2 the pad;
    /// 3 the driver.
    fn world() -> LevelWorld {
        let part = ThingState {
            parent: Some(0),
            ..Default::default()
        };
        let pad = ThingState {
            parent: Some(1),
            ..Default::default()
        };
        LevelWorld {
            things: vec![ctl(Some(3), Some(0)), part, pad, ThingState::default()],
            ..Default::default()
        }
    }

    #[test]
    fn pad_reads_the_link_driver_of_its_grandparent() {
        let mut w = world();
        assert_eq!(w.pad_controlinator(2), Some(0));
        assert_eq!(
            w.pad_controlinator(1),
            None,
            "the parent alone is not looked at"
        );
        assert_eq!(w.controlinator_driver(0), Some(3));
        w.things[0].controlinator.as_mut().unwrap().part.link = None;
        assert_eq!(w.controlinator_driver(0), None, "no link: no driver");
        w.things[1] = ctl(Some(1), None);
        w.things[0].controlinator.as_mut().unwrap().part.link = Some(1);
        assert_eq!(w.controlinator_driver(0), Some(1), "the link's driver");
    }

    #[test]
    fn mode_eight_in_state_two_reads_the_attached_player() {
        let mut w = world();
        let c = w.things[0].controlinator.as_mut().unwrap();
        c.part.mode = 8;
        c.part.attached = Some(1);
        assert_eq!(
            w.controlinator_driver(0),
            Some(3),
            "state 0: the link's driver"
        );
        w.things[0].controlinator.as_mut().unwrap().state = 2;
        assert_eq!(w.controlinator_driver(0), Some(1));
        w.things[0].controlinator.as_mut().unwrap().part.mode = 7;
        assert_eq!(w.controlinator_driver(0), Some(3), "only mode 8");
    }

    fn seat_world() -> LevelWorld {
        let mut w = world();
        let c = w.things[0].controlinator.as_mut().unwrap();
        c.part.driver = None;
        c.part.input_scale = 2.0;
        w.things[3].creature = Some(crate::CreatureState::default());
        w
    }

    #[test]
    fn a_creature_in_reach_sits_and_drives() {
        let mut w = seat_world();
        assert!(!w.controlinator_in_reach(0, 3, 2.0), "the radius is strict");
        assert!(w.controlinator_in_reach(0, 3, 1.5));
        w.things[3].creature.as_mut().unwrap().state = crate::creature::DYING;
        assert!(!w.controlinator_in_reach(0, 3, 1.5), "dying may not sit");
        w.things[3].creature.as_mut().unwrap().state = 0;
        assert!(!w.controlinator_auto(0));
        w.controlinator_enter(0, 3);
        assert_eq!(w.controlinator_driver(0), Some(3));
        assert_eq!(w.things[3].creature.unwrap().state, SEATED);
        assert_eq!(w.things[0].controlinator.unwrap().part.ref_18, None);
        assert!(!w.controlinator_in_reach(0, 3, 1.5), "taken");
        assert_eq!(w.controlinator_seat_tick(0), None, "still seated");
    }

    #[test]
    fn the_seat_lets_go_once_the_driver_stands() {
        let mut w = seat_world();
        w.things[0].controlinator.as_mut().unwrap().part.flags_50[1] = 1;
        assert!(w.controlinator_auto(0));
        w.controlinator_enter(0, 3);
        assert_eq!(w.things[0].controlinator.unwrap().part.ref_18, Some(3));
        w.things[3].creature.as_mut().unwrap().state = 0;
        assert_eq!(w.controlinator_seat_tick(0), Some(3));
        assert_eq!(w.things[0].controlinator.unwrap().part.driver, None);
        assert_eq!(w.controlinator_seat_tick(0), None, "empty");
    }

    #[test]
    fn a_driver_whose_seat_is_destroyed_is_dying() {
        let mut w = seat_world();
        w.controlinator_enter(0, 3);
        let cr = w.things[3].creature.as_mut().unwrap();
        cr.lethal = 2;
        assert!(!cr.seated_tick(), "the seat is still there");
        w.seat_destroyed(1);
        assert!(
            !w.things[3].creature.as_mut().unwrap().seated_tick(),
            "another Thing"
        );
        w.seat_destroyed(0);
        let cr = w.things[3].creature.as_mut().unwrap();
        assert!(cr.seated_tick());
        assert_eq!(
            (cr.state, cr.lethal, cr.frames),
            (crate::creature::DYING, 2, 0)
        );
        assert!(!cr.seated_tick(), "only from state 4");
    }

    #[test]
    fn a_driven_controlinator_holds_its_mode() {
        let mut w = world();
        w.things[1] = ctl(Some(3), None);
        assert!(!w.controlinator_holds_group(0, 1), "state 0 is empty");
        w.things[1].controlinator.as_mut().unwrap().state = 1;
        assert!(w.controlinator_holds_group(0, 1));
        w.things[1].controlinator.as_mut().unwrap().part.mode = 2;
        assert!(!w.controlinator_holds_group(0, 1), "another mode");
        w.things[1].controlinator.as_mut().unwrap().part.mode = 0;
        w.things[1].deactivated = true;
        assert!(!w.controlinator_holds_group(0, 1));
        w.things[1].deactivated = false;
        w.things[1].controlinator.as_mut().unwrap().part.driver = None;
        assert!(!w.controlinator_holds_group(0, 1), "no driver");
        assert!(!w.controlinator_holds_group(0, 2), "not a controlinator");
    }
}
