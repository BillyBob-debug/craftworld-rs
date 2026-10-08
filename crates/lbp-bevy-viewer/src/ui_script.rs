//! U12b: `--ui-script CLASS`, draws what one of the user's own UI scripts builds. Loads
//! every script from the catalog (under its MAP GUIDs), runs one function headlessly on the
//! script VM with the Gooey host (`lbp-ui-script`, stub natives) and shows the frame tree
//! it built, with the run's counts on top. The layout is a placeholder (frame property
//! meanings are unknown); labels and structure come from the script.
//! Usage: `--ui-script CLASS --snapshot SNAPSHOT --game-dir DIR [--ui-script-function NAME]
//! [--ui-script-report] [--qa-hide-ui] [--qa-capture local-analysis/NEW.png]`. The default function is
//! `UpdateUI__Q5Gooey` (the Pod menus' draw entry, PC run D136).
use crate::gooey_ui::run_gooey;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_gooey::Uid;
use lbp_preview::story::load_strings;
use lbp_script_vm::Vm;
use lbp_ui_script::{load, outline, run, tree::ui_tree, GooeyHost, RunReport};
use std::path::PathBuf;

pub const ROOT: Uid = Uid(0x5553_0000);
const BUDGET: u64 = 5_000_000;
pub const DEFAULT_FUNCTION: &str = "UpdateUI__Q5Gooey";

struct Args {
    snapshot: PathBuf,
    game: PathBuf,
    class: String,
    function: String,
    report: bool,
    hide_header: bool,
}

fn args() -> Result<Args, String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    Ok(Args {
        snapshot: value("--snapshot")
            .ok_or("--ui-script requires --snapshot")?
            .into(),
        game: value("--game-dir")
            .ok_or("--ui-script requires --game-dir")?
            .into(),
        class: value("--ui-script").ok_or("--ui-script requires a CLASS")?,
        function: value("--ui-script-function").unwrap_or_else(|| DEFAULT_FUNCTION.to_owned()),
        report: all.iter().any(|a| a == "--ui-script-report"),
        hide_header: all.iter().any(|a| a == "--qa-hide-ui"),
    })
}

/// Count lines shown above the tree and printed by the report.
pub fn summary(r: &RunReport, scripts: usize, not_read: usize) -> Vec<String> {
    let c = &r.counts;
    vec![
        format!(
            "{}.{}: {} after {} instructions (scripts loaded {scripts}, not read {not_read})",
            r.class,
            r.function,
            match &r.result {
                Ok(v) => format!("returned {v}"),
                Err(f) => format!(
                    "FAULT {f} at {}",
                    r.fault_site.as_deref().unwrap_or("?")
                ),
            },
            r.instructions
        ),
        format!(
            "Native calls {} ({} distinct, stubbed unless Gooey/LAMS); strings translated {}/{}; frames unclosed {}, unbalanced ends {}",
            c.native_calls,
            c.by_native.len(),
            c.translated,
            c.translated + c.untranslated,
            c.unclosed,
            c.unbalanced_end
        ),
        format!(
            "Supplied objects {}: {}",
            r.supplied.len(),
            if r.supplied.is_empty() {
                "none".to_owned()
            } else {
                r.supplied.join("; ")
            }
        ),
        "Placeholder layout: frame property meanings are unknown; labels and nesting come from the script.".to_owned(),
    ]
}

/// Loads every script, runs `class.function` headlessly and returns the report with its
/// count lines (also printed, with the string-table states).
pub fn run_script(
    snapshot: &std::path::Path,
    game: &std::path::Path,
    class_name: &str,
    function: &str,
) -> Result<(RunReport, Vec<String>), String> {
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(snapshot, &control)?;
    let source = AttachedSource::attach(game, &catalog, &control)?;
    let (strings, tables) = load_strings(&source, &catalog, &control);
    let (scripts, not_read) = load::load(&source, &catalog, &control)?;
    let count = scripts.len();
    let mut vm = Vm::default();
    let mut target = None;
    for (guid, s) in scripts {
        if s.class_name == class_name && target.is_none() {
            target = Some((vm.load(s.clone(), guid), s));
        } else {
            vm.load(s, guid);
        }
    }
    let (class, script) = target.ok_or_else(|| format!("class {class_name} not found"))?;
    let translate = |k: u32| strings.get(k).map(str::to_owned);
    let mut host = GooeyHost::new(&translate);
    let report = run(&mut vm, &mut host, class, &script, function, BUDGET);
    let header = summary(&report, count, not_read);
    for line in &header {
        println!("{line}");
    }
    for t in &tables {
        println!("table g{}: {:?} {}", t.guid, t.state, t.detail);
    }
    Ok((report, header))
}

pub fn run_ui_script() -> Result<(), String> {
    let a = args()?;
    let (report, header) = run_script(&a.snapshot, &a.game, &a.class, &a.function)?;
    if a.report {
        for line in outline(&report.tree, 60) {
            println!("  {line}");
        }
        return Ok(());
    }
    let root = report.tree;
    // `--qa-hide-ui`: the count lines are printed only, so the tree gets the whole window.
    let header = if a.hide_header { Vec::new() } else { header };
    run_gooey(
        "UI script",
        Box::new(move |input| ui_tree(&root, ROOT, &header, input)),
        Vec::new(),
        None,
        None,
    );
    Ok(())
}
