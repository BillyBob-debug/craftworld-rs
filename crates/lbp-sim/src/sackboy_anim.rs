//! Which Sackboy clip plays, and at which frame, for the controller state of one 30 Hz tick
//! (slice 1 of local-analysis/results/2026-10-11-sackboy-animation-plan.md). Bevy-free.
//!
//! Evidence: the clip GUIDs, frame counts and the 30 fps rate are FOUND (MAP paths under
//! `gamedata/mesh_library/player_character/sack_boy_pinched_anims/` and the d116 animation
//! census). The game picks clips in C++ that reads hard-coded GUID fields (EBOOT loader at
//! VA 0x204438); those readers are not traced, so **every state -> clip rule, threshold and
//! pacing below is INFERRED** (plan section 4). Hard cuts, no blending; one clip frame per
//! tick at rate 1 (30 fps clips, 30 Hz ticks).
use crate::sackboy::{Sackboy, WALK_SPEED};

/// The clips this picker uses (plan slice 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipId {
    Stand,
    WalkSlow,
    WalkMedium,
    WalkFast,
    /// Lane change toward the camera (+z, the lane in front).
    WalkZForwards,
    /// Lane change away from the camera (-z).
    WalkZBackwards,
    JumpStand,
    JumpWalk,
    /// The landing squash (INFERRED: the only crouch-like clip; plan section 2).
    StretchCrouch,
}

impl ClipId {
    pub const ALL: [ClipId; 9] = [
        ClipId::Stand,
        ClipId::WalkSlow,
        ClipId::WalkMedium,
        ClipId::WalkFast,
        ClipId::WalkZForwards,
        ClipId::WalkZBackwards,
        ClipId::JumpStand,
        ClipId::JumpWalk,
        ClipId::StretchCrouch,
    ];

    /// Index into [`ClipId::ALL`] (for clip banks).
    pub fn index(self) -> usize {
        self as usize
    }

    /// The clip's animation GUID (FOUND, MAP + census).
    pub fn guid(self) -> u32 {
        match self {
            ClipId::Stand => 0x1241,
            ClipId::WalkSlow => 0x1242,
            ClipId::WalkMedium => 0x1243,
            ClipId::WalkFast => 0x1244,
            ClipId::WalkZForwards => 0x2fa7,
            ClipId::WalkZBackwards => 0x2fa5,
            ClipId::JumpStand => 0x1249,
            ClipId::JumpWalk => 0x124a,
            ClipId::StretchCrouch => 0x125a,
        }
    }

    /// Frame count from the d116 census (FOUND).
    pub fn frames(self) -> u16 {
        match self {
            ClipId::Stand => 19,
            ClipId::JumpStand | ClipId::JumpWalk => 31,
            ClipId::StretchCrouch => 9,
            _ => 13,
        }
    }

    /// The MAP name, for logs.
    pub fn name(self) -> &'static str {
        match self {
            ClipId::Stand => "stand",
            ClipId::WalkSlow => "walk_slow",
            ClipId::WalkMedium => "walk_medium",
            ClipId::WalkFast => "walk_fast",
            ClipId::WalkZForwards => "walk_z_forwards",
            ClipId::WalkZBackwards => "walk_z_backwards",
            ClipId::JumpStand => "jump_stand",
            ClipId::JumpWalk => "jump_walk",
            ClipId::StretchCrouch => "stretch_crouch",
        }
    }

    /// Loops (wraps) rather than holding its last frame. Every clip has loopStart 0, so
    /// the game decides this in code: INFERRED.
    pub fn looping(self) -> bool {
        !matches!(
            self,
            ClipId::JumpStand | ClipId::JumpWalk | ClipId::StretchCrouch
        )
    }

    fn is_walk(self) -> bool {
        matches!(
            self,
            ClipId::WalkSlow | ClipId::WalkMedium | ClipId::WalkFast
        )
    }

    fn is_jump(self) -> bool {
        matches!(self, ClipId::JumpStand | ClipId::JumpWalk)
    }
}

/// The clip shown this tick and its frame position (fractional; show `frame as i64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimState {
    pub clip: ClipId,
    pub frame: f32,
}

/// Below this |vx| (units per tick) on the ground he stands. INFERRED.
pub const STAND_MAX: f32 = 1.0;
/// Walk band tops (|vx| per tick): slow up to 10, medium up to 20, fast up to
/// [`WALK_SPEED`]. INFERRED (unmeasured thresholds; plan section 4).
pub const WALK_BANDS: [f32; 3] = [10.0, 20.0, WALK_SPEED];
/// Slowest walk playback rate, so a band's low end does not crawl. INFERRED.
pub const MIN_WALK_RATE: f32 = 0.5;
/// `FootAnimationSpeed` in `yellowhead.cha` (FOUND); its use as a rate factor is INFERRED.
pub const FOOT_ANIMATION_SPEED: f32 = 1.0;
/// Take-off |vx| above which the jump is a walking jump. INFERRED.
pub const JUMP_WALK_MIN: f32 = 5.0;
/// Frame an off-ledge fall (airborne without a take-off) enters the jump clip at: its
/// middle, past the push-off. INFERRED.
pub const FALL_ENTRY_FRAME: f32 = 15.0;
/// Landing squash ticks: the stretch_crouch clip's 9 frames, one per tick from the landing
/// tick, inside the 15-tick LANDING_DIP. INFERRED.
pub const SQUASH_TICKS: usize = 9;
/// |lane - z| above which the lane-change clip plays. INFERRED.
pub const LANE_EPS: f32 = 0.5;

