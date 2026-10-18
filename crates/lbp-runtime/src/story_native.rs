//! D353 (part 100), D358 (part 105): the Pod's search state (`PodStateSearch`) natively,
//! for a player without the game's scripts.
//!
//! The game's scripts exist only in the encrypted patch.edat (D133); with the disc and
//! the update alone the search state never runs and the Story search never opens. This
//! is the script's state: its activation, its per-frame update once the search is
//! loaded, and its deactivation when Back pops it. Browsing, the cursor and level launch
//! are native Planets code in the game too (the script only sets their modes); the
//! planets, the pod and the camera are other scripts' (the Popit lane's pod.bin and
//! planets world stand-ins).
//!
//! The mode comes from the search details the state was pushed with
//! ([`SearchDetails`]); the pod computer's Story button pushes the story levels'.
//!
//! MEASURED: every value below is the scripted run's (the VM over the decrypted scripts,
//! a golden trace of the Pod with Story chosen on tick 640 and Back pressed on tick 700;
//! the trace is derived data, kept with the run in ignored local-analysis/). The tests
//! check the native state gives the trace's search state on the same ticks.

use std::collections::BTreeMap;

use crate::{GooeyFrame, GooeyItem, LevelWorld, STORY_DEFAULT_GROUP};

/// The script class this is.
pub const POD_STATE_SEARCH_CLASS: &str = "PodStateSearch";

/// The Story frame's id and "UID high bits" (MEASURED: named frame 1 under high bits 2,
/// the search state's own).
pub const STORY_FRAME_KEY: (i32, i32) = (2, 1);

/// D359: the first group search's frame key (MEASURED: named frame 1 under high bits 3,
/// the third state the pod computer made: the main menu 1, Story 2, the group 3).
pub const GROUP_FRAME_KEY: (i32, i32) = (3, 1);

/// The Story frame's title (MEASURED: the text the script passed).
pub const STORY_TITLE: &str = "STORY";

/// The search the state was pushed with (the script's `SearchDetails` subclasses).
/// FOUND (01.33 listings): the Story button pushes `SearchDetails.CreateStoryLevels()`.
/// D359: Cross on a group's planet pushes `SearchDetails.CreateGroup(slot, true)`, a
/// `SearchDetailsGroup` for the group's slot id.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchDetails {
    #[default]
    StoryLevels,
    Group((i32, i32)),
}

impl SearchDetails {
    /// `GetPlanet`: the planet selected while the search is up (MEASURED: Earth, 1).
    pub fn planet(self) -> i32 {
        match self {
            Self::StoryLevels | Self::Group(_) => 1,
        }
    }

    /// `GetDefaultBrowseMode` (FOUND: 1 for the story levels; MEASURED, tick 641). A
    /// group's is 1 for a slot of type 4 or 9 and 0 for any other (FOUND,
    /// `SearchDetailsGroup.GetDefaultBrowseMode`; MEASURED 1 for the default group).
    pub fn default_browse_mode(self) -> i32 {
        match self {
            Self::StoryLevels => 1,
            Self::Group((kind, _)) => i32::from(kind == 4 || kind == 9),
        }
    }

    /// `GetAllowedBrowseModes` (FOUND: 0 for the story levels; MEASURED, tick 641, and 0
    /// for the default group, tick 681).
    pub fn allowed_browse_modes(self) -> i32 {
        match self {
            Self::StoryLevels | Self::Group(_) => 0,
        }
    }

    /// The frame key of the state's menu.
    pub fn frame_key(self) -> (i32, i32) {
        match self {
            Self::StoryLevels => STORY_FRAME_KEY,
            Self::Group(_) => GROUP_FRAME_KEY,
        }
    }
}

/// `PodStateSearch`.
#[derive(Debug, Clone, Default)]
pub struct PodStateSearch {
    details: SearchDetails,
    /// `PWorld.Frame` of the tick the state was activated on; `None` while it is not.
    opened: Option<i32>,
    /// Ticks since then.
    ticks: u32,
    /// `BrowseMode`, set from the details when the state is pushed (kept across a
    /// deactivation the state survives, as the script's member is).
    browse_mode: i32,
}

