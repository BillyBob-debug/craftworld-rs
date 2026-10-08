//! Rt PC check (docs/level-scripts.md): which scripts levels use and how they run.
//!
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON [--entry=NAME]... [--ticks=N]
//! level_scripts SNAPSHOT GAME-DIRS --all NEW-JSON
//!
//! Loads every magic-confirmed script with its MAP GUID (the U12 loader), reads the level(s)
//! and reports, per level: scripted Things (PScript with a resource GUID), how many resolve
//! to a loaded class, unresolved GUIDs, and whether the stored field names match the class
//! layout (class fields and superclass fields). Over all classes in use it counts function
//! names that have code, which is how the create/update entry names get settled. With
//! `--entry`, one level runs that function on every scripted Thing and resumes yielded
//! scripts for `--ticks` ticks (default 30), reporting outcomes, faults, unbound natives
//! and `WRITE` lines. Names and counts only; no bytecode. Output: a NEW JSON file under
//! ignored `local-analysis/`. Counting unit: Thing with a PScript part (instances) or
//! distinct script class (function names).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{instances, LevelWorld, Outcome, Runner};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const BUDGET_PER_CALL: u64 = 1_000_000;
const MAX_FAULTS: usize = 30;

fn main() {
    if let Err(e) = run() {
        eprintln!("level_scripts: {e}");
        std::process::exit(1);
    }
}

#[derive(Default)]
struct Totals {
    levels: usize,
    levels_not_read: usize,
    things: usize,
    scripted: usize,
    with_fields: usize,
    resolved: usize,
    unresolved: BTreeMap<u32, usize>,
    by_class: BTreeMap<String, usize>,
    fields_matched: usize,
    fields_missing_in_class: usize,
    unconverted: usize,
    classes: BTreeSet<lbp_script_vm::ClassId>,
}

