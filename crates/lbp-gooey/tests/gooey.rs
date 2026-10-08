//! U6 UI model: authored synthetic screens only.
use lbp_gooey::{
    navigate, settle_focus, Align, Builder, Direction, FrameInput, ImageRef, Layout, PanelStyle,
    Rect, TextStyle, UiTree, Uid, Widget,
};

fn screen(input: FrameInput) -> (UiTree, bool) {
    let mut b = Builder::new(Uid(1), input);
    b.text("Goodies", TextStyle::Title);
    let mut pressed = false;
    b.frame(
        Uid(2),
        Layout::Row,
        Align::Start,
        PanelStyle::Filled,
        4.,
        2.,
        |b| {
            pressed = b.button(Uid(10), "Materials").activated;
            b.button(Uid(11), "Objects");
            b.button_with(
                Uid(12),
                "Locked",
                Some(ImageRef::Swatch([255, 0, 0, 255])),
                false,
            );
        },
    );
    b.spacer(0., 8.);
    (b.finish(), pressed)
}

#[test]
fn u6_builder_records_tree_in_order_with_parents() {
    let (tree, pressed) = screen(FrameInput::default());
    assert!(!pressed);
    assert_eq!(tree.nodes.len(), 7);
    assert_eq!(tree.nodes[0].uid, Uid(1));
    assert_eq!(tree.nodes[0].children, vec![1, 2, 6]);
    assert_eq!(tree.nodes[2].children, vec![3, 4, 5]);
    assert_eq!(tree.nodes[4].parent, Some(2));
    assert!(matches!(tree.nodes[1].widget, Widget::Text { .. }));
    // Disabled buttons are not focusable.
    assert_eq!(tree.focusable(), vec![Uid(10), Uid(11)]);
    assert!(tree.duplicate_ids.is_empty());
}

#[test]
fn u6_input_answers_only_the_matching_enabled_widget() {
    let (_, pressed) = screen(FrameInput {
        focused: Some(Uid(10)),
        activated: Some(Uid(10)),
    });
    assert!(pressed);
    let (_, pressed) = screen(FrameInput {
        focused: None,
        activated: Some(Uid(11)),
    });
    assert!(!pressed);
    let mut b = Builder::new(
        Uid(1),
        FrameInput {
            focused: Some(Uid(5)),
            activated: Some(Uid(5)),
        },
    );
    let r = b.button_with(Uid(5), "off", None, false);
    assert!(!r.activated && !r.focused);
}

#[test]
fn u6_auto_ids_are_stable_and_distinct() {
    let (a, _) = screen(FrameInput::default());
    let (b, _) = screen(FrameInput {
        focused: Some(Uid(11)),
        activated: None,
    });
    let ids = |t: &UiTree| t.nodes.iter().map(|n| n.uid).collect::<Vec<_>>();
    assert_eq!(ids(&a), ids(&b));
    let mut sorted = ids(&a);
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), a.nodes.len());
    assert_eq!(a, b);
}

#[test]
fn u6_duplicate_ids_are_reported() {
    let mut b = Builder::new(Uid(1), FrameInput::default());
    b.button(Uid(3), "a");
    b.button(Uid(3), "b");
    let tree = b.finish();
    assert_eq!(tree.duplicate_ids, vec![Uid(3)]);
    assert_eq!(tree.nodes.len(), 3);
}

fn rect(x: f32, y: f32) -> Rect {
    Rect {
        x,
        y,
        width: 10.,
        height: 10.,
    }
}

#[test]
fn u6_navigation_moves_along_rows_and_columns() {
    // 2x2 grid plus one item far right on the top row.
    let grid = [
        (Uid(1), rect(0., 0.)),
        (Uid(2), rect(20., 0.)),
        (Uid(3), rect(0., 20.)),
        (Uid(4), rect(20., 20.)),
        (Uid(5), rect(80., 2.)),
    ];
    assert_eq!(navigate(&grid, None, Direction::Down), Some(Uid(1)));
    assert_eq!(
        navigate(&grid, Some(Uid(1)), Direction::Right),
        Some(Uid(2))
    );
    assert_eq!(navigate(&grid, Some(Uid(1)), Direction::Down), Some(Uid(3)));
    assert_eq!(
        navigate(&grid, Some(Uid(2)), Direction::Right),
        Some(Uid(5))
    );
    assert_eq!(navigate(&grid, Some(Uid(4)), Direction::Up), Some(Uid(2)));
    assert_eq!(navigate(&grid, Some(Uid(1)), Direction::Up), None);
    assert_eq!(
        navigate(&grid, Some(Uid(99)), Direction::Left),
        Some(Uid(1))
    );
    assert_eq!(navigate(&[], None, Direction::Left), None);
}

#[test]
fn u6_focus_settles_after_rebuild() {
    let (tree, _) = screen(FrameInput::default());
    assert_eq!(settle_focus(&tree, Some(Uid(11))), Some(Uid(11)));
    assert_eq!(settle_focus(&tree, Some(Uid(12))), Some(Uid(10)));
    assert_eq!(settle_focus(&tree, None), Some(Uid(10)));
    let empty = Builder::new(Uid(1), FrameInput::default()).finish();
    assert_eq!(settle_focus(&empty, Some(Uid(10))), None);
}
