//! D400: the Popit card from the game's own `Poppet` script (`gamedata/scripts/poppet.ff`).
//!
//! `Poppet.DoMainMenu(Gooey)` runs on the Gooey host with the game state the card depends on
//! pinned ([`answers`]): it opens frame 10000 (layout, child spacing, highlight, default
//! node) and adds one `DoImageSDFButtonNamed` per button: node id `10001 + index`, the icon
//! (an animated texture GUID, from `Resource.LoadAnimatedTextureByFilename` in `.init`),
//! the SDF colour, the icon size and a tint (grey 0.5 when a progression flag is unset,
//! white when set; observed with the flag answered both ways). Rows come from the script's
//! `DoBreak` calls and the frame's column count. `GetMainMenuTooltip(index)` returns each
//! button's LAMS key.
//!
//! What the script reads (FOUND, `DoMainMenuEdit` listing): edit mode is
//! `Thing.IsEditing` (the player's), the tutorial button needs `GetPlayOrEditMode() == 1`,
//! one PS3 in the session, the overall party host and not a tutorial level. The pinned
//! answers are a single-player session in create mode with every progression flag set.
use crate::{entry_registers, make_object, Frame, GooeyHost, Node};
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_level::script::Script;
use lbp_resource::animated_texture::{parse_animated_texture, AnimatedTexture};
use lbp_script_vm::{ClassId, Step, Value, Vm};
use std::collections::BTreeMap;

pub const CLASS: &str = "Poppet";
pub const CARD_FUNCTION: &str = "DoMainMenu__Q5Gooey";
pub const TOOLTIP_FUNCTION: &str = "GetMainMenuTooltip__i";
/// Node id of button 0; button `i` is `FIRST_BUTTON + i` (FOUND, `DoMainMenuButton`).
pub const FIRST_BUTTON: i32 = 10001;
const BUDGET: u64 = 1_000_000;