/// The clip and frame for this tick. Call once per tick after [`Sackboy::step`], with
/// `grounded` the floor contact the step used and `prev` the last tick's result (`None` on
/// the first tick). All rules INFERRED (module doc).
pub fn pick_clip(s: &Sackboy, grounded: bool, prev: Option<AnimState>) -> AnimState {
    let speed = s.vx.abs();
    // Take-off this tick: the jump profile just played its first entry.
    if s.jump_tick() == Some(1) {
        let clip = if speed > JUMP_WALK_MIN {
            ClipId::JumpWalk
        } else {
            ClipId::JumpStand
        };
        return AnimState { clip, frame: 0. };
    }
    if !grounded || s.jump_tick().is_some() {
        // Keep the jump clip and hold its last frame while falling; an off-ledge fall
        // enters it mid-clip.
        return match prev {
            Some(p) if p.clip.is_jump() => advance(p, 1.),
            _ => AnimState {
                clip: if speed > JUMP_WALK_MIN {
                    ClipId::JumpWalk
                } else {
                    ClipId::JumpStand
                },
                frame: FALL_ENTRY_FRAME,
            },
        };
    }
    if let Some(t) = s.dip_tick().filter(|&t| t < SQUASH_TICKS) {
        return AnimState {
            clip: ClipId::StretchCrouch,
            frame: t as f32,
        };
    }
    let lane = s.lane - s.z;
    let (clip, rate) = if lane.abs() > LANE_EPS {
        let clip = if lane > 0. {
            ClipId::WalkZForwards
        } else {
            ClipId::WalkZBackwards
        };
        (clip, 1.)
    } else if speed < STAND_MAX {
        (ClipId::Stand, 1.)
    } else {
        let band = WALK_BANDS
            .iter()
            .position(|&top| speed <= top)
            .unwrap_or(WALK_BANDS.len() - 1);
        let clip = [ClipId::WalkSlow, ClipId::WalkMedium, ClipId::WalkFast][band];
        let rate = (speed / WALK_BANDS[band]).clamp(MIN_WALK_RATE, 1.) * FOOT_ANIMATION_SPEED;
        (clip, rate)
    };
    match prev {
        Some(p) if p.clip == clip => advance(p, rate),
        // Walk bands share the 13-frame stride: keep the phase across a band change.
        Some(p) if p.clip.is_walk() && clip.is_walk() => advance(AnimState { clip, ..p }, rate),
        _ => AnimState { clip, frame: 0. },
    }
}

