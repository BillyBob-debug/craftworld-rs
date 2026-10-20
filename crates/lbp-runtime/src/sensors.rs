//! The tag sensor, the player sensor and the impact sensor of the switch pass
//! (docs/level-scripts.md part 15, docs/logic.md Part 15).
//!
//! FOUND in the 01.33 executable (tag sensor 0x118d5c, player sensor 0x119910, their
//! ternary rule 0x11873c, the tag map 0xceac0 / 0xd5c1c, the tag's signal 0x12f6f4, the
//! team filter 0xf9a64):
//! - Both run every tick at their place in the pass order and give every output of the
//!   switch the same signal; the inversion of an inverted switch is applied afterwards
//!   ([`crate::compute_output`]), also to "off".
//! - Distances are between world positions, x and y only.
//!
//! Modelled since D1141, and not modelled:
//! - The cone of a sensor whose AngleRange is below 180 is modelled
//!   ([`LevelWorld::cone_direction`]), except for a sensor on a cursor and the chip-table
//!   angle of a Thing without a PPos.
//! - The layer test of a sensor whose IncludeTouching is not 0 (0x1196b0, 0x11a13c) is
//!   modelled from each body's PShape slab ([`LevelWorld::z_span`]); a tilted Thing's
//!   outline is approximated by its local box, and the unspawned-checkpoint rule of a
//!   player sensor (0x11a31c) is not modelled.
//! - A sensor or tag on a player's cursor; the body's owning player (the player a tag
//!   without one is given, and the "own" and "other" team filters of a versus level).
//! - The body centre the player sensor measures between: the Thing positions are used.
//! - A tag skipped by a Thing flag in one game state (the meaning of both not settled).
//! - Players out of play and DetectUnspawnedPlayers on a player sensor (players waiting
//!   at a checkpoint): the world has neither state.
//! - The impact sensor has no cone or layer test at all (D1142, docs/logic.md Part 44).
//!   Its own leftovers: IncludeRigidConnectors (PSwitch+0xca, the whole rigid group's
//!   contacts), the collision mask and shape overlap tests of a touching contact, the
//!   game-state check of its tag filter, creatures counting as players and the tag's
//!   own player. See [`impact_sensor`].
use crate::switches::{clamp, held_ternary, wired, SwitchContext, SIGNAL_OFF};
use crate::{LevelWorld, SwitchState, TagRecord};
use lbp_script_vm::natives::SwitchInput;

/// A sensor's or a tag's body for the layer test.
enum Body {
    /// Neither the Thing nor its proxy has the part, and there is no parent.
    NoParent,
    /// The holder has no slab (no PShape or no PPos).
    None,
    /// The slab's world z span.
    Span(f32, f32),
}

/// The local box of a PShape: x and y bounds of its vertices and its thickness.
pub(crate) fn shape_box(s: &lbp_level::parts::Shape) -> Option<[f32; 5]> {
    let mut b = [f32::MAX, f32::MIN, f32::MAX, f32::MIN, s.thickness];
    for v in &s.vertices {
        b = [
            b[0].min(v[0]),
            b[1].max(v[0]),
            b[2].min(v[1]),
            b[3].max(v[1]),
            b[4],
        ];
    }
    (!s.vertices.is_empty()).then_some(b)
}