impl PodStateSearch {
    pub fn new(details: SearchDetails) -> Self {
        Self {
            details,
            ..Self::default()
        }
    }

    pub fn class(&self) -> &'static str {
        POD_STATE_SEARCH_CLASS
    }

    pub fn details(&self) -> SearchDetails {
        self.details
    }

    pub fn is_open(&self) -> bool {
        self.opened.is_some()
    }

    /// `OnActivate(pushed)`, on the tick the state is pushed. MEASURED (tick 640): the
    /// details' planet selected, the search details asked for, the cursor mode 1, and
    /// the info bubble's page counting from this frame; the default result is the
    /// default story group's.
    ///
    /// D359: a group's push (MEASURED, tick 680) selects the planet, asks for the search
    /// details and sets the cursor mode 1 the same, but the default result stays the one
    /// the Story state under it saved and the info bubble keeps counting from Story's
    /// push.
    pub fn open(&mut self, world: &mut LevelWorld) {
        let frame = world.pworld.frame;
        self.opened = Some(frame);
        self.ticks = 0;
        self.browse_mode = self.details.default_browse_mode();
        if let SearchDetails::Group(group) = self.details {
            // D360: the group's results ask for the zoom and the planet picks the
            // group's default level ([`LevelWorld::group_default_result`]).
            let pick = world.group_default_result(group);
            let search = &mut world.pod.search;
            search.selected_planet = Some(self.details.planet());
            search.want_search_details_calls += 1;
            search.cursor_mode = Some(1);
            search.group = Some(group);
            search.cur_result = pick;
            return;
        }
        let default = world
            .story_slots
            .iter()
            .position(|s| s.id == STORY_DEFAULT_GROUP);
        let search = &mut world.pod.search;
        search.selected_planet = Some(self.details.planet());
        search.want_search_details_calls += 1;
        search.cursor_mode = Some(1);
        search.default_result = default;
        search.info_bubble_since = Some(frame);
    }

    /// `OnDeactivate(false)`: Back popped the state (`PodComputer.OnPlanetCancel`).
    /// MEASURED (tick 700): the cursor mode 0, the search details cleared (a second
    /// call), no planet selected, the browse and allowed modes 0, no loader and Earth no
    /// longer in control; the default result is kept (only a deactivation that keeps
    /// the selection saves it, and Back does not).
    pub fn close(&mut self, world: &mut LevelWorld) {
        self.deactivate(world, false);
    }

    /// D359, `OnDeactivate(true)`: a state pushed over this one (a group chosen). FOUND
    /// (`OnDeactivateImpl`): the selected result is saved as the default result before
    /// the same clearing as [`Self::close`] (MEASURED, tick 680).
    pub fn suspend(&mut self, world: &mut LevelWorld) {
        self.deactivate(world, true);
    }

    /// D359, `OnActivate(false)`: the state back on top after the one pushed over it
    /// popped. MEASURED (tick 720): the planet selected, the search details asked for
    /// and the cursor mode 1 as on a push, but the default result and the info bubble's
    /// page are the ones it had; the browse mode is the one saved at the suspend.
    pub fn resume(&mut self, world: &mut LevelWorld) {
        self.opened = Some(world.pworld.frame);
        self.ticks = 0;
        let search = &mut world.pod.search;
        search.selected_planet = Some(self.details.planet());
        search.want_search_details_calls += 1;
        search.cursor_mode = Some(1);
    }

    fn deactivate(&mut self, world: &mut LevelWorld, keep: bool) {
        if self.opened.take().is_none() {
            return;
        }
        self.ticks = 0;
        let search = &mut world.pod.search;
        // D360: a group search closed: the Story search's own cursor again.
        if !keep && matches!(self.details, SearchDetails::Group(_)) && search.group.take().is_some()
        {
            search.cur_result = None;
        }
        if keep {
            search.default_result = search.cur_result.or(search.default_result);
        }
        search.cursor_mode = Some(0);
        search.want_search_details_calls += 1;
        search.selected_planet = Some(0);
        search.browse_mode = Some(0);
        search.allowed_browse_modes = Some(0);
        search.loader = Some(0);
        search.earth_has_control = Some(false);
    }

    /// `PerFrameUpdate`, after [`Self::open`]'s tick. MEASURED (tick 641 on): the results
    /// are in on the first update (offline the search returns at once), so the loaded
    /// branch runs: the cursor mode 4, the cursor badge published, Earth has control,
    /// the details' allowed browse modes, the state's browse mode, no loader; and the
    /// Story frame built.
    pub fn update(&mut self, world: &mut LevelWorld) {
        let Some(_) = self.opened else { return };
        self.ticks += 1;
        if self.ticks == 1 {
            let search = &mut world.pod.search;
            search.cursor_mode = Some(4);
            search.publish_cursor_badge_calls += 1;
            search.earth_has_control = Some(true);
            search.allowed_browse_modes = Some(self.details.allowed_browse_modes());
            search.browse_mode = Some(self.browse_mode);
            search.loader = Some(0);
        }
        let frame = world.pworld.frame;
        let key = self.details.frame_key();
        let (menu, rest, world_rest) = match self.details {
            SearchDetails::StoryLevels => (story_frame(), STORY_FRAME_MATRIX, STORY_WORLD_REST),
            SearchDetails::Group(g) => (
                group_frame(&world.group_title(g)),
                GROUP_FRAME_MATRIX,
                GROUP_WORLD_REST,
            ),
        };
        world.gooey.first_built.entry(key).or_insert(frame);
        let matrix = sliding_frame_matrix(world, rest, world_rest);
        let g = &mut world.gooey;
        g.frames.insert(key, menu);
        g.item_matrices.insert(key, matrix);
        g.item_matrix_frames.insert(key, frame);
    }
}

