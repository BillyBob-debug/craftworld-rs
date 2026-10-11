//! Sackboy's animation layers, frames and facing for the controller state of one 30 Hz tick
//! (D232, docs/d231-sackboy-animations.md). Bevy-free.
//!
//! Evidence: the clip GUIDs, frame counts and the 30 fps rate are FOUND (MAP paths under
//! `gamedata/mesh_library/player_character/sack_boy_pinched_anims/` and the d116 census).
//! The selection rules are FOUND in the 01.33 EBOOT (Ghidra pass,
//! `local-analysis/results/2026-10-11-ghidra-sackboy-anim-rules.md`, function addresses on
//! each constant): continuous walk weights over stand with one shared 13-tick phase, facing
//! eased 0.4 per tick, the jump frame driven by vy, the land as the jump clip's tail, 5-tick
//! stop/turn ramps, 3-tick lane ramps, the idle LCG chains and the blink overlay. A second
//! pass (`local-analysis/results/2026-10-11-ghidra-sackboy-anim-open-items.md`) settled the
//! idle roll range, the layer order, "idle allowed", the seed, the first blink, the landing
//! exponent and the walk_z sign. What neither pass resolved is marked INFERRED or OPEN where
//! it is used.
//!
//! The output is a stack of [`Layer`]s applied in order, as the game does onto its final
//! layer: the first sets the pose, each [`Blend::Lerp`] layer then lerps the pose toward its
//! clip by its weight, and each [`Blend::Additive`] layer adds its frame minus a reference
//! frame. One clip frame per tick (30 fps clips, 30 Hz ticks).
use crate::sackboy::{Sackboy, WALK_SPEED};
use std::f32::consts::FRAC_PI_2;

/// The clips the picker uses (EBOOT anim-object fields in the doc of each variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipId {
    /// 0x280 (and its second copy 0x284, "one more stand cycle").
    Stand,
    /// 0x2f4 / 0x2f8 / 0x2fc.
    WalkSlow,
    WalkMedium,
    WalkFast,
    /// 0x300 / 0x304.
    WalkFastStop,
    WalkFastTurn,
    /// 0x308: lane change with dz > 0 (FOUND, 0x21685c..0x216b74; here dz = lane - z, a
    /// step toward the camera in our z. That the game's +z also points at the camera is
    /// INFERRED from its camera eye at +1218 z).
    WalkZForwards,
    /// 0x30c: dz < 0.
    WalkZBackwards,
    /// 0x310 / 0x314.
    JumpStand,
    JumpWalk,
    /// 0x36c: an additive spring pose in the game, driven on the ground path by shape
    /// overlap depth `max over contacts of min(depth / 350, 1)` (FOUND, 0x6c56c). Not
    /// driven here: we model no body overlap.
    StretchCrouch,
    /// 0x408: additive eyelid overlay.
    Blink,
    /// 0x2a4 / 0x2a8 / 0x2ac.
    IdleYawn,
    IdleShiftFeet,
    IdleFidget,
    /// 0x2dc / 0x2e0 / 0x2e4.
    StandIIdleBored,
    IdleBored,
    IdleBoredIStand,
    /// 0x2e8 / 0x2ec / 0x2f0.
    StandIIdleSit,
    IdleSit,
    IdleSitIStand,
}

impl ClipId {
    /// Stand first: the viewer's clip bank binds the rest to stand's rig.
    pub const ALL: [ClipId; 21] = [
        ClipId::Stand,
        ClipId::WalkSlow,
        ClipId::WalkMedium,
        ClipId::WalkFast,
        ClipId::WalkFastStop,
        ClipId::WalkFastTurn,
        ClipId::WalkZForwards,
        ClipId::WalkZBackwards,
        ClipId::JumpStand,
        ClipId::JumpWalk,
        ClipId::StretchCrouch,
        ClipId::Blink,
        ClipId::IdleYawn,
        ClipId::IdleShiftFeet,
        ClipId::IdleFidget,
        ClipId::StandIIdleBored,
        ClipId::IdleBored,
        ClipId::IdleBoredIStand,
        ClipId::StandIIdleSit,
        ClipId::IdleSit,
        ClipId::IdleSitIStand,
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
            ClipId::WalkFastStop => 0x1247,
            ClipId::WalkFastTurn => 0x1248,
            ClipId::WalkZForwards => 0x2fa7,
            ClipId::WalkZBackwards => 0x2fa5,
            ClipId::JumpStand => 0x1249,
            ClipId::JumpWalk => 0x124a,
            ClipId::StretchCrouch => 0x125a,
            ClipId::Blink => 0x22ba,
            ClipId::IdleYawn => 0x3110,
            ClipId::IdleShiftFeet => 0x316b,
            ClipId::IdleFidget => 0x317f,
            ClipId::StandIIdleBored => 0x310f,
            ClipId::IdleBored => 0x310e,
            ClipId::IdleBoredIStand => 0x3111,
            ClipId::StandIIdleSit => 0x2ff5,
            ClipId::IdleSit => 0x301d,
            ClipId::IdleSitIStand => 0x301c,
        }
    }

    /// Frame count from the d116 census (FOUND).
    pub fn frames(self) -> u16 {
        match self {
            ClipId::Stand => 19,
            ClipId::WalkFastStop => 21,
            ClipId::WalkFastTurn => 19,
            ClipId::JumpStand | ClipId::JumpWalk => 31,
            ClipId::StretchCrouch => 9,
            ClipId::IdleYawn => 76,
            ClipId::IdleShiftFeet => 26,
            ClipId::IdleFidget => 81,
            ClipId::StandIIdleBored => 33,
            ClipId::IdleBored => 25,
            ClipId::IdleBoredIStand => 28,
            ClipId::StandIIdleSit => 22,
            ClipId::IdleSit => 17,
            ClipId::IdleSitIStand => 21,
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
            ClipId::WalkFastStop => "walk_fast_stop",
            ClipId::WalkFastTurn => "walk_fast_turn",
            ClipId::WalkZForwards => "walk_z_forwards",
            ClipId::WalkZBackwards => "walk_z_backwards",
            ClipId::JumpStand => "jump_stand",
            ClipId::JumpWalk => "jump_walk",
            ClipId::StretchCrouch => "stretch_crouch",
            ClipId::Blink => "blink",
            ClipId::IdleYawn => "idle_yawn",
            ClipId::IdleShiftFeet => "idle_shift_feet",
            ClipId::IdleFidget => "idle_fidget",
            ClipId::StandIIdleBored => "stand_i_idle_bored",
            ClipId::IdleBored => "idle_bored",
            ClipId::IdleBoredIStand => "idle_bored_i_stand",
            ClipId::StandIIdleSit => "stand_i_idle_sit",
            ClipId::IdleSit => "idle_sit",
            ClipId::IdleSitIStand => "idle_sit_i_stand",
        }
    }
}

