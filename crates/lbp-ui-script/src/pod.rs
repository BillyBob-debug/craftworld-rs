//! P1b: places the Pod main menu the script builds onto the globe and moon, as the game
//! shows it. Renderer-independent: positions are fractions of a 16:9 frame (x of the
//! width, y of the height, from the top left) and sphere radii are fractions of the
//! height.
//!
//! Evidence: the RPCS3 reference of the user's own 01.33 copy
//! (/mnt/project-files/lbp2/research/pod-reference-rpcs3.md, CONFIRMED_FROM_LOCAL_DATA for
//! the measured points, about 0.01 accuracy). With the Puter open the main menu is the
//! cloth Earth globe (centre 0.35/0.40, radius 0.22) with pill labels curving down its
//! right side, from Story at 0.40/0.32 to Store at 0.38/0.53. The moon (centre 0.62/0.40,
//! radius 0.12) carries the "Me" pill, and the profile Polaroid sits at 0.50/0.36.
//! Inferred (STRONG_INFERENCE): the script's buttons in order are the globe's pills, and
//! the last one is the moon's ("Me" is last in both the trace and the capture).
//! `DoPlayerBigIcon` is the Polaroid (it sits between Store and Me in the trace). The pills
//! between the two measured ends follow an arc through the globe's right side (shape
//! UNKNOWN between the ends), and the Me pill sits on the moon's centre (position UNKNOWN).
//! How the game itself places them (script or engine) is UNKNOWN until the PC lists the
//! menu script's natives.
use crate::{base_name, Frame, Node};

/// Globe centre and radius (fractions of the frame; radius of the height).
pub const GLOBE: ([f32; 2], f32) = ([0.35, 0.40], 0.22);
/// Moon centre and radius.
pub const MOON: ([f32; 2], f32) = ([0.62, 0.40], 0.12);
/// Profile Polaroid centre.
pub const POLAROID: [f32; 2] = [0.50, 0.36];
/// First and last globe pill centres (Story, Store).
pub const PILL_ENDS: [[f32; 2]; 2] = [[0.40, 0.32], [0.38, 0.53]];
/// Frame aspect the fractions were measured in.
pub const ASPECT: f32 = 16. / 9.;

#[derive(Clone, Debug, PartialEq)]
pub enum Anchor {
    Globe,
    Moon,
}

/// One menu pill: its label, centre (frame fractions) and which sphere carries it.
#[derive(Clone, Debug, PartialEq)]
pub struct Pill {
    pub label: String,
    pub centre: [f32; 2],
    pub on: Anchor,
}

/// The menu as placed on the Pod: pills in the script's order, and the Polaroid when the
/// script drew the player icon.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PodMenu {
    pub pills: Vec<Pill>,
    pub polaroid: Option<[f32; 2]>,
}

fn collect(f: &Frame, labels: &mut Vec<String>, icon: &mut bool) {
    for c in &f.children {
        match c {
            Node::Frame(c) => collect(c, labels, icon),
            Node::Button {
                label, function, ..
            } => labels.push(if label.is_empty() {
                format!("[{}]", base_name(function))
            } else {
                label.clone()
            }),
            Node::Widget { function, .. } if base_name(function) == "DoPlayerBigIcon" => {
                *icon = true
            }
            _ => {}
        }
    }
}

/// Point on the arc from `a` to `b` at `t` in 0..=1. The arc bulges away from the globe
/// centre so the pills hug its right side; a straight line when the ends are too close.
fn arc(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    // Work in height units so the arc is round on screen.
    let to = |p: [f32; 2]| [p[0] * ASPECT, p[1]];
    let (a, b, c) = (to(a), to(b), to(GLOBE.0));
    let mid = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
    let out = [mid[0] - c[0], mid[1] - c[1]];
    let len = (out[0] * out[0] + out[1] * out[1]).sqrt();
    // Bulge: a fifth of the chord, outward from the globe centre.
    let chord = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let bulge = if len > 1e-4 { chord / 5. / len } else { 0. };
    let ctrl = [mid[0] + out[0] * bulge, mid[1] + out[1] * bulge];
    // Quadratic Bezier through the control point.
    let u = 1. - t;
    let p = [
        u * u * a[0] + 2. * u * t * ctrl[0] + t * t * b[0],
        u * u * a[1] + 2. * u * t * ctrl[1] + t * t * b[1],
    ];
    [p[0] / ASPECT, p[1]]
}

/// Places the script's buttons: all but the last down the globe's right side, the last
/// on the moon. One button goes to the globe's first pill position.
pub fn layout(root: &Frame) -> PodMenu {
    let (mut labels, mut icon) = (Vec::new(), false);
    collect(root, &mut labels, &mut icon);
    let mut pills = Vec::with_capacity(labels.len());
    let moon = (labels.len() > 1).then(|| labels.pop()).flatten();
    let n = labels.len();
    for (i, label) in labels.into_iter().enumerate() {
        let t = if n > 1 { i as f32 / (n - 1) as f32 } else { 0. };
        pills.push(Pill {
            label,
            centre: arc(PILL_ENDS[0], PILL_ENDS[1], t),
            on: Anchor::Globe,
        });
    }
    if let Some(label) = moon {
        pills.push(Pill {
            label,
            centre: MOON.0,
            on: Anchor::Moon,
        });
    }
    PodMenu {
        pills,
        polaroid: icon.then_some(POLAROID),
    }
}
