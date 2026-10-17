//! `--play-sackboy` Popit overlay (D235, docs/d235-popit.md). The game's Popit is the Fish
//! script `poppet.ff` on the Gooey UI: the card is a gradient frame holding 128-unit SDF icon
//! buttons in a centred wrapping grid, the focused button is enlarged on a teal tile and its
//! tooltip is the label pill; "Close Popit" is the bottom prompt. Open/close takes 10 ticks
//! with a smoothstep (EBOOT `SetOpen` 0x515da8, easing 0x4099ac).
//!
//! D400: the card comes from running the game's own `Poppet.DoMainMenu` on the Gooey host
//! (`lbp_ui_script::popit`): rows, icons (animated textures: sheet, frames, cell), tooltip
//! keys, default focus and the cell pitch (icon 128 + child spacing 16 units). The constants
//! below (FOUND in the script by hand: create mode 6,1,8 / 4,2,0 / 7,5,3 with the cursor
//! focused; play mode 6 / 4,0 / 7,3 with the costume button focused) are the fallback when
//! the scripts are not in the catalog, and must agree with the script run. Screen
//! sizes, colours, label placement and the camera nudge are measured on RPCS3 1280x720 frames
//! (docs/d235-popit.md): they are screen-space measurements, not game constants.
//!
//! Keys: Z or the West (Square) pad button opens and closes it; while it is open the arrow
//! keys move the focus instead of walking. `--popit-mode edit|play` picks the menu (default
//! edit, the Moon's create mode). `--popit-open` starts it open (QA captures).
use bevy::prelude::*;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::{decode_icon, load_strings, ResourceKey};
use std::collections::BTreeMap;

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
/// Play mode in our Pod (D402, `DoMainMenuPlay`): Pod Decoration joins row 1, Retry goes.
const PLAY_ROWS_POD: &[&[u32]] = &[&[6, 10], &[4, 0], &[7]];
const EDIT_CARD: [f32; 2] = [170., 177.];
const PLAY_CARD: [f32; 2] = [109., 146.];
/// Cell pitch and sizes (measured, 720p px): icon 128 units shown at 0.323 px/unit.
const PITCH: f32 = 46.5;
/// 720p px per Gooey unit on the card (MEASURED: the 46.5 px pitch over 128 + 16 units).
const UNIT_PX: f32 = PITCH / 144.;
const ICON_PX: f32 = 41.;
const FOCUS_TILE_PX: f32 = 68.;
const FOCUS_ICON_PX: f32 = 56.;
/// Card bottom sits this far above Sackboy's head (px at 720p, measured).
const ABOVE_HEAD: f32 = 40.;
/// Open camera: dolly in and shift the aim so the scene drops by 58 px (measured, create mode).
const POPIT_ZOOM: f32 = 1.1;
const POPIT_AIM_DROP: f32 = 0.0879;

/// Card colour ramp (MEASURED on RPCS3 create-mode frames): the fill is a function of the
/// distance `t` from the tail's attach point on the card's bottom edge, 0 at the attach point
/// (the player-colour yellow) and 1 at the farthest top corner (pink). Linear between stops.
const CARD_RAMP: [(f32, [u8; 3]); 20] = [
    (0.00, [205, 178, 40]),
    (0.05, [202, 173, 49]),
    (0.10, [199, 168, 59]),
    (0.15, [196, 164, 68]),
    (0.20, [193, 160, 77]),
    (0.25, [190, 156, 85]),
    (0.30, [186, 153, 95]),
    (0.35, [185, 149, 100]),
    (0.40, [183, 145, 107]),
    (0.45, [182, 142, 111]),
    (0.50, [181, 138, 116]),
    (0.55, [181, 134, 120]),
    (0.60, [180, 130, 123]),
    (0.65, [181, 126, 126]),
    (0.70, [181, 121, 129]),
    (0.75, [181, 117, 133]),
    (0.80, [181, 112, 136]),
    (0.85, [183, 105, 138]),
    (0.90, [182, 102, 142]),
    (1.00, [183, 100, 143]),
];
/// Where the tail attaches along the card's bottom edge (fraction of the width, measured).
const CARD_ATTACH_X: f32 = 0.547;
const PLAY_ATTACH_X: f32 = 0.51;
const CARD_RADIUS: f32 = 24.;
/// Tooltip pill: height, text size, per-character width estimate (px at 720p, measured).
const PILL_H: f32 = 30.;
const PILL_CHAR_W: f32 = 13.;
const TAIL_LEN_V: f32 = 13.;
const TAIL_LEN_H: f32 = 8.;
const TAIL_STEPS: usize = 5;
const TEAL: Color = Color::srgb(0.271, 0.796, 0.722);
const PILL: Color = Color::srgb(0.808, 0.706, 0.141);
const PROMPT: Color = Color::srgba(0.745, 0.647, 0.118, 0.72);
const CORD: Color = Color::srgb(0.84, 0.74, 0.16);
const CORD_DOTS: usize = 70;

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
    /// Create and play cards: rows of button ids, default focus, pitch (720p px).
    cards: [(Vec<Vec<u32>>, u32, f32); 2],
    /// Where the cards came from (the script run, or the fallback constants and why).
    card_source: String,
    square: Image,
    close: String,
    pages: Vec<PageData>,
    /// D401: create and play mode, button -> the inventory collection it opens.
    opens: [BTreeMap<u32, i32>; 2],
}

/// One inventory page behind a main-menu button: title and the first icons by category.
struct PageData {
    id: u32,
    title: String,
    /// Play-mode title of the Personal page ("Heart Items", the collection's second page).
    heart_title: String,
    sections: Vec<(String, Vec<(String, Image)>)>,
    message: Option<&'static str>,
    /// One-column list page (Tools = the "Sounds" list): rows of (label, icon).
    list: Vec<(String, Image)>,
    /// Personal page: the function tiles (label, icon) in reading order, 6 per row.
    functions: Vec<(String, Image)>,
    /// Personal page: the focusable "Gameplay Kits" header's items (label, icon).
    kits: Vec<(String, Image)>,
    /// D404: the collection's page titles in order, and the index of the page drawn above
    /// (the measured profile's open page; the other pages show their title only).
    tabs: Vec<String>,
    home: usize,
}

impl PageData {
    /// The pages L1/R1 can reach. MEASURED 2026-10-12: play mode's Personal collection shows
    /// "Heart Items" with no page counter, so it is the only visible page there. Elsewhere all
    /// default pages are kept: the create-mode Tools Bag counts 8 (MEASURED "5/8", 18:12:52),
    /// though the game hides empty pages of custom type 0 (FOUND, filter 0x471a80).
    fn visible(&self, mode: Mode) -> Vec<usize> {
        if mode == Mode::Play && self.id == PERSONAL_BUTTON {
            vec![self.tabs.len().saturating_sub(1)]
        } else if self.tabs.is_empty() {
            vec![self.home]
        } else {
            (0..self.tabs.len()).collect()
        }
    }

    /// The open page: the collection's kept page, else the profile's saved one (MEASURED).
    fn current(&self, p: &Popit) -> usize {
        p.tabs.get(&self.id).copied().unwrap_or(
            if p.mode == Mode::Play && self.id == PERSONAL_BUTTON {
                self.tabs.len().saturating_sub(1)
            } else {
                self.home
            },
        )
    }
}