/// How many Things above a sensor the cone's rotation sum walks at most.
const MAX_CONE_DEPTH: usize = 64;

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0. {
        v
    } else {
        v.map(|x| x / l)
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Determinant of the upper 3 x 3 of a column-major matrix.
fn det3(m: &[f32; 16]) -> f32 {
    let c = cross([m[0], m[1], m[2]], [m[4], m[5], m[6]]);
    c[0] * m[8] + c[1] * m[9] + c[2] * m[10]
}

/// The cone test of a sensor whose AngleRange is below 180 (FOUND, 0x118d5c and
/// 0x119910): the cosine of the angle between `dir` and the candidate's offset must be at
/// least cos(AngleRange), the half angle in degrees. INFERRED: a candidate at the
/// sensor's own point passes; a sensor without a direction sees nothing.
fn in_cone(s: &SwitchState, dir: Option<[f32; 2]>, dx: f32, dy: f32, d2: f32) -> bool {
    if s.angle_range >= 180. {
        return true;
    }
    let Some([x, y]) = dir else { return false };
    d2 == 0. || (x * dx + y * dy) / d2.sqrt() >= s.angle_range.to_radians().cos()
}

/// The signal of a tag with nothing wired to it (FOUND, 0x12f6f4).
const TAG_ON: SwitchInput = SwitchInput {
    activation: 1.,
    ternary: 1,
    player: -1,
};

impl LevelWorld {
    /// The Thing's microchip Thing when it has one, else the Thing itself: the Thing a
    /// sensor or a tag is placed by (FOUND: Thing+0x5c when set; which chip, see
    /// [`LevelWorld::microchip_of`]).
    pub fn sensor_proxy(&self, thing: u32) -> u32 {
        self.microchip_of(thing).unwrap_or(thing)
    }

    /// World z span (nearest, farthest) of a Thing's PShape slab, from its local box and
    /// its world matrix; `None` without a PShape or a PPos.
    ///
    /// INFERRED: the game's body box (+0x160 of the body) holds the slab's world bounds; the
    /// local x / y box stands in for the outline, the same when the Thing is not tilted.
    pub fn z_span(&self, thing: u32) -> Option<(f32, f32)> {
        let t = self.things.get(thing as usize)?;
        let ([x0, x1, y0, y1, d], m) = (t.shape_box?, t.world_matrix?);
        let z = |x: f32, y: f32| m[2] * x + m[6] * y + m[14];
        let lo = z(x0, y0).min(z(x0, y1)).min(z(x1, y0)).min(z(x1, y1));
        let hi = z(x0, y0).max(z(x0, y1)).max(z(x1, y0)).max(z(x1, y1));
        let depth = (m[10] * d).abs();
        Some((lo - depth, hi + depth))
    }

    /// The body a sensor or a tag is measured by (FOUND 0x1196b0, 0x11984c): its proxy's
    /// when the proxy has the body part, else the proxy's parent's.
    ///
    /// INFERRED: the part tested (bit 31 of Thing+0x10) is the PShape.
    fn body_of(&self, thing: u32) -> Body {
        let p = self.sensor_proxy(thing);
        let Some(t) = self.things.get(p as usize) else {
            return Body::NoParent;
        };
        let holder = if t.parts.contains(&"SHAPE") {
            p
        } else if let Some(parent) = t.parent {
            parent
        } else {
            return Body::NoParent;
        };
        self.z_span(holder)
            .map_or(Body::None, |(lo, hi)| Body::Span(lo, hi))
    }

    /// The layer test of a sensor whose IncludeTouching is not 0 against a tag
    /// (FOUND 0x119844): accepted when the sensor has no body or the tag's body is empty,
    /// rejected when the tag has neither the part nor a parent, else the z spans must
    /// overlap.
    fn tag_in_layer(&self, sensor: u32, tag: u32) -> bool {
        let Body::Span(s0, s1) = self.body_of(sensor) else {
            return true;
        };
        match self.body_of(tag) {
            Body::NoParent => false,
            Body::None => true,
            Body::Span(t0, t1) => !(s0 >= t1 || t0 >= s1),
        }
    }

    /// The layer test of a player sensor whose IncludeTouching is not 0 (FOUND 0x11a13c):
    /// accepted when the sensor has no body; rejected when its span is 20 or thinner;
    /// else the player's middle layer (z / 200, floored) must lie between the sensor's
    /// nearest and farthest layer (rounded).
    ///
    /// INFERRED: 0x758784 is floor; the player's body span is [`PlayerState::layers`].
    fn player_in_layer(&self, sensor: u32, layers: (f32, f32)) -> bool {
        let Body::Span(s0, s1) = self.body_of(sensor) else {
            return true;
        };
        if s1 - s0 <= 20. {
            return false;
        }
        let (lo, hi) = ((s0 / 200. + 0.5).floor(), (s1 / 200. + 0.5).floor());
        let p = ((layers.0 + layers.1) * 0.5 / 200.).floor();
        lo <= p && p <= hi
    }

    /// The x / y direction a sensor's cone points along (FOUND 0x1141f4, 0x113fb0,
    /// 0x110140): its proxy's local +Y, with the tilt of the proxy's local Z turned out by
    /// the shortest arc onto world Z, flattened and normalised, then turned by the
    /// sensor's rotation on its chip. A sensor that is its own proxy points along its own
    /// +Y. `None` without a PPos on the proxy.
    ///
    /// The rotation (PSwitch+0xdc) adds, for the sensor and each Thing above it until one
    /// that is not on a chip, the inverse of its in-plane rotation in its local matrix,
    /// and negates the sum after a mirrored one; the direction is the base turned the
    /// other way round, negated again for a mirrored proxy.
    ///
    /// A Thing without a PPos adds the angle of its entry in its chip's component table
    /// instead, and negates the sum when the entry is flipped; one not in the table adds
    /// nothing (D1143).
    ///
    /// INFERRED: the game runs the sum when a level is loaded or edited, this runtime on
    /// every test; table word 5 is the component's `angle` (words 3..8 read as x, y,
    /// angle, scaleX, scaleY, flipped). Not modelled: a sensor under a cursor chip, which
    /// looks along its cursor's angle (Part 45).
    pub fn cone_direction(&self, sensor: u32) -> Option<[f32; 2]> {
        let proxy = self.sensor_proxy(sensor);
        let m = self.things.get(proxy as usize)?.world_matrix?;
        let n = normalize3([m[8], m[9], m[10]]);
        let y = [m[4], m[5], m[6]];
        // Rodrigues with the unnormalised axis a = n x Z (|a| = sin, c = cos).
        let (a, c) = ([n[1], -n[0], 0.], n[2]);
        let v = if c > -1. + 1e-6 {
            let ay = cross(a, y);
            let k = (a[0] * y[0] + a[1] * y[1]) / (1. + c);
            [0, 1].map(|i| y[i] * c + ay[i] + a[i] * k)
        } else {
            [y[0], y[1]]
        };
        let len = v[0].hypot(v[1]);
        if len == 0. {
            return None;
        }
        let mut angle = 0f32;
        let mut t = sensor;
        for _ in 0..MAX_CONE_DEPTH {
            if self.microchip_of(t).is_none_or(|c| c == t) {
                break;
            }
            let thing = &self.things[t as usize];
            if let Some(l) = thing.local_matrix {
                angle += -(-l[4]).atan2(l[5]);
                if det3(&l) < 0. {
                    angle = -angle;
                }
            } else if let Some(c) = self.chip_entry(t) {
                // No PPos: its entry in the chip's component table (0x1103f8).
                angle += c.angle;
                if c.flipped {
                    angle = -angle;
                }
            }
            let Some(parent) = thing.parent else { break };
            t = parent;
        }
        if det3(&m) < 0. {
            angle = -angle;
        }
        let (sin, cos) = angle.sin_cos();
        let (x, y) = (v[0] / len, v[1] / len);
        Some([x * cos + y * sin, y * cos - x * sin])
    }

    /// A Thing's entry in the component table of the chip whose board is its parent
    /// (FOUND 0x110140: the chip is the board's parent, the Thing's grandparent).
    fn chip_entry(&self, thing: u32) -> Option<crate::ChipAngle> {
        let board = self.things.get(thing as usize)?.parent?;
        let chip = *self.board_owners.get(&board)?;
        let table = &self.things.get(chip as usize)?.chip_components;
        table.iter().find(|c| c.thing == thing).copied()
    }

    /// World x and y of a Thing with a PPos.
    fn point(&self, thing: u32) -> Option<[f32; 2]> {
        let m = self.things.get(thing as usize)?.world_matrix?;
        Some([m[12], m[13]])
    }

    /// Builds the world's tag map again: per (colour, name) the tags with their
    /// positions, by ascending x.
    ///
    /// FOUND (0xd5c1c, the first call of the switch pass 0xdd228): every record's x and y
    /// are taken from the tag's proxy Thing and each key's array is sorted by x, once per
    /// tick before any switch is updated. So all sensors of a tick see the tags where
    /// they were when the pass started. A tag whose proxy has no PPos is left out here
    /// (the game drops it in the sensor, 0x118d5c).
    pub fn refresh_tags(&mut self) {
        let mut tags = std::mem::take(&mut self.tags);
        for list in tags.values_mut() {
            list.clear();
        }
        for (i, t) in self.things.iter().enumerate() {
            let (Some(key), thing) = (t.switch_key, i as u32) else {
                continue;
            };
            if let Some([x, y]) = self.point(self.sensor_proxy(thing)) {
                tags.entry(key).or_default().push(TagRecord { x, y, thing });
            }
        }
        tags.retain(|_, list| !list.is_empty());
        for list in tags.values_mut() {
            list.sort_by(|a, b| a.x.total_cmp(&b.x));
        }
        self.tags = tags;
    }

    /// The signal a tag carries.
    ///
    /// FOUND (0x12f6f4): off while the tag's Thing is deactivated; else the signal of the
    /// output wired to the Thing's input port 0, as it stands at that moment of the pass;
    /// with nothing wired {1.0, 1, -1}.
    pub fn tag_signal(&self, tag: u32) -> SwitchInput {
        match self.things.get(tag as usize) {
            Some(t) if !t.deactivated => wired(self, tag, 0).map_or(TAG_ON, |(_, s)| s),
            _ => SIGNAL_OFF,
        }
    }
}

/// Whether a distance (squared) lies in the sensor's range.
///
/// FOUND (0x1192f4, 0x119f50): MinRadius^2 <= d^2 <= Radius^2, compared as squares; no
/// test at all with a negative Radius.
fn in_range(s: &SwitchState, d2: f32) -> bool {
    s.radius < 0. || (s.min_radius * s.min_radius <= d2 && d2 <= s.radius * s.radius)
}

/// How close a candidate at distance `d` is, 1 at the sensor; not clamped.
///
/// FOUND (0x1192f4, 0x119f50; the constant 0.75 read from the executable): 1.0 with a
/// negative Radius; (1 - d / Radius) / 0.75 with MinRadius 0; else
/// 1 - (d - MinRadius) / (Radius - MinRadius).
fn closeness(s: &SwitchState, d: f32) -> f32 {
    if s.radius < 0. {
        1.
    } else if s.min_radius == 0. {
        (1. - d / s.radius) / 0.75
    } else {
        1. - (d - s.min_radius) / (s.radius - s.min_radius)
    }
}

/// The ternary of a sensor's activation.
///
/// FOUND (0x11873c; 3.75, 0.25 and 0.75 read from the executable): with s = activation *
/// 3.75, off at s <= 0.25, on at s >= 0.75, in between what output 0 holds. With
/// MinRadius 0 that is on within 0.85 of the Radius and off beyond 0.95 of it.
fn sensor_ternary(cx: &SwitchContext<'_>, activation: f32) -> i32 {
    let s = activation * 3.75;
    if s <= 0.25 {
        0
    } else if s >= 0.75 {
        1
    } else {
        held_ternary(cx)
    }
}

/// The team filter of a tag sensor: whether the tag signal's player passes.
///
/// FOUND (0xf9a64), by the PWorld's game mode. `owner` is the player who owns the
/// sensor's body.
/// - Mode 1 (versus): filters 0, 3 and 4 pass; 1 passes the owner, 2 any other player;
///   no player fails both.
/// - Mode 5 (teams): 0 passes; 1 the owner's team, 2 another team, 3 team 0, 4 team 1;
///   no player fails these four.
/// - Any other mode: 0, 1, 3 and 4 pass, 2 fails.
///
/// INFERRED: a filter value past 4 fails in modes 1 and 5 as well.
pub(crate) fn team_filter(world: &LevelWorld, player: i32, filter: i32, owner: i32) -> bool {
    let records = &world.player_records;
    let team = |p: i32| {
        records
            .iter()
            .find(|r| r.number == p)
            .map_or(-1, |r| r.team)
    };
    match (world.pworld.game_mode, filter) {
        (_, 0) | (1, 3 | 4) => true,
        (1, 1) => player != -1 && player == owner,
        (1, 2) => player != -1 && player != owner,
        (5, 1) => player != -1 && team(player) == team(owner),
        (5, 2) => player != -1 && team(player) != team(owner),
        (5, 3) => player != -1 && team(player) == 0,
        (5, 4) => player != -1 && team(player) == 1,
        (1 | 5, _) => false,
        (_, f) => matches!(f, 1 | 3 | 4),
    }
}

/// The body's owning player, which the runtime does not keep: nobody.
pub(crate) const NO_OWNER: i32 = -1;

/// The tag sensor (switch type 4), before the inversion.
///
/// FOUND (0x118d5c):
/// - The tags are those of the world's tag map under the sensor's colour and name
///   ([`LevelWorld::refresh_tags`]). With a Radius of 0 or more only the tags whose x lies
///   within the Radius of the sensor's x are looked at, by ascending x.
/// - The sensor's point is the position of its proxy Thing, a tag's the one in the map.
/// - KeySensorMode 1 hands on the tag's own signal; any other mode measures closeness.
/// - Per tag, with its signal ([`LevelWorld::tag_signal`]): in mode 1 a tag whose
///   activation is 0 is skipped, and one whose ternary is not 0 is counted and its
///   ternary remembered, before the tests that follow; in the other modes a tag whose
///   ternary is 0 is skipped. Then the team filter on the signal's player; a tag on the
///   sensor's own proxy is skipped unless DetectUnspawnedPlayers is set; then the range.
///   In the closeness modes the signal's activation becomes the closeness.
/// - The tags that pass are kept by falling magnitude of activation (a later tag goes
///   before an equal earlier one), cut to BulletsRequired entries. Fewer than that, or
///   BulletsRequired 0: off.
/// - The result comes from the last kept entry, the BulletsRequired-th strongest, with
///   its player. Mode 1: its activation as it is and its ternary; a ternary of 0 is
///   replaced by the remembered one when at least BulletsRequired tags were counted.
///   Other modes: the closeness clamped to 0..1 with the ternary of [`sensor_ternary`].
///
/// INFERRED: a negative BulletsRequired gives off.
pub(crate) fn tag_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let me = w.sensor_proxy(cx.thing);
    let (Some(tags), Some([sx, sy])) = (w.tags.get(&s.key), w.point(me)) else {
        return SIGNAL_OFF;
    };
    let near = if s.radius >= 0. {
        let from = tags.partition_point(|r| r.x < sx - s.radius);
        let to = tags.partition_point(|r| r.x <= sx + s.radius);
        &tags[from..to.max(from)]
    } else {
        &tags[..]
    };
    let dir = w.cone_direction(cx.thing);
    let signal_mode = s.key_sensor_mode == 1;
    let needed = usize::try_from(s.bullets_required).unwrap_or(0);
    let mut best: Vec<SwitchInput> = Vec::new();
    let (mut counted, mut last_ternary) = (0usize, 0);
    for r in near {
        let mut signal = w.tag_signal(r.thing);
        if signal_mode {
            if signal.activation == 0. {
                continue;
            }
            if signal.ternary != 0 {
                counted += 1;
                last_ternary = signal.ternary;
            }
        } else if signal.ternary == 0 {
            continue;
        }
        if !team_filter(w, signal.player, s.team_filter, NO_OWNER) {
            continue;
        }
        if w.sensor_proxy(r.thing) == me && !s.detect_unspawned_players {
            continue;
        }
        let (dx, dy) = (r.x - sx, r.y - sy);
        let d2 = dx * dx + dy * dy;
        if !in_range(s, d2) || !in_cone(s, dir, dx, dy, d2) {
            continue;
        }
        if s.include_touching != 0 && !w.tag_in_layer(cx.thing, r.thing) {
            continue;
        }
        if !signal_mode {
            signal.activation = closeness(s, d2.sqrt());
        }
        let strength = signal.activation.abs();
        let at = best.iter().position(|b| b.activation.abs() <= strength);
        best.insert(at.unwrap_or(best.len()), signal);
        best.truncate(needed);
    }
    let Some(e) = best.get(needed.wrapping_sub(1)).copied() else {
        return SIGNAL_OFF;
    };
    if signal_mode {
        let held = e.ternary == 0 && counted >= needed;
        return SwitchInput {
            ternary: if held { last_ternary } else { e.ternary },
            ..e
        };
    }
    let activation = clamp(e.activation, 0., 1.);
    SwitchInput {
        activation,
        ternary: sensor_ternary(cx, activation),
        player: e.player,
    }
}

