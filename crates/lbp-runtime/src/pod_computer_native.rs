//! D357 (part 104): the native stand-in for the pod computer's menu states
//! (`PodComputer`, `PodStateMainMenu` and the search state's Story entry), for a player
//! without the game's scripts.
//!
//! The game's scripts exist only in the encrypted patch.edat (D133); with the disc and
//! the update alone no pod computer state runs, so engaging the pod computer opened the
//! pod and nothing more. This keeps the scripts' state stack: the main menu when the pod
//! computer is in use, Story chosen from it, and back out of each.
//!
//! FOUND (the 01.33 scripts, read from the decrypted overlay for research only; nothing of
//! them is in the repository): `PodComputer.PerFrameUpdate` pushes a new main menu when
//! the pod computer is in use and its stack is empty. A push asks the planets world for
//! the new top state's view (`GetPlanetsViewType`: 1 for the main menu, the search
//! state's 6 for Story); a pop invalidates the state's frame, and the last pop asks the
//! host to stop using the pod computer and the planets world for view 0. The main menu
//! focuses Story on its first build; choosing Story (its button answers 256) pushes the
//! Story search; the root frame's `EndFrameInput(512)` answering 512 pops the state.
//!
//! MEASURED: the frame, its matrix and the ticks are the scripted run's (the VM over the
//! decrypted scripts, a golden trace of the Pod in use from tick 600 with Story chosen on
//! tick 640; the trace is derived data, kept in ignored local-analysis/).

use std::collections::BTreeMap;

use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Value;

use crate::{
    start_native, GooeyButton, GooeyFrame, GooeyItem, LevelWorld, PodStateSearch, SearchDetails,
    GROUP_FRAME_KEY, PLANETS_WORLD_PORT, STORY_FRAME_KEY,
};

/// The main menu frame's key (MEASURED: named frame 1 under high bits 1, the state's own).
pub const MAIN_MENU_FRAME_KEY: (i32, i32) = (1, 1);

/// The button the main menu focuses on its first build (FOUND:
/// `EnsureNodeOrDescendantHasFocus(6)`), and Story's id.
pub const MAIN_MENU_STORY_BUTTON: i32 = 6;

/// FOUND (`GetPlanetsViewType__`): the planets view of each state, and of none.
pub const MAIN_MENU_PLANETS_VIEW: i32 = 1;
/// INFERRED (part 82): the search state's view under Earth for Story.
pub const STORY_PLANETS_VIEW: i32 = 6;
/// D360: a Story group's search (the zoomed Story view).
pub const GROUP_PLANETS_VIEW: i32 = 7;
pub const NO_STATE_PLANETS_VIEW: i32 = 0;

/// D369 (part 116): the planets level's world Thing (Thing 0) where the main menu frame's
/// matrix was measured, tick 620 (MEASURED, d362 trace).
pub const MAIN_MENU_WORLD_REST: [f32; 16] = [
    1.995102,
    -0.047966495,
    0.1314048,
    0.,
    0.,
    1.8787454,
    0.6857956,
    0.,
    -0.1398857,
    -0.6841161,
    1.8741444,
    0.,
    -1400.,
    4479.9565,
    -10021.8,
    1.,
];

/// The main menu frame's matrix, root frame units to the planets level's world. MEASURED:
/// settled from tick 620 (it follows the world Thing's move in from tick 601; D369 plays
/// the move with [`crate::sliding_frame_matrix`]).
pub const MAIN_MENU_FRAME_MATRIX: [f32; 16] = [
    3.0380177,
    0.010329125,
    -0.006813987,
    0.,
    0.0076333545,
    -2.8810456,
    -0.9639663,
    0.,
    -0.009739255,
    0.96393716,
    -2.88106,
    0.,
    -3660.7512,
    2835.1982,
    -2010.9536,
    1.,
];

/// A pod computer state on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuterState {
    MainMenu,
    StorySearch,
    /// D359: a group of the Story search chosen (Cross on a group's planet).
    GroupSearch,
}

impl PuterState {
    pub fn planets_view(self) -> i32 {
        match self {
            Self::MainMenu => MAIN_MENU_PLANETS_VIEW,
            Self::StorySearch => STORY_PLANETS_VIEW,
            // D360: a group's results ask for the zoom (`GetPlanetsViewType`: 7 with
            // `GetWantZoom`, FOUND 0x44dfc0).
            Self::GroupSearch => GROUP_PLANETS_VIEW,
        }
    }
}

