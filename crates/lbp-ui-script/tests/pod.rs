//! P1b Pod menu placement on an authored frame tree (no game data).
use lbp_ui_script::pod::{layout, Anchor, MOON, PILL_ENDS, POLAROID};
use lbp_ui_script::{Frame, Node};

fn button(label: &str) -> Node {
    Node::Button {
        label: label.into(),
        function: "DoSpaceButtonNamed__iQ6String".into(),
        args: vec![],
    }
}

#[test]
fn p1b_buttons_go_down_the_globe_and_the_last_onto_the_moon() {
    let inner = Frame {
        start: "StartFrame".into(),
        props: Default::default(),
        children: vec![
            button("Story"),
            button("Community"),
            button("Recent Activity"),
            button("Store"),
            Node::Widget {
                function: "DoPlayerBigIcon__".into(),
                args: vec![],
            },
            button("Me"),
        ],
    };
    let root = Frame {
        start: "root".into(),
        props: Default::default(),
        children: vec![Node::Frame(inner)],
    };
    let menu = layout(&root);
    let labels: Vec<&str> = menu.pills.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Story", "Community", "Recent Activity", "Store", "Me"]
    );
    assert!(menu.pills[..4].iter().all(|p| p.on == Anchor::Globe));
    assert_eq!(menu.pills[4].on, Anchor::Moon);
    assert_eq!(menu.pills[4].centre, MOON.0);
    let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5;
    assert!(close(menu.pills[0].centre, PILL_ENDS[0]));
    assert!(close(menu.pills[3].centre, PILL_ENDS[1]));
    // The middle pills bulge right of the straight line between the ends, going down.
    for (i, p) in menu.pills[1..3].iter().enumerate() {
        let t = (i + 1) as f32 / 3.;
        let straight = PILL_ENDS[0][0] + (PILL_ENDS[1][0] - PILL_ENDS[0][0]) * t;
        assert!(p.centre[0] > straight, "{p:?}");
        assert!(p.centre[1] > menu.pills[i].centre[1]);
    }
    assert_eq!(menu.polaroid, Some(POLAROID));
}

#[test]
fn p1b_one_button_stays_on_the_globe() {
    let root = Frame {
        start: "root".into(),
        props: Default::default(),
        children: vec![button("Story")],
    };
    let menu = layout(&root);
    assert_eq!(menu.pills.len(), 1);
    assert_eq!(menu.pills[0].on, Anchor::Globe);
    assert_eq!(menu.polaroid, None);
}