/// The player sensor (switch type 3), before the inversion.
///
/// FOUND (0x119910):
/// - The candidates are the players' Sackboys in the PWorld's list, in its order. With
///   PlayerMode 0 only the leader's counts. Each one that counts is "eligible", in range
///   or not.
/// - The squared distances of those in range are kept ascending, cut to 4 entries with
///   RequireAll and to BulletsRequired without. The output's player is the nearest one.
/// - Needed in range: every eligible player with RequireAll, else BulletsRequired.
///   Fewer, or none needed: off. The activation is the closeness of the last one needed,
///   clamped to 0..1 (1.0 with a negative Radius), with the ternary of
///   [`sensor_ternary`].
///
/// INFERRED: the leader is the first player record's number (as
/// `Game.GetPartyLeadersPlayerNumber__`); the points are the positions of the sensor's
/// proxy Thing and of the Sackboy's Thing (the game takes the centre of each one's body);
/// a sensor without a position gives off.
pub(crate) fn player_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let Some([sx, sy]) = w.point(w.sensor_proxy(cx.thing)) else {
        return SIGNAL_OFF;
    };
    let dir = w.cone_direction(cx.thing);
    let leader = w.player_records.first().map(|r| r.number);
    let bullets = usize::try_from(s.bullets_required).unwrap_or(0);
    let cap = if s.require_all { 4 } else { bullets };
    let mut nearest: Vec<f32> = Vec::new();
    let (mut player, mut eligible) = (-1, 0usize);
    for p in &w.players {
        if s.player_mode == 0 && Some(p.number) != leader {
            continue;
        }
        eligible += 1;
        let Some([px, py]) = w.point(p.thing) else {
            continue;
        };
        let (dx, dy) = (px - sx, py - sy);
        let d2 = dx * dx + dy * dy;
        if !in_range(s, d2) || !in_cone(s, dir, dx, dy, d2) {
            continue;
        }
        if s.include_touching != 0 && !w.player_in_layer(cx.thing, p.layers) {
            continue;
        }
        if nearest.first().is_none_or(|&n| d2 < n) {
            player = p.number;
        }
        let at = nearest.iter().position(|&n| n >= d2);
        nearest.insert(at.unwrap_or(nearest.len()), d2);
        nearest.truncate(cap);
    }
    let needed = if s.require_all { eligible } else { bullets };
    let Some(d2) = nearest.get(needed.wrapping_sub(1)) else {
        return SIGNAL_OFF;
    };
    let activation = clamp(closeness(s, d2.sqrt()), 0., 1.);
    SwitchInput {
        activation,
        ternary: sensor_ternary(cx, activation),
        player,
    }
}

