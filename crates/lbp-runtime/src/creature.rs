//! PCreature (a creature): how lethal contact and crushing kill a creature that is not a
//! player (D1137, docs/logic.md Part 39).
//!
//! FOUND (01.33): the runtime state byte is creature+0xc14 (0 at load, 0x829b4), set by
//! 0x7b454; frames in the state count at +0xc18 and the lethal type last applied is
//! +0xd68. The lethal processor 0x81824 (called from the update 0x8d2bc) scans 14 contact
//! slots and takes the strongest type 1 to 14; the gas types 6 to 11 count as 6, and a
//! creature with +0xf4c clear takes type 0xe whatever it touched. A creature with no
//! player part goes to state 5 (dying); after a duration from its config (+0xbf4: +0x40
//! fire, +0x48 electric, +0x50 type 0xe, +0x4c the rest) it goes to state 6 (dead), which
//! the processor then skips. Crushing (0x7d6ec) counts frames while the penetration
//! passes cfg+0x74 and kills with type 4 once the count passes cfg+0x78.
//!
//! The player path (0x63f28 notifies only) and the respawn are in checkpoint_lives.rs
//! (D1139). Not modelled: the early outs on Thing+0x78, 0x88f7dc,
//! world+0x804 and 0x63b80, the fire stun (state 1, cfg+0x58), the other lethal-4 writer
//! 0x952cc, and the PEnemy brain, whose reader was NOT FOUND. The config values are NOT
//! FOUND, so the host supplies them.

/// State 5: dying.
pub const DYING: u8 = 5;
/// State 6: dead.
pub const DEAD: u8 = 6;
/// Lethal type 4: crush.
pub const CRUSH: u8 = 4;
/// Lethal type 0xe: what a creature with +0xf4c clear takes.
pub const UNCLAIMED: u8 = 0xe;

/// The config durations and crush limits (creature+0xbf4), supplied by the host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureConfig {
    /// +0x40: dying frames after fire.
    pub fire_frames: u32,
    /// +0x48: dying frames after electricity.
    pub electric_frames: u32,
    /// +0x50: dying frames after type 0xe.
    pub unclaimed_frames: u32,
    /// +0x4c: dying frames after any other type. The strings FramesFrom{Fire,Ice,Electric,
    /// Crush,Drowned}DeathTillRespawn suggest +0x40..+0x50 in that order (INFERRED, D1139).
    pub other_frames: u32,
    /// +0x74: the penetration a crush frame must pass.
    pub crush_depth: f32,
    /// +0x78: the crush frames that kill.
    pub crush_frames: u32,
}

/// A creature's runtime state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CreatureState {
    /// +0xc14: 0 alive, 5 dying, 6 dead (other alive states are not modelled).
    pub state: u8,
    /// +0xc18: frames in the state.
    pub frames: u32,
    /// +0xd68: the lethal type last applied.
    pub lethal: u8,
    /// +0xd80: crush frames.
    pub crush: u32,
}

/// The strongest lethal type touched (0x81824): `contact[i]` is how hard type `i + 1` is
/// touched; ties keep the lower type, gas (6 to 11) reads as 6.
pub fn strongest_lethal(contact: &[f32; 14]) -> Option<u8> {
    let mut best: Option<(u8, f32)> = None;
    for (i, &v) in contact.iter().enumerate() {
        if v > 0.0 && best.is_none_or(|(_, b)| v > b) {
            best = Some((i as u8 + 1, v));
        }
    }
    best.map(|(t, _)| if (6..=11).contains(&t) { 6 } else { t })
}

impl CreatureState {
    fn set(&mut self, state: u8) {
        self.state = state;
        self.frames = 0;
    }

    fn kill(&mut self, lethal: u8) {
        if self.state != DYING && self.state != DEAD {
            self.lethal = lethal;
            self.set(DYING);
        }
    }