/// Inventory pages by collection (`InventoryCollection.EInventoryCollection`, the argument
/// of `Poppet.PushSubModeInventory`): (collection, title LAMS key, English fallback, plan
/// page, empty message). The title is the open page's own key (D402: the EBOOT's default
/// profile pages, `GetTitleKey`); which page is open is the profile's (MEASURED). The
/// contents are drawn by the native `DoInventoryGrid` (MEASURED).
type PageRow = (
    i32,
    u32,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
);
const PAGES: [PageRow; 6] = [
    (0, 0x9843_f40a, "Body", Some("costume"), None),
    (1, 0xdae7_f79a, "Stickers", Some("sticker"), None),
    // MEASURED 2026-10-11 18:50: Goodies Bag opens "Community Objects", Personal opens
    // "Tools and Used Objects", Tools Bag opens the Sounds list.
    (
        2,
        0xfead_43d3,
        "Community Objects",
        None,
        Some("Get your friends to send you objects, and they'll appear here."),
    ),
    (3, 0x3bd3_366a, "Tools", None, None),
    (4, 0x5036_ae03, "Tools and Used Objects", None, None),
    (8, 0x15f8_630f, "My Pods", None, None),
];
/// D404: each collection's pages in order, (title LAMS key, English fallback), from the
/// EBOOT's default profile builder (FOUND, 0x465688; the shared pages' order is creation
/// order, INFERRED). L1/R1 step through them without wrapping (FOUND, 0x463b98).
const COLLECTION_PAGES: [(i32, &[(u32, &str)]); 6] = [
    (
        0,
        &[
            (0xb238_4143, "Costumes"),
            (0xb074_e7b4, "Head"),
            (0x9843_f40a, "Body"),
            (0xb21e_cdf8, "Materials"),
            (0x2684_06ab, "Player Colour"),
        ],
    ),
    (
        1,
        &[
            (0xdae7_f79a, "Stickers"),
            (0xde04_7861, "Decorations"),
            (0xa317_6f13, "PlayStation Eye Camera"),
            (0xf5fa_8331, "My Pictures"),
            (0xc303_7f54, "Community Photos"),
        ],
    ),
    (
        2,
        &[
            (0xb21e_cdf8, "Materials"),
            (0xeb8d_7d70, "Collected Objects"),
            (0x5959_5c00, "My Objects"),
            (0xfead_43d3, "Community Objects"),
        ],
    ),
    (
        3,
        &[
            (0x3bd3_366a, "Tools"),
            (0x18ba_f616, "Gadgets"),
            (0x7f8a_51ba, "Gameplay Kits"),
            (0xe8f2_0178, "Music"),
            (0xea9b_0d51, "Sounds"),
            (0x775a_c2b6, "Gameplay Controls"),
            (0xd0ce_e297, "Global Controls"),
            (0x5161_8d4b, "Backgrounds"),
        ],
    ),
    (
        4,
        &[
            (0x5036_ae03, "Tools and Used Objects"),
            (0x4d45_f1f1, "Heart Items"),
        ],
    ),
    (8, &[(0x15f8_630f, "My Pods")]),
];
/// Page title keys (D402): the Tools Bag's "Sounds" page and the Personal collection's
/// "Heart Items" page.
const SOUNDS_TITLE: (u32, &str) = (0xea9b_0d51, "Sounds");
const HEART_TITLE: (u32, &str) = (0x4d45_f1f1, "Heart Items");
/// Fallback button -> collection (create mode; play mode lacks the Goodies Bag, 1), equal
/// to the D401 run of `Poppet.DoMainMenuButtonResponse`.
const FALLBACK_OPENS: [(u32, i32); 6] = [(0, 1), (1, 2), (4, 4), (5, 3), (6, 0), (10, 8)];
const PERSONAL_BUTTON: u32 = 4;
/// Function tile icons (MEASURED 2026-10-11 frames; tile order and names from the tooltips):
/// (label, texture GUID of the 64x64 field icon). Popit Cursor comes from the button sheet;
/// the glue and cutter glyphs are the HUD sheet's, UV Tool and Slice 'N Dice are drawn
/// (their game textures are not located).
const FUNCTION_TILES: [(&str, u32); 10] = [
    ("Take A Photo", 42928),
    ("Corner Editor", 42407),
    ("Material Changer", 42406),
    ("Sticker & Decoration Edit Tool", 42408),
    ("Capture Object", 42929),
    ("Electric Tool", 42932),
    ("Flame Tool", 42933),
    ("Plasma Tool", 42934),
    ("Horrible Gas Tool", 42935),
    ("Unlethalize Tool", 42936),
];
/// HUD glyph sheet (4x2 cells of 64, white in alpha): cell 5 = splat, cell 6 = scissors.
const HUD_GLYPH_SHEET: u32 = 0x1cfb9;
const KITS_PLAN: &str = "palette_gameplay_579.plan";
const PAGE_ICONS: usize = 36;
const TOOLS_BUTTON: u32 = 5;
/// Sounds list (MEASURED, frame 18:12:52): first row centre y, row pitch, label x, visible rows.
const LIST_Y0: f32 = 140.;
const LIST_PITCH: f32 = 46.;
const LIST_LABEL_X: f32 = 1000.;
const LIST_VISIBLE: usize = 11;
/// Page arrows (MEASURED, frame 18:12:52): left / right centre x, centre y (720p px).
const PAGE_ARROW_X: (f32, f32) = (928., 1212.);
const PAGE_ARROW_Y: f32 = 350.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Edit,
    Play,
}

/// Runtime state: open flag, blend 0..1 (linear, eased on use), focus (button id), frame clock.
#[derive(Resource)]
struct Popit {
    mode: Mode,
    cards: [(Vec<Vec<u32>>, u32, f32); 2],
    open: bool,
    blend: f32,
    focus: u32,
    clock: f32,
    spawned: bool,
    /// Inventory page open behind this main-menu button, its eased blend, grid focus (row, col).
    page: Option<u32>,
    page_blend: f32,
    cell: (usize, usize),
    /// D404: each collection's current page (kept per collection, FOUND 0x463b98 +0x38).
    tabs: BTreeMap<u32, usize>,
    built: Option<(u32, usize, usize, usize)>,
    /// Cursor mode (Popit Cursor button): cursor position in 720p screen px.
    cursor: Option<Vec2>,
}

/// Cursor mode: starts at the frame centre (MEASURED 639,358); ring 22 px, speed px/s.
const CURSOR_START: Vec2 = Vec2::new(640., 358.);
const CURSOR_SPEED: f32 = 320.;
const CURSOR_PROMPT: &str = "Set Camera";

/// Inventory page layout, 720p px (MEASURED on RPCS3 create frames 18:12): panel rectangle,
/// 6 icon columns, 46 px row pitch from the first grid row.
const PANEL: [f32; 4] = [918.5, 53.5, 304., 614.];
const GRID_X0: f32 = 957.;
const GRID_PITCH: f32 = 46.;
const GRID_COLS: usize = 6;
/// Page camera: how much closer than the gameplay camera, and the aim drop (viewer fit).
const PAGE_ZOOM: f32 = 2.6;
const PAGE_AIM_DROP: f32 = 0.;
/// Page view: Sackboy's feet sit this many px below the screen centre, centred horizontally.
const PAGE_FEET_BELOW_CENTRE: f32 = 165.;

#[derive(Component)]
struct PageNode;

#[derive(Component)]
struct Part(Role);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Above,
    Below,
    Right,
}

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
    CursorRing,
    CursorDot,
    Tail(usize),
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

/// A 64x64 tool-icon texture (MEASURED): the glyph's soft field is in R with the packing
/// markers (an X and a "JU" box) added into R and repeated alone in B, so the glyph is
/// `R - B`; the thin-outline look of the game tiles fits an edge at 24/255 over 6/255.
fn field_icon(rgba: &[u8], w: u32, h: u32) -> Image {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for px in rgba.as_chunks::<4>().0 {
        let v = f32::from(px[0]) - f32::from(px[2]) - 24.;
        let t = (v / 6.).clamp(0., 1.);
        out.extend_from_slice(&[255, 255, 255, (t * t * (3. - 2. * t) * 255.) as u8]);
    }
    image_from(w, h, out)
}

/// One cell of a white-in-alpha glyph sheet.
fn alpha_cell(rgba: &[u8], sheet_w: u32, x0: u32, y0: u32, cell: u32) -> Image {
    let mut out = Vec::with_capacity((cell * cell * 4) as usize);
    for y in 0..cell {
        for x in 0..cell {
            let i = (((y0 + y) * sheet_w + x0 + x) * 4 + 3) as usize;
            out.extend_from_slice(&[255, 255, 255, rgba[i]]);
        }
    }
    image_from(cell, cell, out)
}

/// A 64x64 white-on-alpha mask from a per-pixel coverage function (0..1, texel centres).
fn drawn(cover: impl Fn(f32, f32) -> f32) -> Image {
    let mut out = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64 {
        for x in 0..64 {
            let a = cover(x as f32 + 0.5, y as f32 + 0.5).clamp(0., 1.);
            out.extend_from_slice(&[255, 255, 255, (a * 255.) as u8]);
        }
    }
    image_from(64, 64, out)
}

/// Sackboy figure coverage (128x128) from the Body button's last icon frame.
fn figure_cover(rgba: &[u8], sheet_w: u32, frames: u32) -> Vec<f32> {
    let cols = sheet_w / 128;
    let f = frames - 1;
    let (x0, y0) = ((f % cols) * 128, (f / cols) * 128);
    let mut out = Vec::with_capacity(128 * 128);
    for y in 0..128 {
        for x in 0..128 {
            let i = (((y0 + y) * sheet_w + x0 + x) * 4) as usize;
            let v = f32::from(rgba[i]) / 255.;
            let t = ((v - (SDF_EDGE - SDF_SOFT)) / (2. * SDF_SOFT)).clamp(0., 1.);
            out.push(t * t * (3. - 2. * t));
        }
    }
    out
}