fn add_level(vm: &Vm, world: &LevelWorld, t: &mut Totals) -> serde_json::Value {
    t.levels += 1;
    t.things += world.things.len();
    t.unconverted += world.unconverted;
    let found = instances(vm, world);
    let mut level_classes = BTreeMap::<String, usize>::new();
    let mut unresolved = BTreeSet::new();
    for inst in &found {
        t.scripted += 1;
        let state = &world.things[inst.thing as usize];
        t.with_fields += usize::from(state.members.is_some());
        match inst.class {
            Some(c) => {
                t.resolved += 1;
                t.classes.insert(c);
                let name = vm.class_name(c).to_owned();
                *level_classes.entry(name.clone()).or_default() += 1;
                *t.by_class.entry(name).or_default() += 1;
                if let Some(m) = &state.members {
                    let layout: BTreeSet<String> = vm.field_names(c).into_iter().collect();
                    for f in m.keys() {
                        if layout.contains(f) {
                            t.fields_matched += 1;
                        } else {
                            t.fields_missing_in_class += 1;
                        }
                    }
                }
            }
            None => {
                *t.unresolved.entry(inst.guid).or_default() += 1;
                unresolved.insert(format!("{:x}", inst.guid));
            }
        }
    }
    serde_json::json!({
        "things": world.things.len(),
        "scripted": found.len(),
        "classes": level_classes,
        "unresolved_guids": unresolved,
    })
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let entries: Vec<&str> = all
        .iter()
        .filter_map(|a| a.strip_prefix("--entry="))
        .collect();
    let ticks: usize = all
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .map_or(Ok(30), str::parse)
        .map_err(|e| format!("--ticks: {e}"))?;
    let args: Vec<&String> = all
        .iter()
        .filter(|a| !a.starts_with("--entry=") && !a.starts_with("--ticks="))
        .collect();
    if args.len() != 5 {
        return Err(
            "usage: level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON \
                    [--entry=NAME]... [--ticks=N]"
                .into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let every = args[3] == "--all";
    if every && !entries.is_empty() {
        return Err("--entry runs one level, not --all".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let (scripts, scripts_not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
    let mut vm = Vm::default();
    let scripts_loaded = scripts.len();
    let with_guid = scripts.iter().filter(|(g, _)| g.is_some()).count();
    for (guid, s) in scripts {
        vm.load(s, guid);
    }

    let mut keys = Vec::new();
    if every {
        let mut hashes = BTreeMap::new();
        for r in &catalog.snapshot.resources {
            if r.display_type() == "level" {
                hashes
                    .entry(r.map_row.sha1.to_ascii_lowercase())
                    .or_insert_with(|| r.map_row.path.clone());
            }
        }
        for (h, path) in hashes {
            if let Ok(key) = lbp_preview::resolve(&catalog, None, Some(&h)) {
                keys.push((key, path));
            }
        }
    } else {
        let (key, label) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(args[3]))?;
        keys.push((key, label));
    }

    let mut totals = Totals::default();
    let mut levels = Vec::new();
    let mut last_world = None;
    for (key, label) in &keys {
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let report = match read {
            Ok(r) => r,
            Err(_) => {
                totals.levels_not_read += 1;
                continue;
            }
        };
        let world = LevelWorld::from_things(&report.things);
        let mut row = add_level(&vm, &world, &mut totals);
        row["level"] = serde_json::json!(label);
        row["stopped"] = serde_json::json!(report.stop.is_some());
        levels.push(row);
        last_world = Some(world);
    }

    // Function names with code over the classes in use.
    let mut names = BTreeMap::<String, usize>::new();
    for &c in &totals.classes {
        for n in vm.function_names(c) {
            *names.entry(n.to_owned()).or_default() += 1;
        }
    }
    let mut ranked: Vec<_> = names.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(60);

    // Optional run on the single level.
    let mut runs = serde_json::Map::new();
    if let (false, Some(mut world)) = (entries.is_empty(), last_world) {
        world.add_class_defaults(&vm);
        let mut host = Bindings::new(world);
        let found = instances(&vm, &host.world);
        let mut runner = Runner::default();
        for entry in &entries {
            let mut outcomes = BTreeMap::<String, usize>::new();
            let mut faults = Vec::new();
            // Faults by site and message, Thing numbers removed.
            let mut fault_kinds = BTreeMap::<String, usize>::new();
            let mut note = |o: &Outcome, thing: u32, outcomes: &mut BTreeMap<String, usize>| {
                let key = match o {
                    Outcome::Fault(_) => "fault",
                    Outcome::Done => "done",
                    Outcome::Yielded => "yielded",
                    Outcome::NoClass => "no_class",
                    Outcome::NoFunction => "no_function",
                };
                *outcomes.entry(key.into()).or_default() += 1;
                if let Outcome::Fault(f) = o {
                    let kind = f.replace(&format!("thing {thing} "), "thing # ");
                    *fault_kinds.entry(kind).or_default() += 1;
                    if faults.len() < MAX_FAULTS {
                        faults.push(format!("thing {thing}: {f}"));
                    }
                }
            };
            for inst in &found {
                let mut budget = BUDGET_PER_CALL;
                let o = runner.start(&mut vm, &mut host, *inst, entry, &mut budget);
                note(&o, inst.thing, &mut outcomes);
            }
            let mut resumed = BTreeMap::<String, usize>::new();
            for _ in 0..ticks {
                if runner.pending() == 0 {
                    break;
                }
                let mut budget = BUDGET_PER_CALL.saturating_mul(runner.pending() as u64);
                for (thing, o) in runner.tick(&mut vm, &mut host, &mut budget) {
                    note(&o, thing, &mut resumed);
                }
            }
            runs.insert(
                (*entry).to_owned(),
                serde_json::json!({
                    "started": outcomes,
                    "resumed": resumed,
                    "still_pending": runner.pending(),
                    "first_faults": faults,
                    "faults_by_kind": fault_kinds,
                }),
            );
        }
        let mut unbound: Vec<_> = host.unbound.iter().collect();
        unbound.sort_by(|a, b| b.1.cmp(a.1));
        unbound.truncate(40);
        runs.insert(
            "unbound_natives".into(),
            serde_json::json!(unbound
                .iter()
                .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
                .collect::<Vec<_>>()),
        );
        runs.insert(
            "written".into(),
            serde_json::json!(host.world.written.iter().take(40).collect::<Vec<_>>()),
        );
    }

    let mut by_class: Vec<_> = totals.by_class.iter().collect();
    by_class.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let unresolved: BTreeMap<String, usize> = totals
        .unresolved
        .iter()
        .map(|(g, n)| (format!("{g:x}"), *n))
        .collect();
    let doc = serde_json::json!({
        "schema_version": 1,
        "unit": "Thing with a PScript resource GUID; function names per distinct class",
        "scripts_loaded": scripts_loaded,
        "scripts_with_guid": with_guid,
        "scripts_not_read": scripts_not_read,
        "levels": totals.levels,
        "levels_not_read": totals.levels_not_read,
        "things": totals.things,
        "scripted_things": totals.scripted,
        "with_fields": totals.with_fields,
        "resolved": totals.resolved,
        "unresolved_by_guid": unresolved,
        "classes_in_use": totals.classes.len(),
        "instances_by_class": by_class,
        "fields_matched": totals.fields_matched,
        "fields_missing_in_class": totals.fields_missing_in_class,
        "unconverted_values": totals.unconverted,
        "function_names": ranked,
        "per_level": levels,
        "runs": runs,
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "scripts {scripts_loaded} (with GUID {with_guid}, not read {scripts_not_read}); \
         levels {} (not read {}); scripted things {} resolved {} (classes {}), unresolved \
         GUIDs {}; fields matched {} missing in class {}; unconverted values {}",
        totals.levels,
        totals.levels_not_read,
        totals.scripted,
        totals.resolved,
        totals.classes.len(),
        totals.unresolved.len(),
        totals.fields_matched,
        totals.fields_missing_in_class,
        totals.unconverted
    );
    for (entry, r) in &runs {
        if entry != "unbound_natives" && entry != "written" {
            println!("--entry={entry}: {r}");
        }
    }
    Ok(())
}
