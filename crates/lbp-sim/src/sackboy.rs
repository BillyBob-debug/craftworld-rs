//! Sackboy's movement rules, Bevy-free: one call per 30 Hz game tick turns the pad state and
//! what the body touches into a per-tick velocity and a lane. Collision stays with the
//! caller (the physics world says whether Sackboy stands on something and which lanes have
//! floor); this module holds only the rules measured in RPCS3.
//!
//! Evidence: RPCS3 frame-by-frame reads of Sackboy's position (01.33): first in Rookie Test
//! (PC runs S9 and d190), then repeated in the owner's blank Moon level with the default
//! camera (PC run d211, 2026-10-09; repeats agree within about 1 unit), which is the
//! reference: CONFIRMED_FROM_LOCAL_DATA (emulated) for every constant below unless a
//! comment says otherwise. Units are LBP world units and game ticks. Not modelled
//! yet: the crouch/landing dips (about 35 units for 1-2 and 8 ticks; visual only), slopes,
//! grabbing, swimming, terminal velocity, the 10 u/tick plateau seen once after landing.

/// Gravity per tick². A 65-tick fall from the Moon's top (d230 fly-1, 173.7 per tick at the
/// end, no clamp) fits `vy = vy * AIR_DRAG - GRAVITY_TICK` with 2.698 to 0.015 rms; Ghidra
/// finds the same -2.7 at the normal gravity setting. Earlier fits: 2.693 (Rookie), 2.697.
pub const GRAVITY_TICK: f32 = 2.698;
/// Speed kept per airborne tick on both axes (the fall fit gives 0.9997; the horizontal
/// carry loses about 0.009 per tick at 29, the same ratio; Ghidra shows a matching 0.9996).
/// Releasing the stick in the air does not slow Sackboy; only this drag acts. It makes the
/// terminal fall speed about 9,000 per tick, so the world MaxVel of 100 seen in the code
/// never applied (173.7 was reached).
pub const AIR_DRAG: f32 = 0.9997;
/// Y offset from his resting position, per tick after touch-down: the legs compress to a
/// hard stop 35 below rest (reached by every landing seen: jumps, a 18.9/tick spawn drop,
/// a 29/tick fall) and spring back. Moon d230 (airrel-1/airrev-1), tick 0 = first tick
/// with floor contact. Position only; the walking physics above are unaffected.
pub const LANDING_DIP: [f32; 15] = [
    -26.19, -35.0, -35.0, -31.75, -25.28, -15.57, -8.57, -4.27, -2.53, -1.48, -0.86, -0.49, -0.26,
    -0.13, -0.05,
];
/// Ground acceleration and deceleration per tick² (linear: 3, 6, ... 30).
pub const WALK_ACCEL: f32 = 3.0;
/// Ground top speed per tick (900 u/s), reached in 10 ticks.
pub const WALK_SPEED: f32 = 30.0;
/// Air acceleration per tick² while the stick is held.
pub const AIR_ACCEL: f32 = 2.4;
/// The stick cannot push past this in the air (speed carried from the ground is kept).
/// Moon level 22.5 (Rookie Test read 22).
pub const AIR_SPEED: f32 = 22.5;
/// Upward speed per tick for each tick the jump button is held (index 0 = take-off tick).
/// The first three ticks always play; after the button is released, or after the table,
/// gravity takes over. A tap peaks at 143 units, a full hold at 260 (saturates by tick 10).
pub const JUMP_PROFILE: [f32; 10] = [12.0, 19.3, 23.3, 25.1, 25.5, 24.8, 23.5, 21.8, 19.7, 17.4];
/// Ticks of the jump profile that play even on a tap.
pub const JUMP_MIN_TICKS: usize = 3;
/// Distance between lanes (Moon and Rookie Test lanes at z -400, -200, 0).
pub const LANE_SPACING: f32 = 200.0;
/// Lane change speed per tick (linear slide, about 6 ticks per lane).
pub const LANE_SPEED: f32 = 35.0;

/// Pad state for one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    /// Stick x: -1 left .. 1 right (its sign is used; analogue walk speed is not measured).
    pub x: f32,
    pub jump: bool,
    /// Lane behind (-z), the W key in the RPCS3 mapping.
    pub back: bool,
    /// Lane in front (+z), the S key.
    pub front: bool,
}

/// What the world says this tick.
pub struct Contact<'a> {
    /// Standing on something.
    pub grounded: bool,
    /// Whether a lane centred at this z has floor under Sackboy.
    pub lane_has_floor: &'a dyn Fn(f32) -> bool,
}