/// How a layer combines with the pose built so far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Blend {
    /// pose = lerp(pose, clip at `frame`, weight) (`0x1eae54`, FOUND).
    Lerp,
    /// pose += weight * (clip at `frame` - clip at `reference`) (`0x1eafc0`, FOUND; how the
    /// rotation difference is composed is INFERRED in the viewer).
    Additive { reference: f32 },
}

/// One clip sampled at one frame with a weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub clip: ClipId,
    /// Clip frame; fractional for the vy-driven jump frame (sample between frames).
    pub frame: f32,
    pub weight: f32,
    /// The game's reverse/mirror flag (`0x1ebb48` mode 5): walk layers walking backwards,
    /// jump_walk and stop/turn when moving left. The viewer mirrors such a layer left/right
    /// through the skeleton's mirror table.
    pub mirror: bool,
    pub blend: Blend,
}

/// The pose recipe for one tick and the facing angle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimFrame {
    /// Applied in order; the first is the base (weight 1).
    pub layers: Vec<Layer>,
    /// Turn about the vertical axis, radians: 0 faces the camera, +pi/2 is profile walking
    /// right (+x), -pi/2 walking left. Already eased (FOUND 0.4 per tick).
    pub yaw: f32,
}

impl AnimFrame {
    /// Each lerp layer's share of the final pose (its weight times what later lerp layers
    /// leave); additive layers get 0. Sums to 1 over the lerp layers.
    pub fn effective_weights(&self) -> Vec<f32> {
        let mut out = vec![0.; self.layers.len()];
        let mut keep = 1.;
        for (i, l) in self.layers.iter().enumerate().rev() {
            if l.blend != Blend::Lerp {
                continue;
            }
            let w = if i == 0 { 1. } else { l.weight };
            out[i] = w * keep;
            keep *= 1. - w;
        }
        out
    }

    /// The lerp layer with the largest share (for logs and tests).
    pub fn dominant(&self) -> Option<Layer> {
        let w = self.effective_weights();
        (0..self.layers.len())
            .max_by(|&a, &b| w[a].total_cmp(&w[b]))
            .map(|i| self.layers[i])
    }
}

/// `WalkSpeedMaxDesired` = 30 x speedMod (FOUND at 0x201be0; speedMod 1 INFERRED).
pub const WALK_MAX_DESIRED: f32 = WALK_SPEED;
/// Walk weights from r = |vx| / [`WALK_MAX_DESIRED`] (FOUND, 0x201680): slow `r/0.1`,
/// medium `(r-0.1)/0.3`, fast `(r-0.4)/0.6`, each clamped to [0, 1], lerped in that order
/// over stand.
pub const WALK_RAMPS: [(f32, f32); 3] = [(0.0, 0.1), (0.1, 0.3), (0.4, 0.6)];
/// The shared walk phase: all three 13-frame walk clips sampled at `tick mod 13`, not
/// speed-scaled (FOUND, 0x20178c).
pub const WALK_PHASE_FRAMES: u32 = 13;
/// Ticks of opposite sign before the walk layers' backwards flag flips (FOUND 5-tick
/// hysteresis at 0x968/0x96c; the exact counting is INFERRED).
pub const BACKWARDS_HYSTERESIS: u32 = 5;
/// Facing eases this fraction of the way to its target each tick (FOUND, 0x201cc0).
pub const FACING_EASE: f32 = 0.4;
/// Facing during a lane change, radians (72 degrees, FOUND 1.2566 at 0x2002ac; the 0x1128
/// scale is taken as 1 and the sign as the side he already faces: INFERRED).
pub const LANE_YAW: f32 = 1.2566;
/// Stick |x| below which walk_fast_stop starts, above which walk_fast_turn may (FOUND
/// 0xc4ab20 / 0xc4ab00), and the w_fast both need (FOUND).
pub const STOP_STICK: f32 = 0.1;
pub const TURN_STICK: f32 = 0.9;
pub const SHOT_MIN_FAST: f32 = 0.5;
/// Stop/turn weight ramps in and out over this many frames (FOUND).
pub const SHOT_RAMP: f32 = 5.;
/// |lane - z| that starts a lane clip, the frames it skips and its ramp (FOUND, 0x216b0c
/// and 0x201418).
pub const LANE_TRIGGER: f32 = 100.;
pub const LANE_SKIP: u32 = 3;
pub const LANE_RAMP: f32 = 3.;
/// Jump layer (FOUND, 0x20a5f4): weight = clamp(peak/10), the peak |vy| follower eases at
/// 0.125 per tick (how it follows a rising |vy| is INFERRED: it jumps up at once).
pub const JUMP_PEAK_FULL: f32 = 10.;
pub const JUMP_PEAK_EASE: f32 = 0.125;
/// Stand-vs-walk jump mix per |vx| in the air (FOUND 0.045, full at 22.2).
pub const JUMP_MIX_PER_VX: f32 = 0.045;
/// Jump frame (FOUND): take-off ticks 1..5 show frame = tick; rising
/// `8 * (1 - clamp(vy / 73))`; falling `8 + 6 * clamp(-vy / 30.26)`.
pub const JUMP_TAKEOFF_TICKS: u32 = 5;
pub const JUMP_RISE_VY: f32 = 73.;
pub const JUMP_FALL_VY: f32 = 30.26;
pub const JUMP_APEX_FRAME: f32 = 8.;
pub const JUMP_FALL_FRAMES: f32 = 6.;
/// Landing (FOUND): the jump clip plays on from `int((airFrame + 15) * 0.5)` at one frame
/// per tick, weight times `clamp(1 - (f - 15) / (n - 15))^k` with k = 1 (FOUND: the fade is
/// Perlin's bias(t, 0.5) = t, 0x20a7fc..0x20a888).
pub const LAND_FADE_FROM: f32 = 15.;
pub const LAND_FADE_POW: f32 = 1.;
/// Ticks between blinks: `30 + rand(70)` (FOUND, 0x2040a0).
pub const BLINK_GAP: u32 = 30;
pub const BLINK_GAP_RAND: u32 = 70;
/// The LCG state the anim object's constructor stores (FOUND 0x2a at 0x1ff64c; shared by
/// the idle roll and the blink gap).
pub const LCG_SEED: u32 = 42;
/// The idle roll's range: `mulli 0xfa` gives 0..249 (FOUND, 0x20e0ac..0x20e288).
pub const IDLE_ROLL: u32 = 250;
/// The sit loop's stand-up roll: `mulli 0x3e8`, 0..999, stands up when > 950 (FOUND,
/// 0x20e044).
pub const SIT_ROLL: u32 = 1000;
pub const SIT_STAND_UP_ABOVE: u32 = 950;
/// "Idle allowed" (0x1fb4a8) needs the no-input counter 0x75c above this many ticks
/// (FOUND, 0x1fb51c..0x1fb59c): about 10 s.
pub const IDLE_AFTER_TICKS: u32 = 300;