/// A costume-tool tile (Reset / Random / Save / Wash Costume): the figure scaled to a
/// `size` px box centred at (cx, cy) plus a drawn mark. INFERRED art: the game's four
/// textures (guids 42924-42926, 46763) are not in the disc data.
fn costume_tile(
    figure: &[f32],
    (cx, cy, size): (f32, f32, f32),
    mark: impl Fn(f32, f32) -> f32,
) -> Image {
    drawn(|x, y| {
        let sample = |px: f32, py: f32| {
            let (u, v) = ((px - cx) / size + 0.5, (py - cy) / size + 0.5);
            if !(0. ..1.).contains(&u) || !(0. ..1.).contains(&v) {
                return 0.;
            }
            figure[(v * 128.) as usize * 128 + (u * 128.) as usize]
        };
        let fig = if size > 0. {
            (sample(x - 0.25, y - 0.25)
                + sample(x + 0.25, y - 0.25)
                + sample(x - 0.25, y + 0.25)
                + sample(x + 0.25, y + 0.25))
                / 4.
        } else {
            0.
        };
        fig.max(mark(x, y))
    })
}

/// Distance from (x, y) to the segment a-b.
fn seg(x: f32, y: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy).max(1e-6)).clamp(0., 1.);
    ((x - a.0 - t * dx).powi(2) + (y - a.1 - t * dy).powi(2)).sqrt()
}

/// Random Costume mark: "!?" above the figure.
fn mark_random(x: f32, y: f32) -> f32 {
    let line = |d: f32| (1.6 - d).clamp(0., 1.);
    let bang =
        line(seg(x, y, (14., 6.), (14., 15.))).max(line(((x - 14.).hypot(y - 20.) - 0.6).max(0.)));
    let (qx, qy) = (30., 12.);
    let ring = ((x - qx).hypot(y - qy) - 6.).abs();
    let open = x < qx && y > qy + 2.;
    let hook = if open { 0. } else { line(ring) };
    let stem = line(seg(x, y, (qx, qy + 6.), (qx, qy + 10.)));
    let dot = line(((x - qx).hypot(y - 20.) - 0.6).max(0.));
    bang.max(hook).max(stem).max(dot)
}

/// Save Costume mark: four corner brackets round the figure.
fn mark_brackets(x: f32, y: f32) -> f32 {
    let line = |d: f32| (1.8 - d).clamp(0., 1.);
    let mut v = 0f32;
    for (sx, sy) in [(6., 6.), (58., 6.), (6., 58.), (58., 58.)] {
        let (ex, ey) = (
            if sx < 32. { 1. } else { -1. },
            if sy < 32. { 1. } else { -1. },
        );
        v = v
            .max(line(seg(x, y, (sx, sy), (sx + 13. * ex, sy))))
            .max(line(seg(x, y, (sx, sy), (sx, sy + 13. * ey))));
    }
    v
}

/// Wash Costume: a rounded square with an X cut out (INFERRED from the frame).
fn mark_wash(x: f32, y: f32) -> f32 {
    let (u, v) = ((x - 32.).abs(), (y - 32.).abs());
    let inside = (22. - u.max(v)).clamp(0., 1.);
    let corner = ((u - 14.).max(0.)).hypot((v - 14.).max(0.));
    let rounded = (8. - corner).clamp(0., 1.).min(inside);
    let cut = (2.2 - seg(x, y, (22., 22.), (42., 42.)).min(seg(x, y, (42., 22.), (22., 42.))))
        .clamp(0., 1.);
    (rounded - cut).clamp(0., 1.)
}

/// UV Tool: a 2x2 checker with the top-right and bottom-left squares filled, outlined
/// (INFERRED from the frame; the game texture is not located).
fn drawn_uv_tool() -> Image {
    drawn(|x, y| {
        let (u, v) = ((x - 12.) / 40., (y - 12.) / 40.);
        if !(0. ..1.).contains(&u) || !(0. ..1.).contains(&v) {
            return 0.;
        }
        let edge = (u.min(1. - u).min(v).min(1. - v) * 40.) < 3.;
        let mid = ((u - 0.5).abs().min((v - 0.5).abs()) * 40.) < 1.5;
        let filled = (u >= 0.5) != (v >= 0.5);
        if edge || mid || filled {
            1.
        } else {
            0.
        }
    })
}

/// Slice 'N Dice: a filled block and an outlined block split by a zigzag (INFERRED).
fn drawn_slice() -> Image {
    drawn(|x, y| {
        if !(8. ..56.).contains(&y) || !(8. ..58.).contains(&x) {
            return 0.;
        }
        let cut = 30.
            + if ((y - 8.) / 8.).floor() as i32 % 2 == 0 {
                0.
            } else {
                8.
            };
        let outline = ((x - 8.).min(58. - x).min(y - 8.).min(56. - y)) < 2.5;
        if x < cut - 3. || outline && x >= cut + 3. {
            1.
        } else {
            0.
        }
    })
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

/// The card fill: rounded rectangle coloured by `CARD_RAMP` from the tail attach point.
fn card_image(card: [f32; 2], attach_x: f32) -> Image {
    card_image_at(card, [attach_x * card[0], card[1]], CARD_RADIUS)
}

/// As `card_image` with the attach point in card px and the corner radius.
fn card_image_at(card: [f32; 2], attach: [f32; 2], radius: f32) -> Image {
    const SS: f32 = 2.;
    let (w, h) = ((card[0] * SS) as u32, (card[1] * SS) as u32);
    let r = radius * SS;
    let (ax, ay) = (attach[0] * SS, attach[1] * SS);
    let far = ((ax.max(w as f32 - ax)).powi(2) + ay.max(h as f32 - ay).powi(2)).sqrt();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Signed distance to the rounded rectangle.
            let qx = (px - w as f32 / 2.).abs() - (w as f32 / 2. - r);
            let qy = (py - h as f32 / 2.).abs() - (h as f32 / 2. - r);
            let sd = qx.max(0.).hypot(qy.max(0.)) + qx.max(qy).min(0.) - r;
            let a = (0.5 - sd).clamp(0., 1.);
            let t = (((px - ax).powi(2) + (py - ay).powi(2)).sqrt() / far).clamp(0., 1.);
            let c = ramp(t);
            out.extend_from_slice(&[c[0], c[1], c[2], (a * 255.) as u8]);
        }
    }
    image_from(w, h, out)
}

fn ramp(t: f32) -> [u8; 3] {
    let i = CARD_RAMP
        .iter()
        .position(|(s, _)| *s >= t)
        .unwrap_or(CARD_RAMP.len() - 1)
        .max(1);
    let ((t0, c0), (t1, c1)) = (CARD_RAMP[i - 1], CARD_RAMP[i]);
    let f = ((t - t0) / (t1 - t0)).clamp(0., 1.);
    [0, 1, 2].map(|k| (f32::from(c0[k]) + (f32::from(c1[k]) - f32::from(c0[k])) * f) as u8)
}

