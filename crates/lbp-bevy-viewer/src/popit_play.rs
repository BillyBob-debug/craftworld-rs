//! `--play-sackboy` Popit overlay (D235, docs/d235-popit.md). The game's Popit is the Fish
//! script `poppet.ff` on the Gooey UI: the card is a gradient frame holding 128-unit SDF icon
//! buttons in a centred wrapping grid, the focused button is enlarged on a teal tile and its
//! tooltip is the label pill; "Close Popit" is the bottom prompt. Open/close takes 10 ticks
//! with a smoothstep (EBOOT `SetOpen` 0x515da8, easing 0x4099ac).
//!
//! Item order and icon sheets are FOUND in the script (create mode 6,1,8 / 4,2,0 / 7,5,3 with
//! the cursor focused; play mode 6 / 4,0 / 7,3 with the costume button focused). Screen
//! sizes, colours, label placement and the camera nudge are measured on RPCS3 1280x720 frames
//! (docs/d235-popit.md): they are screen-space measurements, not game constants.
//!
//! Keys: Z or the West (Square) pad button opens and closes it; while it is open the arrow
//! keys move the focus instead of walking. `--popit-mode edit|play` picks the menu (default
//! edit, the Moon's create mode). `--popit-open` starts it open (QA captures).
use bevy::prelude::*;
use bevy::ui::{ColorStop, Gradient, LinearGradient};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::{decode_icon, load_strings, ResourceKey};

/// Seconds for the open/close blend: 10 ticks at 30 Hz.
const BLEND_SECONDS: f32 = 10. / 30.;
/// The sheets hold the shape at ~0.1..0.3 and 0 outside (measured), so the silhouette edge
/// is at 10/255 with 3/255 softness: `HUD.SDF_INFO(10, 127, 3)` (127 is INFERRED to be the
/// inner-detail level, 255 in the face).
const SDF_EDGE: f32 = 10. / 255.;
const SDF_SOFT: f32 = 3. / 255.;
const FRAME_H: f32 = 720.;

/// (button id, atlas texture GUID, cell px, frame count, tooltip LAMS key, English fallback).
const ITEMS: [(u32, u32, u32, u32, u32, &str); 10] = [
    (0, 0x184cc, 64, 25, 1752658457, "Stickers & Decorations"),
    (1, 0x18bdf, 128, 29, 1409650159, "Goodies Bag"),
    (2, 0x184c6, 64, 32, 3398591419, "Popit Cursor"),
    (3, 0x19075, 128, 8, 2641668085, "Hold To Retry!"),
    (4, 0x184ca, 64, 16, 3186033225, "Personal"),
    (5, 0x190e0, 128, 16, 1915210447, "Tools Bag"),
    (6, 0x19149, 128, 32, 1033526369, "Customize Character"),
    (7, 0x18be7, 128, 32, 1859424545, "Text Chat"),
    (8, 0x1e1f8, 128, 64, 1318692229, "Play Next Tutorial"),
    (10, 0x1ff9d, 128, 32, 1265163842, "Pod Decoration"),
];
const GLYPH_SHEET: u32 = 0x20c4;
const CLOSE_POPIT_KEY: u32 = 0x9c2a_eec8;

/// Create-mode and play-mode rows (script order, `DoBreak` rows; FOUND), the default focus
/// and the measured card size in 1280x720 pixels.
const EDIT_ROWS: &[&[u32]] = &[&[6, 1, 8], &[4, 2, 0], &[7, 5, 3]];
const PLAY_ROWS: &[&[u32]] = &[&[6], &[4, 0], &[7, 3]];
const EDIT_CARD: [f32; 2] = [170., 177.];
const PLAY_CARD: [f32; 2] = [109., 146.];
/// Cell pitch and sizes (measured, 720p px): icon 128 units shown at 0.323 px/unit.
const PITCH: f32 = 46.5;
const ICON_PX: f32 = 41.;
const FOCUS_TILE_PX: f32 = 68.;
const FOCUS_ICON_PX: f32 = 56.;
/// Card bottom sits this far above Sackboy's head (px at 720p, measured).
const ABOVE_HEAD: f32 = 40.;
/// Open camera: dolly in and shift the aim so the scene drops by 58 px (measured, create mode).
const POPIT_ZOOM: f32 = 1.1;
const POPIT_AIM_DROP: f32 = 0.0879;

