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

/// The Story frame's title (MEASURED: the text the script passed).
pub const STORY_TITLE: &str = "STORY";

/// The search the state was pushed with (the script's `SearchDetails` subclasses).
/// FOUND (01.33 listings): the Story button pushes `SearchDetails.CreateStoryLevels()`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchDetails {
    #[default]
    StoryLevels,
}

impl SearchDetails {
    /// `GetPlanet`: the planet selected while the search is up (MEASURED: Earth, 1).
    pub fn planet(self) -> i32 {
        match self {
            Self::StoryLevels => 1,
        }
    }

    /// `GetDefaultBrowseMode` (FOUND: 1 for the story levels; MEASURED, tick 641).
    pub fn default_browse_mode(self) -> i32 {
        match self {
            Self::StoryLevels => 1,
        }
    }

    /// `GetAllowedBrowseModes` (FOUND: 0 for the story levels; MEASURED, tick 641).
    pub fn allowed_browse_modes(self) -> i32 {
        match self {
            Self::StoryLevels => 0,
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
    pub fn open(&mut self, world: &mut LevelWorld) {
        let frame = world.pworld.frame;
        self.opened = Some(frame);
        self.ticks = 0;
        self.browse_mode = self.details.default_browse_mode();
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
        if self.opened.take().is_none() {
            return;
        }
        self.ticks = 0;
        let search = &mut world.pod.search;
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
        let title = story_frame();
        let g = &mut world.gooey;
        g.first_built.entry(STORY_FRAME_KEY).or_insert(frame);
        g.frames.insert(STORY_FRAME_KEY, title);
        g.item_matrices.insert(STORY_FRAME_KEY, STORY_FRAME_MATRIX);
        g.item_matrix_frames.insert(STORY_FRAME_KEY, frame);
    }
}

/// The Story frame's matrix, root frame units to the planets level's world. MEASURED:
/// settled from tick 659 (it follows the world Thing's move in from tick 641; the move
/// is not played here, the frame is drawn where it comes to rest).
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

    #[test]
    fn d353_no_default_group_no_default_result() {
        let mut w = LevelWorld::default();
        PodStateSearch::default().open(&mut w);
        assert_eq!(w.pod.search.default_result, None);
        assert!(w.story_open());
    }
}