/// The natives the card reads, answered as in a single-player session in create mode
/// (`edit`) or play mode.
pub fn answers(edit: bool) -> BTreeMap<String, Value> {
    [
        ("Thing.IsEditing", Value::Bool(edit)),
        ("Game.GetPlayOrEditMode", Value::Int(i32::from(edit))),
        ("Game.GetNumPS3s", Value::Int(1)),
        ("Game.AmIOverallPartyHost", Value::Bool(true)),
        ("Game.IsTutorialLevel", Value::Bool(false)),
        ("Profile.IsLBP2GameProgressionFlagSet", Value::Bool(true)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    /// Button index (`node id - FIRST_BUTTON`), the index `GetMainMenuTooltip` takes.
    pub index: u32,
    /// Animated texture GUID of the icon.
    pub icon: Option<u32>,
    /// Icon size in Gooey units.
    pub size: [f32; 2],
    /// False when the script tints the button grey (locked).
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Card {
    pub rows: Vec<Vec<Button>>,
    pub columns: usize,
    /// `SetFrameDefaultChildSpacing` (Gooey units).
    pub spacing: [f32; 2],
    /// `SetDefaultNode` as a button index.
    pub default_index: Option<u32>,
}

impl Card {
    /// Gooey units from one button centre to the next (icon width + child spacing).
    pub fn pitch(&self) -> f32 {
        let icon = self.rows.iter().flatten().next().map_or(0., |b| b.size[0]);
        icon + self.spacing[0]
    }
}

/// Numbers in a recorded argument string (`Ref(5)`, `Vec4([0.5, 1.0])`, `obj, 16, 16`).
/// Text before the last `(` is a type name (`Vec4`) and is skipped.
fn numbers(s: &str) -> Vec<f32> {
    let s = s.rsplit_once('(').map_or(s, |(_, t)| t);
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn tail<const N: usize>(frame: &Frame, prop: &str) -> Option<[f32; N]> {
    let n = numbers(frame.props.get(prop)?);
    n.get(n.len().checked_sub(N)?..)?.try_into().ok()
}

/// The card from the frame tree of one `DoMainMenu` run: the first frame with buttons.
pub fn card_from_tree(root: &Frame) -> Option<Card> {
    let frame = root.children.iter().find_map(|n| match n {
        Node::Frame(f) if f.children.iter().any(|c| matches!(c, Node::Button { .. })) => Some(f),
        _ => None,
    })?;
    let columns = tail::<2>(frame, "Layout").map_or(usize::MAX, |[c, _]| (c as usize).max(1));
    let mut card = Card {
        columns,
        spacing: tail::<2>(frame, "DefaultChildSpacing").unwrap_or([0., 0.]),
        default_index: tail::<1>(frame, "DefaultNode")
            .and_then(|[n]| u32::try_from(n as i32 - FIRST_BUTTON).ok()),
        ..Default::default()
    };
    let mut row = Vec::new();
    for n in &frame.children {
        match n {
            Node::Button { args, .. } => {
                let id = args.get(1).and_then(|a| a.parse::<i32>().ok())?;
                let icon = args
                    .get(2)
                    .filter(|a| a.starts_with("Ref("))
                    .and_then(|a| numbers(a).first().map(|&g| g as u32));
                let size = args
                    .get(4)
                    .map(|a| numbers(a))
                    .and_then(|v| Some([*v.first()?, *v.get(1)?]))
                    .unwrap_or([0., 0.]);
                let enabled = args
                    .get(5)
                    .and_then(|a| numbers(a).first().copied())
                    .is_none_or(|t| t >= 1.);
                row.push(Button {
                    index: u32::try_from(id - FIRST_BUTTON).ok()?,
                    icon,
                    size,
                    enabled,
                });
                if row.len() == columns {
                    card.rows.push(std::mem::take(&mut row));
                }
            }
            Node::Break { .. } if !row.is_empty() => card.rows.push(std::mem::take(&mut row)),
            _ => {}
        }
    }
    if !row.is_empty() {
        card.rows.push(row);
    }
    Some(card)
}

/// The card plus each button's tooltip LAMS key, from a VM with the scripts loaded.
pub fn run_card(
    vm: &mut Vm,
    class: ClassId,
    script: &Script,
    edit: bool,
) -> Result<(Card, BTreeMap<u32, u32>), String> {
    let none = |_| None;
    let mut host = GooeyHost::new(&none);
    host.answers = answers(edit);
    let r = crate::run(vm, &mut host, class, script, CARD_FUNCTION, BUDGET);
    r.result.map_err(|f| format!("{CARD_FUNCTION}: {f}"))?;
    let card = card_from_tree(&r.tree).ok_or("Poppet card: no button frame")?;
    let mut keys = BTreeMap::new();
    for b in card.rows.iter().flatten() {
        if let Some(k) = tooltip_key(vm, &mut host, class, script, b.index) {
            keys.insert(b.index, k);
        }
    }
    Ok((card, keys))
}

/// `Poppet.GetMainMenuTooltip(index)`: the button's LAMS key.
fn tooltip_key(
    vm: &mut Vm,
    host: &mut GooeyHost,
    class: ClassId,
    script: &Script,
    index: u32,
) -> Option<u32> {
    match call_with_index(vm, host, class, script, TOOLTIP_FUNCTION, index)? {
        Step::Done(Value::Int(k)) => Some(k as u32),
        _ => None,
    }
}

/// Runs a `Poppet` member whose last argument is a button index (the other arguments get
/// their defaults, a `Gooey` argument a fresh instance).
fn call_with_index(
    vm: &mut Vm,
    host: &mut GooeyHost,
    class: ClassId,
    script: &Script,
    function: &str,
    index: u32,
) -> Option<Step> {
    let mut left = BUDGET;
    let mut supplied = Vec::new();
    let this = Value::Object(Some(
        make_object(vm, host, class, &mut left, &mut supplied).ok()?,
    ));
    let e = entry_registers(vm, host, script, function, this, &mut left, &mut supplied).ok()?;
    let at = *e.offsets.iter().rev().find(|&&o| o != 0)?;
    let regs: Vec<_> = e
        .registers
        .into_iter()
        .map(|(r, v)| {
            if i32::from(r) == at {
                (r, Value::Int(index as i32))
            } else {
                (r, v)
            }
        })
        .collect();
    vm.start_registers(host, class, function, &regs, &mut left)
        .ok()
}

/// D401: what pressing a button does, from `Poppet.DoMainMenuButtonResponse(gooey, index)`.
pub const RESPONSE_FUNCTION: &str = "DoMainMenuButtonResponse__Q5Gooeyi";
/// `PushSubModeInventory(collection)` sends `SendSimpleMessage(37, collection)` (FOUND).
pub const INVENTORY_MESSAGE: i32 = 37;

/// The inventory collection (`InventoryCollection.EInventoryCollection`) a button opens:
/// the second argument of the `SendSimpleMessage(37, collection)` the response sends.
/// `None` for buttons that do something else (cursor, restart, chat, tutorials) or are
/// refused in this mode (the response plays `poppet/error`).
pub fn opened_collection(trace: &[crate::TraceCall]) -> Option<i32> {
    trace.iter().find_map(|t| {
        if t.class != CLASS || crate::base_name(&t.function) != "SendSimpleMessage" {
            return None;
        }
        let n: Vec<i32> = t.args.iter().filter_map(|a| a.parse().ok()).collect();
        match n.as_slice() {
            [m, c] if *m == INVENTORY_MESSAGE => Some(*c),
            _ => None,
        }
    })
}

/// Button index -> inventory collection for every button in `buttons`, in edit or play mode.
pub fn run_responses(
    vm: &mut Vm,
    class: ClassId,
    script: &Script,
    edit: bool,
    buttons: impl IntoIterator<Item = u32>,
) -> BTreeMap<u32, i32> {
    let none = |_| None;
    let mut out = BTreeMap::new();
    for b in buttons {
        let mut host = GooeyHost::new(&none);
        host.answers = answers(edit);
        host.answers
            .insert("Game.GetNumAvatars".into(), Value::Int(1));
        host.answers
            .insert("Poppet.GetAllowChat".into(), Value::Bool(true));
        if call_with_index(vm, &mut host, class, script, RESPONSE_FUNCTION, b).is_none() {
            continue;
        }
        if let Some(c) = opened_collection(&host.trace) {
            out.insert(b, c);
        }
    }
    out
}

/// Reads and parses an animated texture by GUID.
pub fn animated_texture(
    source: &AttachedSource,
    catalog: &Catalog,
    guid: u32,
    control: &LoadControl,
) -> Result<AnimatedTexture, String> {
    let key =
        lbp_preview::story::resolve_key(catalog, lbp_preview::story::ResourceKey::Guid(guid))?;
    let bytes = crate::load::read_all(source, &key, control)?;
    parse_animated_texture(&bytes, &|| control.check().is_err()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(id: i32, icon: u32, tint: f32) -> Node {
        Node::Button {
            label: String::new(),
            function: "DoImageSDFButtonNamed__".into(),
            args: vec![
                "Object(Some(ObjectId(3)))".into(),
                id.to_string(),
                format!("Ref({icon})"),
                "Vec4([0.039, 0.498, 0.011, 0.0])".into(),
                "Vec4([128.0, 128.0, 0.0, 0.0])".into(),
                format!("Vec4([{tint}, {tint}, {tint}, 1.0])"),
            ],
        }
    }

    #[test]
    fn d400_rows_follow_breaks_and_the_column_count() {
        let brk = Node::Break {
            function: "DoBreak__".into(),
        };
        let mut f = Frame {
            start: "StartFrameNamed(obj, 10000, 0)".into(),
            ..Default::default()
        };
        for (k, v) in [
            ("Layout", "Object(Some(ObjectId(3))), 3, 3"),
            ("DefaultChildSpacing", "Object(Some(ObjectId(3))), 16, 16"),
            ("DefaultNode", "Object(Some(ObjectId(3))), 10003"),
        ] {
            f.props.insert(k.into(), v.into());
        }
        // The create card's call order: three, a break, then six that wrap at three.
        f.children = vec![
            button(10007, 7, 1.),
            button(10002, 2, 1.),
            button(10009, 9, 0.5),
            brk,
        ];
        f.children
            .extend([10005, 10003, 10001, 10008, 10006, 10004].map(|i| button(i, 1, 1.)));
        let root = Frame {
            start: "root".into(),
            children: vec![Node::Frame(f)],
            ..Default::default()
        };
        let card = card_from_tree(&root).unwrap();
        let rows: Vec<Vec<u32>> = card
            .rows
            .iter()
            .map(|r| r.iter().map(|b| b.index).collect())
            .collect();
        assert_eq!(rows, [vec![6, 1, 8], vec![4, 2, 0], vec![7, 5, 3]]);
        assert_eq!(card.default_index, Some(2));
        assert_eq!(card.rows[0][0].icon, Some(7));
        assert!(!card.rows[0][2].enabled);
        assert_eq!(card.pitch(), 144.);
    }

    #[test]
    fn d401_collection_is_the_inventory_message_argument() {
        let call = |class: &str, function: &str, args: &[&str]| crate::TraceCall {
            class: class.into(),
            function: function.into(),
            args: args.iter().map(|a| a.to_string()).collect(),
        };
        let trace = [
            call(
                "Poppet",
                "TelemetryPoppetMenu__i",
                &["Object(Some(ObjectId(0)))", "5"],
            ),
            call(
                "Poppet",
                "SendSimpleMessage__i",
                &["Object(Some(ObjectId(0)))", "0"],
            ),
            call(
                "Poppet",
                "SendSimpleMessage__ii",
                &["Object(Some(ObjectId(0)))", "37", "3"],
            ),
        ];
        assert_eq!(opened_collection(&trace), Some(3));
        assert_eq!(opened_collection(&trace[..2]), None);
    }
}