/// What a tick of the pod computer did that the session must answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodComputerEvent {
    /// A state was pushed.
    Pushed(PuterState),
    /// A state was popped and another is on top.
    Popped(PuterState),
    /// The last state was popped: the host is asked to stop using the pod computer.
    Released,
}

/// The pod computer's state stack.
#[derive(Debug, Clone, Default)]
pub struct NativePodComputer {
    stack: Vec<PuterState>,
    search: PodStateSearch,
    /// D359: the group search pushed over Story.
    group: PodStateSearch,
    /// The last state was popped and the host asked to stop using the pod computer: no
    /// new main menu until the answer has come (the pod computer seen not in use).
    released: bool,
    /// D372: a state was pushed before this tick's world update.
    pushed_now: bool,
    /// D372: the main menu's frame under a state pushed before the tick: drawn on that
    /// tick, dropped on the next.
    covered: bool,
}

impl NativePodComputer {
    pub fn top(&self) -> Option<PuterState> {
        self.stack.last().copied()
    }

    pub fn states(&self) -> &[PuterState] {
        &self.stack
    }

    /// `--story`: the Story search pushed over the main menu at once, on this tick. The
    /// main menu is deactivated under it and never drawn.
    pub fn open_story(&mut self, host: &mut Bindings<LevelWorld>) {
        if self.stack.is_empty() {
            self.stack.push(PuterState::MainMenu);
        }
        self.push(host, PuterState::StorySearch);
    }

    /// D372 (part 119): before the tick's world update (`PodComputer.PerFrameUpdate`, the
    /// first script call of the frame, so the planets world blends to a pushed state's
    /// view on the same frame; FOUND, D301): a main menu when the pod computer is in use
    /// with nothing on the stack, and the main menu's Story button. MEASURED (D371 runs):
    /// the scripted world Thing starts each of those moves a tick before the native one
    /// did. INFERRED: the button's press comes in the main menu's `UpdateUI`, so the menu
    /// is built on that tick and Story's frame on the next (MEASURED, 641).
    pub fn before_tick(&mut self, host: &mut Bindings<LevelWorld>) -> Option<PodComputerEvent> {
        let in_use = host.world.pod_computer_in_use;
        self.released &= in_use;
        match self.top() {
            None if in_use && !self.released => {
                self.push(host, PuterState::MainMenu);
                Some(PodComputerEvent::Pushed(PuterState::MainMenu))
            }
            Some(PuterState::MainMenu) => {
                let g = &host.world.gooey;
                let story = g.focus == Some((1, MAIN_MENU_STORY_BUTTON));
                if !g.input.choose || g.input.back || !story {
                    return None;
                }
                host.world.gooey.input.choose = false;
                self.main_menu_ui(&mut host.world);
                self.pushed_now = true;
                self.push(host, PuterState::StorySearch);
                Some(PodComputerEvent::Pushed(PuterState::StorySearch))
            }
            _ => None,
        }
    }

    /// After a tick of the planets level: Back, then the top state's update with the menu
    /// input it is given (the frames' matrices set after the world's move, as `Draw`).
    pub fn after_tick(&mut self, host: &mut Bindings<LevelWorld>) -> Option<PodComputerEvent> {
        let input = std::mem::take(&mut host.world.gooey.input);
        self.released &= host.world.pod_computer_in_use;
        let pushed_now = std::mem::take(&mut self.pushed_now);
        if std::mem::take(&mut self.covered) {
            if pushed_now {
                self.covered = true;
                self.main_menu_matrix(&mut host.world);
            } else {
                drop_frame(&mut host.world, MAIN_MENU_FRAME_KEY);
            }
        }
        let top = self.top()?;
        // D362: `PodComputer.OnPlanetCancel` returns without popping while a level launch
        // is pending (`IsLaunchingLevel`, pc 0019-0021, listing 16e6e8c9; MEASURED: the
        // scripted run keeps the group search on top after Back at tick 775).
        if input.back {
            if host.world.pod.launch.is_some() {
                return None;
            }
            return Some(self.pop(host));
        }
        match top {
            PuterState::MainMenu => self.main_menu_ui(&mut host.world),
            // Story's frame is first built on the tick after its push.
            PuterState::StorySearch if pushed_now => {}
            PuterState::StorySearch => self.search.update(&mut host.world),
            PuterState::GroupSearch => self.group.update(&mut host.world),
        }
        None
    }

