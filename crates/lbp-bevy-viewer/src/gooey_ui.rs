//! U6: Bevy adapter for the `lbp-gooey` UI model, plus the `--ui-demo` screen.
//!
//! Each frame the active screen rebuilds its `UiTree`; when the tree differs from the last
//! one, the bevy_ui entities are despawned and spawned again (menus are small, and focus is
//! kept by widget id, so a full rebuild is simpler than a keyed diff). Keyboard arrows move
//! focus with `lbp_gooey::navigate` over the laid-out rectangles; Enter/Space activate;
//! mouse hover focuses and click activates. Gamepad input is not wired (bevy_gilrs is not
//! enabled). Text uses Bevy's default font until game fonts (U2 atlas) are hooked up.
//! The demo content is authored and synthetic: no game data, not a copy of game screens.
use bevy::prelude::*;
use lbp_gooey::{
    navigate, settle_focus, Align, Builder, Direction, FrameInput, ImageRef, Layout, PanelStyle,
    Rect as UiRectangle, TextStyle, UiTree, Uid, Widget,
};
use std::collections::HashMap;

#[derive(Component, Clone, Copy)]
struct GooeyId(Uid);

#[derive(Resource, Default)]
struct GooeyHost {
    focused: Option<Uid>,
    activated: Option<Uid>,
    tree: UiTree,
    root: Option<Entity>,
    buttons: HashMap<Uid, Entity>,
    rebuilds: u64,
}

/// Authored demo state (synthetic, Popit-like tabs and an item grid).
#[derive(Resource, Default)]
struct DemoState {
    tab: usize,
    picked: Option<String>,
}

const TABS: [&str; 4] = ["Materials", "Objects", "Stickers", "Tools"];
const ROOT: Uid = Uid(0x5549_0000);
const TAB_BASE: u64 = 0x5549_1000;
const ITEM_BASE: u64 = 0x5549_2000;

fn demo_screen(input: FrameInput, state: &mut DemoState, rebuilds: u64) -> UiTree {
    let mut b = Builder::new(ROOT, input);
    b.frame(
        Uid(0x5549_0001),
        Layout::Column,
        Align::Start,
        PanelStyle::Filled,
        16.,
        10.,
        |b| {
            b.text(
                "UI layer demo (authored synthetic content, not game UI)",
                TextStyle::Title,
            );
            b.frame(
                Uid(0x5549_0002),
                Layout::Row,
                Align::Start,
                PanelStyle::Plain,
                0.,
                8.,
                |b| {
                    for (i, name) in TABS.iter().enumerate() {
                        if b.button(Uid(TAB_BASE + i as u64), *name).activated {
                            state.tab = i;
                        }
                    }
                },
            );
            b.frame(
                Uid(0x5549_0003),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Bubble,
                8.,
                8.,
                |b| {
                    for i in 0..12u64 {
                        let shade = (40 + i * 16) as u8;
                        let swatch = match state.tab {
                            0 => [shade, 90, 60, 255],
                            1 => [60, shade, 90, 255],
                            2 => [90, 60, shade, 255],
                            _ => [shade, shade, 60, 255],
                        };
                        let label = format!("{} {}", TABS[state.tab], i + 1);
                        let enabled = i != 11;
                        let uid = Uid(ITEM_BASE + state.tab as u64 * 0x100 + i);
                        if b.button_with(uid, label.clone(), Some(ImageRef::Swatch(swatch)), enabled)
                            .activated
                        {
                            state.picked = Some(label);
                        }
                    }
                },
            );
            b.text(
                format!(
                    "Picked: {}   |   arrows move, Enter activates, mouse works too   |   rebuilds {rebuilds}",
                    state.picked.as_deref().unwrap_or("nothing")
                ),
                TextStyle::Small,
            );
        },
    );
    b.finish()
}

pub fn run_ui_demo() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.045)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LBP2 Asset Explorer - UI layer demo".into(),
                resolution: (1280, 800).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<GooeyHost>()
        .init_resource::<DemoState>()
        .insert_resource(DemoCapture(demo_capture_path()))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (
                crate::place_on_second_monitor,
                gooey_input,
                gooey_rebuild,
                gooey_paint_focus,
                demo_capture,
            )
                .chain(),
        )
        .run();
}

