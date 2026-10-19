//! D383 (part 130): the Pod's `LevelUnlockingThread` (FOUND, script ec688550) and its
//! `PendingLinkCluster`s (FOUND, script aff692cc), run natively as the planets world's
//! state 2 (`PlanetsWorld.UpdateState__`), one tick per yield. Read in part 127 (D380).
//!
//! D384 (part 131): with scripts loaded, the game's own thread runs in the VM instead;
//! [`LevelWorld::unlock_query`] answers its natives.
//!
//! D399 (part 146): a finished cluster asks for its voice-over (`PlayVO__`).
//!
//! D1014 (part 161): at the end, a group under the cursor has its search pushed (by the
//! native pod computer, [`LevelWorld::unlock_group_push`]) and the cursor moves to the
//! group's default level.
//!
//! Not ported: `CanStartEndOfLevelSequence__` (always
//! true here), the swap from a group search back to Story (the return opens Story), the
//! cached-info updates, the link drawing and the state-2 camera. No audio plays; the
//! sounds are counted.

use crate::{LevelWorld, PendingLink};
use lbp_script_vm::{ObjectId, Value};

/// `PendingLinkCluster`'s angle test: a link without a group joins when its target's
/// direction is within 30 degrees of the cluster's `AverageDir` (FOUND, 0.5235988).
pub const CLUSTER_COS: f32 = 0.866_025_4;
/// State 2: link j starts on tick `LINK_DELAY * (j + 1)` (FOUND).
pub const LINK_DELAY: i32 = 10;
/// State 2: `gameplay/unlocking/unlock_beeps` on ticks 5, 20, 35, ... (FOUND).
pub const BEEP_FIRST: i32 = 5;
pub const BEEP_EVERY: i32 = 15;
/// D385: `PendingLink.Update__` (FOUND, the 01.33 listing): `Time` grows by
/// `LINK_TIME_STEP` a tick up to `LINK_TIME_MAX * (pi^2 - (pi - a)^2) / pi^2`, `a` the
/// link's angular separation, and `Progress` is `SmoothStep(0, that, Time)`.
pub const LINK_TIME_STEP: f32 = 2.;
pub const LINK_TIME_MAX: f32 = 120.;

/// D385: the end of a link's `Time` for its angular separation `a` (radians).
pub fn link_time_end(a: f32) -> f32 {
    use std::f32::consts::PI;
    LINK_TIME_MAX * (PI * PI - (PI - a) * (PI - a)) / (PI * PI)
}

/// `Math.SmoothStep__fff` (an equal pair of edges is a step).
fn smooth_step(e0: f32, e1: f32, x: f32) -> f32 {
    if e0 == e1 {
        return if x < e0 { 0. } else { 1. };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
/// State 3's hold before the cursor moves to the last target (FOUND).
pub const HOLD_TICKS: i32 = 30;

pub const UNLOCK_BEEPS: &str = "gameplay/unlocking/unlock_beeps";
pub const UNLOCK_LEVEL: &str = "gameplay/unlocking/unlock_level";
pub const UNLOCK_THEME: &str = "gameplay/unlocking/unlock_theme";

/// One `PendingLinkCluster`: links from one badge, to one group or one direction.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingLinkCluster {
    pub from: (i32, i32),
    /// The first target's group.
    pub group: Option<(i32, i32)>,
    pub links: Vec<PendingLink>,
    /// `AverageDir`: the normalised centre of the min/max box of the targets'
    /// directions.
    pub average_dir: Option<[f32; 3]>,
    pub state: u8,
    pub ticks: i32,
    pub progress: Vec<f32>,
    /// Each link's `PendingLink.Time` (D385).
    pub times: Vec<f32>,
    /// State 2 made the group the results' default (one tick before the unlock).
    pub defaulted: bool,
}

/// The thread's state between ticks ([`LevelWorld::level_unlocking_tick`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelUnlocking {
    pub started: bool,
    pub clusters: Vec<PendingLinkCluster>,
    pub current: usize,
    /// D1014: the group whose search the end asked for, once asked.
    pub group: Option<(i32, i32)>,
}

fn unit(l: [f32; 4]) -> Option<[f32; 3]> {
    let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
    (n > 0.).then(|| [l[0] / n, l[1] / n, l[2] / n])
}

impl LevelWorld {
    fn slot_index(&self, id: (i32, i32)) -> Option<usize> {
        self.story_slots.iter().position(|s| s.id == id)
    }

