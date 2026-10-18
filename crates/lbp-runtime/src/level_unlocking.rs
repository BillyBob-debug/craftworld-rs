//! D383 (part 130): the Pod's `LevelUnlockingThread` (FOUND, script ec688550) and its
//! `PendingLinkCluster`s (FOUND, script aff692cc), run natively as the planets world's
//! state 2 (`PlanetsWorld.UpdateState__`), one tick per yield. Read in part 127 (D380).
//!
//! Not ported: the voice-overs (`PlayVO__`), `CanStartEndOfLevelSequence__` (always
//! true here), the swap from a group search back to Story (the return opens Story), the
//! cached-info updates, the push of the focus planet's group at the end, the link
//! drawing and the state-2 camera. No audio plays; the sounds are counted.

use crate::{LevelWorld, PendingLink};

/// `PendingLinkCluster`'s angle test: a link without a group joins when its target's
/// direction is within 30 degrees of the cluster's `AverageDir` (FOUND, 0.5235988).
pub const CLUSTER_COS: f32 = 0.866_025_4;
/// State 2: link j starts on tick `LINK_DELAY * (j + 1)` (FOUND).
pub const LINK_DELAY: i32 = 10;
/// State 2: `gameplay/unlocking/unlock_beeps` on ticks 5, 20, 35, ... (FOUND).
pub const BEEP_FIRST: i32 = 5;
pub const BEEP_EVERY: i32 = 15;
/// INFERRED: a link's progress per tick (the step was not read; one second a link).
pub const LINK_STEP: f32 = 1. / 30.;
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
    /// State 2 made the group the results' default (one tick before the unlock).
    pub defaulted: bool,
}

/// The thread's state between ticks ([`LevelWorld::level_unlocking_tick`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelUnlocking {
    pub started: bool,
    pub clusters: Vec<PendingLinkCluster>,
    pub current: usize,
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
                defaulted: false,
            });
        }
        clusters
    }

    fn count_sound(&mut self, name: &str) {
        *self.sounds_played.entry(name.to_owned()).or_default() += 1;
    }

    fn cursor_to_badge(&mut self, id: (i32, i32)) {
        let Some(i) = self.badge_result(id) else {
            return;
        };
        let to = self.story_slots[i].location;
        let from = (self.pod.search.cur_result)
            .and_then(|c| self.story_slots.get(c))
            .map(|s| s.location);
        self.pod.search.move_cursor_to(i, Some(to), from, false);
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
                self.cursor_to_badge(from);
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
                    c.ticks += 1;
                    let t = c.ticks;
                    for (j, p) in c.progress.iter_mut().enumerate() {
                        if t >= LINK_DELAY * (j as i32 + 1) {
                            *p = (*p + LINK_STEP).min(1.);
                        }
                    }
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
                        // `Game.ClearPendingLink` and `UnlockBadge`.
                        let p = &mut self.story_progress;
                        p.pending_links.retain(|x| *x != l);
                        p.unlocking.retain(|x| *x != l.to);
                        if !p.unlocked.contains(&l.to) {
                            p.unlocked.push(l.to);
                        }
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
                        self.cursor_to_badge(to);
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
            if u.clusters.is_empty() {
                self.pod.search.cursor_mode = Some(4);
                return false;
            }
            return true;
        }
        while u.current < u.clusters.len() {
            if !self.cluster_update(u, u.current) {
                return true;
            }
            u.current += 1;
        }
        self.pod.search.cursor_mode = Some(4);
        false
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
}
