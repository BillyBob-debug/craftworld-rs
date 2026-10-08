//! U6: renderer-independent immediate-mode UI model for front-end screens.
//!
//! Shape only, no game code: LBP2 draws its menus with a native immediate-mode widget
//! system driven by scripts (docs/ui-research.md §2; behaviour evidence from an unlicensed
//! project, used for vocabulary only: frames with layout/style, text, buttons, images,
//! spacers, widgets keyed by 64-bit ids, input results returned per widget). This crate is
//! an independent design in that shape so native Rust screens now, and a script VM later,
//! can describe UI the same way. It has no Bevy, GPU or game-format dependency
//! (AGENTS.md rule 5); `lbp-bevy-viewer::gooey_ui` turns a [`UiTree`] into bevy_ui nodes.
//!
//! Each frame a screen calls [`Builder`] methods; the builder records a [`UiTree`] and
//! answers input questions (was this button activated, is it focused) from the
//! [`FrameInput`] the host collected for the previous frame. Focus moves between
//! focusable widgets with [`navigate`] over the rectangles the host laid out.

/// Stable widget id. Interactive widgets need an explicit id; others may use
/// [`Builder::auto_id`]-derived ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uid(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Column,
    Row,
    /// Children flow left to right and wrap.
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    /// Children share the free space.
    Full,
}

/// Placeholder text styles; real ones come from game fonts later (U2 atlas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextStyle {
    Title,
    #[default]
    Body,
    Small,
}

/// Placeholder panel looks; LBP2's chalky/spiky/bubbly borders are later fidelity work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelStyle {
    #[default]
    Plain,
    Filled,
    Bubble,
}

/// What an image widget shows; the host resolves it (game texture, or a placeholder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageRef {
    /// Texture by catalog GUID.
    Guid(u32),
    /// Texture by SHA-1.
    Hash([u8; 20]),
    /// Authored placeholder swatch (linear RGBA), for synthetic screens.
    Swatch([u8; 4]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Frame {
        layout: Layout,
        align: Align,
        style: PanelStyle,
        padding: f32,
        gap: f32,
        /// Width and height as fractions of the screen; `None` fits the content.
        size: [Option<f32>; 2],
    },
    Text {
        text: String,
        style: TextStyle,
    },
    Button {
        label: String,
        image: Option<ImageRef>,
        enabled: bool,
    },
    Image {
        image: ImageRef,
        width: f32,
        height: f32,
    },
    Spacer {
        width: f32,
        height: f32,
    },
}

impl Widget {
    pub fn focusable(&self) -> bool {
        matches!(self, Widget::Button { enabled: true, .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub uid: Uid,
    pub widget: Widget,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// One frame's widgets in build order; index 0 is the root frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiTree {
    pub nodes: Vec<Node>,
    /// Ids used more than once in this frame (the later widget is still recorded).
    pub duplicate_ids: Vec<Uid>,
}

impl UiTree {
    pub fn find(&self, uid: Uid) -> Option<&Node> {
        self.nodes.iter().find(|node| node.uid == uid)
    }
    /// Focusable widget ids in build order.
    pub fn focusable(&self) -> Vec<Uid> {
        self.nodes
            .iter()
            .filter(|node| node.widget.focusable())
            .map(|node| node.uid)
            .collect()
    }
}

/// Input the host collected since the last frame, keyed by widget id.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameInput {
    pub focused: Option<Uid>,
    /// Widget activated (confirm button, click) since the last frame.
    pub activated: Option<Uid>,
}

/// Result of an interactive widget for this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Response {
    pub activated: bool,
    pub focused: bool,
}

pub struct Builder {
    tree: UiTree,
    stack: Vec<usize>,
    input: FrameInput,
}

impl Builder {
    /// Start a frame with a root column frame `root`.
    pub fn new(root: Uid, input: FrameInput) -> Self {
        let mut builder = Self {
            tree: UiTree::default(),
            stack: Vec::new(),
            input,
        };
        builder.push(
            root,
            Widget::Frame {
                layout: Layout::Column,
                align: Align::Start,
                style: PanelStyle::Plain,
                padding: 0.,
                gap: 0.,
                size: [None, None],
            },
        );
        builder.stack.push(0);
        builder
    }

    fn push(&mut self, uid: Uid, widget: Widget) -> usize {
        if self.tree.nodes.iter().any(|node| node.uid == uid) {
            self.tree.duplicate_ids.push(uid);
        }
        let index = self.tree.nodes.len();
        let parent = self.stack.last().copied();
        if let Some(parent) = parent {
            self.tree.nodes[parent].children.push(index);
        }
        self.tree.nodes.push(Node {
            uid,
            widget,
            parent,
            children: Vec::new(),
        });
        index
    }