/// D369 (part 116): the planets level's world Thing (Thing 0) where the Story frame's
/// matrix was measured, tick 659 (MEASURED, d362 trace).
pub const STORY_WORLD_REST: [f32; 16] = [
    1.5296844, 0.44180164, -1.210321, 0., 0., 1.8787454, 0.6857956, 0., 1.2884353, -0.5245254,
    1.4369438, 0., 1912.1056, 3423.4658, -9460.587, 1.,
];

/// D369: the world Thing where the group frame's matrix was measured, tick 695
/// (MEASURED, d362 trace).
pub const GROUP_WORLD_REST: [f32; 16] = [
    1.5296844, 0.44180164, -1.210321, 0., 0., 1.8787454, 0.6857956, 0., 1.2884353, -0.5245254,
    1.4369438, 0., 1912.1056, 1973.3236, -5172.7827, 1.,
];

/// D369 (part 116): the frame's matrix as the menu slides. The scripts set it every
/// tick to the world Thing's position times a fixed view (`SetupFrameMatrix`, FOUND), so
/// the frame moves with the world Thing: `W(t) * inverse(world_rest) * rest`. With no
/// world Thing, `rest`. D371 (part 118): followed every tick, not only for a window after
/// the first build, so Story built again after a group slides back with the world Thing
/// and stays where it settles (MEASURED, not at its first rest).
pub fn sliding_frame_matrix(
    world: &LevelWorld,
    rest: [f32; 16],
    world_rest: [f32; 16],
) -> [f32; 16] {
    let now = world.world.and_then(|t| world.thing_display(t)?.world);
    match (now, crate::transform::inverse(&world_rest)) {
        (Some(now), Some(inv)) => crate::transform::mul(&crate::transform::mul(&now, &inv), &rest),
        _ => rest,
    }
}

/// D359: the group frame's matrix. MEASURED: settled from tick 695 (it slides in with the
/// world Thing from tick 681; D369 plays the slide, [`sliding_frame_matrix`]).
pub const GROUP_FRAME_MATRIX: [f32; 16] = [
    1.7360102,
    0.0059023574,
    -0.003893707,
    0.,
    0.004361921,
    -1.6463118,
    -0.55083793,
    0.,
    -0.0055652764,
    0.55082124,
    -1.64632,
    0.,
    -2090.9067,
    1179.0103,
    160.82642,
    1.,
];

