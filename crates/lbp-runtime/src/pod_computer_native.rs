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
    PLANETS_WORLD_PORT, STORY_FRAME_KEY,
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
pub const NO_STATE_PLANETS_VIEW: i32 = 0;

/// The main menu frame's matrix, root frame units to the planets level's world. MEASURED:
/// settled from tick 620 (it follows the world Thing's move in from tick 601; the move is
/// not played here, the frame is drawn where it comes to rest).
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
}

impl PuterState {
    fn planets_view(self) -> i32 {
        match self {
            Self::MainMenu => MAIN_MENU_PLANETS_VIEW,
            Self::StorySearch => STORY_PLANETS_VIEW,
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
    /// The last state was popped and the host asked to stop using the pod computer: no
    /// new main menu until the answer has come (the pod computer seen not in use).
    released: bool,
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

    /// After a tick of the planets level (`PodComputer.PerFrameUpdate`): a main menu when
    /// the pod computer is in use with nothing on the stack, then the top state's update
    /// with the menu input it is given.
    pub fn after_tick(&mut self, host: &mut Bindings<LevelWorld>) -> Option<PodComputerEvent> {
        let input = std::mem::take(&mut host.world.gooey.input);
        let in_use = host.world.pod_computer_in_use;
        self.released &= in_use;
        let Some(top) = self.top() else {
            if in_use && !self.released {
                self.push(host, PuterState::MainMenu);
                self.main_menu_ui(&mut host.world);
                return Some(PodComputerEvent::Pushed(PuterState::MainMenu));
            }
            return None;
        };
        if input.back {
            return Some(self.pop(host));
        }
        match top {
            PuterState::MainMenu => {
                let focus = host.world.gooey.focus;
                if input.choose && focus == Some((1, MAIN_MENU_STORY_BUTTON)) {
                    self.push(host, PuterState::StorySearch);
                    return Some(PodComputerEvent::Pushed(PuterState::StorySearch));
                }
                self.main_menu_ui(&mut host.world);
            }
            PuterState::StorySearch => self.search.update(&mut host.world),
        }
        None
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
            self.deactivate(&mut host.world, under);
        }
        self.stack.push(state);
        Self::set_view(host, state.planets_view());
        if state == PuterState::StorySearch {
            // The Story button's push: `PodStateSearch(SearchDetails.CreateStoryLevels())`.
            self.search = PodStateSearch::new(SearchDetails::StoryLevels);
            self.search.open(&mut host.world);
        }
    }

    /// `PodComputer.PopState`.
    fn pop(&mut self, host: &mut Bindings<LevelWorld>) -> PodComputerEvent {
        if let Some(state) = self.stack.pop() {
            self.deactivate(&mut host.world, state);
        }
        match self.top() {
            Some(top) => {
                Self::set_view(host, top.planets_view());
                if top == PuterState::MainMenu {
                    self.main_menu_ui(&mut host.world);
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

    /// `OnDeactivate`: the state's frame invalidated (it fades in again when rebuilt).
    fn deactivate(&mut self, world: &mut LevelWorld, state: PuterState) {
        let key = match state {
            PuterState::MainMenu => MAIN_MENU_FRAME_KEY,
            PuterState::StorySearch => {
                self.search.close(world);
                STORY_FRAME_KEY
            }
        };
        let g = &mut world.gooey;
        g.frames.remove(&key);
        g.first_built.remove(&key);
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
        g.frames.insert(MAIN_MENU_FRAME_KEY, main_menu_frame());
        g.item_matrices
            .insert(MAIN_MENU_FRAME_KEY, MAIN_MENU_FRAME_MATRIX);
        g.item_matrix_frames.insert(MAIN_MENU_FRAME_KEY, frame);
        g.focus_moved = false;
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
        host.world.pworld.frame += 1;
        host.world.gooey.input = input;
        puter.after_tick(host)
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
        assert!(!h.world.gooey.frames.contains_key(&MAIN_MENU_FRAME_KEY));
        tick(&mut puter, &mut h, NONE);
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