    fn slot_dir(&self, id: (i32, i32)) -> Option<[f32; 3]> {
        self.slot_index(id)
            .and_then(|i| unit(self.story_slots[i].location))
    }

    fn slot_group(&self, id: (i32, i32)) -> Option<(i32, i32)> {
        self.slot_index(id).and_then(|i| self.story_slots[i].group)
    }

    /// The badge a slot shows as in the Story search: its group's when it has one in the
    /// list (INFERRED: the Story search shows a group's levels as the group).
    fn badge_result(&self, id: (i32, i32)) -> Option<usize> {
        let i = self.slot_index(id)?;
        let group = self.story_slots[i].group;
        Some(group.and_then(|g| self.slot_index(g)).unwrap_or(i))
    }

    fn average_dir(&self, links: &[PendingLink]) -> Option<[f32; 3]> {
        let dirs: Vec<[f32; 3]> = links.iter().filter_map(|l| self.slot_dir(l.to)).collect();
        let first = *dirs.first()?;
        let (mut lo, mut hi) = (first, first);
        for d in &dirs {
            for k in 0..3 {
                lo[k] = lo[k].min(d[k]);
                hi[k] = hi[k].max(d[k]);
            }
        }
        unit([
            (lo[0] + hi[0]) / 2.,
            (lo[1] + hi[1]) / 2.,
            (lo[2] + hi[2]) / 2.,
            0.,
        ])
    }

    /// `PendingLinkCluster`'s accept test for `link`.
    fn cluster_accepts(&self, c: &PendingLinkCluster, link: PendingLink) -> bool {
        if c.from != link.from {
            return false;
        }
        if c.group.is_some() {
            return self.slot_group(link.to) == c.group;
        }
        match (c.average_dir, self.slot_dir(link.to)) {
            (Some(a), Some(d)) => a[0] * d[0] + a[1] * d[1] + a[2] * d[2] > CLUSTER_COS,
            _ => false,
        }
    }

    /// The thread's grouping: each pending link in the search (its target is a story
    /// slot) joins the first cluster that accepts it, else starts one.
    pub fn pending_link_clusters(&self) -> Vec<PendingLinkCluster> {
        let mut clusters: Vec<PendingLinkCluster> = Vec::new();
        for &link in &self.story_progress.pending_links {
            if self.slot_index(link.to).is_none() {
                continue;
            }
            if let Some(k) = (0..clusters.len()).find(|&k| self.cluster_accepts(&clusters[k], link))
            {
                clusters[k].links.push(link);
                clusters[k].progress.push(0.);
                clusters[k].times.push(0.);
                clusters[k].average_dir = self.average_dir(&clusters[k].links);
                continue;
            }
            clusters.push(PendingLinkCluster {
                from: link.from,
                group: self.slot_group(link.to),
                links: vec![link],
                average_dir: self.average_dir(&[link]),
                state: 0,
                ticks: 0,
                progress: vec![0.],
                times: vec![0.],
                defaulted: false,
            });
        }
        clusters
    }