/// The impact sensor (switch type 18).
///
/// FOUND (0x11c2a8 → 0x11ae80, per contact 0x900f54):
/// - The body is the sensor's proxy ([`LevelWorld::sensor_proxy`]), or its parent when
///   the proxy has no shape (0x11b47c); with neither, off. Nothing is stored and there is
///   no end-of-update step (table 0x124b0c entry 18): it is worked out from this tick's
///   contacts ([`LevelWorld::contacts`]).
/// - A contact counts when it carried an impulse (0x90103c, no tolerance), or, with
///   IncludeTouching set, when it only touches (0x9011e8). A contact with the body
///   itself does not count (0x900fb0).
/// - Who counts: with the tag filter (PSwitch+0xc9, read as RequireAll) any object
///   carrying a tag of the sensor's key; otherwise PlayerMode 2 any object and other
///   modes only players' Sackboys, and PlayerMode 0 only the first player's (0x11b6d8,
///   0x900fc0, 0x900fcc).
/// - The signal: activation 1.0 and ternary 1 (0x11873c, threshold 0.25) when one
///   counts, with the player of the first one that is a player's; else off.
/// - No cone and no layer test (D1142): AngleRange (PSwitch+0x7c) is never read, the
///   cone and layer helpers are never called, and the per-contact function 0x900f54
///   loads no floats.
///
/// INFERRED: a tag counts when it is the other Thing or a child of it (the game compares
/// the other Thing's u16 +0x7a with the tag's id); creatures do not count as players.
pub(crate) fn impact_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let shaped = |t: u32| {
        w.things
            .get(t as usize)
            .is_some_and(|t| t.parts.contains(&"SHAPE"))
    };
    let proxy = w.sensor_proxy(cx.thing);
    let body = if shaped(proxy) {
        proxy
    } else {
        match w.things.get(proxy as usize).and_then(|t| t.parent) {
            Some(p) => p,
            None => return SIGNAL_OFF,
        }
    };
    let leader = w.player_records.first().map(|r| r.number);
    let player_of = |t: u32| w.players.iter().find(|p| p.thing == t).map(|p| p.number);
    let tagged = |t: u32| {
        let has = |t: u32| w.things.get(t as usize).and_then(|t| t.switch_key) == Some(s.key);
        has(t)
            || w.things
                .iter()
                .any(|c| c.parent == Some(t) && c.switch_key == Some(s.key))
    };
    let mut hit: Option<i32> = None;
    for c in &w.contacts {
        let other = match (c.a == body, c.b == body) {
            (true, false) => c.b,
            (false, true) => c.a,
            _ => continue,
        };
        if !c.force && s.include_touching == 0 {
            continue;
        }
        let player = player_of(other);
        let passes = if s.require_all {
            tagged(other)
        } else {
            match s.player_mode {
                0 => player.is_some() && player == leader,
                2 => true,
                _ => player.is_some(),
            }
        };
        if !passes {
            continue;
        }
        let player = player.unwrap_or(-1);
        hit = Some(match hit {
            Some(p) if p != -1 => p,
            _ => player,
        });
    }
    match hit {
        Some(player) => SwitchInput {
            activation: 1.,
            ternary: 1,
            player,
        },
        None => SIGNAL_OFF,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contact, SwitchOutputState, TagKey, ThingState};

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    fn at(x: f32, y: f32) -> Option<[f32; 16]> {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
        (m[12], m[13]) = (x, y);
        Some(m)
    }

    /// A switch of type `ty` at (x, y) with one output that stores "off" and feeds the
    /// given (Thing, port) inputs.
    fn switch(ty: i32, x: f32, y: f32, targets: &[(u32, i32)]) -> ThingState {
        ThingState {
            world_matrix: at(x, y),
            switch: Some(SwitchState {
                switch_type: ty,
                outputs: vec![Some(SwitchOutputState {
                    signal: SIGNAL_OFF,
                    stored: SIGNAL_OFF,
                    targets: targets.iter().map(|&(t, p)| (Some(t), p)).collect(),
                    random_on: false,
                })],
                radius: 100.,
                angle_range: 180.,
                bullets_required: 1,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    const KEY: TagKey = TagKey { colour: 2, name: 0 };

    /// A tag sensor at the origin (radius 100, one tag needed) looking for [`KEY`].
    fn tag_sensor_thing() -> ThingState {
        let mut t = switch(4, 0., 0., &[]);
        t.switch.as_mut().unwrap().key = KEY;
        t
    }

    fn tag(x: f32, y: f32) -> ThingState {
        ThingState {
            world_matrix: at(x, y),
            switch_key: Some(KEY),
            ..Default::default()
        }
    }

    /// A battery holding `value`, wired to the given inputs.
    fn battery(value: SwitchInput, targets: &[(u32, i32)]) -> ThingState {
        let mut t = switch(28, 0., 0., targets);
        t.switch.as_mut().unwrap().memory.manual = value;
        t
    }

    fn world(things: Vec<ThingState>) -> LevelWorld {
        let mut w = LevelWorld {
            things,
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn state(w: &mut LevelWorld, thing: usize) -> &mut SwitchState {
        w.things[thing].switch.as_mut().unwrap()
    }

    /// One pass, then output 0 of Thing `thing`.
    fn pass(w: &mut LevelWorld, thing: usize) -> SwitchInput {
        w.update_switches();
        w.things[thing].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    fn move_to(w: &mut LevelWorld, thing: usize, x: f32, y: f32) {
        w.things[thing].world_matrix = at(x, y);
    }

    #[test]
    fn tag_sensor_range_edges_and_the_hold_ring() {
        let mut w = world(vec![tag_sensor_thing(), tag(0., 0.)]);
        // The sensor's own Thing is its proxy here; the tag is another Thing.
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // On up to 0.85 of the radius, y counts as x does.
        move_to(&mut w, 1, 0., 85.);
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.2).abs() < 1e-5 && s.ternary == 1);
        // Between 0.85 and 0.95 the ternary holds: on while it was on.
        move_to(&mut w, 1, 90., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // Off from 0.95, then the ring holds "off" on the way back in.
        move_to(&mut w, 1, 96., 0.);
        let s = pass(&mut w, 0);
        assert!(s.activation > 0. && s.ternary == 0);
        move_to(&mut w, 1, 90., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 0);
        move_to(&mut w, 1, 84., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // The edge itself is in range (activation 0); past it there is no candidate.
        move_to(&mut w, 1, 100., 0.);
        assert_eq!(pass(&mut w, 0), sig(0., 0, -1));
        move_to(&mut w, 1, 100.5, 0.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A negative radius has no range: full strength anywhere.
        state(&mut w, 0).radius = -1.;
        move_to(&mut w, 1, 5000., -900.);
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // Another colour or name is another tag.
        w.things[1].switch_key = Some(TagKey { colour: 3, name: 0 });
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.things[1].switch_key = Some(TagKey { colour: 2, name: 1 });
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_min_radius() {
        let mut w = world(vec![tag_sensor_thing(), tag(10., 0.)]);
        state(&mut w, 0).min_radius = 20.;
        // Inside the inner edge: not a candidate.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // On the inner edge: full strength; it falls to 0 at the outer edge.
        move_to(&mut w, 1, 20., 0.);
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        move_to(&mut w, 1, 0., -60.);
        assert_eq!(pass(&mut w, 0), sig(0.5, 1, -1));
        move_to(&mut w, 1, 100., 0.);
        assert_eq!(pass(&mut w, 0), sig(0., 0, -1));
    }

    #[test]
    fn tag_sensor_takes_the_nth_strongest() {
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(-25., 0.),
            tag(40., 0.),
            tag(70., 0.),
        ]);
        let strength = |d: f32| (1. - d / 100.) / 0.75;
        assert_eq!(pass(&mut w, 0).activation, 1.);
        state(&mut w, 0).bullets_required = 2;
        assert!((pass(&mut w, 0).activation - strength(40.)).abs() < 1e-5);
        state(&mut w, 0).bullets_required = 3;
        assert!((pass(&mut w, 0).activation - strength(70.)).abs() < 1e-5);
        // More needed than there are, none needed, or a negative count: off.
        for n in [4, 0, -1] {
            state(&mut w, 0).bullets_required = n;
            assert_eq!(pass(&mut w, 0), SIGNAL_OFF, "{n}");
        }
    }

    #[test]
    fn tag_signal_unwired_wired_and_deactivated() {
        // Thing 0 the sensor, 1 a tag, 2 a battery on the tag's port 0.
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(50., 0.),
            battery(sig(0.5, 1, 3), &[(1, 0)]),
        ]);
        w.things[2].world_matrix = None;
        assert_eq!(w.tag_signal(1), sig(0., 0, -1));
        // The battery powers up in the pass; the sensor (Thing 0) ran before it.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        assert_eq!(w.tag_signal(1), sig(0.5, 1, 3));
        // The tag's player is handed on, the activation is the closeness.
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.5 / 0.75).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 3));
        // A tag that is off is not seen.
        state(&mut w, 2).memory.manual = sig(0.5, 0, 3);
        w.update_switches();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // Nothing wired: on, no player.
        w.things[2].switch = None;
        w.wire_inputs();
        assert_eq!(w.tag_signal(1), sig(1., 1, -1));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // A deactivated tag is off, wired or not.
        w.set_deactivated(1, true);
        assert_eq!(w.tag_signal(1), SIGNAL_OFF);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_signal_mode_hands_on_the_tags_signal() {
        // Tags 1 and 2 fed by batteries 3 and 4. The sensor is first in the pass order,
        // so it sees a battery's value one pass later.
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(10., 0.),
            tag(95., 0.),
            battery(sig(0.25, 1, 5), &[(1, 0)]),
            battery(sig(-0.75, -1, 6), &[(2, 0)]),
        ]);
        (w.things[3].world_matrix, w.things[4].world_matrix) = (None, None);
        state(&mut w, 0).key_sensor_mode = 1;
        w.update_switches();
        // The strongest signal by magnitude, as it is: no closeness, no clamp.
        assert_eq!(pass(&mut w, 0), sig(-0.75, -1, 6));
        state(&mut w, 0).bullets_required = 2;
        assert_eq!(pass(&mut w, 0), sig(0.25, 1, 5));
        // A tag with activation but ternary 0 is kept and not counted: one tag counted
        // is too few for two needed, so its 0 stands.
        state(&mut w, 3).memory.manual = sig(0.25, 0, 5);
        state(&mut w, 0).bullets_required = 1;
        w.update_switches();
        assert_eq!(pass(&mut w, 0), sig(-0.75, -1, 6));
        state(&mut w, 0).bullets_required = 2;
        assert_eq!(pass(&mut w, 0), sig(0.25, 0, 5));
        // A tag outside the circle but inside the x window is still counted, and with
        // enough counted the last counted ternary stands in for the 0.
        move_to(&mut w, 2, 95., 80.);
        state(&mut w, 0).bullets_required = 1;
        assert_eq!(pass(&mut w, 0), sig(0.25, -1, 5));
        // Activation 0 is skipped.
        state(&mut w, 3).memory.manual = sig(0., 1, 5);
        w.update_switches();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_inversion_own_chip_and_team_filter() {
        // Thing 0 a chip whose board is Thing 1; the sensor (2) and a tag (3) sit on the
        // board, so both are placed by the chip.
        let chip = ThingState {
            world_matrix: at(0., 0.),
            circuit_board: Some(1),
            ..Default::default()
        };
        let child = |mut t: ThingState| {
            (t.parent, t.world_matrix) = (Some(1), None);
            t
        };
        let mut w = world(vec![
            chip,
            ThingState::default(),
            child(tag_sensor_thing()),
            child(tag(0., 0.)),
        ]);
        w.board_owners.insert(1, 0);
        assert_eq!(w.sensor_proxy(2), 0);
        // The tag on the sensor's own chip is not seen, unless the flag asks for it.
        assert_eq!(pass(&mut w, 2), SIGNAL_OFF);
        state(&mut w, 2).detect_unspawned_players = true;
        assert_eq!(pass(&mut w, 2), sig(1., 1, -1));
        // Inversion comes after the sensor: on becomes off, and "nothing found" on.
        state(&mut w, 2).inverted = true;
        assert_eq!(pass(&mut w, 2), sig(0., 0, -1));
        state(&mut w, 2).detect_unspawned_players = false;
        assert_eq!(pass(&mut w, 2), sig(1., 1, -1));
        // The team filter outside versus and team levels: only 2 rejects.
        let s = state(&mut w, 2);
        (s.inverted, s.detect_unspawned_players) = (false, true);
        for (filter, on) in [(0, 1), (1, 1), (2, 0), (3, 1), (4, 1), (5, 0)] {
            state(&mut w, 2).team_filter = filter;
            assert_eq!(pass(&mut w, 2).ternary, on, "{filter}");
        }
        // Versus: a tag without a player fails "own" and "others".
        w.pworld.game_mode = 1;
        for (filter, on) in [(0, 1), (1, 0), (2, 0), (3, 1), (4, 1)] {
            state(&mut w, 2).team_filter = filter;
            assert_eq!(pass(&mut w, 2).ternary, on, "{filter}");
        }
    }

    #[test]
    fn tags_are_placed_when_the_pass_starts() {
        let mut w = world(vec![tag_sensor_thing(), tag(50., 0.), tag(-30., 0.)]);
        w.update_switches();
        let xs: Vec<_> = w.tags[&KEY].iter().map(|r| (r.x, r.thing)).collect();
        assert_eq!(xs, [(-30., 2), (50., 1)]);
        // A tag without a position leaves the map; an empty key goes with it.
        (w.things[1].world_matrix, w.things[2].world_matrix) = (None, None);
        w.refresh_tags();
        assert!(w.tags.is_empty());
    }

    /// A world with an impact sensor (Thing 0, with a shape), a box (Thing 1, a shape),
    /// and a tag of [`KEY`] on the box (Thing 2).
    fn impact_world() -> LevelWorld {
        let mut sensor = switch(18, 0., 0., &[]);
        sensor.parts = vec!["SHAPE"];
        let shape = ThingState {
            parts: vec!["SHAPE"],
            ..Default::default()
        };
        let mut t = tag(0., 0.);
        t.parent = Some(1);
        let mut w = world(vec![sensor, shape, t]);
        state(&mut w, 0).key = KEY;
        w
    }

    fn hit(a: u32, b: u32, force: bool) -> Contact {
        Contact {
            a,
            b,
            force,
            impulse: 0.,
        }
    }

    #[test]
    fn impact_sensor_counts_hits_by_player_mode() {
        let mut w = impact_world();
        let p = w.add_player([0., 0., 0.], (0., 0.));
        let sackboy = p;
        // No contacts: off.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // Mode 1 (players only): the box does not count, Sackboy does, either way round.
        state(&mut w, 0).player_mode = 1;
        w.contacts = vec![hit(0, 1, true)];
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.contacts = vec![hit(sackboy, 0, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, 0));
        // Mode 2: any object, and the player of the first player hit.
        state(&mut w, 0).player_mode = 2;
        w.contacts = vec![hit(0, 1, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        w.contacts = vec![hit(0, 1, true), hit(0, sackboy, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, 0));
        // Mode 0: only the first player's Sackboy.
        state(&mut w, 0).player_mode = 0;
        let q = w.add_player([0., 0., 0.], (0., 0.));
        let second = q;
        w.contacts = vec![hit(0, second, true)];
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.contacts = vec![hit(0, sackboy, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, 0));
        // Nothing is stored: the next tick without contacts is off; inverted, on.
        w.contacts.clear();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).inverted = true;
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
    }

    #[test]
    fn impact_sensor_touching_tags_and_body() {
        let mut w = impact_world();
        state(&mut w, 0).player_mode = 2;
        // Touching only counts with IncludeTouching.
        w.contacts = vec![hit(0, 1, false)];
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).include_touching = 1;
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // A contact of the body with itself, or of two other bodies, never counts.
        w.contacts = vec![hit(0, 0, true), hit(1, 2, true)];
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // Tag filter: the box carries a tag of the key; another key does not count.
        state(&mut w, 0).require_all = true;
        w.contacts = vec![hit(0, 1, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        state(&mut w, 0).key = TagKey { colour: 3, name: 0 };
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A sensor Thing with no shape uses its parent's contacts; with no parent, off.
        let mut w = impact_world();
        state(&mut w, 0).player_mode = 2;
        w.things[0].parts.clear();
        w.contacts = vec![hit(1, 2, true)];
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.things[0].parent = Some(1);
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
    }

    #[test]
    fn impact_sensor_has_no_cone_or_layer_test() {
        // D1142: AngleRange is never read and IncludeTouching only adds touching
        // contacts, so a hit from below, on another layer, still counts.
        let mut w = impact_world();
        let s = state(&mut w, 0);
        (s.player_mode, s.angle_range, s.include_touching) = (2, 10., 1);
        move_to(&mut w, 1, 0., -50.);
        slab(&mut w, 0, 0., 10.);
        slab(&mut w, 1, 400., 10.);
        w.contacts = vec![hit(0, 1, true)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        w.contacts = vec![hit(1, 0, false)];
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
    }

    #[test]
    fn player_sensor_range_hold_and_inversion() {
        let mut w = world(vec![switch(3, 0., 0., &[])]);
        // No player in the level: off, and an inverted sensor is on.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).inverted = true;
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        state(&mut w, 0).inverted = false;
        let p = w.add_player([30., 40., 0.], (0., 0.));
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.5 / 0.75).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 0));
        // The hold ring keeps on, then off.
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [97., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        // Inverted: the activation is turned round, and the ring holds the output, read
        // back through the inversion.
        state(&mut w, 0).inverted = true;
        let s = pass(&mut w, 0);
        assert!((s.activation - (1. - 0.1 / 0.75)).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (0, 0));
        w.move_player(p, [97., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [10., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        // Out of range: off before the inversion, so on after it, with no player.
        w.move_player(p, [500., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // MinRadius: a player inside the inner edge is not in range.
        let s = state(&mut w, 0);
        (s.inverted, s.min_radius) = (false, 50.);
        w.move_player(p, [10., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.move_player(p, [75., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(0.5, 1, 0));
    }

    #[test]
    fn player_sensor_counts_players() {
        let mut w = world(vec![switch(3, 0., 0., &[])]);
        state(&mut w, 0).player_mode = 1;
        let a = w.add_player([60., 0., 0.], (0., 0.));
        let b = w.add_player([0., 30., 0.], (0., 0.));
        let strength = |d: f32| (1. - d / 100.) / 0.75;
        // One needed: the nearest gives the strength and the player.
        let s = pass(&mut w, 0);
        assert!((s.activation - strength(30.)).abs() < 1e-5);
        assert_eq!(s.player, 1);
        // Two needed: the strength of the second nearest, the player still the nearest.
        state(&mut w, 0).bullets_required = 2;
        let s = pass(&mut w, 0);
        assert!((s.activation - strength(60.)).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 1));
        state(&mut w, 0).bullets_required = 3;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // RequireAll: every eligible player in range, whatever the count says.
        let s = state(&mut w, 0);
        (s.require_all, s.bullets_required) = (true, 1);
        assert!((pass(&mut w, 0).activation - strength(60.)).abs() < 1e-5);
        w.move_player(a, [300., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).require_all = false;
        assert_eq!(pass(&mut w, 0).player, 1);
        // PlayerMode 0: the leader only (player 0, who is out of range).
        state(&mut w, 0).player_mode = 0;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.move_player(a, [10., 0., 0.], (0., 0.));
        w.move_player(b, [1., 0., 0.], (0., 0.));
        let s = pass(&mut w, 0);
        assert!((s.activation - 1.).abs() < 1e-5 && s.player == 0);
        // A negative radius: any eligible player, full strength.
        state(&mut w, 0).radius = -1.;
        w.move_player(a, [9000., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(1., 1, 0));
    }

    /// Gives Thing `thing` a PShape slab of half depth `depth` (a 20 x 20 outline) at z.
    fn slab(w: &mut LevelWorld, thing: usize, z: f32, depth: f32) {
        let t = &mut w.things[thing];
        t.parts.push("SHAPE");
        t.shape_box = Some([-10., 10., -10., 10., depth]);
        t.world_matrix.as_mut().unwrap()[14] = z;
    }

    #[test]
    fn tag_sensor_layer_test() {
        let mut w = world(vec![tag_sensor_thing(), tag(10., 0.), tag(0., 0.)]);
        w.things[2].switch_key = None;
        slab(&mut w, 0, 0., 50.);
        slab(&mut w, 1, 200., 10.);
        // IncludeTouching 0: every layer counts.
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // Otherwise the slabs must overlap in z.
        state(&mut w, 0).include_touching = 1;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.things[1].world_matrix.as_mut().unwrap()[14] = 40.;
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // Touching faces do not overlap.
        w.things[1].world_matrix.as_mut().unwrap()[14] = 60.;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A tag with no slab and no parent is rejected; one whose parent has no slab is
        // accepted.
        w.things[1].parts.clear();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.things[1].parent = Some(2);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // The parent's slab is the tag's when the tag has none.
        slab(&mut w, 2, 300., 10.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A sensor with no body accepts every layer.
        w.things[0].parts.clear();
        assert_eq!(pass(&mut w, 0).ternary, 1);
    }

    #[test]
    fn player_sensor_layer_test() {
        let mut w = world(vec![switch(3, 0., 0., &[])]);
        slab(&mut w, 0, 0., 50.);
        state(&mut w, 0).include_touching = 1;
        let p = w.add_player([10., 0., 0.], (-10., 10.));
        // The sensor spans layer 0 only (z -50..50); the player's middle is in layer 0.
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [10., 0., 0.], (190., 210.));
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A deeper slab reaches the next layer (z -50..250 rounds to layers 0..1).
        slab(&mut w, 0, 100., 150.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // A slab 20 deep or thinner sees no player.
        slab(&mut w, 0, 200., 10.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).include_touching = 0;
        assert_eq!(pass(&mut w, 0).ternary, 1);
    }

    /// Turns Thing `thing`'s world and local matrices `deg` degrees counter-clockwise in
    /// the plane, keeping its position.
    fn turn(w: &mut LevelWorld, thing: usize, deg: f32) {
        let (sin, cos) = deg.to_radians().sin_cos();
        let t = &mut w.things[thing];
        for m in [&mut t.world_matrix, &mut t.local_matrix] {
            let m = m.get_or_insert_with(|| at(0., 0.).unwrap());
            (m[0], m[1], m[4], m[5]) = (cos, sin, -sin, cos);
        }
    }

    #[test]
    fn tag_sensor_cone_points_along_local_up() {
        let mut w = world(vec![tag_sensor_thing(), tag(0., 50.)]);
        state(&mut w, 0).angle_range = 45.;
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // 37 degrees off up is inside a 45 degree half angle; 90 is not.
        move_to(&mut w, 1, 30., 40.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        move_to(&mut w, 1, 50., 0.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // Turned 90 degrees counter-clockwise, the cone points along -x.
        move_to(&mut w, 1, -50., 0.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        turn(&mut w, 0, 90.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // AngleRange 180 sees every direction.
        state(&mut w, 0).angle_range = 180.;
        move_to(&mut w, 1, 50., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
    }

    #[test]
    fn cone_direction_on_a_chip_and_tilted() {
        // Thing 0 a chip turned 90 degrees, board 1, sensor 2 on the board turned 90 more.
        let chip = ThingState {
            world_matrix: at(0., 0.),
            circuit_board: Some(1),
            ..Default::default()
        };
        let mut sensor = tag_sensor_thing();
        sensor.parent = Some(1);
        let board = ThingState {
            parent: Some(0),
            ..Default::default()
        };
        let mut w = world(vec![chip, board, sensor]);
        w.board_owners.insert(1, 0);
        turn(&mut w, 0, 90.);
        let close = |d: Option<[f32; 2]>, e: [f32; 2]| {
            let d = d.unwrap();
            (d[0] - e[0]).abs() < 1e-5 && (d[1] - e[1]).abs() < 1e-5
        };
        // The chip's up is -x; the sensor's own rotation on the board turns it on to -y.
        assert!(close(w.cone_direction(2), [-1., 0.]));
        turn(&mut w, 2, 90.);
        assert!(close(w.cone_direction(2), [0., -1.]));
        // A chip leaning back about x still points its cone along its flattened +y.
        let m = w.things[0].world_matrix.as_mut().unwrap();
        let (sin, cos) = 30f32.to_radians().sin_cos();
        *m = at(0., 0.).unwrap();
        (m[5], m[6], m[9], m[10]) = (cos, sin, -sin, cos);
        assert!(close(w.cone_direction(0), [0., 1.]));
    }

    #[test]
    fn cone_rotation_reads_the_chip_table_without_a_ppos() {
        // D1143: chip 0 turned 90 degrees, board 1, sensor 2 on it with no local matrix.
        let chip = ThingState {
            world_matrix: at(0., 0.),
            circuit_board: Some(1),
            ..Default::default()
        };
        let mut sensor = tag_sensor_thing();
        sensor.parent = Some(1);
        let board = ThingState {
            parent: Some(0),
            ..Default::default()
        };
        let mut w = world(vec![chip, board, sensor]);
        w.board_owners.insert(1, 0);
        turn(&mut w, 0, 90.);
        w.things[2].local_matrix = None;
        let close = |d: Option<[f32; 2]>, e: [f32; 2]| {
            let d = d.unwrap();
            (d[0] - e[0]).abs() < 1e-5 && (d[1] - e[1]).abs() < 1e-5
        };
        // Not in the table: adds nothing, the chip's up.
        assert!(close(w.cone_direction(2), [-1., 0.]));
        let entry = |angle, flipped| crate::ChipAngle {
            thing: 2,
            angle,
            flipped,
        };
        let quarter = std::f32::consts::FRAC_PI_2;
        w.things[0].chip_components = vec![entry(quarter, false)];
        assert!(close(w.cone_direction(2), [0., 1.]));
        // Flipped: the sum is negated after the angle is added.
        w.things[0].chip_components = vec![entry(quarter, true)];
        assert!(close(w.cone_direction(2), [0., -1.]));
        // A Thing with a local matrix ignores its table entry.
        turn(&mut w, 2, 0.);
        assert!(close(w.cone_direction(2), [-1., 0.]));
    }
}