    /// D359, `PodComputer.OnPlanetGroupSelected(slot, true, true)`: Cross on a group's
    /// planet with the Story search on top (after the tick, as the engine's cursor
    /// answers). FOUND: a new search state for `SearchDetails.CreateGroup(slot, true)`
    /// pushed over it, the cursor mode carried over.
    pub fn choose_group(
        &mut self,
        host: &mut Bindings<LevelWorld>,
        slot: (i32, i32),
    ) -> Option<PodComputerEvent> {
        if self.top() != Some(PuterState::StorySearch) {
            return None;
        }
        self.group = PodStateSearch::new(SearchDetails::Group(slot));
        self.push(host, PuterState::GroupSearch);
        Some(PodComputerEvent::Pushed(PuterState::GroupSearch))
    }

    fn set_view(host: &mut Bindings<LevelWorld>, view: i32) {
        if let Some(world) = host.world.world {
            let args = [(4, Value::Int(view))];
            let set_view = "SetViewFromRemotePodComputer__i";
            start_native(&PLANETS_WORLD_PORT, host, world, set_view, &args);
        }
    }

    /// `PodComputer.PushState`: the state under it deactivated (every state here
    /// deactivates the ones under it), the new one activated with its planets view.
    fn push(&mut self, host: &mut Bindings<LevelWorld>, state: PuterState) {
        if let Some(under) = self.top() {
            self.deactivate(&mut host.world, under, true);
        }
        self.stack.push(state);
        Self::set_view(host, state.planets_view());
        if state == PuterState::StorySearch {
            // The Story button's push: `PodStateSearch(SearchDetails.CreateStoryLevels())`.
            self.search = PodStateSearch::new(SearchDetails::StoryLevels);
            self.search.open(&mut host.world);
        }
        if state == PuterState::GroupSearch {
            self.group.open(&mut host.world);
        }
    }

    /// `PodComputer.PopState`.
    fn pop(&mut self, host: &mut Bindings<LevelWorld>) -> PodComputerEvent {
        if let Some(state) = self.stack.pop() {
            self.deactivate(&mut host.world, state, false);
        }
        match self.top() {
            Some(top) => {
                Self::set_view(host, top.planets_view());
                // D373: the main menu is built on the next tick's `UpdateUI`, as the
                // scripts' `OnPlanetCancel` pops after the frame's update (MEASURED: the
                // scripted run draws Story on Back's tick 740, the main menu from 741).
                if top == PuterState::StorySearch {
                    self.search.resume(&mut host.world);
                }
                PodComputerEvent::Popped(top)
            }
            None => {
                self.released = true;
                Self::set_view(host, NO_STATE_PLANETS_VIEW);
                PodComputerEvent::Released
            }
        }
    }

    /// `OnDeactivate(pushed)`: the state's frame no longer drawn; a popped state's frame
    /// is invalidated (it fades in again when rebuilt), while one a state was pushed over
    /// keeps its first build (MEASURED, D359: Story's frame back on tick 721 with the
    /// build of 219906).
    fn deactivate(&mut self, world: &mut LevelWorld, state: PuterState, pushed: bool) {
        let key = match state {
            // D372: the main menu pushed over before the tick is still drawn on it.
            PuterState::MainMenu if pushed && self.pushed_now => {
                self.covered = true;
                if world
                    .gooey
                    .focus
                    .is_some_and(|f| f.0 == MAIN_MENU_FRAME_KEY.0)
                {
                    world.gooey.focus = None;
                }
                return;
            }
            PuterState::MainMenu => MAIN_MENU_FRAME_KEY,
            PuterState::StorySearch if pushed => {
                self.search.suspend(world);
                STORY_FRAME_KEY
            }
            PuterState::StorySearch => {
                self.search.close(world);
                STORY_FRAME_KEY
            }
            PuterState::GroupSearch => {
                self.group.close(world);
                GROUP_FRAME_KEY
            }
        };
        let g = &mut world.gooey;
        g.frames.remove(&key);
        if !pushed || state == PuterState::MainMenu {
            g.first_built.remove(&key);
        }
        g.item_matrices.remove(&key);
        g.item_matrix_frames.remove(&key);
        if g.focus.is_some_and(|f| f.0 == key.0) {
            g.focus = None;
        }
    }