const CARD_TOP: Color = Color::srgba(0.752, 0.382, 0.506, 0.92);
const CARD_BOTTOM: Color = Color::srgba(0.737, 0.565, 0.380, 0.92);
const TEAL: Color = Color::srgb(0.271, 0.796, 0.722);
const PILL: Color = Color::srgb(0.808, 0.706, 0.141);
const PROMPT: Color = Color::srgba(0.745, 0.647, 0.118, 0.72);
const CORD: Color = Color::srgb(0.84, 0.74, 0.16);
const CORD_DOTS: usize = 30;

/// One main-menu button: its animated icon (every frame as white-on-alpha) and its label.
pub(crate) struct Item {
    id: u32,
    frames: Vec<Image>,
    label: String,
}

/// Decoded game data for the overlay, built on the load worker.
#[derive(Resource)]
pub(crate) struct PopitAssets {
    items: Vec<Item>,
    square: Image,
    close: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Edit,
    Play,
}

/// Runtime state: open flag, blend 0..1 (linear, eased on use), focus (button id), frame clock.
#[derive(Resource)]
struct Popit {
    mode: Mode,
    open: bool,
    blend: f32,
    focus: u32,
    clock: f32,
    spawned: bool,
}

#[derive(Component)]
struct Part(Role);

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Card,
    Tile,
    Icon(u32),
    Label,
    LabelText,
    Prompt,
    PromptText,
    Glyph,
    Cord(usize),
}

#[derive(Resource)]
struct IconHandles(Vec<(u32, Vec<Handle<Image>>)>);

/// White icon with the SDF value as coverage.
fn sdf_cell(rgba: &[u8], sheet_w: u32, x0: u32, y0: u32, cell: u32) -> Image {
    let mut out = Vec::with_capacity((cell * cell * 4) as usize);
    for y in 0..cell {
        for x in 0..cell {
            let i = (((y0 + y) * sheet_w + x0 + x) * 4) as usize;
            let v = rgba[i] as f32 / 255.;
            let t = ((v - (SDF_EDGE - SDF_SOFT)) / (2. * SDF_SOFT)).clamp(0., 1.);
            let a = t * t * (3. - 2. * t);
            out.extend_from_slice(&[255, 255, 255, (a * 255.) as u8]);
        }
    }
    image_from(cell, cell, out)
}