/// D359: the group search's frame (MEASURED, tick 681): a 2400 x 1250 frame holding a
/// style 9 panel at most 700 wide with the group's title (`SearchDetailsGroup.DoTitle`;
/// empty for the default Story group offline, MEASURED; D362: [`LevelWorld::group_title`]).
pub fn group_frame(title: &str) -> GooeyFrame {
    let bits = GROUP_FRAME_KEY.0;
    let other = |pairs: &[(&str, f32)]| -> BTreeMap<String, Vec<f32>> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), vec![*v]))
            .collect()
    };
    let panel = GooeyFrame {
        high_bits: bits,
        layout: Some((0, 0, 1.)),
        sizing: Some([0., 0.]),
        borders: Some([32., 32., 32., 32.]),
        spacing: Some([0., 32.]),
        other: other(&[
            ("SetFrameCornerRadius", 64.),
            ("SetFrameCurvature", 20.),
            ("SetFrameMaxWidth", 700.),
            ("SetFrameOutlineWidth", 35.),
            ("SetFrameStyle", 9.),
        ]),
        children: vec![GooeyItem::Text {
            text: title.to_owned(),
            style: 1,
            font_scale: 1.,
        }],
        ..Default::default()
    };
    let column = GooeyFrame {
        high_bits: bits,
        layout: Some((3, 0, 1.)),
        sizing: Some([0., 0.]),
        other: other(&[("SetFrameMaxWidth", 800.)]),
        children: vec![GooeyItem::Frame(panel)],
        ..Default::default()
    };
    GooeyFrame {
        id: Some(GROUP_FRAME_KEY.1),
        high_bits: bits,
        layout: Some((0, 1, 1.)),
        sizing: Some([2400., 1250.]),
        borders: Some([64., 64., 64., 0.]),
        transition: Some([15., 15.]),
        other: BTreeMap::from([("SetFrameUseBlurForTransitions".to_owned(), vec![])]),
        children: vec![GooeyItem::Frame(column)],
        ..Default::default()
    }
}

/// The Story frame's matrix, root frame units to the planets level's world. MEASURED:
/// settled from tick 659 (it follows the world Thing's move in from tick 641; D369 plays
/// the move, [`sliding_frame_matrix`]).
pub const STORY_FRAME_MATRIX: [f32; 16] = [
    3.2550192,
    0.01106692,
    -0.0073007005,
    0.,
    0.008178601,
    -3.0868344,
    -1.032821,
    0.,
    -0.010434893,
    1.0327897,
    -3.08685,
    0.,
    -8094.7627,
    3633.8105,
    -3971.2896,
    1.,
];

