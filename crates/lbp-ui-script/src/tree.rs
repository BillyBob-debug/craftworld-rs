//! U12b: turns the frame tree a script built through the Gooey host into an
//! `lbp-gooey` [`UiTree`] the viewer can draw. Renderer-independent.
//!
//! What the frame properties mean is UNKNOWN (only their names and argument values are
//! recorded), so this mapping is a placeholder layout, not the game's:
//! - every frame is a column; `DefaultChildSpacing` (first number) is the gap and
//!   `Borders` (first number) the padding, both clamped to 0..=64 (NaN/absent = 0/4);
//!   a frame with properties is drawn filled, the implicit root plain;
//! - text is body text, buttons are focusable buttons with their label (`[name]` when the
//!   script passed no string), breaks are 6-pixel spacers, other widgets are small text
//!   naming the native (for example `[DoPlayerBigIcon]`).
use crate::{base_name, Frame, Node};
use lbp_gooey::{Align, Builder, FrameInput, Layout, PanelStyle, TextStyle, UiTree, Uid};

fn first_number(props: &Frame, key: &str) -> Option<f32> {
    props
        .props
        .get(key)?
        .split(',')
        .next()?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
}

fn frame(b: &mut Builder, f: &Frame, uid: Uid) {
    let gap = first_number(f, "DefaultChildSpacing").map_or(4., |v| v.clamp(0., 64.));
    let padding = first_number(f, "Borders").map_or(4., |v| v.clamp(0., 64.));
    let style = if f.props.is_empty() {
        PanelStyle::Plain
    } else {
        PanelStyle::Filled
    };
    b.frame(
        uid,
        Layout::Column,
        Align::Start,
        style,
        padding,
        gap,
        |b| {
            for c in &f.children {
                match c {
                    Node::Frame(child) => {
                        let id = b.auto_id();
                        frame(b, child, id);
                    }
                    Node::Text { text, .. } => b.text(text.clone(), TextStyle::Body),
                    Node::Button {
                        label, function, ..
                    } => {
                        let id = b.auto_id();
                        let label = if label.is_empty() {
                            format!("[{}]", base_name(function))
                        } else {
                            label.clone()
                        };
                        b.button(id, label);
                    }
                    Node::Break { .. } => b.spacer(0., 6.),
                    Node::Widget { function, .. } => {
                        b.text(format!("[{}]", base_name(function)), TextStyle::Small)
                    }
                }
            }
        },
    );
}

/// The script's root frame as a [`UiTree`] under `root`, with `header` lines (counts,
/// faults) drawn above it as small text.
pub fn ui_tree(script_root: &Frame, root: Uid, header: &[String], input: FrameInput) -> UiTree {
    let mut b = Builder::new(root, input);
    for line in header {
        b.text(line.clone(), TextStyle::Small);
    }
    let id = b.auto_id();
    frame(&mut b, script_root, id);
    b.finish()
}