/// The game's idle LCG step (FOUND, 0x20dee0): `seed * 0x2e90edd + 0xe908fc1`, 30 bits.
pub fn lcg_next(seed: u32) -> u32 {
    seed.wrapping_mul(0x2e90edd).wrapping_add(0xe908fc1) & 0x3fff_ffff
}

/// `rand(n)` from a seed: `((seed >> 16) * n) >> 14`, 0..n-1 (FOUND).
pub fn lcg_rand(seed: u32, n: u32) -> u32 {
    ((seed >> 16) * n) >> 14
}

/// The idle the stand-cycle roll (0..249) picks (FOUND table, 0x20dee0): 0..2 sit, 3..4
/// yawn, 5..6 bored, 7..9 fidget, 10 shift_feet, otherwise one more stand cycle. Odds per
/// roll: 3, 2, 2, 3 and 1 in 250.
pub fn idle_for_roll(r: u32) -> ClipId {
    match r {
        0..=2 => ClipId::StandIIdleSit,
        3..=4 => ClipId::IdleYawn,
        5..=6 => ClipId::StandIIdleBored,
        7..=9 => ClipId::IdleFidget,
        10 => ClipId::IdleShiftFeet,
        _ => ClipId::Stand,
    }
}

/// Walk weights (slow, medium, fast) for |vx| (FOUND, [`WALK_RAMPS`]).
pub fn walk_weights(vx: f32) -> [f32; 3] {
    let r = (vx.abs() / WALK_MAX_DESIRED).clamp(0., 1.);
    WALK_RAMPS.map(|(from, span)| ((r - from) / span).clamp(0., 1.))
}

/// Facing target for the anim speed: `clamp(vx / WalkMax) * pi/2` (FOUND, 0x2002ac).
pub fn facing_target(vx: f32) -> f32 {
    (vx / WALK_MAX_DESIRED).clamp(-1., 1.) * FRAC_PI_2
}

/// The vy-driven jump frame after the take-off ticks (FOUND).
pub fn jump_frame(vy: f32) -> f32 {
    if vy > 0. {
        JUMP_APEX_FRAME * (1. - (vy / JUMP_RISE_VY).clamp(0., 1.))
    } else {
        JUMP_APEX_FRAME + JUMP_FALL_FRAMES * (-vy / JUMP_FALL_VY).clamp(0., 1.)
    }
}

/// Weight of frame `i` of an `n`-frame one-shot that ramps in and out over `k` frames.
fn ramp(i: u32, n: u16, k: f32) -> f32 {
    let i = i as f32;
    (i / k).min((f32::from(n) - i) / k).clamp(0., 1.)
}

/// A one-shot clip playing one frame per tick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shot {
    clip: ClipId,
    i: u32,
    mirror: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Air {
    /// Airborne ticks so far (1 on the first).
    since: u32,
    takeoff: bool,
    blend_in: f32,
    frame: f32,
}

/// Per-Sackboy animation state between ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct SackboyAnim {
    now: u64,
    walk_tick: u32,
    backwards: bool,
    flip_count: u32,
    yaw: f32,
    peak: f32,
    jump_mix: f32,
    jump_mirror: bool,
    air: Option<Air>,
    land: Option<f32>,
    lane: Option<Shot>,
    lane_armed: bool,
    shot: Option<Shot>,
    idle: ClipId,
    idle_frame: u32,
    idle_loops: u32,
    /// Ticks with no input on the ground (the game's 0x75c).
    quiet_ticks: u32,
    seed: u32,
    blink: Option<u32>,
    next_blink: u64,
}

impl Default for SackboyAnim {
    fn default() -> Self {
        Self::new()
    }
}

impl SackboyAnim {
    /// Standing, facing the camera, seed 42, first blink on the first tick (FOUND: the ctor
    /// zeroes the blink start 0x750 and the tick counter 0x754, 0x1f9984/0x1f9988).
    pub fn new() -> Self {
        Self {
            now: 0,
            walk_tick: 0,
            backwards: false,
            flip_count: 0,
            yaw: 0.,
            peak: 0.,
            jump_mix: 0.,
            jump_mirror: false,
            air: None,
            land: None,
            lane: None,
            lane_armed: true,
            shot: None,
            idle: ClipId::Stand,
            idle_frame: 0,
            idle_loops: 0,
            quiet_ticks: 0,
            seed: LCG_SEED,
            blink: None,
            next_blink: 0,
        }
    }

    /// The eased facing angle (radians, see [`AnimFrame::yaw`]).
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    fn rand(&mut self, n: u32) -> u32 {
        self.seed = lcg_next(self.seed);
        lcg_rand(self.seed, n)
    }