    /// `PodStateMainMenu.UpdateUI` and `Draw`: the menu frame built, Story focused on the
    /// first build, the frame's matrix set.
    fn main_menu_ui(&self, world: &mut LevelWorld) {
        let frame = world.pworld.frame;
        let g = &mut world.gooey;
        if g.focus.is_none_or(|f| f.0 != MAIN_MENU_FRAME_KEY.0) {
            g.focus = Some((MAIN_MENU_FRAME_KEY.0, MAIN_MENU_STORY_BUTTON));
            g.focus_since = Some(frame);
        }
        g.high_bits = MAIN_MENU_FRAME_KEY.0;
        g.first_built.entry(MAIN_MENU_FRAME_KEY).or_insert(frame);
        world
            .gooey
            .frames
            .insert(MAIN_MENU_FRAME_KEY, main_menu_frame());
        self.main_menu_matrix(world);
        world.gooey.focus_moved = false;
    }

    /// `PodStateMainMenu.Draw`: the frame's matrix set.
    fn main_menu_matrix(&self, world: &mut LevelWorld) {
        let frame = world.pworld.frame;
        let matrix =
            crate::sliding_frame_matrix(world, MAIN_MENU_FRAME_MATRIX, MAIN_MENU_WORLD_REST);
        let g = &mut world.gooey;
        g.item_matrices.insert(MAIN_MENU_FRAME_KEY, matrix);
        g.item_matrix_frames.insert(MAIN_MENU_FRAME_KEY, frame);
    }
}

/// D372: a menu frame and its builds dropped.
fn drop_frame(world: &mut LevelWorld, key: (i32, i32)) {
    let g = &mut world.gooey;
    g.frames.remove(&key);
    g.first_built.remove(&key);
    g.item_matrices.remove(&key);
    g.item_matrix_frames.remove(&key);
    if g.focus.is_some_and(|f| f.0 == key.0) {
        g.focus = None;
    }
}

