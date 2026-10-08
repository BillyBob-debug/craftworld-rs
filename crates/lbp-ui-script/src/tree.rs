//! U12b: turns the frame tree a script built through the Gooey host into an
//! `lbp-gooey` [`UiTree`] the viewer can draw. Renderer-independent.
//!
//! What the frame properties mean is UNKNOWN (only their names and argument values are
//! recorded), so this mapping is a placeholder layout, not the game's:
//! - `Sizing` (two numbers, script units) becomes a size relative to the first frame with a
//!   positive `Sizing`, which is taken as the full screen (D146: the Pod menus' outer
//!   frame is 2400 x 1250, `GetPuterWidth(2400)`); zero or negative values fit the content
//!   (their meaning is UNKNOWN);
//! - every frame is a column; `DefaultChildSpacing` (first number) is the gap and
//!   `Borders` (first number) the padding, both clamped to 0..=64 (NaN/absent = 0/4);
//!   a frame with properties is drawn filled, the implicit root plain;
//! - text is body text, buttons are focusable buttons with their label (`[name]` when the
//!   script passed no string), breaks are 6-pixel spacers, other widgets are small text
//!   naming the native (for example `[DoPlayerBigIcon]`).
use crate::{base_name, Frame, Node};
use lbp_gooey::{Align, Builder, FrameInput, Layout, PanelStyle, TextStyle, UiTree, Uid};

fn numbers(f: &Frame, key: &str) -> Vec<f32> {
    f.props
        .get(key)
        .map(|v| {
            v.split(',')
                .filter_map(|x| x.trim().parse::<f32>().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The first positive `Sizing` in pre-order: the screen size in script units.
pub fn reference_size(f: &Frame) -> Option<[f32; 2]> {
    if let [w, h, ..] = numbers(f, "Sizing")[..] {
        if w > 0. && h > 0. && w.is_finite() && h.is_finite() {
            return Some([w, h]);
        }
    }
    f.children.iter().find_map(|c| match c {
        Node::Frame(c) => reference_size(c),
        _ => None,
    })
}

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

fn frame(b: &mut Builder, f: &Frame, uid: Uid, reference: Option<[f32; 2]>) {
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
            if let (Some(r), [w, h, ..]) = (reference, &numbers(f, "Sizing")[..]) {
                let part = |v: f32, r: f32| (v > 0.).then_some(v / r);
                b.set_frame_size([part(*w, r[0]), part(*h, r[1])]);
            }
            for c in &f.children {
                match c {
                    Node::Frame(child) => {
                        let id = b.auto_id();
                        frame(b, child, id, reference);
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
    frame(&mut b, script_root, id, reference_size(script_root));
    b.finish()
}