/// Movement state between ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Sackboy {
    /// Velocity in units per tick.
    pub vx: f32,
    pub vy: f32,
    /// Current and target lane z.
    pub z: f32,
    pub lane: f32,
    /// Ticks since take-off while the jump profile is playing.
    jump_tick: Option<usize>,
    was_grounded: bool,
    prev: Input,
    /// Ticks since touch-down while the landing dip plays.
    dip_tick: Option<usize>,
}

impl Sackboy {
    /// Y offset of his position from rest this tick (0 outside a landing).
    pub fn landing_dip(&self) -> f32 {
        self.dip_tick.map_or(0., |t| LANDING_DIP[t])
    }

    pub fn at_lane(z: f32) -> Self {
        Self {
            z,
            lane: z,
            was_grounded: true,
            ..Self::default()
        }
    }

    /// Advances one tick and returns the displacement (dx, dy, dz) to apply.
    pub fn step(&mut self, input: Input, contact: &Contact) -> [f32; 3] {
        let grounded = contact.grounded && self.jump_tick.is_none();
        let dir = if input.x > 0. {
            1.
        } else if input.x < 0. {
            -1.
        } else {
            0.
        };
        // Horizontal.
        if grounded {
            if dir != 0. {
                self.vx = approach(self.vx, dir * WALK_SPEED, WALK_ACCEL);
            } else {
                self.vx = approach(self.vx, 0., WALK_ACCEL);
            }
        } else if dir != 0. && self.vx * dir < AIR_SPEED {
            self.vx = approach(self.vx, dir * AIR_SPEED, AIR_ACCEL);
        }
        // Vertical.
        let pressed = |now: bool, before: bool| now && !before;
        if grounded && pressed(input.jump, self.prev.jump) {
            self.jump_tick = Some(0);
        }
        match self.jump_tick {
            Some(t) if t < JUMP_PROFILE.len() && (t < JUMP_MIN_TICKS || input.jump) => {
                self.vy = JUMP_PROFILE[t];
                self.jump_tick = Some(t + 1);
            }
            _ => {
                self.jump_tick = None;
                self.vy = if grounded {
                    0.
                } else {
                    self.vy * AIR_DRAG - GRAVITY_TICK
                };
            }
        }
        // Lanes: a press on the ground, or a key held on the landing tick (an air press
        // released before landing is dropped).
        if grounded {
            let landed = !self.was_grounded;
            let back = pressed(input.back, self.prev.back) || (landed && input.back);
            let front = pressed(input.front, self.prev.front) || (landed && input.front);
            let step = match (back, front) {
                (true, false) => -LANE_SPACING,
                (false, true) => LANE_SPACING,
                _ => 0.,
            };
            if step != 0. && (contact.lane_has_floor)(self.lane + step) {
                self.lane += step;
            }
        }
        // Air drag and the landing dip.
        if !grounded {
            self.vx *= AIR_DRAG;
            self.dip_tick = None;
        } else if !self.was_grounded {
            self.dip_tick = Some(0);
        } else if let Some(t) = self.dip_tick {
            self.dip_tick = (t + 1 < LANDING_DIP.len()).then_some(t + 1);
        }
        let dz = approach(self.z, self.lane, LANE_SPEED) - self.z;
        self.z += dz;
        self.was_grounded = grounded;
        self.prev = input;
        [self.vx, self.vy, dz]
    }
}

fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat floor at y = 0 with lanes 0 and -200; returns per-tick (x, y, z).
    fn run(inputs: impl Fn(usize) -> Input, ticks: usize) -> Vec<[f32; 3]> {
        let lanes = |z: f32| z == 0. || z == -200.;
        let mut s = Sackboy::at_lane(0.);
        let mut p = [0.; 3];
        let mut out = Vec::new();
        for t in 0..ticks {
            let grounded = p[1] <= 0.;
            let d = s.step(
                inputs(t),
                &Contact {
                    grounded,
                    lane_has_floor: &lanes,
                },
            );
            p = [p[0] + d[0], (p[1] + d[1]).max(0.), p[2] + d[2]];
            out.push(p);
        }
        out
    }

    fn apex(path: &[[f32; 3]]) -> (usize, f32) {
        path.iter()
            .enumerate()
            .map(|(t, p)| (t + 1, p[1]))
            .fold((0, f32::MIN), |a, b| if b.1 > a.1 { b } else { a })
    }

    #[test]
    fn walk_reaches_top_speed_in_ten_ticks_and_stops_in_ten() {
        let path = run(
            |t| Input {
                x: if t < 30 { 1. } else { 0. },
                ..Default::default()
            },
            60,
        );
        let speed = |t: usize| path[t][0] - path[t - 1][0];
        assert_eq!(speed(9), 30.);
        assert_eq!(speed(8), 27.);
        // Measured stop distance 158 (release-tick alignment unknown): 135..165 here.
        let stop = path[59][0] - path[29][0];
        assert!((135. ..=165.).contains(&stop), "{stop}");
    }

    #[test]
    fn jump_heights_match_the_measured_tap_and_holds() {
        let jump_for = |held: usize| {
            run(
                move |t| Input {
                    jump: (1..=held).contains(&t),
                    ..Default::default()
                },
                40,
            )
        };
        // Tap: apex 143.4 at tick 11 after take-off; 7-tick hold 244.4; long hold 260.1.
        let (t, h) = apex(&jump_for(2));
        assert!((h - 143.4).abs() < 1. && t - 1 == 11, "tap {t} {h}");
        assert!((apex(&jump_for(7)).1 - 244.4).abs() < 1.);
        assert!((apex(&jump_for(19)).1 - 260.1).abs() < 1.);
        // Air time about 21 ticks for a tap.
        let air = jump_for(2).iter().filter(|p| p[1] > 0.).count();
        assert!((20..=22).contains(&air), "{air}");
    }

    #[test]
    fn air_control_is_capped_but_ground_speed_carries() {
        // Standing jump, then right held in the air: 2.4 per tick up to 22.5.
        let path = run(
            |t| Input {
                jump: (1..=19).contains(&t),
                x: if t >= 3 { 1. } else { 0. },
                ..Default::default()
            },
            20,
        );
        let speed = |t: usize| path[t][0] - path[t - 1][0];
        assert!(
            (speed(4) - 4.8).abs() < 0.01 && (speed(19) - 22.5).abs() < 0.05,
            "{path:?}"
        );
        // Running jump: top speed 30 kept in the air.
        let path = run(
            |t| Input {
                x: 1.,
                jump: (12..=20).contains(&t),
                ..Default::default()
            },
            30,
        );
        assert!((path[25][0] - path[24][0] - 30.).abs() < 0.2);
    }

    #[test]
    fn air_drag_is_tiny_and_landing_plays_the_measured_dip() {
        // Running jump, stick released in the air: speed barely changes (d230: -0.009/tick).
        let path = run(
            |t| Input {
                x: if t < 22 { 1. } else { 0. },
                jump: (16..=24).contains(&t),
                ..Default::default()
            },
            40,
        );
        let speed = |t: usize| path[t][0] - path[t - 1][0];
        assert!(
            (speed(17) - speed(29)).abs() < 0.2 && speed(29) > 29.,
            "{}",
            speed(29)
        );
        // The dip starts on the first grounded tick and ends at rest.
        let mut s = Sackboy::at_lane(0.);
        let lanes = |_: f32| true;
        let air = Contact {
            grounded: false,
            lane_has_floor: &lanes,
        };
        let floor = Contact {
            grounded: true,
            lane_has_floor: &lanes,
        };
        s.step(Input::default(), &air);
        assert_eq!(s.landing_dip(), 0.);
        s.step(Input::default(), &floor);
        assert_eq!(s.landing_dip(), LANDING_DIP[0]);
        for _ in 0..LANDING_DIP.len() {
            s.step(Input::default(), &floor);
        }
        assert_eq!(s.landing_dip(), 0.);
    }

    #[test]
    fn free_fall_matches_the_moon_drop_without_a_clamp() {
        // d230 fly-1: dy per tick -2.7, -5.4, -8.1 ... -173.66 on the 65th tick.
        let mut s = Sackboy::at_lane(0.);
        let lanes = |_: f32| true;
        let air = Contact {
            grounded: false,
            lane_has_floor: &lanes,
        };
        let mut v = Vec::new();
        for _ in 0..65 {
            v.push(s.step(Input::default(), &air)[1]);
        }
        assert!(
            (v[0] + 2.698).abs() < 0.01 && (v[2] + 8.09).abs() < 0.05,
            "{v:?}"
        );
        assert!((v[64] + 173.66).abs() < 0.6, "{}", v[64]);
    }

    #[test]
    fn lanes_change_on_the_ground_only_and_slide_35_per_tick() {
        // Ground press: z slides to -200 at 35 per tick; a second press finds no lane.
        let path = run(
            |t| Input {
                back: t == 1 || t == 12,
                ..Default::default()
            },
            20,
        );
        assert_eq!(path[1][2], -35.);
        assert_eq!(path[7][2], -200.);
        assert_eq!(path[19][2], -200.);
        // Pressed and released in the air: ignored.
        let path = run(
            |t| Input {
                jump: (1..=19).contains(&t),
                back: (10..=14).contains(&t),
                ..Default::default()
            },
            40,
        );
        assert_eq!(path[39][2], 0.);
        // Held through the landing: applies on landing.
        let path = run(
            |t| Input {
                jump: (1..=19).contains(&t),
                back: (25..=35).contains(&t),
                ..Default::default()
            },
            45,
        );
        assert_eq!(path[44][2], -200.);
    }
}