    /// Lethal contact for a creature with no player part. `claimed` is +0xf4c != 0.
    /// Returns the type applied.
    pub fn touch(&mut self, contact: &[f32; 14], claimed: bool) -> Option<u8> {
        if self.state == DEAD || self.state == DYING {
            return None;
        }
        let t = strongest_lethal(contact)?;
        let t = if claimed { t } else { UNCLAIMED };
        self.kill(t);
        Some(t)
    }

    /// One crush frame with this penetration (0x7d6ec); true when it kills.
    pub fn crush_frame(&mut self, depth: f32, cfg: &CreatureConfig) -> bool {
        if self.state == DYING || self.state == DEAD || depth <= cfg.crush_depth {
            return false;
        }
        self.crush += 1;
        if self.crush <= cfg.crush_frames {
            return false;
        }
        self.kill(CRUSH);
        true
    }

    /// One frame: counts the state's frames and ends dying; true on the frame it dies.
    pub fn tick(&mut self, cfg: &CreatureConfig) -> bool {
        self.frames += 1;
        if self.state != DYING {
            return false;
        }
        let limit = match self.lethal {
            1 => cfg.fire_frames,
            2 => cfg.electric_frames,
            UNCLAIMED => cfg.unclaimed_frames,
            _ => cfg.other_frames,
        };
        if self.frames < limit {
            return false;
        }
        self.set(DEAD);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: CreatureConfig = CreatureConfig {
        fire_frames: 3,
        electric_frames: 4,
        unclaimed_frames: 5,
        other_frames: 6,
        crush_depth: 0.5,
        crush_frames: 2,
    };

    fn touching(pairs: &[(u8, f32)]) -> [f32; 14] {
        let mut c = [0.0; 14];
        for &(t, v) in pairs {
            c[t as usize - 1] = v;
        }
        c
    }

    #[test]
    fn the_strongest_type_wins_and_gas_reads_as_six() {
        assert_eq!(strongest_lethal(&[0.0; 14]), None);
        assert_eq!(strongest_lethal(&touching(&[(1, 0.2), (2, 0.9)])), Some(2));
        assert_eq!(strongest_lethal(&touching(&[(9, 1.0), (1, 0.5)])), Some(6));
        assert_eq!(strongest_lethal(&touching(&[(3, 1.0), (5, 1.0)])), Some(3));
        assert_eq!(strongest_lethal(&touching(&[(12, 1.0)])), Some(12));
    }

    #[test]
    fn contact_kills_after_the_types_duration() {
        let mut c = CreatureState::default();
        assert_eq!(c.touch(&touching(&[(2, 1.0)]), true), Some(2));
        assert_eq!((c.state, c.lethal), (DYING, 2));
        assert_eq!(c.touch(&touching(&[(1, 1.0)]), true), None, "already dying");
        for _ in 0..3 {
            assert!(!c.tick(&CFG));
        }
        assert!(c.tick(&CFG), "electric: 4 frames");
        assert_eq!(c.state, DEAD);
        assert_eq!(
            c.touch(&touching(&[(1, 1.0)]), true),
            None,
            "dead is skipped"
        );
    }

    #[test]
    fn an_unclaimed_creature_takes_type_e() {
        let mut c = CreatureState::default();
        assert_eq!(c.touch(&touching(&[(1, 1.0)]), false), Some(UNCLAIMED));
        let frames = (0..10).take_while(|_| !c.tick(&CFG)).count() + 1;
        assert_eq!(frames, 5);
    }

    #[test]
    fn crushing_counts_deep_frames_then_kills() {
        let mut c = CreatureState::default();
        assert!(!c.crush_frame(0.5, &CFG), "not past the depth");
        assert!(!c.crush_frame(1.0, &CFG));
        assert!(!c.crush_frame(1.0, &CFG));
        assert!(c.crush_frame(1.0, &CFG), "past 2 frames");
        assert_eq!((c.state, c.lethal, c.crush), (DYING, CRUSH, 3));
        let frames = (0..10).take_while(|_| !c.tick(&CFG)).count() + 1;
        assert_eq!(frames, 6, "crush uses the other duration");
    }
}