/// Moves `rate` frames on: looping clips wrap, others hold their last frame.
fn advance(p: AnimState, rate: f32) -> AnimState {
    let frames = f32::from(p.clip.frames());
    let next = p.frame + rate;
    let frame = if p.clip.looping() {
        next.rem_euclid(frames)
    } else {
        next.min(frames - 1.)
    };
    AnimState {
        clip: p.clip,
        frame,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sackboy::{Contact, Input};

    fn at(vx: f32, z: f32, lane: f32) -> Sackboy {
        let mut s = Sackboy::at_lane(lane);
        s.vx = vx;
        s.z = z;
        s
    }

    fn state(clip: ClipId, frame: f32) -> Option<AnimState> {
        Some(AnimState { clip, frame })
    }

    #[test]
    fn clip_guids_match_the_plan_table() {
        let guids: Vec<u32> = ClipId::ALL.iter().map(|c| c.guid()).collect();
        assert_eq!(
            guids,
            [0x1241, 0x1242, 0x1243, 0x1244, 0x2fa7, 0x2fa5, 0x1249, 0x124a, 0x125a]
        );
        for (i, c) in ClipId::ALL.iter().enumerate() {
            assert_eq!(c.index(), i);
        }
    }

    #[test]
    fn stands_still_and_loops_the_stand_clip() {
        let a = pick_clip(&at(0., 0., 0.), true, None);
        assert_eq!(
            a,
            AnimState {
                clip: ClipId::Stand,
                frame: 0.
            }
        );
        let a = pick_clip(&at(0.5, 0., 0.), true, state(ClipId::Stand, 18.));
        assert_eq!(
            a,
            AnimState {
                clip: ClipId::Stand,
                frame: 0.
            }
        );
    }

    #[test]
    fn walk_bands_and_pacing() {
        let pick = |vx: f32| pick_clip(&at(vx, 0., 0.), true, None).clip;
        assert_eq!(pick(5.), ClipId::WalkSlow);
        assert_eq!(pick(-15.), ClipId::WalkMedium);
        assert_eq!(pick(30.), ClipId::WalkFast);
        // Full rate at a band's top, proportional below it, never under MIN_WALK_RATE.
        let step = |vx: f32, clip| pick_clip(&at(vx, 0., 0.), true, state(clip, 2.)).frame;
        assert_eq!(step(30., ClipId::WalkFast), 3.);
        assert_eq!(step(-25.5, ClipId::WalkFast), 2.85);
        assert_eq!(step(3., ClipId::WalkSlow), 2.5);
        // Wraps at 13 frames.
        let a = pick_clip(&at(30., 0., 0.), true, state(ClipId::WalkFast, 12.5));
        assert_eq!(a.frame, 0.5);
        // A band change keeps the stride phase.
        let a = pick_clip(&at(20., 0., 0.), true, state(ClipId::WalkFast, 7.));
        assert_eq!(
            a,
            AnimState {
                clip: ClipId::WalkMedium,
                frame: 8.
            }
        );
    }

    #[test]
    fn lane_change_picks_the_direction() {
        // Lane in front (+z) is toward the camera.
        let a = pick_clip(&at(0., -200., 0.), true, None);
        assert_eq!(a.clip, ClipId::WalkZForwards);
        let a = pick_clip(&at(30., 0., -200.), true, None);
        assert_eq!(a.clip, ClipId::WalkZBackwards);
    }

    #[test]
    fn falling_off_a_ledge_enters_the_jump_mid_clip_and_holds_the_last_frame() {
        let a = pick_clip(&at(20., 0., 0.), false, state(ClipId::WalkMedium, 4.));
        assert_eq!(
            a,
            AnimState {
                clip: ClipId::JumpWalk,
                frame: FALL_ENTRY_FRAME
            }
        );
        let a = pick_clip(&at(0., 0., 0.), false, state(ClipId::JumpStand, 29.6));
        assert_eq!(
            a,
            AnimState {
                clip: ClipId::JumpStand,
                frame: 30.
            }
        );
        let a = pick_clip(&at(0., 0., 0.), false, Some(a));
        assert_eq!(a.frame, 30.);
    }

    /// Drives the real controller over a flat floor and returns the clip sequence (clip
    /// changes only) and the per-tick states.
    fn run(inputs: impl Fn(usize) -> Input, ticks: usize) -> (Vec<ClipId>, Vec<AnimState>) {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        let mut y = 0f32;
        let mut prev = None;
        let mut states = Vec::new();
        let mut seq: Vec<ClipId> = Vec::new();
        for t in 0..ticks {
            let grounded = y <= 0. && s.vy <= 0.;
            let d = s.step(
                inputs(t),
                &Contact {
                    grounded,
                    lane_has_floor: &lanes,
                },
            );
            y = (y + d[1]).max(0.);
            let a = pick_clip(&s, grounded, prev);
            if seq.last() != Some(&a.clip) {
                seq.push(a.clip);
            }
            states.push(a);
            prev = Some(a);
        }
        (seq, states)
    }

    #[test]
    fn standing_jump_plays_jump_stand_then_squashes_then_stands() {
        let (seq, states) = run(
            |t| Input {
                jump: (5..=8).contains(&t),
                ..Default::default()
            },
            50,
        );
        assert_eq!(
            seq,
            [
                ClipId::Stand,
                ClipId::JumpStand,
                ClipId::StretchCrouch,
                ClipId::Stand
            ]
        );
        // Take-off on tick 5 shows frame 0; the squash plays its 9 frames one per tick.
        assert_eq!(
            states[5],
            AnimState {
                clip: ClipId::JumpStand,
                frame: 0.
            }
        );
        let squash: Vec<f32> = states
            .iter()
            .filter(|a| a.clip == ClipId::StretchCrouch)
            .map(|a| a.frame)
            .collect();
        assert_eq!(squash, (0..9).map(|f| f as f32).collect::<Vec<_>>());
    }

    #[test]
    fn running_jump_walks_up_the_bands_then_jump_walk() {
        let (seq, _) = run(
            |t| Input {
                x: 1.,
                jump: (20..=24).contains(&t),
                ..Default::default()
            },
            60,
        );
        assert_eq!(
            seq,
            [
                ClipId::WalkSlow,
                ClipId::WalkMedium,
                ClipId::WalkFast,
                ClipId::JumpWalk,
                ClipId::StretchCrouch,
                ClipId::WalkFast
            ]
        );
    }

    #[test]
    fn lane_press_plays_the_lane_clip_until_the_slide_ends() {
        let (seq, states) = run(
            |t| Input {
                back: t == 2,
                ..Default::default()
            },
            15,
        );
        assert_eq!(seq, [ClipId::Stand, ClipId::WalkZBackwards, ClipId::Stand]);
        // 200 units at 35 per tick: 6 ticks, the sixth arriving, so 5 show the lane clip.
        let lane = states
            .iter()
            .filter(|a| a.clip == ClipId::WalkZBackwards)
            .count();
        assert_eq!(lane, 5);
    }
}