    /// Id derived from the current parent and its child count, for non-interactive widgets.
    pub fn auto_id(&self) -> Uid {
        let parent = *self.stack.last().expect("root frame is always open");
        let node = &self.tree.nodes[parent];
        // splitmix64 step over (parent id, child index): stable across frames.
        let mut x = node.uid.0 ^ (node.children.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Uid(x ^ (x >> 31))
    }

    /// Open a child frame, build its contents with `contents`, close it.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        uid: Uid,
        layout: Layout,
        align: Align,
        style: PanelStyle,
        padding: f32,
        gap: f32,
        contents: impl FnOnce(&mut Self),
    ) {
        let index = self.push(
            uid,
            Widget::Frame {
                layout,
                align,
                style,
                padding,
                gap,
                size: [None, None],
            },
        );
        self.stack.push(index);
        contents(self);
        self.stack.pop();
    }

    /// Sets the size of the innermost open frame (fractions of the screen, clamped to
    /// 0..=1; `None` or a non-finite value fits the content).
    pub fn set_frame_size(&mut self, size: [Option<f32>; 2]) {
        let open = *self.stack.last().expect("root frame is always open");
        if let Widget::Frame { size: s, .. } = &mut self.tree.nodes[open].widget {
            *s = size.map(|v| v.filter(|v| v.is_finite()).map(|v| v.clamp(0., 1.)));
        }
    }

    pub fn text(&mut self, text: impl Into<String>, style: TextStyle) {
        let uid = self.auto_id();
        self.push(
            uid,
            Widget::Text {
                text: text.into(),
                style,
            },
        );
    }

    pub fn button(&mut self, uid: Uid, label: impl Into<String>) -> Response {
        self.button_with(uid, label, None, true)
    }

    pub fn button_with(
        &mut self,
        uid: Uid,
        label: impl Into<String>,
        image: Option<ImageRef>,
        enabled: bool,
    ) -> Response {
        self.push(
            uid,
            Widget::Button {
                label: label.into(),
                image,
                enabled,
            },
        );
        Response {
            activated: enabled && self.input.activated == Some(uid),
            focused: enabled && self.input.focused == Some(uid),
        }
    }

    pub fn image(&mut self, image: ImageRef, width: f32, height: f32) {
        let uid = self.auto_id();
        self.push(
            uid,
            Widget::Image {
                image,
                width,
                height,
            },
        );
    }

    pub fn spacer(&mut self, width: f32, height: f32) {
        let uid = self.auto_id();
        self.push(uid, Widget::Spacer { width, height });
    }

    pub fn finish(self) -> UiTree {
        self.tree
    }
}

/// Screen rectangle of a laid-out widget (host coordinates, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2., self.y + self.height / 2.)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Next focus from `from` in `direction` among `targets` (focusable widgets with their
/// laid-out rectangles). Picks the nearest centre in the half-plane of the direction,
/// weighting sideways distance double so moves stay in a row or column. With no current
/// focus (or it is gone), returns the first target. `None` when nothing lies that way.
pub fn navigate(targets: &[(Uid, Rect)], from: Option<Uid>, direction: Direction) -> Option<Uid> {
    let Some(origin) = from.and_then(|uid| targets.iter().find(|(id, _)| *id == uid)) else {
        return targets.first().map(|(uid, _)| *uid);
    };
    let (ox, oy) = origin.1.center();
    targets
        .iter()
        .filter(|(uid, _)| *uid != origin.0)
        .filter_map(|(uid, rect)| {
            let (x, y) = rect.center();
            let (along, across) = match direction {
                Direction::Up => (oy - y, (x - ox).abs()),
                Direction::Down => (y - oy, (x - ox).abs()),
                Direction::Left => (ox - x, (y - oy).abs()),
                Direction::Right => (x - ox, (y - oy).abs()),
            };
            (along > 0.5).then_some((along + 2. * across, *uid))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, uid)| uid)
}

/// Keep focus valid after a rebuild: the old focus if still focusable, else the first
/// focusable widget, else none.
pub fn settle_focus(tree: &UiTree, focused: Option<Uid>) -> Option<Uid> {
    let focusable = tree.focusable();
    focused
        .filter(|uid| focusable.contains(uid))
        .or_else(|| focusable.first().copied())
}