    /// The layers and facing for this tick. Call once per tick after [`Sackboy::step`],
    /// with `grounded` the floor contact the step used.
    pub fn tick(&mut self, s: &Sackboy, grounded: bool) -> AnimFrame {
        self.now += 1;
        let airborne = !grounded || s.jump_tick().is_some();
        let vx = s.vx;
        let input = s.prev_input();
        let stick = input.x;
        let [w_slow, w_med, w_fast] = walk_weights(vx);
        let mut layers = Vec::with_capacity(8);

        // Base: stand or the idle chain (hard switches at clip ends, FOUND).
        // Idle allowed (0x1fb4a8, FOUND): creature state in {0, 1, 7, 10} (taken as
        // grounded: INFERRED), no mood level above 0 (moods are not modelled, so always
        // true here) and the no-input counter 0x75c > 300. The counter (0x20d3a0) counts
        // ticks with no pad input in those states and resets otherwise; "no input" for the
        // 0x7feff mask is taken as stick 0 and no jump/lane button (INFERRED).
        let quiet = !airborne && stick == 0. && !input.jump && !input.back && !input.front;
        self.quiet_ticks = if quiet { self.quiet_ticks + 1 } else { 0 };
        let idle_ok = self.quiet_ticks > IDLE_AFTER_TICKS;
        layers.push(Layer {
            clip: self.idle,
            frame: self.idle_frame as f32,
            weight: 1.,
            mirror: false,
            blend: Blend::Lerp,
        });
        self.advance_idle(idle_ok);

        // Walk: continuous weights over the base, shared 13-tick phase (FOUND).
        if (vx < 0.) != self.backwards {
            self.flip_count += 1;
            if self.flip_count >= BACKWARDS_HYSTERESIS {
                self.backwards = !self.backwards;
                self.flip_count = 0;
            }
        } else {
            self.flip_count = 0;
        }
        let phase = (self.walk_tick % WALK_PHASE_FRAMES) as f32;
        self.walk_tick = self.walk_tick.wrapping_add(1);
        for (clip, weight) in [
            (ClipId::WalkSlow, w_slow),
            (ClipId::WalkMedium, w_med),
            (ClipId::WalkFast, w_fast),
        ] {
            if weight > 0. {
                layers.push(Layer {
                    clip,
                    frame: phase,
                    weight,
                    mirror: self.backwards,
                    blend: Blend::Lerp,
                });
            }
        }

        // Layer order after the walk (FOUND, dispatcher 0x20e28c): stop -> turn -> jump and
        // landing tail -> lane.

        // walk_fast_stop / walk_fast_turn (FOUND triggers; INFERRED: cut short in the air).
        if airborne {
            self.shot = None;
        } else if self.shot.is_none() && w_fast > SHOT_MIN_FAST {
            let clip = if stick.abs() < STOP_STICK {
                Some(ClipId::WalkFastStop)
            } else if stick.abs() > TURN_STICK && stick.signum() != vx.signum() {
                Some(ClipId::WalkFastTurn)
            } else {
                None
            };
            self.shot = clip.map(|clip| Shot {
                clip,
                i: 0,
                mirror: vx < 0.,
            });
        }
        if let Some(mut sh) = self.shot.take() {
            let n = sh.clip.frames();
            layers.push(Layer {
                clip: sh.clip,
                frame: sh.i as f32,
                weight: ramp(sh.i, n, SHOT_RAMP),
                mirror: sh.mirror,
                blend: Blend::Lerp,
            });
            sh.i += 1;
            if sh.i < u32::from(n) {
                self.shot = Some(sh);
            } else if sh.clip == ClipId::WalkFastTurn {
                // The turn's end resets the walk phase and the idle frame (FOUND).
                self.walk_tick = 0;
                self.idle_frame = 0;
            }
        }

        // Jump layer: vy-driven frame in the air, the clip's tail after landing (FOUND).
        let vy = s.vy.abs();
        self.peak = if vy > self.peak {
            vy
        } else {
            self.peak + JUMP_PEAK_EASE * (vy - self.peak)
        };
        let mut jump = None;
        if airborne {
            let air = self.air.get_or_insert(Air {
                since: 0,
                takeoff: s.jump_tick() == Some(1),
                // INFERRED: an off-ledge fall uses the same blend-in.
                blend_in: 1. + 4. * vx.abs() / 20.,
                frame: 0.,
            });
            air.since += 1;
            air.frame = if air.takeoff && air.since <= JUMP_TAKEOFF_TICKS {
                air.since as f32
            } else if air.frame > JUMP_APEX_FRAME {
                // Past the apex the frame only moves on: the touchdown tick reports vy 0
                // one tick before `grounded` (our controller), which mapped back to the
                // apex frame 8 and started the landing tail at 11 instead of 14 (INFERRED).
                jump_frame(s.vy).max(air.frame)
            } else {
                jump_frame(s.vy)
            };
            self.land = None;
            self.jump_mix = (vx * JUMP_MIX_PER_VX).abs().min(1.);
            self.jump_mirror = vx < 0.;
            let w = (self.peak / JUMP_PEAK_FULL).clamp(0., 1.)
                * (air.since as f32 / air.blend_in).clamp(0., 1.);
            jump = Some((air.frame, w));
        } else if let Some(air) = self.air.take() {
            self.land = Some(((air.frame + LAND_FADE_FROM) * 0.5).floor());
        }
        if let (false, Some(f)) = (airborne, self.land) {
            let n = f32::from(ClipId::JumpStand.frames());
            let fade = (1. - (f - LAND_FADE_FROM) / (n - LAND_FADE_FROM))
                .clamp(0., 1.)
                .powf(LAND_FADE_POW);
            jump = Some((f, (self.peak / JUMP_PEAK_FULL).clamp(0., 1.) * fade));
            self.land = (f + 1. < n).then_some(f + 1.);
        }
        if let Some((frame, w)) = jump.filter(|&(_, w)| w > 0.01) {
            // lerp(pose, lerp(jump_stand, jump_walk, m), w) as two sequential lerps.
            let m = self.jump_mix;
            let rest = 1. - w * m;
            let w_stand = if rest > 1e-6 { w * (1. - m) / rest } else { 0. };
            for (clip, weight, mirror) in [
                (ClipId::JumpStand, w_stand, false),
                (ClipId::JumpWalk, w * m, self.jump_mirror),
            ] {
                if weight > 0. {
                    layers.push(Layer {
                        clip,
                        frame,
                        weight,
                        mirror,
                        blend: Blend::Lerp,
                    });
                }
            }
        }

        // Lane change, after the jump (FOUND order): one clip per slide (re-armed once the
        // slide is within the trigger; INFERRED), frame i + 3, 3-frame ramps (FOUND).
        let dz = s.lane - s.z;
        if dz.abs() < LANE_TRIGGER {
            self.lane_armed = true;
        } else if self.lane_armed && self.lane.is_none() && !airborne {
            self.lane_armed = false;
            self.lane = Some(Shot {
                clip: if dz > 0. {
                    ClipId::WalkZForwards
                } else {
                    ClipId::WalkZBackwards
                },
                i: 0,
                mirror: false,
            });
        }
        let lane_active = self.lane.is_some();
        if let Some(mut l) = self.lane.take() {
            let n = l.clip.frames();
            let end = u32::from(n) - LANE_SKIP;
            let weight = ((l.i + 1) as f32 / LANE_RAMP).clamp(0., 1.)
                * ((end - l.i) as f32 / LANE_RAMP).clamp(0., 1.);
            layers.push(Layer {
                clip: l.clip,
                frame: (l.i + LANE_SKIP) as f32,
                weight,
                mirror: false,
                blend: Blend::Lerp,
            });
            l.i += 1;
            if l.i < end {
                self.lane = Some(l);
            }
        }

        // stretch_crouch is an additive spring (FOUND, 0x2011d4) whose input 0x6c56c is shape
        // overlap depth, max(min(depth / 350, 1)), on the ground path only (FOUND). We model
        // no body overlap, so it stays at rest and is not layered.

        // Blink: additive frame i minus frame 0, weight 1, first on tick 0, then every
        // 30 + rand(70) ticks (FOUND). Its slot in the stack is INFERRED (last): 0x2040a0
        // is reached through a pointer, not from the dispatcher.
        if self.blink.is_none() && self.now >= self.next_blink {
            self.blink = Some(0);
        }
        if let Some(i) = self.blink {
            layers.push(Layer {
                clip: ClipId::Blink,
                frame: i as f32,
                weight: 1.,
                mirror: false,
                blend: Blend::Additive { reference: 0. },
            });
            if i + 1 < u32::from(ClipId::Blink.frames()) {
                self.blink = Some(i + 1);
            } else {
                self.blink = None;
                let gap = BLINK_GAP + self.rand(BLINK_GAP_RAND);
                self.next_blink = self.now + u64::from(gap);
            }
        }

        // 0x2002ac returns +-72 degrees by phys+0x490 during a lane change and also while a
        // stop/turn clip plays (0x77c >= 0) (FOUND). The sign is the side he faces: the
        // stick's side, else his travel, else the current yaw (INFERRED; 0x490 is the same
        // +-1 the grab branch turns into +-90 degrees). Holding the side matters for the
        // stop: its body turns to the camera within the clip (-90 relative to its root by
        // frame 20), so easing to 0 underneath it overshot to a three-quarter left view.
        let target = if lane_active || self.shot.is_some() {
            let side = if stick != 0. {
                stick
            } else if vx != 0. {
                vx
            } else if self.yaw != 0. {
                self.yaw
            } else {
                1.
            };
            LANE_YAW.copysign(side)
        } else {
            facing_target(vx)
        };
        self.yaw += FACING_EASE * (target - self.yaw);
        AnimFrame {
            layers,
            yaw: self.yaw,
        }
    }