    /// D385: `PodComputer.GetLinkAngularSeparation__Q8SlotLink` (FOUND 0x42c870: a
    /// function, INFERRED acos, of the dot of the two badges' directions; 0 without
    /// them).
    pub fn link_separation(&self, l: PendingLink) -> f32 {
        match self.link_dirs(l) {
            Some((a, b)) => (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .clamp(-1., 1.)
                .acos(),
            None => 0.,
        }
    }

    /// D390 (FOUND 0x6ff5d8: the link record's byte +0x4c, 1 at 0x6ff700): a link is
    /// drawn wide when both its slots resolve and each slot's `DeveloperLevelType` is 0, 5,
    /// 7 or 8 (INFERRED names: main path, boss, cut scene, DLC hub).
    pub fn link_is_wide(&self, l: PendingLink) -> bool {
        [l.from, l.to].iter().all(|&id| {
            self.slot_index(id).is_some_and(|i| {
                matches!(
                    self.story_slots[i].developer_level_type.unwrap_or(0),
                    0 | 5 | 7 | 8
                )
            })
        })
    }

    /// D391 (FOUND 0x426680-0x4268bc, the test of 0x420890): a link between two group
    /// slots, each of `SlotID` type 3 or 4 (INFERRED: developer and local groups).
    pub fn link_joins_groups(l: PendingLink) -> bool {
        [l.from, l.to].iter().all(|id| matches!(id.0, 3 | 4))
    }

    /// D386: the directions of a link's two ends (its slots' locations), for drawing.
    pub fn link_dirs(&self, l: PendingLink) -> Option<([f32; 3], [f32; 3])> {
        Some((self.slot_dir(l.from)?, self.slot_dir(l.to)?))
    }

    /// D385: `PodComputer.SetLinkProgress__Q8SlotLinkf`, kept for the link's drawing
    /// (FOUND 0x42c7c4: clamped to 0..1 in the planets' link record).
    pub fn set_link_progress(&mut self, l: PendingLink, progress: f32) {
        let progress = progress.clamp(0., 1.);
        match self.link_progress.iter_mut().find(|(x, _)| *x == l) {
            Some((_, p)) => *p = progress,
            None => self.link_progress.push((l, progress)),
        }
    }

    fn count_sound(&mut self, name: &str) {
        *self.sounds_played.entry(name.to_owned()).or_default() += 1;
    }

    fn cursor_to_badge(&mut self, id: (i32, i32), snap: bool) {
        let Some(i) = self.badge_result(id) else {
            return;
        };
        let to = self.story_slots[i].location;
        let from = (self.pod.search.cur_result)
            .and_then(|c| self.story_slots.get(c))
            .map(|s| s.location);
        self.pod.search.move_cursor_to(i, Some(to), from, snap);
    }

    /// The cursor flies to a direction, keeping its result (state 1's move to
    /// `AverageDir`).
    fn cursor_to_dir(&mut self, target: [f32; 3]) {
        let search = &mut self.pod.search;
        match search.cursor_dir {
            Some(start) => {
                search.cursor_flight = Some(crate::CursorFlight {
                    start,
                    target,
                    progress: 0.,
                });
                search.cursor_step = 0.;
            }
            None => search.cursor_dir = Some(target),
        }
    }

    /// One tick of the cluster `k`'s `Update__`; whether it has finished (state 4).
    fn cluster_update(&mut self, u: &mut LevelUnlocking, k: usize) -> bool {
        let c = &mut u.clusters[k];
        match c.state {
            0 => {
                for l in &c.links {
                    if !self.story_progress.unlocking.contains(&l.to) {
                        self.story_progress.unlocking.push(l.to);
                    }
                }
                let from = c.from;
                c.state = 1;
                self.cursor_to_badge(from, false);
                self.unlock_events
                    .push(format!("cluster {k}: cursor to the source {from:?}"));
            }
            1 if self.pod.search.cursor_flight.is_none() => {
                c.state = 2;
                c.ticks = 0;
                if let Some(dir) = c.average_dir {
                    self.cursor_to_dir(dir);
                }
                self.unlock_events
                    .push(format!("cluster {k}: cursor to the links' direction"));
            }
            2 => {
                let last = c.progress.len() - 1;
                if c.progress[last] < 1. {
                    // FOUND: the links below `Tick / 10` update, then `Tick` grows.
                    let live = usize::try_from(c.ticks / LINK_DELAY).unwrap_or(0);
                    for j in 0..c.links.len().min(live) {
                        let a = self.link_separation(c.links[j]);
                        let end = link_time_end(a);
                        c.times[j] = (c.times[j] + LINK_TIME_STEP).min(end);
                        c.progress[j] = smooth_step(0., end, c.times[j]);
                        self.set_link_progress(c.links[j], c.progress[j]);
                    }
                    c.ticks += 1;
                    let t = c.ticks;
                    if t >= BEEP_FIRST && (t - BEEP_FIRST) % BEEP_EVERY == 0 {
                        self.count_sound(UNLOCK_BEEPS);
                    }
                } else if !c.defaulted {
                    c.defaulted = true;
                    let group = c.group.and_then(|g| self.slot_index(g));
                    if group.is_some() {
                        self.pod.search.default_result = group;
                    }
                } else {
                    let (links, group) = (c.links.clone(), c.group);
                    c.state = 3;
                    c.ticks = 0;
                    for l in links {
                        let same = group.is_some() && self.slot_group(l.to) == group;
                        self.count_sound(if same { UNLOCK_LEVEL } else { UNLOCK_THEME });
                        // `Game.ClearPendingLink` (D1011: it opens the target) and
                        // `UnlockBadge`.
                        let p = &mut self.story_progress;
                        p.clear_pending_link(l);
                        p.unlocking.retain(|x| *x != l.to);
                        self.unlock_events
                            .push(format!("cluster {k}: {:?} unlocked", l.to));
                    }
                }
            }
            3 => {
                c.ticks += 1;
                if c.ticks >= HOLD_TICKS {
                    c.state = 4;
                    let to = c.links.last().map(|l| l.to);
                    if let Some(to) = to {
                        self.cursor_to_badge(to, false);
                    }
                    self.unlock_events
                        .push(format!("cluster {k}: cursor to the last target"));
                }
            }
            4 => return true,
            _ => {}
        }
        false
    }

    /// One tick of `LevelUnlockingThread`; whether it is still running. It waits while
    /// the Story search the return asked for ([`Self::story_search_requested`]) is not up
    /// yet, and ends at once with no search up (FOUND: it returns unless the pod
    /// computer's state is a search).
    pub fn level_unlocking_tick(&mut self) -> bool {
        let mut u = self.level_unlocking.take().unwrap_or_default();
        let running = self.level_unlocking_step(&mut u);
        if running {
            self.level_unlocking = Some(u);
        }
        running
    }

    fn level_unlocking_step(&mut self, u: &mut LevelUnlocking) -> bool {
        if !u.started {
            if self.pod.search.selected_planet.is_none() {
                return self.story_search_requested;
            }
            self.story_search_requested = false;
            u.started = true;
            self.pod.search.cursor_mode = Some(2);
            u.clusters = self.pending_link_clusters();
            self.unlock_events.push(format!(
                "{} pending link(s) in {} cluster(s)",
                self.story_progress.pending_links.len(),
                u.clusters.len()
            ));
            if !u.clusters.is_empty() {
                return true;
            }
        }
        if let Some(group) = u.group {
            return self.unlock_group_wait(group);
        }
        while u.current < u.clusters.len() {
            if !self.cluster_update(u, u.current) {
                return true;
            }
            self.play_unlock_voice_over(&u.clusters[u.current].links);
            u.current += 1;
        }
        self.pod.search.cursor_mode = Some(4);
        // D1014 (FOUND, script ec688550 `Run__` 0199-0241): a focus planet whose cursor
        // result is a slot (`GetType__` 1) of a group (`SlotID.Type` 4) has
        // `SearchDetails(CreateGroup__(slot, false))` pushed with the cursor mode 2.
        let group = (self.pod.search.selected_planet)
            .and_then(|_| self.selected_result())
            .and_then(|i| self.story_slots.get(i))
            .map(|s| s.id)
            .filter(|id| id.0 == 4);
        let Some(group) = group else {
            return false;
        };
        self.unlock_events.push(format!("push the group {group:?}"));
        self.pod.search.cursor_mode = Some(2);
        self.unlock_group_push = Some(group);
        u.group = Some(group);
        true
    }

    /// D1014 (FOUND, `Run__` 0242-0278): the thread yields while the view blends
    /// (`ViewTransition` < 1), the search is busy or its decorations load, then moves the
    /// cursor (no snap) to the focus planet's default result (`PickDefaultResult__`) and
    /// sets the cursor mode 4. INFERRED: the pushed group is ready the tick after its
    /// push, as the native search answers at once.
    fn unlock_group_wait(&mut self, group: (i32, i32)) -> bool {
        if self.unlock_group_push.is_some() {
            return true;
        }
        if self.pod.search.group == Some(group) {
            if let Some(i) = self.group_default_result(group) {
                let to = self.story_slots[i].location;
                let from = (self.pod.search.cur_result)
                    .and_then(|c| self.story_slots.get(c))
                    .map(|s| s.location);
                self.pod.search.move_cursor_to(i, Some(to), from, false);
            }
        }
        self.pod.search.cursor_mode = Some(4);
        false
    }
}

/// D399: `PendingLinkCluster.PlayVO__`'s line for a finished cluster's links (FOUND,
/// script aff692cc): the first link to a level (`Type` 0) whose `SlotNumber` has a line
/// decides; other levels are passed over. The second name is the one a profile that
/// played the first game and unlocked its Moon hears.
pub const UNLOCK_VOICE_OVERS: [(i32, &str, Option<&str>); 5] = [
    (124819, "pod_unlock_level_2", Some("pod_unlock_level_2_alt")),
    (124808, "pod_unlock_level_3", None),
    (124821, "pod_unlock_level_4", None),
    (124814, "pod_unlock_level_5", None),
    (
        127134,
        "pod_complete_theme_1",
        Some("pod_complete_theme_1_alt"),
    ),
];

impl LevelWorld {
    /// D399: `PlayVO__`. The four level lines go through the story state's `StartVO__`
    /// and the theme line straight to `Audio.RequestVoiceOver`; both ask with interrupt
    /// set and replay clear (FOUND, script 110fc2e8 for `StartVO__`). D1012: `StartVO__`
    /// keeps whether its line started (`_StartedVO`), which the state's close reads.
    fn play_unlock_voice_over(&mut self, links: &[PendingLink]) {
        let line = links.iter().find_map(|l| {
            (l.to.0 == 0)
                .then(|| UNLOCK_VOICE_OVERS.iter().find(|v| v.0 == l.to.1))
                .flatten()
        });
        let Some(&(_, name, alt)) = line else {
            return;
        };
        let moon = matches!(
            lbp_script_vm::natives::World::state_query(
                self,
                "Profile.HasPlayedLBP1AndUnlockedMoon__"
            ),
            Ok(Value::Bool(true))
        );
        let name = alt.filter(|_| moon).unwrap_or(name);
        let args = [
            Value::Str(name.into()),
            Value::Bool(true),
            Value::Bool(false),
        ];
        self.media_call("RequestVoiceOver", None, &args);
        if !name.starts_with("pod_complete_theme") {
            self.pod.search.story_vo_started = self.media.last_started;
        }
        self.unlock_events.push(format!("voice-over {name}"));
    }
}

impl LevelWorld {
    /// D384: the slot a SlotID object the world made stands for: a Story result's, or
    /// one made for a pending link's end outside the search (`link_slot_ids`).
    pub(crate) fn slot_of(&self, v: &Value) -> Option<(i32, i32)> {
        let Value::Object(Some(o)) = v else {
            return None;
        };
        let result = self.story_results.iter().position(|r| r.slot_id == *o);
        result
            .and_then(|i| self.story_slots.get(i))
            .map(|s| s.id)
            .or_else(|| {
                self.link_slot_ids
                    .iter()
                    .find(|(x, _)| x == o)
                    .map(|(_, id)| *id)
            })
    }

