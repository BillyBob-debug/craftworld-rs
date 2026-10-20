//! Checkpoint lives and the player respawn (D1139, docs/logic.md Part 41).
//!
//! FOUND (01.33): a player's lethal contact (0x81824) calls 0x63f28, which only notifies
//! (tags to the killer, script hooks) and never sets the dying state. A creature in state 5
//! goes to state 6 once its frames reach the config duration (creature.rs). The respawn
//! 0x7fb00 sets state 0 and calls 0x5a2d8, which spawns at the checkpoint when 0x5a134
//! allows it. The checkpoint's lives are its spawnsLeft (+0x10, -1 until first activated)
//! and maxSpawnsLeft (+0x14). Activation 0x599dc sets both to lifeMultiplier (+0x38,
//! INFERRED) times (game+0x1e8 + 2) when spawnsLeft is -1; game+0x1e8 is INFERRED to be a
//! player count. Lives are unlimited (0x5a0c4 false) when game+0x180 != 0, in the pod
//! (game+0x3b5) or when the checkpoint's +0x41 is set (0x5a038, INFERRED to be
//! instanceInfiniteSpawns; 0x3ad08, 0x59e94 and a global also make it unlimited). A plain
//! respawn takes one life when lives are limited. Scripts read the player as dead while the
//! state is 5 or 6 (native_IsPlayerDead 0x5fe12c).
//!
//! Not modelled: the spawn slots (+0x28/+0x2c, four at most), the type 's' checkpoint and
//! the body test in 0x5a134, the failure counter creature+0xc48, who sends the respawn
//! message (0x7feb8), the lives_zero sound (0x18c948) and the game over.

use crate::creature::{CreatureState, DEAD, DYING};

/// A checkpoint's lives (PCheckpoint +0x10, +0x14, +0x34, +0x38, +0x41).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointLives {
    /// +0x10 spawnsLeft: -1 until the checkpoint is first activated.
    pub left: i32,
    /// +0x14 maxSpawnsLeft.
    pub max: i32,
    /// +0x34: a spawn waits while it is nonzero.
    pub delay: i32,
    /// +0x38 lifeMultiplier.
    pub multiplier: i32,
    /// +0x41: this checkpoint spawns without limit.
    pub infinite: bool,
}

/// The game state the lives read: game+0x1e8 and whether lives are unlimited
/// (game+0x180 != 0 or the pod, game+0x3b5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LivesRules {
    /// game+0x1e8 (INFERRED: the player count).
    pub players: i32,
    /// game+0x180 != 0 or game+0x3b5.
    pub unlimited: bool,
}

impl From<lbp_level::parts::Checkpoint> for CheckpointLives {
    fn from(c: lbp_level::parts::Checkpoint) -> Self {
        Self {
            left: c.spawns_left,
            max: c.max_spawns_left,
            delay: c.spawning_delay,
            multiplier: c.life_multiplier,
            infinite: c.infinite_spawns,
        }
    }
}

/// native_IsPlayerDead (0x5fe12c): the state is dying or dead.
pub fn is_player_dead(c: &CreatureState) -> bool {
    c.state == DYING || c.state == DEAD
}

impl CheckpointLives {
    /// 0x5a0c4: lives are counted.
    pub fn limited(&self, rules: LivesRules) -> bool {
        !rules.unlimited && !self.infinite
    }

    /// 0x599dc: the first activation gives multiplier x (players + 2) lives.
    pub fn activate(&mut self, rules: LivesRules) {
        if rules.players != 0 && self.left == -1 {
            self.left = self.multiplier * (rules.players + 2);
            self.max = self.left;
        }
    }

    /// 0x5a134, the lives part: no delay, and a life left when lives are counted.
    pub fn can_spawn(&self, rules: LivesRules, forced: bool) -> bool {
        self.delay == 0 && (forced || !self.limited(rules) || self.left > 0)
    }

    /// 0x5a2d8: spawns when it can. A plain spawn takes a life when lives are counted; a
    /// forced one tops the lives up toward players + 2 by the multiplier (three times it
    /// when spawnsLeft was negative).
    pub fn spawn(&mut self, rules: LivesRules, forced: bool) -> bool {
        if !self.can_spawn(rules, forced) {
            return false;
        }
        if !forced {
            if self.limited(rules) {
                self.left -= 1;
            }
            return true;
        }
        let was = self.left;
        self.left = was.max(0);
        let target = rules.players + 2;
        if self.max < target {
            let step = if was < 0 { 3 } else { 1 } * self.multiplier;
            let add = step.min(target - self.max);
            self.max += add;
            self.left += add;
        }
        true
    }

    /// 0x7fb00: a dead player respawns here in state 0 when the checkpoint spawns them;
    /// otherwise they stay dead.
    pub fn respawn(&mut self, c: &mut CreatureState, rules: LivesRules) -> bool {
        if !self.spawn(rules, false) {
            return false;
        }
        *c = CreatureState::default();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: LivesRules = LivesRules {
        players: 1,
        unlimited: false,
    };

    fn fresh(multiplier: i32) -> CheckpointLives {
        CheckpointLives {
            left: -1,
            max: 0,
            delay: 0,
            multiplier,
            infinite: false,
        }
    }

    #[test]
    fn the_first_activation_gives_lives_once() {
        let mut cp = fresh(1);
        cp.activate(ONE);
        assert_eq!((cp.left, cp.max), (3, 3));
        cp.left = 1;
        cp.activate(ONE);
        assert_eq!(cp.left, 1, "only while spawnsLeft is -1");
        let mut two = fresh(2);
        two.activate(LivesRules { players: 2, ..ONE });
        assert_eq!((two.left, two.max), (8, 8));
        let mut none = fresh(1);
        none.activate(LivesRules { players: 0, ..ONE });
        assert_eq!(none.left, -1);
    }

    #[test]
    fn respawns_take_lives_until_none_are_left() {
        let mut cp = fresh(1);
        cp.activate(ONE);
        let mut c = CreatureState {
            state: DEAD,
            ..Default::default()
        };
        assert!(is_player_dead(&c));
        for left in [2, 1, 0] {
            c.state = DEAD;
            assert!(cp.respawn(&mut c, ONE));
            assert_eq!((c.state, cp.left), (0, left));
            assert!(!is_player_dead(&c));
        }
        c.state = DEAD;
        assert!(!cp.respawn(&mut c, ONE), "no lives left");
        assert_eq!(c.state, DEAD);
    }

    #[test]
    fn unlimited_lives_and_the_delay() {
        let mut cp = fresh(1);
        cp.left = 0;
        let pod = LivesRules {
            unlimited: true,
            ..ONE
        };
        assert!(cp.spawn(pod, false));
        assert_eq!(cp.left, 0, "not counted");
        cp.infinite = true;
        assert!(cp.spawn(ONE, false));
        cp.delay = 5;
        assert!(!cp.spawn(pod, false), "waits on the delay");
    }

    #[test]
    fn a_forced_spawn_tops_up_toward_players_plus_two() {
        let mut cp = fresh(1);
        assert!(cp.spawn(ONE, true));
        assert_eq!(
            (cp.left, cp.max),
            (3, 3),
            "from -1: three times the multiplier"
        );
        let mut cp = fresh(1);
        cp.left = 0;
        cp.max = 1;
        assert!(cp.spawn(ONE, true));
        assert_eq!((cp.left, cp.max), (1, 2));
        assert!(cp.spawn(ONE, true));
        assert!(cp.spawn(ONE, true));
        assert_eq!((cp.left, cp.max), (2, 3), "stops at players + 2");
    }
}
