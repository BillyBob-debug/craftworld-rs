//! D353 (part 100): the native stand-in for the Pod search state's Story entry
//! (`PodStateSearch`), for a player without the game's scripts.
//!
//! The game's scripts exist only in the encrypted patch.edat (D133); with the disc and
//! the update alone the search state never runs and the Story search never opens. This
//! does what the script does to the world when Story is chosen, and nothing else: the
//! planets, the pod and the camera are other scripts' (the Popit lane's pod.bin and
//! planets world stand-ins).
//!
//! MEASURED: every value below is the scripted run's (the VM over the decrypted scripts,
//! a golden trace of the Pod with Story chosen on tick 640; the trace is derived data,
//! kept with the run in ignored local-analysis/). The tests check the native entry gives
//! the trace's search state on the same ticks.

use std::collections::BTreeMap;

use crate::{GooeyFrame, GooeyItem, LevelWorld, STORY_DEFAULT_GROUP};

/// The script class this stands in for.
pub const POD_STATE_SEARCH_CLASS: &str = "PodStateSearch";

/// The Story frame's id and "UID high bits" (MEASURED: named frame 1 under high bits 2,
/// the search state's own).
pub const STORY_FRAME_KEY: (i32, i32) = (2, 1);

/// The Story frame's title (MEASURED: the text the script passed).
pub const STORY_TITLE: &str = "STORY";

/// The search state's Story entry.
#[derive(Debug, Clone, Default)]
pub struct PodStateSearchStory {
    /// `PWorld.Frame` of the tick Story was chosen on; `None` while it is not open.
    opened: Option<i32>,
    /// Ticks since then.
    ticks: u32,
}

impl PodStateSearchStory {
    pub fn class(&self) -> &'static str {
        POD_STATE_SEARCH_CLASS
    }

    pub fn is_open(&self) -> bool {
        self.opened.is_some()
    }

    /// Story chosen, on the tick it is chosen. MEASURED (tick 640): Earth selected, the
    /// cursor mode 1, the search details asked for once, and the info bubble's page
    /// counting from this frame; the default result is the default story group's.
    pub fn open(&mut self, world: &mut LevelWorld) {
        let frame = world.pworld.frame;
        self.opened = Some(frame);
        self.ticks = 0;
        let default = world
            .story_slots
            .iter()
            .position(|s| s.id == STORY_DEFAULT_GROUP);
        let search = &mut world.pod.search;
        search.selected_planet = Some(1);
        search.cursor_mode = Some(1);
        search.want_search_details_calls += 1;
        search.default_result = default;
        search.info_bubble_since = Some(frame);
    }

    /// D357: the search state popped (back out of Story): what [`Self::open`] and
    /// [`Self::update`] set is cleared, the counts are kept. INFERRED: the search state's
    /// `OnDeactivate` was not traced; the Story search is no longer up.
    pub fn close(&mut self, world: &mut LevelWorld) {
        self.opened = None;
        self.ticks = 0;
        let search = &mut world.pod.search;
        search.selected_planet = None;
        search.cursor_mode = None;
        search.browse_mode = None;
        search.earth_has_control = None;
        search.default_result = None;
        search.info_bubble_since = None;
    }

    /// One tick of the open Story search, after [`Self::open`]'s tick. MEASURED (tick
    /// 641 on): the cursor mode 4, browse mode 1, Earth has control, the cursor badge
    /// published once, and the Story frame built.
    pub fn update(&mut self, world: &mut LevelWorld) {
        let Some(_) = self.opened else { return };
        self.ticks += 1;
        if self.ticks == 1 {
            let search = &mut world.pod.search;
            search.cursor_mode = Some(4);
            search.browse_mode = Some(1);
            search.earth_has_control = Some(true);
            search.publish_cursor_badge_calls += 1;
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
        let mut story = PodStateSearchStory::default();
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
        let mut story = PodStateSearchStory::default();
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

    #[test]
    fn d353_no_default_group_no_default_result() {
        let mut w = LevelWorld::default();
        PodStateSearchStory::default().open(&mut w);
        assert_eq!(w.pod.search.default_result, None);
        assert!(w.story_open());
    }
}