fn image_from(w: u32, h: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

/// Reads every atlas, the tooltips and the Square glyph (load worker).
pub(crate) fn load_assets(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &LoadControl,
) -> Result<PopitAssets, String> {
    let (strings, _) = load_strings(source, catalog, control);
    let mut items = Vec::new();
    for (id, guid, cell, frames, key, english) in ITEMS {
        let icon = decode_icon(source, catalog, ResourceKey::Guid(guid), control)?;
        let cols = icon.width / cell;
        let mut out = Vec::new();
        for f in 0..frames {
            out.push(sdf_cell(
                &icon.rgba,
                icon.width,
                (f % cols) * cell,
                (f / cols) * cell,
                cell,
            ));
        }
        // The French/other tables do not matter; English comes from the base table.
        let label = strings
            .get(key)
            .map_or_else(|| english.to_owned(), str::to_owned);
        items.push(Item {
            id,
            frames: out,
            label,
        });
    }
    let sheet = decode_icon(source, catalog, ResourceKey::Guid(GLYPH_SHEET), control)?;
    // GI_SQUARE: column 3, row 0 of the 64 px cells.
    let mut square = Vec::new();
    for y in 0..64u32 {
        for x in 0..64u32 {
            let i = ((y * sheet.width + 192 + x) * 4) as usize;
            square.extend_from_slice(&sheet.rgba[i..i + 4]);
        }
    }
    let close = strings
        .get(CLOSE_POPIT_KEY)
        .map_or_else(|| "Close Popit".to_owned(), str::to_owned);
    Ok(PopitAssets {
        items,
        square: image_from(64, 64, square),
        close,
    })
}

pub(crate) fn install(app: &mut App, assets: PopitAssets) {
    let all: Vec<String> = std::env::args().collect();
    let play = all
        .windows(2)
        .any(|w| w[0] == "--popit-mode" && w[1] == "play");
    let mode = if play { Mode::Play } else { Mode::Edit };
    app.insert_resource(assets)
        .insert_resource(Popit {
            mode,
            open: all.iter().any(|a| a == "--popit-open"),
            blend: if all.iter().any(|a| a == "--popit-open") {
                1.
            } else {
                0.
            },
            focus: if play { 6 } else { 2 },
            clock: 0.,
            spawned: false,
        })
        .add_systems(
            Update,
            (popit_input, popit_layout)
                .chain()
                .after(crate::sackboy_play::show_sackboy),
        );
}

fn rows(mode: Mode) -> &'static [&'static [u32]] {
    if mode == Mode::Edit {
        EDIT_ROWS
    } else {
        PLAY_ROWS
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn popit_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    time: Res<Time>,
    mut p: ResMut<Popit>,
    mut sackboy: Query<&mut lbp_sim::character::SackboyBody>,
) {
    let pad = pads.iter().next();
    if keys.just_pressed(KeyCode::KeyZ) || pad.is_some_and(|g| g.just_pressed(GamepadButton::West))
    {
        p.open = !p.open;
    }
    p.clock += time.delta_secs();
    let step = time.delta_secs() / BLEND_SECONDS;
    p.blend = if p.open {
        (p.blend + step).min(1.)
    } else {
        (p.blend - step).max(0.)
    };
    if !p.open {
        return;
    }
    // Arrow keys / d-pad move the focus on the grid (the game moves its cursor with the
    // d-pad while the Popit is open); walking input is dropped meanwhile.
    for mut body in &mut sackboy {
        body.input = Default::default();
    }
    let dir = |k: KeyCode, b: GamepadButton| {
        keys.just_pressed(k) || pad.is_some_and(|g| g.just_pressed(b))
    };
    let (dx, dy) = (
        i32::from(dir(KeyCode::ArrowRight, GamepadButton::DPadRight))
            - i32::from(dir(KeyCode::ArrowLeft, GamepadButton::DPadLeft)),
        i32::from(dir(KeyCode::ArrowDown, GamepadButton::DPadDown))
            - i32::from(dir(KeyCode::ArrowUp, GamepadButton::DPadUp)),
    );
    if dx == 0 && dy == 0 {
        return;
    }
    let grid = rows(p.mode);
    let (mut r, mut c) = (0usize, 0usize);
    for (ri, row) in grid.iter().enumerate() {
        if let Some(ci) = row.iter().position(|&i| i == p.focus) {
            (r, c) = (ri, ci);
        }
    }
    let r2 = (r as i32 + dy).clamp(0, grid.len() as i32 - 1) as usize;
    let c2 = (c as i32 + dx).clamp(0, grid[r2].len() as i32 - 1) as usize;
    p.focus = grid[r2][c2];
    p.clock = 0.;
}

#[allow(clippy::too_many_arguments)]
fn popit_layout(
    mut commands: Commands,
    mut p: ResMut<Popit>,
    assets: Res<PopitAssets>,
    mut images: ResMut<Assets<Image>>,
    handles: Option<Res<IconHandles>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    figure: Query<&GlobalTransform, With<crate::sackboy_play::Figure>>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    mut parts: Query<(&Part, &mut Node, &mut Visibility)>,
    mut images_ui: Query<(&Part, &mut ImageNode)>,
    mut texts: Query<(&Part, &mut Text)>,
    mut colours: Query<(&Part, &mut BackgroundColor)>,
    mut orbit: ResMut<crate::Orbit>,
    mut applied: Local<(f32, f32)>,
) {
    // Camera nudge, relative to what we applied last frame so zoom/orbit stay free.
    let e = smooth(p.blend);
    let (prev_zoom, prev_drop) = *applied;
    let zoom = 1. + (1. / POPIT_ZOOM - 1.) * e;
    let drop = POPIT_AIM_DROP * e;
    if prev_zoom != 0. {
        orbit.distance *= zoom / prev_zoom;
    } else {
        orbit.distance *= zoom;
    }
    // aim drop acts along the camera's up axis; the follow target is rewritten each frame by
    // the play system, so apply the offset (in display units) on top of it.
    orbit.target.y += drop * orbit.distance - 0.;
    let _ = prev_drop;
    *applied = (zoom, drop);

    if !p.spawned {
        // First frame: upload the frames and build the nodes.
        let mut hs = Vec::new();
        for item in &assets.items {
            let v: Vec<Handle<Image>> = item.frames.iter().map(|f| images.add(f.clone())).collect();
            hs.push((item.id, v));
        }
        let square = images.add(assets.square.clone());
        spawn_nodes(&mut commands, &assets, &hs, square);
        commands.insert_resource(IconHandles(hs));
        p.spawned = true;
        return;
    }
    let (Some(handles), Ok(window)) = (handles, windows.single()) else {
        return;
    };
    let (Ok((camera, cam_tf)), Ok(fig), Some(display)) =
        (cameras.single(), figure.single(), display)
    else {
        return;
    };
    let scale = window.height() / FRAME_H;
    // Sackboy's head top on screen (figure origin = feet).
    let s = display.0.x_axis.length();
    let head_world = fig.translation() + Vec3::Y * 200. * s;
    let Ok(head) = camera.world_to_viewport(cam_tf, head_world) else {
        return;
    };
    let _ = cam_tf;
    let hidden = p.blend <= 0.;
    let card = if p.mode == Mode::Edit {
        EDIT_CARD
    } else {
        PLAY_CARD
    };
    let (cw, ch) = (card[0] * scale, card[1] * scale);
    let grow = e.max(0.001);
    let cx = head.x;
    let bottom = head.y - ABOVE_HEAD * scale;
    let (w, h) = (cw * grow, ch * grow);
    let (left, top) = (cx - w / 2., bottom - h);
    let focus_pos = {
        let grid = rows(p.mode);
        let mut found = (0usize, 0usize, 1usize);
        for (ri, row) in grid.iter().enumerate() {
            if let Some(ci) = row.iter().position(|&i| i == p.focus) {
                found = (ri, ci, row.len());
            }
        }
        found
    };
    // Cell centre in card-relative px (top-left origin at 720p).
    let cell_centre = |ri: usize, ci: usize, n: usize| -> [f32; 2] {
        let rows_n = rows(p.mode).len() as f32;
        let y = card[1] / 2. + (ri as f32 - (rows_n - 1.) / 2.) * PITCH + 4.5 - 4.5;
        let x = card[0] / 2. + (ci as f32 - (n as f32 - 1.) / 2.) * PITCH;
        [x, y]
    };
    let px = |v: f32| Val::Px(v * scale);
    let set = |node: &mut Node, l: f32, t: f32, w: f32, h: f32| {
        node.left = Val::Px(l);
        node.top = Val::Px(t);
        node.width = Val::Px(w);
        node.height = Val::Px(h);
    };
    let _ = px;
    for (part, mut node, mut vis) in &mut parts {
        *vis = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        match part.0 {
            Role::Card => {
                set(&mut node, left, top, w, h);
                node.border_radius = BorderRadius::all(Val::Px(21. * scale * grow));
            }
            Role::Tile => {
                let c = cell_centre(focus_pos.0, focus_pos.1, focus_pos.2);
                let sz = FOCUS_TILE_PX * scale * grow;
                set(
                    &mut node,
                    left + c[0] * scale * grow - sz / 2.,
                    top + c[1] * scale * grow - sz / 2.,
                    sz,
                    sz,
                );
                node.border_radius = BorderRadius::all(Val::Px(sz * 0.3));
            }
            Role::Icon(id) => {
                let grid = rows(p.mode);
                let mut place = None;
                for (ri, row) in grid.iter().enumerate() {
                    if let Some(ci) = row.iter().position(|&i| i == id) {
                        place = Some((ri, ci, row.len()));
                    }
                }
                if let Some((ri, ci, n)) = place {
                    let c = cell_centre(ri, ci, n);
                    let focused = id == p.focus;
                    let sz = if focused { FOCUS_ICON_PX } else { ICON_PX } * scale * grow;
                    set(
                        &mut node,
                        left + c[0] * scale * grow - sz / 2.,
                        top + c[1] * scale * grow - sz / 2.,
                        sz,
                        sz,
                    );
                } else {
                    *vis = Visibility::Hidden;
                }
            }
            Role::Label => {
                let c = cell_centre(focus_pos.0, focus_pos.1, focus_pos.2);
                let text_w = assets
                    .items
                    .iter()
                    .find(|i| i.id == p.focus)
                    .map_or(100., |i| i.label.chars().count() as f32 * 12.8 + 20.);
                let (lw, lh) = (text_w * scale, 27. * scale);
                let above = focus_pos.0 == 0;
                let ly = if above {
                    top - 30. * scale - lh + 27. * scale
                } else {
                    bottom + 14. * scale
                };
                let ly = if above { top - 30. * scale } else { ly };
                set(
                    &mut node,
                    left + c[0] * scale * grow - lw / 2. + 3. * scale,
                    ly,
                    lw,
                    lh,
                );
                node.border_radius = BorderRadius::all(Val::Px(lh / 2.));
            }
            Role::Prompt => {
                let (pw, ph) = (165. * scale, 27. * scale);
                set(
                    &mut node,
                    window.width() / 2. - pw / 2. + 2. * scale,
                    window.height() - 75. * scale,
                    pw,
                    ph,
                );
                node.border_radius = BorderRadius::all(Val::Px(ph / 2.));
            }
            Role::Glyph => {
                let sz = 22. * scale;
                set(
                    &mut node,
                    window.width() / 2. - 82.5 * scale + 6. * scale,
                    window.height() - 72.5 * scale,
                    sz,
                    sz,
                );
            }
            Role::LabelText | Role::PromptText => {}
            Role::Cord(i) => {
                let t = (i as f32 + 1.) / CORD_DOTS as f32;
                // Cubic S-curve from the card's bottom edge to Sackboy's chest (measured).
                let p0 = Vec2::new(cx + 8. * scale, bottom);
                let p1 = p0 + Vec2::new(26., 42.) * scale;
                let p2 = p0 + Vec2::new(-34., 82.) * scale;
                let p3 = Vec2::new(cx + 18. * scale, head.y + 30. * scale);
                let u = 1. - t;
                let pt =
                    u * u * u * p0 + 3. * u * u * t * p1 + 3. * u * t * t * p2 + t * t * t * p3;
                let sz = 3.5 * scale;
                set(&mut node, pt.x - sz / 2., pt.y - sz / 2., sz, sz);
                // The cord grows out of the card with the blend.
                if t > e {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    // Icon frames: focused icon animates, the rest rest on their last frame.
    for (part, mut img) in &mut images_ui {
        if let Role::Icon(id) = part.0 {
            if let Some((_, frames)) = handles.0.iter().find(|(i, _)| *i == id) {
                let idx = if id == p.focus {
                    ((p.clock * 30.) as usize) % frames.len()
                } else {
                    frames.len() - 1
                };
                img.image = frames[idx].clone();
            }
        }
    }
    for (part, mut text) in &mut texts {
        match part.0 {
            Role::LabelText => {
                if let Some(i) = assets.items.iter().find(|i| i.id == p.focus) {
                    if text.0 != i.label {
                        text.0 = i.label.clone();
                    }
                }
            }
            Role::PromptText if text.0 != assets.close => {
                text.0 = assets.close.clone();
            }
            _ => {}
        }
    }
    for (part, mut bg) in &mut colours {
        if part.0 == Role::Label || part.0 == Role::Prompt {
            let a = e;
            bg.0 = if part.0 == Role::Label {
                PILL.with_alpha(a)
            } else {
                PROMPT.with_alpha(PROMPT.alpha() * a)
            };
        }
    }
}

fn spawn_nodes(
    commands: &mut Commands,
    assets: &PopitAssets,
    handles: &[(u32, Vec<Handle<Image>>)],
    square: Handle<Image>,
) {
    let abs = || Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    commands.spawn((
        Part(Role::Card),
        abs(),
        BackgroundGradient(vec![Gradient::Linear(LinearGradient {
            angle: LinearGradient::TO_BOTTOM,
            stops: vec![
                ColorStop::new(CARD_TOP, Val::Percent(0.)),
                ColorStop::new(CARD_BOTTOM, Val::Percent(100.)),
            ],
            ..default()
        })]),
        GlobalZIndex(40),
        Visibility::Hidden,
    ));
    commands.spawn((
        Part(Role::Tile),
        abs(),
        BackgroundColor(TEAL),
        GlobalZIndex(41),
        Visibility::Hidden,
    ));
    for (id, frames) in handles {
        commands.spawn((
            Part(Role::Icon(*id)),
            abs(),
            ImageNode::new(frames[frames.len() - 1].clone()),
            GlobalZIndex(42),
            Visibility::Hidden,
        ));
    }
    commands
        .spawn((
            Part(Role::Label),
            abs(),
            BackgroundColor(PILL),
            GlobalZIndex(43),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Part(Role::LabelText),
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(15.),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.),
                    top: Val::Px(4.),
                    ..default()
                },
            ));
        });
    commands
        .spawn((
            Part(Role::Prompt),
            abs(),
            BackgroundColor(PROMPT),
            GlobalZIndex(43),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Part(Role::PromptText),
                Text::new(assets.close.clone()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(15.),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(40.),
                    top: Val::Px(4.),
                    ..default()
                },
            ));
        });
    commands.spawn((
        Part(Role::Glyph),
        abs(),
        ImageNode::new(square).with_color(Color::srgb(0.88, 0.55, 0.65)),
        BackgroundColor(Color::srgba(0.08, 0.06, 0.06, 0.85)),
        GlobalZIndex(44),
        Visibility::Hidden,
    ));
    for i in 0..CORD_DOTS {
        commands.spawn((
            Part(Role::Cord(i)),
            abs(),
            BackgroundColor(CORD),
            GlobalZIndex(39),
            Visibility::Hidden,
        ));
    }
}