/// The main menu frame as the state builds it offline (MEASURED, tick 601): a 2400 x 1250
/// frame; Story open, Community, Recent Activity and Store locked down the left, the
/// player's picture card, Me locked on the right, and an empty friends frame under them.
pub fn main_menu_frame() -> GooeyFrame {
    let bits = MAIN_MENU_FRAME_KEY.0;
    let button = |id, label: &str, state, flags, angle| {
        GooeyItem::Button(GooeyButton {
            id,
            label: label.to_owned(),
            text_style: 1,
            state,
            flags,
            angle,
            font_scale: 1.,
            icon: false,
        })
    };
    let column = |layout, borders, children| GooeyFrame {
        high_bits: bits,
        layout,
        sizing: Some([0., 0.]),
        borders: Some(borders),
        children,
        ..Default::default()
    };
    let buttons = column(
        Some((1, 3, 0.)),
        [32., 32., 32., 32.],
        vec![
            button(MAIN_MENU_STORY_BUTTON, "Story", 0, 256, 0.2),
            GooeyItem::Break,
            button(9, "Community", 6, 0, 0.075),
            GooeyItem::Break,
            button(10, "Recent Activity", 6, 0, -0.075),
            GooeyItem::Break,
            button(11, "Store", 6, 0, -0.2),
        ],
    );
    let player = column(
        None,
        [64., 128., 64., 0.],
        vec![GooeyItem::PlayerIcon { id: None }],
    );
    let me = column(
        Some((0, 3, 0.)),
        [32., 32., 32., 32.],
        vec![button(12, "Me", 6, 0, 0.)],
    );
    let row = GooeyFrame {
        high_bits: bits,
        layout: Some((3, 3, 0.5)),
        sizing: Some([-1., 500.]),
        columns: vec![(-0.5, 1), (0., 3), (-0.5, 0)],
        children: vec![
            GooeyItem::Frame(buttons),
            GooeyItem::Frame(player),
            GooeyItem::Frame(me),
        ],
        ..Default::default()
    };
    let friends = GooeyFrame {
        high_bits: bits,
        layout: Some((3, 3, 0.)),
        sizing: Some([1000., 300.]),
        borders: Some([32., 32., 32., 32.]),
        spacing: Some([0., 16.]),
        ..Default::default()
    };
    GooeyFrame {
        id: Some(MAIN_MENU_FRAME_KEY.1),
        high_bits: bits,
        layout: Some((3, 1, 0.5)),
        sizing: Some([2400., 1250.]),
        borders: Some([0., 0., 0., 150.]),
        transition: Some([15., 15.]),
        other: BTreeMap::from([("SetFrameUseBlurForTransitions".to_owned(), vec![])]),
        children: vec![
            GooeyItem::Frame(row),
            GooeyItem::Break,
            GooeyItem::Frame(friends),
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GooeyInput;

    fn host() -> Bindings<LevelWorld> {
        let mut w = LevelWorld::default();
        w.pworld.frame = 219_867;
        Bindings::new(w)
    }

    fn tick(
        puter: &mut NativePodComputer,
        host: &mut Bindings<LevelWorld>,
        input: GooeyInput,
    ) -> Option<PodComputerEvent> {
        host.world.gooey.input = input;
        let before = puter.before_tick(host);
        host.world.pworld.frame += 1;
        puter.after_tick(host).or(before)
    }

    const NONE: GooeyInput = GooeyInput {
        choose: false,
        back: false,
    };
    const CHOOSE: GooeyInput = GooeyInput {
        choose: true,
        back: false,
    };
    const BACK: GooeyInput = GooeyInput {
        choose: false,
        back: true,
    };

    /// Nothing until the pod computer is in use; then the main menu on that tick, Story
    /// focused, drawn with the trace's settled matrix.
    #[test]
    fn d357_main_menu_when_in_use() {
        let mut h = host();
        let mut puter = NativePodComputer::default();
        assert_eq!(tick(&mut puter, &mut h, NONE), None);
        assert!(h.world.gooey.drawn_menu().is_none());
        h.world.pod_computer_in_use = true;
        assert_eq!(
            tick(&mut puter, &mut h, NONE),
            Some(PodComputerEvent::Pushed(PuterState::MainMenu))
        );
        let g = &h.world.gooey;
        assert_eq!(g.focus, Some((1, MAIN_MENU_STORY_BUTTON)));
        let menu = g.drawn_menu().unwrap();
        assert_eq!(menu.matrix, MAIN_MENU_FRAME_MATRIX);
        assert_eq!(menu.first_built, Some(219_869));
        assert_eq!(menu.frame.sizing, Some([2400., 1250.]));
        // Later ticks keep the first build.
        assert_eq!(tick(&mut puter, &mut h, NONE), None);
        assert_eq!(
            h.world.gooey.drawn_menu().unwrap().first_built,
            Some(219_869)
        );
    }

    /// Choosing Story pushes the search over the menu; the Story frame is drawn from the
    /// next tick; back pops to the main menu with the Story search closed; back again
    /// releases the pod computer, and no new menu comes while it is still in use.
    #[test]
    fn d357_story_and_back_out() {
        let mut h = host();
        h.world.pod_computer_in_use = true;
        let mut puter = NativePodComputer::default();
        tick(&mut puter, &mut h, NONE);
        assert_eq!(
            tick(&mut puter, &mut h, CHOOSE),
            Some(PodComputerEvent::Pushed(PuterState::StorySearch))
        );
        assert!(h.world.story_open());
        assert_eq!(h.world.pod.search.selected_planet, Some(1));
        // D372: the covered main menu is still drawn on the push's tick, then dropped.
        assert!(h.world.gooey.frames.contains_key(&MAIN_MENU_FRAME_KEY));
        tick(&mut puter, &mut h, NONE);
        assert!(!h.world.gooey.frames.contains_key(&MAIN_MENU_FRAME_KEY));
        assert_eq!(
            h.world.gooey.drawn_menu().unwrap().matrix,
            crate::STORY_FRAME_MATRIX
        );
        assert_eq!(h.world.pod.search.cursor_mode, Some(4));

        assert_eq!(
            tick(&mut puter, &mut h, BACK),
            Some(PodComputerEvent::Popped(PuterState::MainMenu))
        );
        assert!(!h.world.story_open());
        assert!(!h.world.gooey.frames.contains_key(&crate::STORY_FRAME_KEY));
        // D373: the main menu is built on the tick after Back's.
        assert!(h.world.gooey.drawn_menu().is_none());
        tick(&mut puter, &mut h, NONE);
        assert_eq!(
            h.world.gooey.drawn_menu().unwrap().matrix,
            MAIN_MENU_FRAME_MATRIX
        );
        assert_eq!(h.world.gooey.focus, Some((1, MAIN_MENU_STORY_BUTTON)));

        assert_eq!(
            tick(&mut puter, &mut h, BACK),
            Some(PodComputerEvent::Released)
        );
        assert!(puter.states().is_empty());
        assert!(h.world.gooey.drawn_menu().is_none());
        assert_eq!(
            tick(&mut puter, &mut h, NONE),
            None,
            "still in use: no new menu"
        );
        h.world.pod_computer_in_use = false;
        assert_eq!(tick(&mut puter, &mut h, NONE), None);
        h.world.pod_computer_in_use = true;
        assert_eq!(
            tick(&mut puter, &mut h, NONE),
            Some(PodComputerEvent::Pushed(PuterState::MainMenu))
        );
    }

    /// D359: a group chosen over Story pushes the group search (Story suspended, its
    /// default result kept); Back pops it and Story comes back on top (MEASURED, ticks
    /// 720 and 721 of the scripted run: want 5, cursor 1 then 4, badge 3).
    #[test]
    fn d359_group_and_back_to_story() {
        let mut h = host();
        h.world.pod_computer_in_use = true;
        let mut puter = NativePodComputer::default();
        tick(&mut puter, &mut h, NONE);
        assert_eq!(puter.choose_group(&mut h, (4, 27)), None, "not over Story");
        tick(&mut puter, &mut h, CHOOSE);
        tick(&mut puter, &mut h, NONE);
        assert_eq!(
            puter.choose_group(&mut h, (4, 27)),
            Some(PodComputerEvent::Pushed(PuterState::GroupSearch))
        );
        assert!(!h.world.gooey.frames.contains_key(&STORY_FRAME_KEY));
        tick(&mut puter, &mut h, NONE);
        let s = &h.world.pod.search;
        assert_eq!((s.want_search_details_calls, s.cursor_mode), (3, Some(4)));
        assert_eq!(s.publish_cursor_badge_calls, 2);
        let menu = h.world.gooey.drawn_menu().unwrap();
        assert_eq!(menu.matrix, crate::GROUP_FRAME_MATRIX);

        assert_eq!(
            tick(&mut puter, &mut h, BACK),
            Some(PodComputerEvent::Popped(PuterState::StorySearch))
        );
        let s = &h.world.pod.search;
        assert_eq!((s.want_search_details_calls, s.cursor_mode), (5, Some(1)));
        assert_eq!(s.selected_planet, Some(1));
        assert!(!h.world.gooey.frames.contains_key(&GROUP_FRAME_KEY));
        tick(&mut puter, &mut h, NONE);
        let s = &h.world.pod.search;
        assert_eq!((s.cursor_mode, s.browse_mode), (Some(4), Some(1)));
        assert_eq!(s.publish_cursor_badge_calls, 3);
        let menu = h.world.gooey.drawn_menu().unwrap();
        assert_eq!(menu.matrix, crate::STORY_FRAME_MATRIX);
        assert_eq!(menu.first_built, Some(219_870), "Story's first build kept");
    }

    /// D362: Back does nothing while a level launch is pending (MEASURED, tick 775 of the
    /// scripted run: the group search stays on top, want 3 and badge 2).
    #[test]
    fn d362_back_waits_for_a_pending_launch() {
        let mut h = host();
        h.world.pod_computer_in_use = true;
        let mut puter = NativePodComputer::default();
        tick(&mut puter, &mut h, NONE);
        tick(&mut puter, &mut h, CHOOSE);
        tick(&mut puter, &mut h, NONE);
        puter.choose_group(&mut h, (4, 27));
        tick(&mut puter, &mut h, NONE);
        h.world.pod.launch = Some((0, 127_795));
        assert_eq!(tick(&mut puter, &mut h, BACK), None);
        let s = &h.world.pod.search;
        assert_eq!((s.want_search_details_calls, s.cursor_mode), (3, Some(4)));
        assert_eq!(s.publish_cursor_badge_calls, 2);
        assert!(h.world.gooey.frames.contains_key(&GROUP_FRAME_KEY));
    }

    /// `--story`: the search over the main menu at once.
    #[test]
    fn d357_open_story() {
        let mut h = host();
        let mut puter = NativePodComputer::default();
        puter.open_story(&mut h);
        assert_eq!(
            puter.states(),
            [PuterState::MainMenu, PuterState::StorySearch]
        );
        assert!(h.world.story_open());
        assert!(h.world.gooey.drawn_menu().is_none());
    }

    /// Choose with another button focused does nothing.
    #[test]
    fn d357_choose_elsewhere_stays() {
        let mut h = host();
        h.world.pod_computer_in_use = true;
        let mut puter = NativePodComputer::default();
        tick(&mut puter, &mut h, NONE);
        h.world.gooey.focus = Some((1, 9));
        assert_eq!(tick(&mut puter, &mut h, CHOOSE), None);
        assert_eq!(puter.top(), Some(PuterState::MainMenu));
    }
}