/// Reads every atlas, the tooltips and the Square glyph (load worker).
pub(crate) fn load_assets(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &LoadControl,
) -> Result<PopitAssets, String> {
    let (strings, _) = load_strings(source, catalog, control);
    let (cards, mut card_source, icons, opens) = match script_cards(source, catalog, control) {
        Ok((c, note, icons, opens)) => (c, note, icons, opens),
        Err(e) => {
            let edit: BTreeMap<u32, i32> = FALLBACK_OPENS.into_iter().collect();
            let mut play = edit.clone();
            play.remove(&1);
            (
                fallback_cards(),
                format!("fallback constants ({e})"),
                Default::default(),
                [edit, play],
            )
        }
    };
    let mut items = Vec::new();
    let mut from_script = 0;
    for (id, guid, cell, frames, key, english) in ITEMS {
        // D400: the script's icon and tooltip key win over the table (which they match).
        let (guid, cell, frames, key) = match icons.get(&id) {
            Some(&v) => {
                from_script += 1;
                v
            }
            None => (guid, cell, frames, key),
        };
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
    let popit = lbp_preview::popit::load_popit(source, catalog, control)?;
    let tr = |key: u32, english: &str| {
        strings
            .get(key)
            .map_or_else(|| english.to_owned(), str::to_owned)
    };
    let mut pages = Vec::new();
    for (&id, collection) in &opens[0] {
        let Some(&(_, title_key, english, plan_page, message)) =
            PAGES.iter().find(|pg| pg.0 == *collection)
        else {
            continue;
        };
        let mut sections: Vec<(String, Vec<(String, Image)>)> = Vec::new();
        let mut taken = 0;
        if let Some(name) = plan_page {
            for &ix in popit.pages.get(name).map_or(&[][..], Vec::as_slice) {
                let item = &popit.items[ix];
                let Some(key) = item.icon.filter(|_| item.icon_resolved) else {
                    continue;
                };
                let Ok(icon) = decode_icon(source, catalog, key, control) else {
                    continue;
                };
                let cat = humanize(item.category.as_deref().unwrap_or_default());
                if cat.is_empty() {
                    continue;
                }
                if sections.last().is_none_or(|(c, _)| *c != cat) {
                    sections.push((cat, Vec::new()));
                }
                let img = image_from(icon.width, icon.height, icon.rgba);
                sections
                    .last_mut()
                    .expect("section just pushed")
                    .1
                    .push((item.title.clone(), img));
                taken += 1;
                if taken >= PAGE_ICONS {
                    break;
                }
            }
        }
        // Tools opens the sound-object list: LBP2's own sound objects by name; the LBP1
        // palette sounds and DLC packs are not in the game's list on the measured profile.
        let mut list = Vec::new();
        if id == TOOLS_BUTTON {
            let mut rows_in: Vec<&lbp_preview::popit::PopitItem> = popit
                .pages
                .get("sound")
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|&ix| &popit.items[ix])
                .filter(|it| {
                    it.icon_resolved && it.paths.iter().any(|p| p.contains("lbp2/unthemed/"))
                })
                .collect();
            rows_in.sort_by(|a, b| a.title.cmp(&b.title));
            for it in rows_in {
                let Some(key) = it.icon else { continue };
                let Ok(icon) = decode_icon(source, catalog, key, control) else {
                    continue;
                };
                list.push((
                    it.title.clone(),
                    image_from(icon.width, icon.height, icon.rgba),
                ));
            }
        }
        let mut functions = Vec::new();
        let mut kits = Vec::new();
        // MEASURED 2026-10-12 (play frames, tooltips): Body = Reset / Random / Save / Wash
        // Costume, Stickers = Sticker & Decoration Edit Tool, Take A Photo.
        if id == 6 {
            if let Ok(sheet) = decode_icon(source, catalog, ResourceKey::Guid(0x19149), control) {
                let fig = figure_cover(&sheet.rgba, sheet.width, 32);
                for (label, geo, mark) in [
                    ("Reset Costume", (32., 32., 46.), None),
                    (
                        "Random Costume",
                        (32., 40., 38.),
                        Some(mark_random as fn(f32, f32) -> f32),
                    ),
                    (
                        "Save Costume",
                        (32., 32., 30.),
                        Some(mark_brackets as fn(f32, f32) -> f32),
                    ),
                    (
                        "Wash Costume",
                        (32., 32., 0.),
                        Some(mark_wash as fn(f32, f32) -> f32),
                    ),
                ] {
                    let img = costume_tile(&fig, geo, |x, y| mark.map_or(0., |m| m(x, y)));
                    functions.push((label.to_owned(), img));
                }
            }
        }
        // MEASURED 2026-10-12 (own Pod frames): the rocket opens "My Pods", two tiles, no grid.
        let tiles: &[(&str, u32)] = match id {
            0 => &[
                ("Sticker & Decoration Edit Tool", 42408),
                ("Take A Photo", 42928),
            ],
            10 => &[("Reset Pod Decoration", 82777), ("Save Pod", 82775)],
            _ => &[],
        };
        for &(label, guid) in tiles {
            if let Ok(i) = decode_icon(source, catalog, ResourceKey::Guid(guid), control) {
                functions.push((label.to_owned(), field_icon(&i.rgba, i.width, i.height)));
            }
        }
        if id == PERSONAL_BUTTON {
            let field = |guid: u32| -> Option<Image> {
                let i = decode_icon(source, catalog, ResourceKey::Guid(guid), control).ok()?;
                Some(field_icon(&i.rgba, i.width, i.height))
            };
            let glyph = |col: u32, row: u32| -> Option<Image> {
                let i = decode_icon(source, catalog, ResourceKey::Guid(HUD_GLYPH_SHEET), control)
                    .ok()?;
                Some(alpha_cell(&i.rgba, i.width, col * 64, row * 64, 64))
            };
            let cursor = items
                .iter()
                .find(|i| i.id == 2)
                .and_then(|i| i.frames.last().cloned());
            let mut by_label: std::collections::HashMap<&str, Image> = FUNCTION_TILES
                .iter()
                .filter_map(|(l, g)| field(*g).map(|img| (*l, img)))
                .collect();
            let mut push = |label: &str, img: Option<Image>| {
                if let Some(img) = img {
                    functions.push((label.to_owned(), img));
                }
            };
            push("Popit Cursor", cursor);
            for (label, img) in [
                ("Corner Editor", None),
                ("Advanced Glue Tool", glyph(1, 1)),
                ("Material Changer", None),
                ("Sticker & Decoration Edit Tool", None),
                ("Sticker Cutter", glyph(2, 1)),
                ("UV Tool", Some(drawn_uv_tool())),
                ("Capture Object", None),
                ("Electric Tool", None),
                ("Flame Tool", None),
                ("Plasma Tool", None),
                ("Horrible Gas Tool", None),
                ("Unlethalize Tool", None),
                ("Slice 'N Dice", Some(drawn_slice())),
            ] {
                let img = img.or_else(|| by_label.remove(label));
                push(label, img);
            }
            if let Some(it) = popit
                .items
                .iter()
                .find(|it| it.paths.iter().any(|p| p.ends_with(KITS_PLAN)))
            {
                if let Some(Ok(icon)) = it
                    .icon
                    .map(|key| decode_icon(source, catalog, key, control))
                {
                    kits.push((
                        it.title.clone(),
                        image_from(icon.width, icon.height, icon.rgba),
                    ));
                }
            }
        }
        let (home_key, home_english) = if id == TOOLS_BUTTON && !list.is_empty() {
            SOUNDS_TITLE
        } else {
            (title_key, english)
        };
        let order = COLLECTION_PAGES
            .iter()
            .find(|c| c.0 == *collection)
            .map_or(&[][..], |c| c.1);
        let tabs: Vec<String> = order.iter().map(|&(k, e)| tr(k, e)).collect();
        info!("popit collection {collection} pages {tabs:?}");
        pages.push(PageData {
            id,
            title: tr(home_key, home_english),
            home: order.iter().position(|&(k, _)| k == home_key).unwrap_or(0),
            tabs,
            heart_title: tr(HEART_TITLE.0, HEART_TITLE.1),
            sections,
            message,
            list,
            functions,
            kits,
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
    card_source.push_str(&format!(
        "; icons and tooltips from the script {from_script}/{}",
        ITEMS.len()
    ));
    let close = strings
        .get(CLOSE_POPIT_KEY)
        .map_or_else(|| "Close Popit".to_owned(), str::to_owned);
    Ok(PopitAssets {
        items,
        cards,
        card_source,
        square: image_from(64, 64, square),
        close,
        pages,
        opens,
    })
}

pub(crate) fn install(app: &mut App, assets: PopitAssets) {
    let all: Vec<String> = std::env::args().collect();
    let play = all
        .windows(2)
        .any(|w| w[0] == "--popit-mode" && w[1] == "play");
    let mode = if play { Mode::Play } else { Mode::Edit };
    info!("popit card: {}", assets.card_source);
    let cards = assets.cards.clone();
    let focus = cards[usize::from(play)].1;
    // `--popit-page BUTTON` (QA): start with that inventory page open.
    let start_page = all
        .windows(2)
        .find(|w| w[0] == "--popit-page")
        .and_then(|w| w[1].parse::<u32>().ok());
    app.insert_resource(assets)
        .insert_resource(Popit {
            mode,
            cards,
            open: all.iter().any(|a| a == "--popit-open") || start_page.is_some(),
            blend: if all.iter().any(|a| a == "--popit-open") || start_page.is_some() {
                1.
            } else {
                0.
            },
            focus,
            clock: 0.,
            spawned: false,
            page: start_page,
            page_blend: if start_page.is_some() { 1. } else { 0. },
            cell: (0, 0),
            tabs: start_page
                .zip(
                    all.windows(2)
                        .find(|w| w[0] == "--popit-tab")
                        .and_then(|w| w[1].parse::<usize>().ok()),
                )
                .into_iter()
                .collect(),
            built: None,
            cursor: all
                .iter()
                .any(|a| a == "--popit-cursor")
                .then_some(CURSOR_START),
        })
        .add_systems(
            Update,
            (popit_input, popit_turn, popit_layout, popit_page_ui)
                .chain()
                .after(crate::sackboy_play::show_sackboy),
        );
}

impl Popit {
    fn card(&self) -> &(Vec<Vec<u32>>, u32, f32) {
        &self.cards[usize::from(self.mode == Mode::Play)]
    }
}

fn rows(p: &Popit) -> Vec<Vec<u32>> {
    p.card().0.clone()
}

fn fallback_cards() -> [(Vec<Vec<u32>>, u32, f32); 2] {
    let grid = |r: &[&[u32]]| r.iter().map(|row| row.to_vec()).collect();
    let play = if crate::sackboy_play::in_pod() {
        PLAY_ROWS_POD
    } else {
        PLAY_ROWS
    };
    [(grid(EDIT_ROWS), 2, PITCH), (grid(play), 6, PITCH)]
}

/// D400: both cards from the game's `Poppet` script (create mode, then play mode).
fn script_cards(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &LoadControl,
) -> Result<ScriptCards, String> {
    let (vm, class, script) = poppet_vm(source, catalog, control)?;
    let mut vm = vm;
    // D402: the play card has the Pod Decoration button only in our Pod, Retry elsewhere.
    let our_pod = crate::sackboy_play::in_pod();
    let mut out = Vec::new();
    let mut icons = BTreeMap::new();
    for edit in [true, false] {
        let (card, keys) = lbp_ui_script::popit::run_card(&mut vm, class, &script, edit, our_pod)?;
        for b in card.rows.iter().flatten() {
            let (Some(atx), Some(&key)) = (b.icon, keys.get(&b.index)) else {
                continue;
            };
            let a = lbp_ui_script::popit::animated_texture(source, catalog, atx, control)?;
            let Some(lbp_resource::animated_texture::TextureRef::Guid(sheet)) = a.texture else {
                return Err(format!("icon {atx:#x}: no sheet GUID"));
            };
            icons.insert(b.index, (sheet, a.cell[0], a.frames, key));
        }
        let rows: Vec<Vec<u32>> = card
            .rows
            .iter()
            .map(|r| r.iter().map(|b| b.index).collect())
            .collect();
        let focus = card
            .default_index
            .filter(|f| rows.iter().flatten().any(|i| i == f))
            .or_else(|| rows.iter().flatten().next().copied())
            .ok_or("empty card")?;
        out.push((rows, focus, card.pitch() * UNIT_PX));
    }
    let play = out.pop().expect("two cards");
    let edit = out.pop().expect("two cards");
    // D401: what each button opens, from `Poppet.DoMainMenuButtonResponse`.
    let mut opens = [true, false].map(|edit| {
        lbp_ui_script::popit::run_responses(&mut vm, class, &script, edit, our_pod, 0..=10)
    });
    if opens[0].is_empty() {
        return Err("DoMainMenuButtonResponse opened no inventory page".into());
    }
    // A page the viewer has no title for stays closed.
    for o in &mut opens {
        o.retain(|_, c| PAGES.iter().any(|pg| pg.0 == *c));
    }
    let note = format!(
        "script Poppet.DoMainMenu: create {:?} focus {}, play {:?} focus {}, pitch {:.1} px; \
         pages (button: collection) create {:?}, play {:?}",
        edit.0, edit.1, play.0, play.1, edit.2, opens[0], opens[1]
    );
    Ok(([edit, play], note, icons, opens))
}

/// Both cards, a note for the log, and per button (sheet GUID, cell px, frames, LAMS key).
type ScriptCards = (
    [(Vec<Vec<u32>>, u32, f32); 2],
    String,
    BTreeMap<u32, (u32, u32, u32, u32)>,
    [BTreeMap<u32, i32>; 2],
);

/// A VM with every catalog script loaded, and the `Poppet` class.
fn poppet_vm(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &LoadControl,
) -> Result<
    (
        lbp_script_vm::Vm,
        lbp_script_vm::ClassId,
        lbp_level::script::Script,
    ),
    String,
> {
    let (scripts, _) = lbp_ui_script::load::load(source, catalog, control)?;
    let mut vm = lbp_script_vm::Vm::default();
    let mut target = None;
    for (guid, s) in scripts {
        if s.class_name == lbp_ui_script::popit::CLASS && target.is_none() {
            target = Some((vm.load(s.clone(), guid), s));
        } else {
            vm.load(s, guid);
        }
    }
    let (class, script) = target.ok_or("no Poppet script in the catalog")?;
    Ok((vm, class, script))
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
    assets: Res<PopitAssets>,
    pod: Option<Res<crate::sackboy_play::PodSackboy>>,
) {
    let pad = pads.iter().next();
    if let Some(at) = p.cursor {
        // Cursor mode: the left stick (WASD) moves the ring, Circle/Square leave it (Square
        // brings the Popit back); Sackboy stands still.
        for mut body in &mut sackboy {
            body.input = Default::default();
        }
        let axis = |neg: KeyCode, pos: KeyCode| {
            f32::from(keys.pressed(pos)) - f32::from(keys.pressed(neg))
        };
        let (mut dx, mut dy) = (
            axis(KeyCode::KeyA, KeyCode::KeyD),
            axis(KeyCode::KeyW, KeyCode::KeyS),
        );
        if let Some(g) = pad {
            dx += g.get(GamepadAxis::LeftStickX).unwrap_or(0.);
            dy -= g.get(GamepadAxis::LeftStickY).unwrap_or(0.);
        }
        let v = Vec2::new(dx, dy).clamp_length_max(1.) * CURSOR_SPEED * time.delta_secs();
        p.cursor = Some((at + v).clamp(Vec2::ZERO, Vec2::new(1280., 720.)));
        let square = keys.just_pressed(KeyCode::KeyZ)
            || pad.is_some_and(|g| g.just_pressed(GamepadButton::West));
        let circle = keys.just_pressed(KeyCode::KeyC)
            || pad.is_some_and(|g| g.just_pressed(GamepadButton::East));
        if square || circle {
            // MEASURED 18:50: Circle also brings the Popit back, focus on the Popit Cursor.
            p.cursor = None;
            p.open = true;
            p.focus = 2;
        }
        return;
    }
    // D403: in the Pod's pad zone (or at the pod computer) Square is the pod computer's.
    let at_computer = !p.open && pod.is_some_and(|s| s.at_computer);
    if !at_computer
        && (keys.just_pressed(KeyCode::KeyZ)
            || pad.is_some_and(|g| g.just_pressed(GamepadButton::West)))
    {
        p.open = !p.open;
    }
    p.clock += time.delta_secs();
    let step = time.delta_secs() / BLEND_SECONDS;
    let pstep = if p.page.is_some() { step } else { -step };
    p.page_blend = (p.page_blend + pstep).clamp(0., 1.);
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
    let confirm = keys.just_pressed(KeyCode::KeyX)
        || pad.is_some_and(|g| g.just_pressed(GamepadButton::South));
    let back = keys.just_pressed(KeyCode::KeyC)
        || pad.is_some_and(|g| g.just_pressed(GamepadButton::East));
    // D404: L1 / R1 (Q / E on the keyboard) are the previous / next page keys (FOUND 0x463b98).
    let shoulder = i32::from(dir(KeyCode::KeyE, GamepadButton::RightTrigger))
        - i32::from(dir(KeyCode::KeyQ, GamepadButton::LeftTrigger));
    let open_page = assets.pages.iter().find(|pg| Some(pg.id) == p.page);
    if p.page.is_some() {
        if back {
            // MEASURED 18:51: returning from a page resets the focus to the Popit Cursor.
            p.page = None;
            p.focus = 2;
        } else if let Some(pg) = open_page.filter(|_| shoulder != 0) {
            // Step to the previous / next visible page and stop at the ends (FOUND
            // 0x463b98); the page is kept per collection. The grid focus returns to the
            // first cell (INFERRED: the game moves focus to the new page's node).
            let vis = pg.visible(p.mode);
            let cur = pg.current(&p);
            let at = vis.iter().position(|&t| t == cur).unwrap_or(0) as i32;
            let to = vis[(at + shoulder).clamp(0, vis.len() as i32 - 1) as usize];
            if to != cur {
                p.tabs.insert(pg.id, to);
                p.cell = (0, 0);
            }
        } else if open_page.is_some_and(|pg| pg.current(&p) != pg.home) {
            // A page whose contents are not loaded has no grid to move on.
        } else {
            let (r, c) = p.cell;
            let page = open_page;
            let list_len = page.map_or(0, |pg| pg.list.len());
            let (nf, nk) = page.map_or((0, 0), |pg| (pg.functions.len(), pg.kits.len()));
            if nf > 0 {
                // Function tiles (6 per row), then the focusable "Gameplay Kits" header
                // (MEASURED: Cross there hides the section), then its items.
                let fr = nf.div_ceil(GRID_COLS);
                let last_row = if nk > 0 { fr + 1 } else { fr - 1 };
                let r2 = (r as i32 + dy).clamp(0, last_row as i32) as usize;
                let cols = if r2 < fr {
                    (nf - r2 * GRID_COLS).min(GRID_COLS)
                } else if r2 == fr {
                    1
                } else {
                    nk
                };
                let c2 = if r2 == fr {
                    0
                } else {
                    (c as i32 + dx).clamp(0, cols as i32 - 1) as usize
                };
                p.cell = (r2, c2);
            } else if list_len > 0 {
                let r2 = (r as i32 + dy).clamp(0, list_len as i32 - 1) as usize;
                p.cell = (r2, 0);
            } else {
                let c2 = (c as i32 + dx).clamp(0, GRID_COLS as i32 - 1) as usize;
                let r2 = (r as i32 + dy).clamp(0, 7) as usize;
                p.cell = (r2, c2);
            }
        }
        return;
    }
    if confirm && p.focus == 2 && p.open {
        p.open = false;
        p.cursor = Some(CURSOR_START);
        return;
    }
    let mode = usize::from(p.mode == Mode::Play);
    if confirm && assets.opens[mode].contains_key(&p.focus) {
        p.page = Some(p.focus);
        p.cell = (0, 0);
        return;
    }
    if dx == 0 && dy == 0 {
        return;
    }
    let grid = rows(&p);
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

/// Sackboy turns his back to the camera while the Popit is open (both modes, MEASURED on
/// RPCS3 frames: back of the head and the cord wound round the body), eased like the card.
fn popit_turn(
    p: Res<Popit>,
    mut figure: Query<&mut Transform, With<crate::sackboy_play::Figure>>,
    mut orbit: ResMut<crate::Orbit>,
    cameras: Query<&Projection, With<Camera3d>>,
) {
    let pe = smooth(p.page_blend);
    if pe > 0. {
        // Page view: aim at a point above his feet so he stands centred, feet 165 px below the
        // middle of the frame (MEASURED, create frames), whatever the follow lead was.
        let fov = cameras.iter().find_map(|c| match c {
            Projection::Perspective(pp) => Some(pp.fov),
            _ => None,
        });
        if let (Some(fov), Ok(t)) = (fov, figure.single()) {
            let per_px = 2. * orbit.distance * (fov / 2.).tan() / FRAME_H;
            let want = t.translation + Vec3::Y * PAGE_FEET_BELOW_CENTRE * per_px;
            orbit.target = orbit.target.lerp(want, pe);
        }
    }
    let turn = std::f32::consts::PI * smooth(p.blend) * (1. - pe);
    if turn <= 0. {
        return;
    }
    for mut t in &mut figure {
        t.rotation = Quat::from_rotation_y(turn) * t.rotation;
    }
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
    mut framings: Local<usize>,
) {
    // The follow camera sets its distance once when it frames; re-apply our nudge after that.
    let framed = crate::sackboy_play::FRAMINGS.load(std::sync::atomic::Ordering::Relaxed);
    if framed != *framings {
        *framings = framed;
        *applied = (0., 0.);
    }
    // Camera nudge, relative to what we applied last frame so zoom/orbit stay free.
    let e = smooth(p.blend);
    let (prev_zoom, prev_drop) = *applied;
    let pe = smooth(p.page_blend);
    let zoom_to = POPIT_ZOOM + (PAGE_ZOOM - POPIT_ZOOM) * pe;
    let zoom = 1. + (1. / zoom_to - 1.) * e;
    let drop = (POPIT_AIM_DROP + (PAGE_AIM_DROP - POPIT_AIM_DROP) * pe) * e;
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
        let (card, attach) = if p.mode == Mode::Edit {
            (EDIT_CARD, CARD_ATTACH_X)
        } else {
            (PLAY_CARD, PLAY_ATTACH_X)
        };
        let card = images.add(card_image(card, attach));
        spawn_nodes(&mut commands, &assets, &hs, square, card);
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
    let feet = camera
        .world_to_viewport(cam_tf, fig.translation())
        .unwrap_or(head + Vec2::Y * 80.);
    let in_page = p.page_blend > 0.02;
    let cursor_at = p.cursor.map(|c| c * scale);
    let hidden = p.blend <= 0. || p.page_blend > 0.02;
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
        let grid = rows(&p);
        let mut found = (0usize, 0usize, 1usize);
        for (ri, row) in grid.iter().enumerate() {
            if let Some(ci) = row.iter().position(|&i| i == p.focus) {
                found = (ri, ci, row.len());
            }
        }
        found
    };
    // Cell centre in card-relative px (top-left origin at 720p).
    let (rows_n, pitch) = (p.card().0.len() as f32, p.card().2);
    let cell_centre = |ri: usize, ci: usize, n: usize| -> [f32; 2] {
        let y = card[1] / 2. + (ri as f32 - (rows_n - 1.) / 2.) * pitch + 4.5 - 4.5;
        let x = card[0] / 2. + (ci as f32 - (n as f32 - 1.) / 2.) * pitch;
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
    // Tooltip pill and tail: above the top row, below the bottom row, else to the right of the
    // tile; the tail points at the tile (MEASURED on create and play frames).
    let (tile_c, tile_half) = {
        let c = cell_centre(focus_pos.0, focus_pos.1, focus_pos.2);
        (
            Vec2::new(left + c[0] * scale * grow, top + c[1] * scale * grow),
            FOCUS_TILE_PX * scale * grow / 2.,
        )
    };
    let place = if focus_pos.0 == 0 {
        Place::Above
    } else if focus_pos.0 + 1 == p.card().0.len() {
        Place::Below
    } else {
        Place::Right
    };
    let chars = assets
        .items
        .iter()
        .find(|i| i.id == p.focus)
        .map_or(10, |i| i.label.chars().count());
    let (pw, ph) = ((chars as f32 * PILL_CHAR_W - 2.) * scale, PILL_H * scale);
    let (pill, tail_at) = match place {
        Place::Above => {
            let tip = Vec2::new(tile_c.x + 3. * scale, tile_c.y - tile_half - 19. * scale);
            (
                (
                    tip.x + 5. * scale - pw / 2.,
                    tip.y - TAIL_LEN_V * scale - ph,
                    pw,
                    ph,
                ),
                tip,
            )
        }
        Place::Below => {
            let tip = Vec2::new(tile_c.x + 3. * scale, tile_c.y + tile_half + 19. * scale);
            (
                (
                    tip.x + 5. * scale - pw / 2.,
                    tip.y + TAIL_LEN_V * scale,
                    pw,
                    ph,
                ),
                tip,
            )
        }
        Place::Right => {
            let tip = Vec2::new(tile_c.x + tile_half + 8. * scale, tile_c.y + 4. * scale);
            ((tip.x + TAIL_LEN_H * scale, tip.y - ph / 2., pw, ph), tip)
        }
    };
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
                let grid = rows(&p);
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
                set(&mut node, pill.0, pill.1, pill.2, pill.3);
                node.border_radius = BorderRadius::all(Val::Px(pill.3 / 2.));
            }
            Role::Tail(i) => {
                // Stacked bars narrowing to the tip; pointing at the focused tile.
                let k = i as f32;
                let n = TAIL_STEPS as f32;
                let width = (11. - k * 2.) * scale;
                let (len, tip_len) = match place {
                    Place::Right => (TAIL_LEN_H * scale / n, TAIL_LEN_H * scale),
                    _ => (TAIL_LEN_V * scale / n, TAIL_LEN_V * scale),
                };
                let _ = tip_len;
                match place {
                    Place::Above => set(
                        &mut node,
                        tail_at.x - width / 2.,
                        pill.1 + pill.3 - 1. * scale + k * len,
                        width,
                        len + 1.,
                    ),
                    Place::Below => set(
                        &mut node,
                        tail_at.x - width / 2.,
                        pill.1 + 1. * scale - (k + 1.) * len,
                        width,
                        len + 1.,
                    ),
                    Place::Right => set(
                        &mut node,
                        pill.0 + 1. * scale - (k + 1.) * len,
                        tail_at.y - width / 2.,
                        len + 1.,
                        width,
                    ),
                }
            }
            Role::CursorRing | Role::CursorDot => {
                let sz = if part.0 == Role::CursorRing { 22. } else { 5. } * scale;
                if let Some(c) = cursor_at {
                    set(&mut node, c.x - sz / 2., c.y - sz / 2., sz, sz);
                    node.border_radius = BorderRadius::all(Val::Px(sz / 2.));
                    *vis = Visibility::Inherited;
                }
            }
            Role::Prompt => {
                if cursor_at.is_some() {
                    *vis = Visibility::Inherited;
                }
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
                if cursor_at.is_some() {
                    *vis = Visibility::Inherited;
                }
                let sz = 22. * scale;
                set(
                    &mut node,
                    window.width() / 2. - 82.5 * scale + 6. * scale,
                    window.height() - 72.5 * scale,
                    sz,
                    sz,
                );
            }
            Role::LabelText => {}
            Role::PromptText => {
                if cursor_at.is_some() {
                    *vis = Visibility::Inherited;
                }
            }
            Role::Cord(i) => {
                let t = (i as f32 + 1.) / CORD_DOTS as f32;
                // From the card's bottom edge to Sackboy's chest: leaves downward, bows right,
                // then turns left onto the body (MEASURED path, create and play frames).
                let (p0, p3) = if let Some(c) = cursor_at {
                    (c, head.lerp(feet, 0.62))
                } else if in_page {
                    // Page view: from the panel's left edge to Sackboy's hips.
                    (
                        Vec2::new(PANEL[0] * scale, (PANEL[1] + 0.57 * PANEL[3]) * scale),
                        head.lerp(feet, 0.72) - Vec2::X * 0.1 * (feet.y - head.y),
                    )
                } else {
                    (
                        Vec2::new(left + w / 2. + 21. * scale, top + h),
                        Vec2::new(cx - 14. * scale, head.y + 52. * scale),
                    )
                };
                let span = p3.y - p0.y;
                let (p1, p2) = if cursor_at.is_some() {
                    let d = p3 - p0;
                    (
                        p0 + d * 0.3 + Vec2::Y * 8. * scale,
                        p0 + d * 0.7 + Vec2::Y * 6. * scale,
                    )
                } else if in_page {
                    let d = p3 - p0;
                    (
                        p0 + d * 0.35 + Vec2::Y * 12. * scale,
                        p0 + d * 0.75 - Vec2::Y * 6. * scale,
                    )
                } else {
                    (
                        p0 + Vec2::new(14. * scale, span * 0.45),
                        p3 + Vec2::new(46. * scale, -span * 0.18),
                    )
                };
                let u = 1. - t;
                let pt =
                    u * u * u * p0 + 3. * u * u * t * p1 + 3. * u * t * t * p2 + t * t * t * p3;
                let sz = 3. * scale;
                set(&mut node, pt.x - sz / 2., pt.y - sz / 2., sz, sz);
                // The cord grows out of the card with the blend.
                if cursor_at.is_some() {
                    *vis = Visibility::Inherited;
                } else if t > e {
                    *vis = Visibility::Hidden;
                } else if in_page {
                    *vis = Visibility::Inherited;
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
            Role::PromptText => {
                let want = if p.cursor.is_some() {
                    CURSOR_PROMPT
                } else {
                    &assets.close
                };
                if text.0 != want {
                    text.0 = want.to_owned();
                }
            }
            _ => {}
        }
    }
    for (part, mut bg) in &mut colours {
        if part.0 == Role::Label || part.0 == Role::Prompt || matches!(part.0, Role::Tail(_)) {
            let a = if p.cursor.is_some() { 1. } else { e };
            bg.0 = if part.0 != Role::Prompt {
                PILL.with_alpha(a)
            } else {
                PROMPT.with_alpha(PROMPT.alpha() * a)
            };
        }
    }
}

/// The inventory page: panel, title, section headers, icon grid, counter, tooltip pill and
/// the USE!/More! prompts (rebuilt whenever the page or the focused cell changes).
fn popit_page_ui(
    mut commands: Commands,
    mut p: ResMut<Popit>,
    assets: Res<PopitAssets>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
    old: Query<Entity, With<PageNode>>,
) {
    let want = p.page.filter(|_| p.page_blend > 0.5).map(|id| {
        let tab = assets
            .pages
            .iter()
            .find(|pg| pg.id == id)
            .map_or(0, |pg| pg.current(&p));
        (id, tab, p.cell.0, p.cell.1)
    });
    if want == p.built {
        return;
    }
    p.built = want;
    for e in &old {
        commands.entity(e).despawn();
    }
    let (Some((id, tab, row, col)), Ok(window)) = (want, windows.single()) else {
        return;
    };
    let Some(page) = assets.pages.iter().find(|pg| pg.id == id) else {
        return;
    };
    let scale = window.height() / FRAME_H;
    let node = |l: f32, t: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(l * scale),
        top: Val::Px(t * scale),
        width: Val::Px(w * scale),
        height: Val::Px(h * scale),
        ..default()
    };
    let text = |commands: &mut Commands, s: &str, x: f32, y: f32, size: f32, z: i32| {
        commands.spawn((
            PageNode,
            Text::new(s),
            TextFont {
                font: bevy::text::FontSource::SansSerif,
                weight: bevy::text::FontWeight::BOLD,
                ..TextFont::from_font_size(size * scale)
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x * scale),
                top: Val::Px((y - size * 0.62) * scale),
                ..default()
            },
            GlobalZIndex(z),
        ));
    };
    let [px, py, pw, ph] = PANEL;
    // MEASURED 2026-10-12 (play frame): in play mode the star opens "Heart Items", a bare
    // panel with the empty-list message and no functions row, counter or focus.
    let heart = id == PERSONAL_BUTTON && tab + 1 == page.tabs.len() && tab != page.home;
    let panel = images.add(card_image_at([pw, ph], [0., 0.57 * ph], 18.));
    commands.spawn((
        PageNode,
        node(px, py, pw, ph),
        ImageNode::new(panel),
        GlobalZIndex(40),
    ));
    // The game's handwritten title is narrower than our sans: shrink long titles to the panel.
    let page_title = if tab == page.home {
        &page.title
    } else if heart {
        &page.heart_title
    } else {
        page.tabs.get(tab).unwrap_or(&page.title)
    };
    let title_px = (30. * 14. / page_title.chars().count().max(1) as f32).clamp(19., 30.);
    text(&mut commands, page_title, px + 18., 84., title_px, 42);
    // D404: page counter "n/m" over the visible pages, drawn only when there is more than
    // one (FOUND 0x463b98), and the left / right page arrows at the panel's sides (MEASURED
    // 18:12:52: "5/8" bottom centre, arrows at x 928 / 1212, y 350). An arrow is drawn only
    // where a page lies on that side (INFERRED from the no-wrap stepping).
    let vis = page.visible(p.mode);
    if vis.len() > 1 {
        let at = vis.iter().position(|&t| t == tab).unwrap_or(0);
        let counter = format!("{}/{}", at + 1, vis.len());
        text(&mut commands, &counter, px + pw / 2. - 14., 646., 20., 42);
        if at > 0 {
            text(
                &mut commands,
                "‹",
                PAGE_ARROW_X.0 - 5.,
                PAGE_ARROW_Y,
                26.,
                42,
            );
        }
        if at + 1 < vis.len() {
            text(
                &mut commands,
                "›",
                PAGE_ARROW_X.1 - 5.,
                PAGE_ARROW_Y,
                26.,
                42,
            );
        }
    }
    if tab != page.home && !heart {
        // Only the profile's open page has loaded contents (Open in D404).
        return;
    }
    if heart {
        let mut y = 134.;
        for l in wrap(
            "You haven't hearted any objects yet - maybe you hate them all?",
            22,
        ) {
            text(&mut commands, &l, px + 56., y, 21., 42);
            y += 28.;
        }
        return;
    }
    let line = |commands: &mut Commands, y: f32| {
        commands.spawn((
            PageNode,
            node(px + 18., y, pw - 36., 1.5),
            BackgroundColor(Color::srgba(0.85, 0.72, 0.15, 0.8)),
            GlobalZIndex(41),
        ));
    };
    let is_list = !page.list.is_empty();
    // Functions row: the page's own main-menu icon on the teal tile (focus at the first cell).
    if !is_list {
        text(&mut commands, "Functions", px + 18., 133., 19., 42);
        line(&mut commands, 148.);
    }
    // Personal's first function tile is the Popit Cursor.
    let main = assets
        .items
        .iter()
        .find(|i| i.id == if id == PERSONAL_BUTTON { 2 } else { id });
    let fn_page = !page.functions.is_empty();
    let focus_fn = !is_list && !fn_page && row == 0 && col == 0;
    let fn_rows = page.functions.len().div_ceil(GRID_COLS);
    let header_focus = fn_page && !page.kits.is_empty() && row == fn_rows;
    let tile = |commands: &mut Commands, cx: f32, cy: f32, size: f32| {
        commands.spawn((
            PageNode,
            Node {
                border_radius: BorderRadius::all(Val::Px(size * 0.3 * scale)),
                ..node(cx - size / 2., cy - size / 2., size, size)
            },
            BackgroundColor(TEAL),
            GlobalZIndex(41),
        ));
    };
    let mut focus_at = (GRID_X0, 180.);
    let mut focus_title = if fn_page {
        String::new()
    } else {
        main.map_or_else(String::new, |i| i.label.clone())
    };
    if fn_page {
        for (k, (title, img)) in page.functions.iter().enumerate() {
            let (r, c) = (k / GRID_COLS, k % GRID_COLS);
            let (cx, cy) = (
                GRID_X0 + c as f32 * GRID_PITCH,
                180. + r as f32 * GRID_PITCH,
            );
            let focused = r == row && c == col && !header_focus;
            let sz = if focused { 52. } else { 38. };
            if focused {
                tile(&mut commands, cx, cy, 64.);
                focus_at = (cx, cy);
                focus_title = title.clone();
            }
            commands.spawn((
                PageNode,
                node(cx - sz / 2., cy - sz / 2., sz, sz),
                ImageNode::new(images.add(img.clone())),
                GlobalZIndex(42),
            ));
        }
        if !page.kits.is_empty() {
            // MEASURED: header 43 px under the last tile row (text y), rule 14 px below, items 45 px
            // under the header; focused header is a teal bar across the panel.
            let hy = 180. + (fn_rows as f32 - 1.) * GRID_PITCH + 43.;
            if header_focus {
                commands.spawn((
                    PageNode,
                    Node {
                        border_radius: BorderRadius::all(Val::Px(14. * scale)),
                        ..node(px + 12., hy - 15., pw - 24., 29.)
                    },
                    BackgroundColor(TEAL),
                    GlobalZIndex(41),
                ));
                focus_at = (px - 20., hy);
            }
            text(&mut commands, "Gameplay Kits", px + 18., hy, 19., 42);
            line(&mut commands, hy + 14.);
            text(&mut commands, "^", px + pw - 30., hy, 16., 42);
            for (k, (title, img)) in page.kits.iter().enumerate() {
                let (cx, cy) = (GRID_X0 + k as f32 * GRID_PITCH, hy + 45.);
                let focused = row == fn_rows + 1 && col == k;
                if focused {
                    commands.spawn((
                        PageNode,
                        Node {
                            border_radius: BorderRadius::all(Val::Px(32. * scale)),
                            ..node(cx - 32., cy - 32., 64., 64.)
                        },
                        BackgroundColor(TEAL),
                        GlobalZIndex(41),
                    ));
                    focus_at = (cx, cy);
                    focus_title = title.clone();
                }
                let sz = if focused { 52. } else { 32. };
                commands.spawn((
                    PageNode,
                    node(cx - sz / 2., cy - sz / 2., sz, sz),
                    ImageNode::new(images.add(img.clone())),
                    GlobalZIndex(42),
                ));
            }
        }
    }
    if focus_fn {
        tile(&mut commands, GRID_X0, 180., 64.);
    }
    if let Some(img) = main
        .and_then(|i| i.frames.last().cloned())
        .filter(|_| !is_list && !fn_page)
    {
        let sz = if focus_fn { 52. } else { 38. };
        commands.spawn((
            PageNode,
            node(GRID_X0 - sz / 2., 180. - sz / 2., sz, sz),
            ImageNode::new(images.add(img)),
            GlobalZIndex(42),
        ));
    }
    if let Some(msg) = page.message {
        let mut y = 392.;
        for l in wrap(msg, 22) {
            text(&mut commands, &l, px + 56., y, 21., 42);
            y += 24.;
        }
    }
    // List page: one row per entry, the focused row on a teal bar with a large icon.
    if is_list {
        let first = row.saturating_sub(LIST_VISIBLE - 1);
        for (k, (title, img)) in page.list.iter().enumerate().skip(first).take(LIST_VISIBLE) {
            let cy = LIST_Y0 + (k - first) as f32 * LIST_PITCH;
            let focused = k == row;
            if focused {
                commands.spawn((
                    PageNode,
                    Node {
                        border_radius: BorderRadius::all(Val::Px(20. * scale)),
                        ..node(GRID_X0 + 3., cy - 20., 1213. - GRID_X0 - 3., 40.)
                    },
                    BackgroundColor(TEAL),
                    GlobalZIndex(41),
                ));
                focus_at = (GRID_X0, cy);
                focus_title = title.clone();
            }
            let sz = if focused { 64. } else { 38. };
            commands.spawn((
                PageNode,
                node(GRID_X0 - sz / 2., cy - sz / 2., sz, sz),
                ImageNode::new(images.add(img.clone())),
                GlobalZIndex(42),
            ));
            text(&mut commands, title, LIST_LABEL_X, cy, 21., 42);
        }
    }
    // Grid: section headers and 6-column icon rows below the functions row.
    let mut y = 220.;
    let mut idx = 0usize;
    for (name, items) in &page.sections {
        if y > 600. {
            break;
        }
        text(&mut commands, name, px + 18., y, 19., 42);
        line(&mut commands, y + 16.);
        let rows_n = items.len().div_ceil(GRID_COLS);
        let top = y + 45.;
        for (k, (title, img)) in items.iter().enumerate() {
            let (r, c) = (k / GRID_COLS, k % GRID_COLS);
            let (cx, cy) = (GRID_X0 + c as f32 * GRID_PITCH, top + r as f32 * GRID_PITCH);
            if cy > 628. {
                idx += 1;
                continue;
            }
            // Cell (0,0) is the functions tile; grid cells are numbered from row 1.
            let focused = !focus_fn && idx + GRID_COLS == row * GRID_COLS + col;
            let sz = if focused { 52. } else { 38. };
            if focused {
                tile(&mut commands, cx, cy, 64.);
                focus_at = (cx, cy);
                focus_title = title.clone();
            }
            commands.spawn((
                PageNode,
                node(cx - sz / 2., cy - sz / 2., sz, sz),
                ImageNode::new(images.add(img.clone())),
                GlobalZIndex(42),
            ));
            idx += 1;
        }
        y = top + rows_n as f32 * GRID_PITCH + 6.;
    }
    // Tooltip pill left of the focus tile, tail pointing right; long titles wrap to two lines.
    let lines = if focus_title.chars().count() > 22 {
        wrap(&focus_title, 22)
    } else {
        vec![focus_title.clone()]
    };
    let widest = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(4)
        .max(4) as f32;
    let (lw, lh) = (widest * 13. - 2., PILL_H + (lines.len() as f32 - 1.) * 22.);
    let (lx, ly) = (focus_at.0 - 32. - 8. - lw, focus_at.1 - 8. - lh / 2.);
    if !header_focus {
        commands.spawn((
            PageNode,
            Node {
                border_radius: BorderRadius::all(Val::Px(PILL_H / 2. * scale)),
                ..node(lx, ly, lw, lh)
            },
            BackgroundColor(PILL),
            GlobalZIndex(43),
        ));
        for k in 0..TAIL_STEPS {
            let (len, h) = (TAIL_LEN_H / TAIL_STEPS as f32, 11. - 2. * k as f32);
            commands.spawn((
                PageNode,
                node(
                    lx + lw - 1. + k as f32 * len,
                    ly + lh / 2. - h / 2.,
                    len + 1.,
                    h,
                ),
                BackgroundColor(PILL),
                GlobalZIndex(43),
            ));
        }
        for (i, l) in lines.iter().enumerate() {
            text(
                &mut commands,
                l,
                lx + 10.,
                ly + PILL_H / 2. + 1. + i as f32 * 22.,
                19.,
                44,
            );
        }
    }
    // The header's prompts are "Hide" above and "More!" below the bar (MEASURED).
    let (cx, cy, prompts) = if header_focus {
        (
            926.,
            focus_at.1,
            [("Hide", -61., -47., "X"), ("More!", -51., 46., "[]")],
        )
    } else {
        (
            lx + lw / 2.,
            ly + lh / 2.,
            [("USE!", -61., -62., "X"), ("More!", -51., 55., "[]")],
        )
    };
    for (label, dx, dy, glyph) in prompts {
        let (hx, hy) = (cx + dx, cy + dy);
        commands.spawn((
            PageNode,
            Node {
                border_radius: BorderRadius::all(Val::Px(18. * scale)),
                ..node(hx - 62., hy - 20., 124., 36.)
            },
            BackgroundColor(Color::srgba(0.66, 0.6, 0.15, 0.9)),
            GlobalZIndex(43),
        ));
        text(&mut commands, label, hx - 50., hy - 1., 24., 44);
        text(&mut commands, glyph, hx + 34., hy - 1., 20., 44);
    }
}

/// "COSTUME_FACE_GLASSES" -> "Face Glasses" (untranslated category tags).
fn humanize(tag: &str) -> String {
    tag.trim_start_matches("COSTUME_")
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for w in s.split_whitespace() {
        let last = out.last_mut().expect("never empty");
        if !last.is_empty() && last.len() + 1 + w.len() > width {
            out.push(w.to_owned());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(w);
        }
    }
    out
}

fn label_font() -> TextFont {
    TextFont {
        font: bevy::text::FontSource::SansSerif,
        weight: bevy::text::FontWeight::BOLD,
        ..TextFont::from_font_size(19.)
    }
}

fn spawn_nodes(
    commands: &mut Commands,
    assets: &PopitAssets,
    handles: &[(u32, Vec<Handle<Image>>)],
    square: Handle<Image>,
    card: Handle<Image>,
) {
    let abs = || Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    commands.spawn((
        Part(Role::Card),
        abs(),
        ImageNode::new(card),
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
    for i in 0..TAIL_STEPS {
        commands.spawn((
            Part(Role::Tail(i)),
            abs(),
            BackgroundColor(PILL),
            GlobalZIndex(43),
            Visibility::Hidden,
        ));
    }
    commands
        .spawn((
            Part(Role::Label),
            Node {
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PILL),
            GlobalZIndex(44),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Part(Role::LabelText),
                Text::new(""),
                label_font(),
                TextColor(Color::WHITE),
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
                label_font(),
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
    commands.spawn((
        Part(Role::CursorRing),
        Node {
            position_type: PositionType::Absolute,
            border: UiRect::all(Val::Px(2.)),
            ..default()
        },
        BorderColor::all(CORD),
        GlobalZIndex(45),
        Visibility::Hidden,
    ));
    commands.spawn((
        Part(Role::CursorDot),
        abs(),
        BackgroundColor(CORD),
        GlobalZIndex(45),
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