fn gooey_input(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&GooeyId, &Interaction), Changed<Interaction>>,
    laid_out: Query<(&GooeyId, &ComputedNode, &UiGlobalTransform), With<Button>>,
    mut host: ResMut<GooeyHost>,
) {
    for (id, interaction) in &interactions {
        match interaction {
            Interaction::Pressed => {
                host.focused = Some(id.0);
                host.activated = Some(id.0);
            }
            Interaction::Hovered => host.focused = Some(id.0),
            Interaction::None => {}
        }
    }
    let direction = [
        (KeyCode::ArrowUp, Direction::Up),
        (KeyCode::ArrowDown, Direction::Down),
        (KeyCode::ArrowLeft, Direction::Left),
        (KeyCode::ArrowRight, Direction::Right),
    ]
    .into_iter()
    .find(|(key, _)| keys.just_pressed(*key))
    .map(|(_, direction)| direction);
    if let Some(direction) = direction {
        let focusable = host.tree.focusable();
        let targets: Vec<(Uid, UiRectangle)> = laid_out
            .iter()
            .filter(|(id, _, _)| focusable.contains(&id.0))
            .map(|(id, node, transform)| {
                let centre = transform.translation;
                (
                    id.0,
                    UiRectangle {
                        x: centre.x - node.size.x / 2.,
                        y: centre.y - node.size.y / 2.,
                        width: node.size.x,
                        height: node.size.y,
                    },
                )
            })
            .collect();
        if let Some(next) = navigate(&targets, host.focused, direction) {
            host.focused = Some(next);
        }
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter]) {
        host.activated = host.focused;
    }
}

fn gooey_rebuild(mut commands: Commands, mut host: ResMut<GooeyHost>, mut demo: ResMut<DemoState>) {
    let input = FrameInput {
        focused: host.focused,
        activated: host.activated.take(),
    };
    let tree = demo_screen(input, &mut demo, host.rebuilds);
    host.focused = settle_focus(&tree, host.focused);
    if tree == host.tree && host.root.is_some() {
        return;
    }
    if !tree.duplicate_ids.is_empty() {
        warn!("UI tree has duplicate widget ids: {:?}", tree.duplicate_ids);
    }
    if let Some(root) = host.root.take() {
        commands.entity(root).despawn();
    }
    host.buttons.clear();
    let mut buttons = HashMap::new();
    let root = spawn_node(&mut commands, &tree, 0, &mut buttons);
    commands.entity(root).insert(Node {
        width: Val::Percent(100.),
        height: Val::Percent(100.),
        padding: UiRect::all(Val::Px(24.)),
        ..default()
    });
    host.root = Some(root);
    host.buttons = buttons;
    host.tree = tree;
    host.rebuilds += 1;
}

fn font(style: TextStyle) -> TextFont {
    TextFont::from_font_size(match style {
        TextStyle::Title => 28.,
        TextStyle::Body => 18.,
        TextStyle::Small => 14.,
    })
}

fn swatch_color(image: ImageRef) -> Color {
    match image {
        ImageRef::Swatch([r, g, b, a]) => Color::srgba_u8(r, g, b, a),
        // Game textures are resolved by a later item; draw a labelled-neutral grey.
        ImageRef::Guid(_) | ImageRef::Hash(_) => Color::srgb(0.35, 0.35, 0.35),
    }
}