    /// Moves the base clip on one frame; at a clip end, the chain's next clip (FOUND).
    fn advance_idle(&mut self, idle_ok: bool) {
        self.idle_frame += 1;
        if self.idle_frame < u32::from(self.idle.frames()) {
            return;
        }
        self.idle_frame = 0;
        let current = self.idle;
        self.idle = match current {
            ClipId::Stand if idle_ok => idle_for_roll(self.rand(IDLE_ROLL)),
            ClipId::StandIIdleSit => ClipId::IdleSit,
            // Stands up 5 % of the loops, or when idle is no longer allowed.
            ClipId::IdleSit => {
                if !idle_ok || self.rand(SIT_ROLL) > SIT_STAND_UP_ABOVE {
                    // Standing up resets the no-input counter (FOUND, 0x20e05c).
                    self.quiet_ticks = 0;
                    ClipId::IdleSitIStand
                } else {
                    ClipId::IdleSit
                }
            }
            ClipId::StandIIdleBored => {
                self.idle_loops = 0;
                ClipId::IdleBored
            }
            ClipId::IdleBored => {
                self.idle_loops += 1;
                if self.idle_loops > 10 || !idle_ok {
                    ClipId::IdleBoredIStand
                } else {
                    ClipId::IdleBored
                }
            }
            _ => ClipId::Stand,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sackboy::{Contact, Input};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn moving(vx: f32) -> Sackboy {
        let mut s = Sackboy::at_lane(0.);
        s.vx = vx;
        s
    }

    /// Moving at `vx` with the stick held toward `x` (one controller step for the input,
    /// then the speed put back).
    fn holding(vx: f32, x: f32) -> Sackboy {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        s.step(
            Input {
                x,
                ..Default::default()
            },
            &Contact {
                grounded: true,
                lane_has_floor: &lanes,
            },
        );
        s.vx = vx;
        s
    }

    fn weight_of(f: &AnimFrame, clip: ClipId) -> f32 {
        f.layers
            .iter()
            .filter(|l| l.clip == clip)
            .map(|l| l.weight)
            .sum()
    }

    fn layer(f: &AnimFrame, clip: ClipId) -> Option<Layer> {
        f.layers.iter().copied().find(|l| l.clip == clip)
    }

    #[test]
    fn clip_table() {
        let guids: Vec<u32> = ClipId::ALL.iter().map(|c| c.guid()).collect();
        assert_eq!(guids.len(), 21);
        assert_eq!(guids[..4], [0x1241, 0x1242, 0x1243, 0x1244]);
        assert_eq!(ClipId::ALL[0], ClipId::Stand);
        for (i, c) in ClipId::ALL.iter().enumerate() {
            assert_eq!(c.index(), i);
        }
    }

    #[test]
    fn walk_weights_follow_the_found_ramps() {
        // Full slow at 3 u/tick, full medium at 12, full fast at 30 (rules note).
        assert_eq!(walk_weights(0.), [0., 0., 0.]);
        assert!(close(walk_weights(1.5)[0], 0.5));
        assert_eq!(walk_weights(3.), [1., 0., 0.]);
        assert!(close(walk_weights(-7.5)[1], 0.5));
        assert_eq!(walk_weights(12.), [1., 1., 0.]);
        // r 0.7 (21 u/tick): w_fast 0.5, the stop/turn gate.
        assert!(close(walk_weights(21.)[2], 0.5));
        assert_eq!(walk_weights(30.), [1., 1., 1.]);
        assert_eq!(walk_weights(-45.), [1., 1., 1.]);
    }

    #[test]
    fn walk_layers_share_one_13_tick_phase_at_any_speed() {
        for vx in [3., 15., 30.] {
            let mut a = SackboyAnim::new();
            let s = moving(vx);
            let frames: Vec<f32> = (0..14)
                .map(|_| layer(&a.tick(&s, true), ClipId::WalkSlow).unwrap().frame)
                .collect();
            let want: Vec<f32> = (0..13).chain(0..1).map(|f| f as f32).collect();
            assert_eq!(frames, want, "vx {vx}");
        }
        // At 15 u/tick: stand, slow 1, medium 1, fast 0.1667; all at the same frame (the
        // tick-0 blink, additive, dropped).
        let mut f = SackboyAnim::new().tick(&moving(15.), true);
        f.layers.retain(|l| l.blend == Blend::Lerp);
        let clips: Vec<ClipId> = f.layers.iter().map(|l| l.clip).collect();
        assert_eq!(
            clips,
            [
                ClipId::Stand,
                ClipId::WalkSlow,
                ClipId::WalkMedium,
                ClipId::WalkFast
            ]
        );
        assert!(close(f.layers[3].weight, 1. / 6.));
        let eff = f.effective_weights();
        assert!(close(eff.iter().sum::<f32>(), 1.));
        assert!(close(eff[0], 0.) && close(eff[1], 0.) && close(eff[3], 1. / 6.));
    }

    #[test]
    fn walking_left_flags_the_walk_layers_after_five_ticks() {
        let mut a = SackboyAnim::new();
        let s = moving(-20.);
        let flags: Vec<bool> = (0..6)
            .map(|_| layer(&a.tick(&s, true), ClipId::WalkFast).unwrap().mirror)
            .collect();
        assert_eq!(flags, [false, false, false, false, true, true]);
    }

    #[test]
    fn facing_eases_toward_the_speed_angle() {
        let mut a = SackboyAnim::new();
        assert_eq!(a.tick(&moving(0.), true).yaw, 0.);
        // Full speed right, stick held: 0.4 of the way per tick toward +90 degrees.
        let mut a = SackboyAnim::new();
        let run = holding(30., 1.);
        let y1 = a.tick(&run, true).yaw;
        assert!(close(y1, 0.4 * FRAC_PI_2));
        for _ in 0..4 {
            a.tick(&run, true);
        }
        // About 90 % of the way in 4.5 ticks (1 - 0.6^5 = 0.922 after five).
        assert!(close(a.yaw(), (1. - 0.6f32.powi(5)) * FRAC_PI_2));
        // Half speed is three-quarter (45 degrees); left is negative.
        let mut a = SackboyAnim::new();
        for _ in 0..60 {
            a.tick(&moving(-15.), true);
        }
        assert!(close(a.yaw(), -FRAC_PI_2 / 2.));
    }

    #[test]
    fn stop_and_turn_hold_72_degrees_on_the_side_faced() {
        // Stop moving left: the target stays -72 while walk_fast_stop plays, then 0.
        let mut a = SackboyAnim::new();
        for _ in 0..30 {
            a.tick(&holding(-30., -1.), true);
        }
        let mut s = moving(-30.);
        let mut yaws = Vec::new();
        for i in 0..40 {
            s.vx = (-30. + 3. * i as f32).min(0.);
            let f = a.tick(&s, true);
            yaws.push((layer(&f, ClipId::WalkFastStop).is_some(), f.yaw));
        }
        let during: Vec<f32> = yaws.iter().filter(|y| y.0).map(|y| y.1).collect();
        let after: Vec<f32> = yaws.iter().filter(|y| !y.0).map(|y| y.1).collect();
        assert_eq!(during.len(), 21);
        // Held through frame 19; the last frame (20) already eases back toward 0.
        assert!(close(during[19], -LANE_YAW), "{}", during[19]);
        assert!(close(during[20], -LANE_YAW * 0.6), "{}", during[20]);
        assert!(after.last().unwrap().abs() < 0.01);
        // Turn: stick pushed right while running left, the target flips to +72 at once.
        let mut a = SackboyAnim::new();
        for _ in 0..30 {
            a.tick(&holding(-30., -1.), true);
        }
        let mut yaw = 0.;
        for _ in 0..18 {
            let f = a.tick(&holding(-30., 1.), true);
            assert!(layer(&f, ClipId::WalkFastTurn).is_some());
            yaw = f.yaw;
        }
        assert!((yaw - LANE_YAW).abs() < 1e-3, "{yaw}");
    }

    #[test]
    fn jump_frame_numbers() {
        assert!(close(jump_frame(0.), 8.)); // apex
        assert!(close(jump_frame(-30.26), 14.)); // full fall
        assert!(close(jump_frame(-200.), 14.));
        assert!(close(jump_frame(-15.13), 11.));
        assert!(close(jump_frame(36.5), 4.));
        assert!(close(jump_frame(73.), 0.));
    }

    #[test]
    fn stop_and_turn_ramp_over_five_frames() {
        assert_eq!(
            (0..21).map(|i| ramp(i, 21, 5.)).collect::<Vec<_>>()[..7],
            [0., 0.2, 0.4, 0.6, 0.8, 1., 1.]
        );
        assert!(close(ramp(20, 21, 5.), 0.2));
        // Stick released at full speed: walk_fast_stop plays 21 frames, mirrored moving left.
        let mut a = SackboyAnim::new();
        let mut s = moving(-30.);
        let f = a.tick(&s, true);
        let stop = layer(&f, ClipId::WalkFastStop).unwrap();
        assert_eq!((stop.frame, stop.weight, stop.mirror), (0., 0., true));
        s.vx = 0.;
        let count = 1
            + (0..30)
                .filter(|_| layer(&a.tick(&s, true), ClipId::WalkFastStop).is_some())
                .count();
        assert_eq!(count, 21);
        // Below w_fast 0.5 (r 0.7) nothing starts.
        let f = SackboyAnim::new().tick(&moving(20.), true);
        assert!(layer(&f, ClipId::WalkFastStop).is_none());
    }

    #[test]
    fn turn_plays_19_frames_then_resets_the_walk_phase() {
        let mut a = SackboyAnim::new();
        let mut s = moving(30.);
        // Push the stick hard the other way.
        let push = |a: &mut SackboyAnim, s: &mut Sackboy, x: f32| {
            let lanes = |_: f32| true;
            let mut t = *s;
            t.step(
                Input {
                    x,
                    ..Default::default()
                },
                &Contact {
                    grounded: true,
                    lane_has_floor: &lanes,
                },
            );
            // Keep the speed fixed so only the stick changes.
            t.vx = s.vx;
            *s = t;
            a.tick(s, true)
        };
        for _ in 0..3 {
            push(&mut a, &mut s, 1.);
        }
        let mut turn = 0;
        let mut last = AnimFrame::default();
        for _ in 0..19 {
            last = push(&mut a, &mut s, -1.);
            turn += usize::from(layer(&last, ClipId::WalkFastTurn).is_some());
        }
        assert_eq!(turn, 19);
        assert_eq!(layer(&last, ClipId::WalkFastTurn).unwrap().frame, 18.);
        // The tick after the turn ends shows walk phase 0 again (and a new turn starts).
        let f = push(&mut a, &mut s, -1.);
        assert_eq!(layer(&f, ClipId::WalkFast).unwrap().frame, 0.);
    }

    #[test]
    fn lane_clip_skips_three_frames_and_ramps_over_three() {
        let mut a = SackboyAnim::new();
        let mut s = Sackboy::at_lane(0.);
        s.lane = -200.;
        let mut got = Vec::new();
        let mut yaw = Vec::new();
        for _ in 0..12 {
            let f = a.tick(&s, true);
            if let Some(l) = layer(&f, ClipId::WalkZBackwards) {
                got.push((l.frame, l.weight));
                yaw.push(f.yaw);
            }
        }
        let frames: Vec<f32> = got.iter().map(|g| g.0).collect();
        assert_eq!(frames, (3..13).map(|f| f as f32).collect::<Vec<_>>());
        let w: Vec<f32> = got.iter().map(|g| g.1).collect();
        for (i, want) in [1. / 3., 2. / 3., 1., 1., 1., 1., 1., 1., 2. / 3., 1. / 3.]
            .iter()
            .enumerate()
        {
            assert!(close(w[i], *want), "{i}: {}", w[i]);
        }
        // Facing turns toward 72 degrees while it plays.
        assert!(close(yaw[9], LANE_YAW * (1. - 0.6f32.powi(10))));
    }

    #[test]
    fn idle_roll_table_and_lcg() {
        assert_eq!(idle_for_roll(0), ClipId::StandIIdleSit);
        assert_eq!(idle_for_roll(2), ClipId::StandIIdleSit);
        assert_eq!(idle_for_roll(4), ClipId::IdleYawn);
        assert_eq!(idle_for_roll(6), ClipId::StandIIdleBored);
        assert_eq!(idle_for_roll(9), ClipId::IdleFidget);
        assert_eq!(idle_for_roll(10), ClipId::IdleShiftFeet);
        assert_eq!(idle_for_roll(11), ClipId::Stand);
        assert_eq!(lcg_next(0), 0xe908fc1);
        assert!(lcg_next(u32::MAX) < 1 << 30);
        assert_eq!(lcg_rand(0x3fff_ffff, 1000), 999);
        assert_eq!(lcg_rand(0, 1000), 0);
        // The idle roll is 0..249 (mulli 0xfa).
        assert_eq!(lcg_rand(0x3fff_ffff, IDLE_ROLL), 249);
        // Over the LCG from seed 42, the five idles take about 11 rolls in 250.
        let mut seed = LCG_SEED;
        let n = 100_000;
        let idles = (0..n)
            .filter(|_| {
                seed = lcg_next(seed);
                idle_for_roll(lcg_rand(seed, IDLE_ROLL)) != ClipId::Stand
            })
            .count();
        let rate = idles as f32 / n as f32;
        assert!((rate - 11. / 250.).abs() < 0.005, "{rate}");
    }

    #[test]
    fn idle_waits_for_300_quiet_ticks() {
        // Standing with no input: stand only until the counter passes 300, then the chain.
        let mut a = SackboyAnim::new();
        let still = moving(0.);
        let first_idle = (0..5_000u32)
            .find(|_| a.tick(&still, true).layers[0].clip != ClipId::Stand)
            .unwrap();
        assert!(first_idle > IDLE_AFTER_TICKS, "{first_idle}");
        // Any input (here the stick, at zero speed) resets the counter: no idle ever starts.
        let mut a = SackboyAnim::new();
        let pushing = holding(0., 1.);
        for t in 0..5_000u32 {
            let s = if t % 250 == 0 { &pushing } else { &still };
            assert_eq!(a.tick(s, true).layers[0].clip, ClipId::Stand, "tick {t}");
        }
    }

    #[test]
    fn layers_run_walk_stop_jump_then_lane() {
        // Start a lane change toward +z (dz > 0: walk_z_forwards), then fall while it plays.
        let mut a = SackboyAnim::new();
        let mut s = moving(0.);
        s.lane = 200.;
        let f = a.tick(&s, true);
        assert!(layer(&f, ClipId::WalkZForwards).is_some());
        s.vy = -25.;
        s.vx = 10.;
        let f = a.tick(&s, false);
        let at = |c: ClipId| f.layers.iter().position(|l| l.clip == c).unwrap();
        assert!(at(ClipId::Stand) < at(ClipId::WalkSlow));
        assert!(at(ClipId::WalkSlow) < at(ClipId::JumpStand));
        assert!(at(ClipId::JumpStand) < at(ClipId::WalkZForwards));
        assert!(at(ClipId::WalkZForwards) < at(ClipId::Blink));
    }

    #[test]
    fn idle_chains_and_blinks_while_standing() {
        let mut a = SackboyAnim::new();
        let s = moving(0.);
        let mut bases = Vec::new();
        let mut blinks = Vec::new();
        for t in 0..20_000u32 {
            let f = a.tick(&s, true);
            assert_eq!(f.layers[0].weight, 1.);
            if bases.last() != Some(&f.layers[0].clip) {
                bases.push(f.layers[0].clip);
            }
            if let Some(b) = layer(&f, ClipId::Blink) {
                assert_eq!(b.blend, Blend::Additive { reference: 0. });
                if b.frame == 0. {
                    blinks.push(t);
                }
            }
        }
        // Intros lead into their loops and loops back out through their outros.
        for w in bases.windows(2) {
            let ok = match w[0] {
                ClipId::Stand => true,
                ClipId::StandIIdleSit => w[1] == ClipId::IdleSit,
                ClipId::IdleSit => w[1] == ClipId::IdleSitIStand,
                ClipId::StandIIdleBored => w[1] == ClipId::IdleBored,
                ClipId::IdleBored => w[1] == ClipId::IdleBoredIStand,
                _ => w[1] == ClipId::Stand,
            };
            assert!(ok, "{:?} -> {:?}", w[0], w[1]);
        }
        assert!(bases.len() > 3, "{bases:?}");
        // First blink on the first tick; the next starts 30 + rand(70) ticks after the last
        // blink frame (12 ticks after its start).
        assert_eq!(blinks[0], 0);
        for w in blinks.windows(2) {
            let gap = w[1] - w[0];
            assert!((42..=111).contains(&gap), "{gap}");
        }
        // Walking with the stick held never starts the chain.
        let mut a = SackboyAnim::new();
        let run = holding(30., 1.);
        for _ in 0..1_000 {
            let f = a.tick(&run, true);
            assert_eq!(f.layers[0].clip, ClipId::Stand);
        }
    }

    /// Drives the real controller over a flat floor; returns the per-tick frames.
    fn run(inputs: impl Fn(usize) -> Input, ticks: usize) -> Vec<AnimFrame> {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        let mut a = SackboyAnim::new();
        let mut y = 0f32;
        (0..ticks)
            .map(|t| {
                let grounded = y <= 0. && s.vy <= 0.;
                let d = s.step(
                    inputs(t),
                    &Contact {
                        grounded,
                        lane_has_floor: &lanes,
                    },
                );
                y = (y + d[1]).max(0.);
                a.tick(&s, grounded)
            })
            .collect()
    }

    #[test]
    fn standing_jump_takes_off_peaks_falls_and_lands_on_the_tail() {
        let frames = run(
            |t| Input {
                jump: (5..=14).contains(&t),
                ..Default::default()
            },
            70,
        );
        let jump: Vec<(usize, f32, f32)> = frames
            .iter()
            .enumerate()
            .filter_map(|(t, f)| layer(f, ClipId::JumpStand).map(|l| (t, l.frame, l.weight)))
            .collect();
        // Take-off on tick 5: frames 1..5 on the first five airborne ticks, full weight.
        let first: Vec<f32> = jump[..5].iter().map(|j| j.1).collect();
        assert_eq!(first, [1., 2., 3., 4., 5.]);
        assert_eq!(jump[0].0, 5);
        assert!(jump[..5].iter().all(|j| j.2 == 1.));
        // The frame reaches the apex (8) and the full-fall range, never past 14 in the air.
        let air_max = jump
            .iter()
            .map(|j| j.1)
            .take_while(|&f| f <= 14.)
            .fold(0f32, f32::max);
        assert!(air_max > 8. && air_max <= 14.);
        // Landing: the tail plays one frame per tick from int((airFrame + 15) / 2) and fades.
        let tail: Vec<(f32, f32)> = jump
            .iter()
            .filter(|j| j.1 > 14.)
            .map(|j| (j.1, j.2))
            .collect();
        assert!(tail.len() > 5, "{jump:?}");
        for w in tail.windows(2) {
            assert_eq!(w[1].0, w[0].0 + 1.);
            assert!(w[1].1 <= w[0].1);
        }
        assert!(tail.last().unwrap().0 <= 30.);
        // Standing still: no jump_walk.
        assert!(frames.iter().all(|f| layer(f, ClipId::JumpWalk).is_none()));
    }

    #[test]
    fn touchdown_tick_with_vy_zero_keeps_the_fall_frame() {
        // Falling at -25 (frame 12.96), then the contact tick reports vy 0 before
        // `grounded`: the frame holds, and the tail starts at int((12.96 + 15) / 2) = 13.
        let mut a = SackboyAnim::new();
        let mut s = moving(0.);
        s.vy = -25.;
        let f = a.tick(&s, false);
        let fall = layer(&f, ClipId::JumpStand).unwrap().frame;
        assert!(close(fall, 8. + 6. * 25. / 30.26));
        s.vy = 0.;
        let f = a.tick(&s, false);
        assert_eq!(layer(&f, ClipId::JumpStand).unwrap().frame, fall);
        let f = a.tick(&s, true);
        assert_eq!(layer(&f, ClipId::JumpStand).unwrap().frame, 13.);
    }

    #[test]
    fn running_jump_mixes_jump_walk_by_air_speed() {
        let frames = run(
            |t| Input {
                x: 1.,
                jump: (20..=24).contains(&t),
                ..Default::default()
            },
            40,
        );
        let f = &frames[22];
        // vx about 30 x 0.045 > 1: all jump_walk; blend-in 1 + 4 x 30 / 20 = 7 ticks.
        assert!(weight_of(f, ClipId::JumpStand) < 1e-4);
        let w: Vec<f32> = frames[20..28]
            .iter()
            .map(|f| weight_of(f, ClipId::JumpWalk))
            .collect();
        assert!(close(w[0], 1. / 7.), "{w:?}");
        assert!(close(w[6], 1.), "{w:?}");
        // Full-speed walk under it, facing right.
        assert!(close(weight_of(&frames[19], ClipId::WalkFast), 1.));
        assert!(frames[30].yaw > 1.4);
    }
}