    /// D384: `Slot.GetSlotGroup__Q6SlotID` for a Story slot: its group's SlotID, or the
    /// (0, 0) SlotID for a slot without one. `None` for a slot not in the search.
    pub(crate) fn story_slot_group(&self, v: &Value) -> Option<Option<ObjectId>> {
        let id = self.slot_of(v)?;
        self.slot_index(id)?;
        let group = self.slot_group(id).and_then(|g| self.slot_index(g));
        Some(
            group
                .map(|i| self.story_results[i].slot_id)
                .or(self.no_group_slot),
        )
    }

    fn link_of(&self, v: Option<&Value>) -> Option<PendingLink> {
        let Some(Value::Object(Some(o))) = v else {
            return None;
        };
        let found = self.pending_link_objects.iter().find(|(_, x)| x == o);
        found.map(|(l, _)| *l)
    }

    /// D384 (part 131): the natives the scripted `LevelUnlockingThread` and its
    /// `PendingLinkCluster`s call, so the game's own thread plays state 2 (the native
    /// thread, D383, is the port without scripts). The names and types are FOUND (the
    /// 01.33 listings); the answers reuse the D383 thread's rules and are INFERRED.
    /// `None` for any other native.
    pub(crate) fn unlock_query(
        &mut self,
        class: &str,
        mangled: &str,
        args: &[Value],
    ) -> Option<Value> {
        let arg = args.first();
        let slot = arg.and_then(|v| self.slot_of(v));
        let snap = matches!(args.get(1), Some(Value::Bool(true)));
        Some(match (class, mangled) {
            // The SlotLink objects made with the Story results, in the profile's order.
            ("Game", "GetPendingLink__i") => {
                let i = match arg {
                    Some(Value::Int(i)) => usize::try_from(*i).ok(),
                    _ => None,
                };
                let link = i.and_then(|i| self.story_progress.pending_links.get(i));
                let o = link.and_then(|l| self.pending_link_objects.iter().find(|(x, _)| x == l));
                Value::Object(o.map(|(_, o)| *o))
            }
            ("Game", "ClearPendingLink__Q8SlotLink") => {
                if let Some(l) = self.link_of(arg) {
                    self.story_progress.clear_pending_link(l);
                    self.unlock_events
                        .push(format!("link to {:?} cleared", l.to));
                }
                Value::Void
            }
            ("PodComputer", "GetLinkAngularSeparation__Q8SlotLink") => {
                let a = self.link_of(arg).map(|l| self.link_separation(l));
                Value::Float(a.unwrap_or(0.))
            }
            ("PodComputer", "SetLinkProgress__Q8SlotLinkf") => {
                if let (Some(l), Some(Value::Float(f))) = (self.link_of(arg), args.get(1)) {
                    self.set_link_progress(l, *f);
                }
                Value::Void
            }
            // D383: no end-of-level sequence waits.
            ("PlanetsWorld", "CanStartEndOfLevelSequence__") => Value::Bool(true),
            // D383's rule: the target is a Story slot and the search is up.
            ("PodComputer", "IsLinkInCurrentSearch__Q8SlotLink") => {
                let up = self.pod.search.selected_planet.is_some();
                if up {
                    self.story_search_requested = false;
                }
                let link = self.link_of(arg);
                Value::Bool(up && link.is_some_and(|l| self.slot_index(l.to).is_some()))
            }
            ("PodComputer", "GetBadgeDir__Q6SlotID") => {
                let d = slot.and_then(|id| self.slot_dir(id)).unwrap_or([0.; 3]);
                Value::Vec4([d[0], d[1], d[2], 0.])
            }
            ("PodComputer", "SetBadgeUnlocking__Q6SlotID") => {
                let new = slot.filter(|id| !self.story_progress.unlocking.contains(id));
                if let Some(id) = new {
                    self.story_progress.unlocking.push(id);
                    self.unlock_events.push(format!("{id:?} unlocking"));
                }
                Value::Void
            }
            // D1011 (FOUND, 0x63d4d0 -> 0x5037dc): the game finds the badge by the slot
            // and calls a method of its object; the lock is left to `ClearPendingLink`.
            ("PodComputer", "UnlockBadge__Q6SlotID") => {
                if let Some(id) = slot {
                    self.story_progress.unlocking.retain(|x| *x != id);
                    self.unlock_events.push(format!("{id:?} unlocked"));
                }
                Value::Void
            }
            // The bool is the snap, as on the planet's `MoveCursorToResult` (D368).
            ("PodComputer", "MoveCursorToPos__rb") => {
                let dir = match arg {
                    Some(Value::Vec4(v)) => unit(*v),
                    _ => None,
                };
                if let Some(d) = dir {
                    if snap {
                        self.pod.search.cursor_flight = None;
                        self.pod.search.cursor_dir = Some(d);
                    } else {
                        self.cursor_to_dir(d);
                    }
                    self.unlock_events.push("cursor to a direction".into());
                }
                Value::Void
            }
            ("PodComputer", "MoveCursorToResult__Q6SlotIDb") => {
                if let Some(id) = slot {
                    self.cursor_to_badge(id, snap);
                    self.unlock_events.push(format!("cursor to {id:?}"));
                }
                Value::Void
            }
            // No cached search details are kept.
            ("SearchDetails", "UpdateAllSearchDetailsCachedInfo__") => Value::Void,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SearchSlot;

    fn world() -> LevelWorld {
        let slot = |n: i32, location: [f32; 4], group: Option<(i32, i32)>| SearchSlot {
            id: (0, n),
            group,
            location,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![
                slot(1, [1., 0., 0., 0.], None),
                slot(2, [0., 1., 0., 0.], None),
                slot(3, [0., 0.99, 0.1, 0.], None),
                slot(4, [0., 0., 1., 0.], None),
            ],
            ..Default::default()
        };
        let link = |from, to| PendingLink {
            from: (0, from),
            to: (0, to),
        };
        w.story_progress.pending_links = vec![link(1, 2), link(1, 3), link(1, 4), link(2, 4)];
        w
    }

    #[test]
    fn d383_clusters() {
        let c = world().pending_link_clusters();
        let to: Vec<Vec<i32>> = c
            .iter()
            .map(|c| c.links.iter().map(|l| l.to.1).collect())
            .collect();
        // 2 and 3 are within 30 degrees, 4 is not; the link from 2 is another source.
        assert_eq!(to, [vec![2, 3], vec![4], vec![4]]);
    }

    #[test]
    fn d383_waits_for_the_search_then_unlocks() {
        let mut w = world();
        assert!(
            !w.level_unlocking_tick(),
            "no search up and none asked for: it ends"
        );
        w.story_search_requested = true;
        assert!(w.level_unlocking_tick(), "waits for the asked-for search");
        w.pod.search.selected_planet = Some(1);
        let mut ticks = 1;
        while w.level_unlocking_tick() {
            w.cursor_flight_step();
            ticks += 1;
            assert!(ticks < 2000);
        }
        assert!(w.story_progress.pending_links.is_empty());
        assert_eq!(w.story_progress.unlocked, [(0, 2), (0, 3), (0, 4)]);
        assert!(w.story_progress.unlocking.is_empty());
        assert_eq!(w.pod.search.cursor_mode, Some(4));
        assert_eq!(w.sounds_played.get(UNLOCK_THEME), Some(&4));
        assert!(w.sounds_played.get(UNLOCK_BEEPS).is_some_and(|&n| n >= 3));
    }

    #[test]
    fn d399_a_finished_cluster_asks_for_its_voice_over() {
        let slot = |id: (i32, i32), location: [f32; 4]| SearchSlot {
            id,
            location,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![
                slot((0, 1), [1., 0., 0., 0.]),
                slot((0, 2), [0., 1., 0., 0.]),
                slot((0, 124808), [0., 0.99, 0.1, 0.]),
                slot((0, 124819), [0., 0.98, 0.15, 0.]),
                slot((1, 124814), [0., 0., 1., 0.]),
            ],
            ..Default::default()
        };
        let link = |to| PendingLink { from: (0, 1), to };
        w.story_progress.pending_links = vec![
            link((0, 2)),
            link((0, 124808)),
            link((0, 124819)),
            link((1, 124814)),
        ];
        for (_, name, _) in UNLOCK_VOICE_OVERS {
            w.media.voice_over_lengths.insert(name.into(), 60);
        }
        w.pod.search.selected_planet = Some(1);
        let mut ticks = 0;
        while w.level_unlocking_tick() {
            w.cursor_flight_step();
            w.media.tick();
            ticks += 1;
            assert!(ticks < 4000);
        }
        let asked: Vec<_> = w.media.voice_overs.iter().map(|v| v.name.clone()).collect();
        // The first level link with a line decides; a non-level slot asks for none.
        assert_eq!(asked, [Some("pod_unlock_level_3".to_string())]);
        assert_eq!(w.media.voice_overs[0].flags, [true, false]);
        assert!(w.media.voice_overs_heard.contains("pod_unlock_level_3"));
    }

    #[test]
    fn d390_wide_links_join_main_path_slots() {
        let mut w = world();
        let link = |from, to| PendingLink {
            from: (0, from),
            to: (0, to),
        };
        w.story_slots[1].developer_level_type = Some(5);
        w.story_slots[2].developer_level_type = Some(1);
        assert!(w.link_is_wide(link(1, 2)));
        assert!(!w.link_is_wide(link(1, 3)));
        assert!(!w.link_is_wide(link(1, 9)), "an end that does not resolve");
        let group = |a, b| PendingLink {
            from: (a, 1),
            to: (b, 2),
        };
        assert!(LevelWorld::link_joins_groups(group(3, 4)));
        assert!(!LevelWorld::link_joins_groups(group(3, 0)));
    }

    #[test]
    fn d385_link_growth() {
        use std::f32::consts::PI;
        assert_eq!(link_time_end(0.), 0.);
        assert!((link_time_end(PI) - LINK_TIME_MAX).abs() < 1e-3);
        assert!((link_time_end(PI / 2.) - 90.).abs() < 1e-3);
        assert_eq!(smooth_step(0., 0., 0.), 1.);
        assert_eq!(smooth_step(0., 90., 45.), 0.5);
        assert_eq!(smooth_step(0., 90., 90.), 1.);
    }

    #[test]
    fn d384_scripted_thread_natives() {
        let mut w = world();
        w.story_results = (0..4)
            .map(|i| crate::StoryResult {
                result: ObjectId(i),
                handle: ObjectId(20 + i),
                slot_id: ObjectId(10 + i),
                parent: None,
            })
            .collect();
        let links = w.story_progress.pending_links.clone();
        w.pending_link_objects = (0..4)
            .map(|i| (links[i], ObjectId(30 + i as u32)))
            .collect();
        w.link_slot_ids = vec![(ObjectId(40), (0, 9))];
        let obj = |i| Value::Object(Some(ObjectId(i)));
        assert_eq!(w.slot_of(&obj(11)), Some((0, 2)));
        assert_eq!(w.slot_of(&obj(40)), Some((0, 9)));
        assert_eq!(w.slot_of(&obj(99)), None);
        assert_eq!(w.story_slot_group(&obj(40)), None, "not a Story slot");
        let q = |w: &mut LevelWorld, c, m, a: &[Value]| w.unlock_query(c, m, a);
        let link = q(&mut w, "Game", "GetPendingLink__i", &[Value::Int(1)]);
        assert_eq!(link, Some(obj(31)));
        let is_in = "IsLinkInCurrentSearch__Q8SlotLink";
        w.story_search_requested = true;
        assert_eq!(
            q(&mut w, "PodComputer", is_in, &[obj(31)]),
            Some(Value::Bool(false))
        );
        assert!(w.story_search_requested, "still waiting for the search");
        w.pod.search.selected_planet = Some(1);
        assert_eq!(
            q(&mut w, "PodComputer", is_in, &[obj(31)]),
            Some(Value::Bool(true))
        );
        assert!(!w.story_search_requested);
        q(
            &mut w,
            "PodComputer",
            "SetBadgeUnlocking__Q6SlotID",
            &[obj(12)],
        );
        assert_eq!(w.story_progress.unlocking, [(0, 3)]);
        q(&mut w, "PodComputer", "UnlockBadge__Q6SlotID", &[obj(12)]);
        assert!(w.story_progress.unlocking.is_empty());
        assert!(
            w.story_progress.unlocked.is_empty(),
            "the badge alone opens nothing"
        );
        q(&mut w, "Game", "ClearPendingLink__Q8SlotLink", &[obj(31)]);
        assert_eq!(w.story_progress.pending_links.len(), 3);
        assert_eq!(
            w.story_progress.unlocked,
            [(0, 3)],
            "D1011: the clear opens it"
        );
        let to = q(
            &mut w,
            "PodComputer",
            "MoveCursorToResult__Q6SlotIDb",
            &[obj(13), Value::Bool(true)],
        );
        assert_eq!(to, Some(Value::Void));
        assert_eq!(w.pod.search.cur_result, Some(3));
        assert_eq!(w.pod.search.cursor_flight, None, "a snap does not fly");
        let dir = q(&mut w, "PodComputer", "GetBadgeDir__Q6SlotID", &[obj(10)]);
        assert_eq!(dir, Some(Value::Vec4([1., 0., 0., 0.])));
        assert_eq!(q(&mut w, "PodComputer", "GetState__", &[]), None);
    }

    #[test]
    fn d1011_clearing_a_link_opens_its_target() {
        use crate::{PendingLink, StoryProgress};
        let mut p = StoryProgress::default();
        let level = PendingLink {
            from: (0, 1),
            to: (0, 2),
        };
        let local = PendingLink {
            from: (0, 1),
            to: (2, 7),
        };
        p.pending_links = vec![level, local];
        p.clear_pending_link(local);
        assert!(
            p.unlocked.is_empty(),
            "a local user level (type 2) has no lock bit set"
        );
        p.clear_pending_link(level);
        assert!(p.pending_links.is_empty());
        assert_eq!(p.unlocked, [(0, 2)]);
    }

    /// D1014: the default group under the cursor at the end is pushed; the thread waits
    /// for the push, then moves the cursor to the group's default level.
    #[test]
    fn d1014_the_end_pushes_the_group_under_the_cursor() {
        let slot = |id: (i32, i32), location: [f32; 4], group| SearchSlot {
            id,
            location,
            group,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![
                slot((4, 27), [1., 0., 0., 0.], None),
                slot((0, 1), [0., 1., 0., 0.], Some((4, 27))),
            ],
            ..Default::default()
        };
        w.pod.search.selected_planet = Some(1);
        w.pod.search.default_result = Some(0);
        assert!(w.level_unlocking_tick(), "no links: the group is asked for");
        assert_eq!(w.unlock_group_push, Some((4, 27)));
        assert_eq!(w.pod.search.cursor_mode, Some(2));
        assert!(w.level_unlocking_tick(), "waits for the push");
        w.unlock_group_push = None;
        w.pod.search.group = Some((4, 27));
        assert!(!w.level_unlocking_tick());
        assert_eq!(w.pod.search.cur_result, Some(1));
        assert_eq!(w.pod.search.cursor_mode, Some(4));
    }
}