fn spawn_node(
    commands: &mut Commands,
    tree: &UiTree,
    index: usize,
    buttons: &mut HashMap<Uid, Entity>,
) -> Entity {
    let node = &tree.nodes[index];
    let entity = match &node.widget {
        Widget::Frame {
            layout,
            align,
            style,
            padding,
            gap,
        } => {
            let justify = match align {
                Align::Start => JustifyContent::FlexStart,
                Align::Center => JustifyContent::Center,
                Align::End => JustifyContent::FlexEnd,
                Align::Full => JustifyContent::SpaceBetween,
            };
            let background = match style {
                PanelStyle::Plain => Color::NONE,
                PanelStyle::Filled => Color::srgba(0.10, 0.13, 0.17, 0.95),
                PanelStyle::Bubble => Color::srgba(0.16, 0.20, 0.26, 0.95),
            };
            commands
                .spawn((
                    Node {
                        flex_direction: match layout {
                            Layout::Column => FlexDirection::Column,
                            Layout::Row | Layout::Wrap => FlexDirection::Row,
                        },
                        flex_wrap: if *layout == Layout::Wrap {
                            FlexWrap::Wrap
                        } else {
                            FlexWrap::NoWrap
                        },
                        justify_content: justify,
                        padding: UiRect::all(Val::Px(*padding)),
                        row_gap: Val::Px(*gap),
                        column_gap: Val::Px(*gap),
                        border_radius: BorderRadius::all(Val::Px(
                            if *style == PanelStyle::Bubble {
                                14.
                            } else {
                                4.
                            },
                        )),
                        ..default()
                    },
                    BackgroundColor(background),
                    GooeyId(node.uid),
                ))
                .id()
        }
        Widget::Text { text, style } => commands
            .spawn((Text::new(text.clone()), font(*style), GooeyId(node.uid)))
            .id(),
        Widget::Button {
            label,
            image,
            enabled,
        } => {
            let text_colour = if *enabled {
                Color::WHITE
            } else {
                Color::srgb(0.45, 0.45, 0.45)
            };
            let button = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.)),
                        row_gap: Val::Px(4.),
                        border: UiRect::all(Val::Px(3.)),
                        border_radius: BorderRadius::all(Val::Px(6.)),
                        min_width: Val::Px(96.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
                    BorderColor::all(Color::NONE),
                    Button,
                    GooeyId(node.uid),
                ))
                .id();
            if let Some(image) = image {
                let swatch = commands
                    .spawn((
                        Node {
                            width: Val::Px(56.),
                            height: Val::Px(56.),
                            border_radius: BorderRadius::all(Val::Px(4.)),
                            ..default()
                        },
                        BackgroundColor(swatch_color(*image)),
                    ))
                    .id();
                commands.entity(button).add_child(swatch);
            }
            let text = commands
                .spawn((
                    Text::new(label.clone()),
                    font(TextStyle::Body),
                    TextColor(text_colour),
                ))
                .id();
            commands.entity(button).add_child(text);
            if *enabled {
                buttons.insert(node.uid, button);
            }
            button
        }
        Widget::Image {
            image,
            width,
            height,
        } => commands
            .spawn((
                Node {
                    width: Val::Px(*width),
                    height: Val::Px(*height),
                    ..default()
                },
                BackgroundColor(swatch_color(*image)),
                GooeyId(node.uid),
            ))
            .id(),
        Widget::Spacer { width, height } => commands
            .spawn((
                Node {
                    width: Val::Px(*width),
                    height: Val::Px(*height),
                    ..default()
                },
                GooeyId(node.uid),
            ))
            .id(),
    };
    for &child in &node.children {
        let child = spawn_node(commands, tree, child, buttons);
        commands.entity(entity).add_child(child);
    }
    entity
}

fn gooey_paint_focus(host: Res<GooeyHost>, mut borders: Query<(&GooeyId, &mut BorderColor)>) {
    for (id, mut border) in &mut borders {
        let colour = if host.focused == Some(id.0) {
            Color::srgb(1.0, 0.78, 0.2)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u6_demo_screen_ids_unique_and_tabs_switch_items() {
        let mut state = DemoState::default();
        let tree = demo_screen(FrameInput::default(), &mut state, 0);
        assert!(tree.duplicate_ids.is_empty());
        // 4 tabs + 11 enabled items are focusable.
        assert_eq!(tree.focusable().len(), 15);
        let tab = Uid(TAB_BASE + 2);
        demo_screen(
            FrameInput {
                focused: Some(tab),
                activated: Some(tab),
            },
            &mut state,
            1,
        );
        assert_eq!(state.tab, 2);
        let item = Uid(ITEM_BASE + 2 * 0x100 + 3);
        let tree = demo_screen(FrameInput::default(), &mut state, 2);
        assert!(tree.find(item).is_some());
        demo_screen(
            FrameInput {
                focused: Some(item),
                activated: Some(item),
            },
            &mut state,
            3,
        );
        assert_eq!(state.picked.as_deref(), Some("Stickers 4"));
        // The disabled last item never activates.
        let locked = Uid(ITEM_BASE + 2 * 0x100 + 11);
        demo_screen(
            FrameInput {
                focused: Some(locked),
                activated: Some(locked),
            },
            &mut state,
            4,
        );
        assert_eq!(state.picked.as_deref(), Some("Stickers 4"));
    }
}

/// `--qa-capture NEW_PNG` with `--ui-demo`: screenshot the demo after it settles, then exit.
#[derive(Resource)]
struct DemoCapture(Option<std::path::PathBuf>);

fn demo_capture_path() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let value = args
        .iter()
        .position(|a| a == "--qa-capture")
        .and_then(|i| args.get(i + 1))?;
    let base = std::fs::canonicalize("local-analysis").ok()?;
    let path = std::path::PathBuf::from(value);
    let parent = std::fs::canonicalize(path.parent()?).ok()?;
    if !parent.starts_with(base) || path.exists() || path.extension().is_none_or(|e| e != "png") {
        eprintln!("Capture must be a new PNG under local-analysis");
        return None;
    }
    Some(parent.join(path.file_name()?))
}

fn demo_capture(
    mut commands: Commands,
    mut capture: ResMut<DemoCapture>,
    time: Res<Time>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames > 120 && time.elapsed_secs() > 3. {
        if let Some(path) = capture.0.take() {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(path))
                .observe(
                    |_: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                     mut exit: MessageWriter<AppExit>| {
                        exit.write(AppExit::Success);
                    },
                );
        }
    }
}