/// The Story frame as the search state builds it (MEASURED, tick 641): a 4800 x 1250
/// frame of three columns with a style 15 speech bubble holding the title.
pub fn story_frame() -> GooeyFrame {
    let other = |pairs: &[(&str, &[f32])]| -> BTreeMap<String, Vec<f32>> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.to_vec()))
            .collect()
    };
    let bubble = GooeyFrame {
        id: None,
        high_bits: STORY_FRAME_KEY.0,
        layout: Some((0, 0, 1.)),
        sizing: Some([0., 0.]),
        borders: Some([45., 45., 45., 45.]),
        spacing: Some([0., 32.]),
        columns: vec![],
        transition: None,
        other: other(&[
            ("SetFrameCornerJauntiness", &[9.]),
            ("SetFrameCornerJauntinessSeed", &[2.]),
            ("SetFrameCornerRadius", &[80.]),
            ("SetFrameCurvature", &[9.]),
            ("SetFrameOutlineWidth", &[35.]),
            (
                "SetFrameSpeechBubbleOutlineTailSettings",
                &[50., 0., 0., 0., 30.],
            ),
            ("SetFrameStyle", &[15.]),
        ]),
        children: vec![GooeyItem::Text {
            text: STORY_TITLE.to_owned(),
            style: 16,
            font_scale: 2.5,
        }],
    };
    GooeyFrame {
        id: Some(STORY_FRAME_KEY.1),
        high_bits: STORY_FRAME_KEY.0,
        layout: Some((0, 3, 0.5)),
        sizing: Some([4800., 1250.]),
        borders: Some([64., 64., 64., 0.]),
        spacing: None,
        columns: vec![(-0.5, 1), (1500., 3), (-0.5, 0)],
        transition: Some([15., 15.]),
        other: other(&[("SetFrameUseBlurForTransitions", &[])]),
        children: vec![
            GooeyItem::Frame(bubble),
            GooeyItem::Other("DoImage".into()),
            GooeyItem::Other("DoImage".into()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SearchSlot;

    fn world() -> LevelWorld {
        let slot = |id| SearchSlot {
            id,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![slot((0, 1)), slot((4, 3)), slot(STORY_DEFAULT_GROUP)],
            ..LevelWorld::default()
        };
        w.pworld.frame = 219_906;
        w
    }

    /// The golden trace's search state on the tick Story is chosen (640) and the next
    /// (641): the native entry gives the same.
    #[test]
    fn d353_story_entry_matches_the_scripted_trace() {
        let mut w = world();
        let mut story = PodStateSearch::default();
        assert!(!w.story_open());
        story.open(&mut w);
        let s = &w.pod.search;
        assert!(w.story_open());
        assert_eq!(s.selected_planet, Some(1));
        assert_eq!(s.cursor_mode, Some(1));
        assert_eq!(s.want_search_details_calls, 1);
        assert_eq!(s.default_result, Some(2));
        assert_eq!(s.info_bubble_since, Some(219_906));
        assert_eq!((s.browse_mode, s.earth_has_control), (None, None));
        w.pworld.frame += 1;
        story.update(&mut w);
        let s = &w.pod.search;
        assert_eq!(s.cursor_mode, Some(4));
        assert_eq!(s.browse_mode, Some(1));
        assert_eq!(s.earth_has_control, Some(true));
        assert_eq!(s.publish_cursor_badge_calls, 1);
        assert_eq!((s.allowed_browse_modes, s.loader), (Some(0), Some(0)));
        // Later ticks change nothing more in the search state (the trace, 642 on).
        w.pworld.frame += 1;
        story.update(&mut w);
        assert_eq!(w.pod.search.publish_cursor_badge_calls, 1);
        assert_eq!(w.pod.search.want_search_details_calls, 1);
    }

    /// The Story frame is the drawn menu from the first update, with the title and the
    /// trace's settled matrix, first built on the tick after the choice.
    #[test]
    fn d353_story_frame_is_drawn() {
        let mut w = world();
        let mut story = PodStateSearch::default();
        story.update(&mut w);
        assert!(
            w.gooey.drawn_menu().is_none(),
            "nothing before Story is chosen"
        );
        story.open(&mut w);
        w.pworld.frame += 1;
        story.update(&mut w);
        let menu = w.gooey.drawn_menu().unwrap();
        assert_eq!(menu.matrix, STORY_FRAME_MATRIX);
        assert_eq!(menu.first_built, Some(219_907));
        assert_eq!(menu.frame.sizing, Some([4800., 1250.]));
        let GooeyItem::Frame(bubble) = &menu.frame.children[0] else {
            panic!("the bubble frame first");
        };
        assert_eq!(
            bubble.children,
            vec![GooeyItem::Text {
                text: "STORY".into(),
                style: 16,
                font_scale: 2.5
            }]
        );
    }

    /// D358: Back on tick 700 of the golden trace: the state's `OnDeactivate(false)`
    /// gives the trace's search state, the Story search is no longer up, and the info
    /// bubble's page stops counting on the next frame.
    #[test]
    fn d358_back_matches_the_scripted_trace() {
        let mut w = world();
        let mut story = PodStateSearch::new(SearchDetails::StoryLevels);
        story.open(&mut w);
        w.pworld.frame += 1;
        story.update(&mut w);
        story.close(&mut w);
        assert!(!story.is_open());
        assert!(!w.story_open());
        let s = &w.pod.search;
        assert_eq!(s.selected_planet, Some(0));
        assert_eq!(s.cursor_mode, Some(0));
        assert_eq!((s.browse_mode, s.allowed_browse_modes), (Some(0), Some(0)));
        assert_eq!((s.loader, s.earth_has_control), (Some(0), Some(false)));
        assert_eq!(s.want_search_details_calls, 2);
        assert_eq!(s.publish_cursor_badge_calls, 1);
        assert_eq!(s.default_result, Some(2), "kept");
        w.advance_frame();
        assert_eq!(w.pod.search.info_bubble_since, None);
        // A second close (nothing open) changes nothing.
        story.close(&mut w);
        assert_eq!(w.pod.search.want_search_details_calls, 2);
    }

    /// D359: the default group chosen on tick 680 of the golden trace: Story's
    /// `OnDeactivate(true)` then the group's `OnActivate`, and the group's first update.
    #[test]
    fn d359_group_choice_matches_the_scripted_trace() {
        let mut w = world();
        let mut story = PodStateSearch::new(SearchDetails::StoryLevels);
        story.open(&mut w);
        w.pworld.frame += 1;
        story.update(&mut w);
        let mut group = PodStateSearch::new(SearchDetails::Group(STORY_DEFAULT_GROUP));
        story.suspend(&mut w);
        group.open(&mut w);
        assert!(!story.is_open() && group.is_open());
        let s = &w.pod.search;
        assert_eq!(s.selected_planet, Some(1));
        assert_eq!(s.cursor_mode, Some(1));
        assert_eq!((s.browse_mode, s.allowed_browse_modes), (Some(0), Some(0)));
        assert_eq!((s.loader, s.earth_has_control), (Some(0), Some(false)));
        assert_eq!(s.want_search_details_calls, 3);
        assert_eq!(s.publish_cursor_badge_calls, 1);
        assert_eq!(s.default_result, Some(2), "the selected result saved");
        assert_eq!(s.info_bubble_since, Some(219_906), "still Story's");
        w.pworld.frame += 1;
        group.update(&mut w);
        let s = &w.pod.search;
        assert_eq!(s.cursor_mode, Some(4));
        assert_eq!(s.browse_mode, Some(1));
        assert_eq!(s.earth_has_control, Some(true));
        assert_eq!(s.publish_cursor_badge_calls, 2);
        let menu = &w.gooey.frames[&GROUP_FRAME_KEY];
        assert_eq!(menu.sizing, Some([2400., 1250.]));
        assert_eq!(w.gooey.item_matrices[&GROUP_FRAME_KEY], GROUP_FRAME_MATRIX);
    }

    /// D360: the group search wants the zoom view (7) and the planets world's extra zoom
    /// (500 for a type 4 group), selects its default level, and Back clears both.
    #[test]
    fn d360_group_zoom_and_default_level() {
        use lbp_script_vm::natives::World;
        let mut w = world();
        let mut story = PodStateSearch::new(SearchDetails::StoryLevels);
        story.open(&mut w);
        w.pworld.frame += 1;
        story.update(&mut w);
        assert_eq!(w.extra_planet_zoom(), Ok(0.));
        let mut group = PodStateSearch::new(SearchDetails::Group(STORY_DEFAULT_GROUP));
        story.suspend(&mut w);
        group.open(&mut w);
        assert_eq!(w.pod.search.group, Some(STORY_DEFAULT_GROUP));
        assert_eq!(
            w.pod.search.cur_result,
            w.group_default_result(STORY_DEFAULT_GROUP)
        );
        assert_eq!(w.selected_result(), w.pod.search.cur_result.or(Some(2)));
        assert_eq!(w.extra_planet_zoom(), Ok(500.));
        group.close(&mut w);
        assert_eq!((w.pod.search.group, w.pod.search.cur_result), (None, None));
        assert_eq!(w.extra_planet_zoom(), Ok(0.));
    }

    #[test]
    fn d361_cursor_moves_to_the_nearest_result_that_way() {
        let g = (4, 27);
        let slot = |id, group, location| crate::pod::SearchSlot {
            id,
            group,
            location,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![
                slot((0, 1), Some(g), [0., 0., 1., 0.]),
                slot((0, 2), Some(g), [0.3, 0., 0.95, 0.]),
                slot((0, 3), Some(g), [-0.6, 0., 0.8, 0.]),
                slot((4, 5), Some(g), [0.6, 0., 0.8, 0.]),
                slot((0, 6), None, [0.8, 0., 0.6, 0.]),
            ],
            ..Default::default()
        };
        w.pod.search.group = Some(g);
        w.pod.search.cur_result = Some(0);
        assert_eq!(w.move_result_cursor([1., 0.]), Some(1));
        while w.cursor_flight_step().is_none() {}
        // No wrap, and neither a group nor a slot of no group counts in a group search.
        assert_eq!(w.move_result_cursor([1., 0.]), None);
        assert_eq!(w.selected_result(), Some(1));
        // Left: the nearer of the two that way.
        assert_eq!(w.move_result_cursor([-1., 0.]), Some(0));
        while w.cursor_flight_step().is_none() {}
        assert_eq!(w.move_result_cursor([-1., 0.]), Some(2));
        while w.cursor_flight_step().is_none() {}
        assert_eq!(w.move_result_cursor([0., 1.]), None);
    }

    #[test]
    fn d368_cursor_flies_to_the_result_and_waits_to_land() {
        let slot = |id, location| crate::pod::SearchSlot {
            id,
            location,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![
                slot((0, 1), [0., 0., 1., 0.]),
                slot((0, 2), [0.6, 0., 0.8, 0.]),
            ],
            ..Default::default()
        };
        w.pod.search.cur_result = Some(0);
        assert_eq!(w.move_result_cursor([1., 0.]), Some(1));
        assert_eq!(w.pod.search.cur_result, Some(1));
        assert_eq!(w.sounds_played.get("pod/pod_cursor_move"), Some(&1));
        // No next move until it lands.
        assert_eq!(w.move_result_cursor([-1., 0.]), None);
        let mut frames = 0;
        let mut last = 0.;
        while w.cursor_flight_step().is_none() {
            frames += 1;
            assert!(w.pod.search.cursor_step >= last && w.pod.search.cursor_step < 1.);
            last = w.pod.search.cursor_step;
        }
        // d = acos(0.8) = 0.6435: (0.6435 / pi * 240 + 60) / 2500 = 0.0437 a frame.
        assert_eq!(frames, 14);
        assert_eq!(w.pod.search.cursor_step, 1.);
        let dir = w.pod.search.cursor_dir.unwrap();
        assert!((dir[0] - 0.6).abs() < 1e-6 && (dir[2] - 0.8).abs() < 1e-6);
        assert_eq!(w.move_result_cursor([-1., 0.]), Some(0));
        // A snap lands at once.
        w.pod
            .search
            .move_cursor_to(1, Some([0.6, 0., 0.8, 0.]), None, true);
        assert_eq!(w.pod.search.cursor_flight, None);
        assert_eq!(w.pod.search.cursor_dir, Some([0.6, 0., 0.8]));
    }

    #[test]
    fn d362_cross_on_a_story_level_asks_for_its_launch() {
        use crate::LevelCross;
        let slot = |id| crate::pod::SearchSlot {
            id,
            ..Default::default()
        };
        let mut w = LevelWorld {
            story_slots: vec![slot((1, 42)), slot((1, 43)), slot((4, 27))],
            ..Default::default()
        };
        assert_eq!(w.cross_on_level(2), LevelCross::NotLevel);
        assert_eq!(w.cross_on_level(0), LevelCross::Launch((1, 42)));
        assert_eq!(w.pod.launch, Some((1, 42)));
        // The pending level again: kind 0, nothing new.
        assert_eq!(w.cross_on_level(0), LevelCross::Current((1, 42)));
        // Another level replaces the pending launch.
        assert_eq!(w.cross_on_level(1), LevelCross::Launch((1, 43)));
        assert_eq!(w.pod.launch, Some((1, 43)));
        // The group's title only with completable levels (none known offline).
        w.story_slots[2].name = "Da Vinci's Hideout".into();
        assert_eq!(w.group_title((4, 27)), "");
        w.pod.group_levels.insert((4, 27), [1, 7]);
        assert_eq!(w.group_title((4, 27)), "Da Vinci's Hideout");
    }

    #[test]
    fn d367_a_launch_raises_its_warnings_and_waits_for_them() {
        use crate::pod::{LaunchWarning, PartyPad};
        let pad = |motion, primary| PartyPad { motion, primary };
        let mut w = LevelWorld {
            story_slots: vec![
                crate::pod::SearchSlot {
                    id: (1, 42),
                    ..Default::default()
                },
                crate::pod::SearchSlot {
                    id: (1, 43),
                    min_players: Some(2),
                    max_players: Some(4),
                    move_recommended: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        // One player on a Wireless Controller, a level for 1-4 without Move: none.
        w.cross_on_level(0);
        assert!(w.pod.launch_warnings.is_empty());
        assert_eq!(w.pod.warning_flags, 0);
        // Move recommended and at least two players: two warnings, both shown in turn.
        w.cross_on_level(1);
        assert_eq!(
            w.pod.launch_warnings,
            [LaunchWarning::MotionNeeded, LaunchWarning::TooFewPlayers(2)]
        );
        assert_eq!(w.pod.warning_flags, 0x11);
        assert_eq!(w.pod_launch_step(), None);
        w.answer_launch_warning(true);
        assert_eq!(w.pod_launch_step(), None);
        w.answer_launch_warning(true);
        assert_eq!(w.pod_launch_step().map(|s| s.slot), Some((1, 43)));
        // Another player without Move gets the party text; five is too many.
        w.pod.party = vec![pad(true, true), pad(false, false)];
        w.pod.launch = None;
        w.cross_on_level(1);
        assert_eq!(w.pod.launch_warnings, [LaunchWarning::MotionNotAll]);
        assert_eq!(w.pod.warning_flags, 0);
        w.answer_launch_warning(false);
        assert_eq!((w.pod.launch, w.pod_launch_step()), (None, None));
        w.pod.party = vec![pad(true, true); 5];
        w.cross_on_level(1);
        assert_eq!(w.pod.launch_warnings, [LaunchWarning::TooManyPlayers(4)]);
    }

    #[test]
    fn d363_a_done_launch_hands_the_root_level_to_the_game() {
        use lbp_level::serial::Descriptor;
        let root = Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(0x1_8fdb),
            sha1: None,
        };
        let mut w = LevelWorld {
            story_slots: vec![crate::pod::SearchSlot {
                id: (1, 42),
                root: Some(root),
                ..Default::default()
            }],
            ..Default::default()
        };
        // Nothing pending: no hand-off.
        assert_eq!(w.pod_launch_step(), None);
        w.cross_on_level(0);
        let start = w.pod_launch_step().unwrap();
        assert_eq!(
            (start.slot, start.root, start.mode),
            ((1, 42), Some(root), 0)
        );
        // Once only, and the launch stays pending until the level takes over.
        assert_eq!(w.pod_launch_step(), None);
        assert_eq!(w.pod.launch, Some((1, 42)));
        assert_eq!(w.pod.level_start, Some(start));
    }

    #[test]
    fn d359_group_default_browse_mode_by_slot_type() {
        assert_eq!(SearchDetails::Group((4, 27)).default_browse_mode(), 1);
        assert_eq!(SearchDetails::Group((9, 1)).default_browse_mode(), 1);
        assert_eq!(SearchDetails::Group((2, 1)).default_browse_mode(), 0);
    }

    #[test]
    fn d353_no_default_group_no_default_result() {
        let mut w = LevelWorld::default();
        PodStateSearch::default().open(&mut w);
        assert_eq!(w.pod.search.default_result, None);
        assert!(w.story_open());
    }

    #[test]
    fn d369_frames_slide_in_with_the_world_thing() {
        let mut w = LevelWorld {
            world: Some(0),
            things: vec![crate::ThingState {
                world_matrix: Some(STORY_WORLD_REST),
                ..Default::default()
            }],
            ..Default::default()
        };
        let at = |w: &LevelWorld| sliding_frame_matrix(w, STORY_FRAME_MATRIX, STORY_WORLD_REST);
        let close =
            |a: [f32; 16], b: [f32; 16]| a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 0.01);
        // At rest the frame is where it was measured.
        assert!(close(at(&w), STORY_FRAME_MATRIX));
        // The world Thing 1000 further down z: the frame goes with it.
        w.things[0].world_matrix.as_mut().unwrap()[14] -= 1000.;
        let m = at(&w);
        let mut moved = STORY_FRAME_MATRIX;
        moved[14] -= 1000.;
        assert!(close(m, moved));
        // With no world Thing it is drawn at rest.
        w.world = None;
        assert_eq!(at(&w), STORY_FRAME_MATRIX);
    }
}
