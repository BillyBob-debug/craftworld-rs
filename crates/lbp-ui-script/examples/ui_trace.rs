//! U12: runs functions of one UI script class headlessly on the VM with the Gooey host
//! and writes the native-call trace and frame tree to a NEW JSON file under ignored
//! `local-analysis/` (docs/u12-gooey-trace.md). Inputs are the user's own scripts; nothing
//! is committed.
//!
//! Usage (catalog mode, as `script_census`; scripts get their MAP GUIDs, so superclasses
//! and GUID type references resolve):
//!   ui_trace SNAPSHOT.json GAME-DIR CLASS [FUNCTION|all NEW-JSON [TRANS-FILE...]]
//! Directory mode (the `.ff` directory `script_check` reads; no GUIDs, so superclasses do
//! not resolve and inherited fields are missing):
//!   ui_trace SCRIPT-DIR CLASS [FUNCTION|all NEW-JSON [TRANS-FILE...]]
//! Without FUNCTION it lists the class's functions. `all` runs every non-native function,
//! each on a fresh instance and host. TRANS-FILEs are `.trans` tables (later files win per
//! key) for `LAMS.Translate`.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::script::{self, Script};
use lbp_level::Limits;
use lbp_resource::translation::{parse_translation_table, TranslationLimits};
use lbp_script_vm::Vm;
use lbp_ui_script::{functions, outline, run, GooeyHost, RunReport};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const PAGE: usize = 4096;
const BUDGET: u64 = 5_000_000;

fn main() {
    if let Err(e) = run_main() {
        eprintln!("ui_trace: {e}");
        std::process::exit(1);
    }
}

/// Scripts with their GUID (if known), and the count not read.
type Loaded = (Vec<(Option<u32>, Script)>, usize);

fn parse(bytes: &[u8]) -> Option<Script> {
    script::read(bytes, Limits::default(), &|| false)
        .ok()
        .and_then(|r| r.result.ok())
}

/// Scripts in a `.ff` directory, without GUIDs.
fn from_dir(dir: &str) -> Result<Loaded, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ff"))
        .collect();
    paths.sort();
    if paths.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let (mut out, mut not_read) = (Vec::new(), 0);
    for p in &paths {
        if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("file size budget".into());
        }
        match parse(&std::fs::read(p).map_err(|e| e.to_string())?) {
            Some(s) => out.push((None, s)),
            None => not_read += 1,
        }
    }
    Ok((out, not_read))
}

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size as u64 > MAX_FILE_BYTES {
        return Err("archived size exceeds input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(
            key,
            bytes.len() as u64,
            (size - bytes.len()).min(PAGE),
            control,
        )?;
        if page.bytes.is_empty() {
            return Err("truncated source read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

/// Every magic-confirmed `script` MAP row, grouped by content hash; one script per GUID
/// (a hash under several GUIDs is loaded once per GUID).
fn from_catalog(snapshot: &str, game_dir: &str) -> Result<Loaded, String> {
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(snapshot), &control)?;
    let source = AttachedSource::attach(Path::new(game_dir), &catalog, &control)?;
    let mut by_hash: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "script" {
            let e = by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default();
            if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
                e.insert(g);
            }
        }
    }
    if by_hash.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let (mut out, mut not_read, mut total) = (Vec::new(), 0, 0u64);
    for (sha, guids) in &by_hash {
        let parsed = lbp_preview::resolve(&catalog, None, Some(sha)).and_then(|key| {
            total += key.size;
            if total > MAX_TOTAL_BYTES {
                return Err("total byte budget".to_owned());
            }
            read_all(&source, &key, &control)
        });
        let Some(s) = parsed.ok().as_deref().and_then(parse) else {
            not_read += 1;
            continue;
        };
        if guids.is_empty() {
            out.push((None, s));
        } else {
            for g in guids {
                out.push((Some(*g), s.clone()));
            }
        }
    }
    Ok((out, not_read))
}

fn run_main() -> Result<(), String> {
    let all: Vec<_> = std::env::args().collect();
    let catalog_mode = all.get(1).is_some_and(|a| a.ends_with(".json"));
    let skip = if catalog_mode { 3 } else { 2 };
    let args = all.get(skip..).unwrap_or_default();
    if args.len() != 1 && args.len() < 3 {
        return Err(
            "usage: ui_trace (SNAPSHOT.json GAME-DIR | SCRIPT-DIR) CLASS \
             [FUNCTION|all NEW-JSON [TRANS-FILE...]]"
                .into(),
        );
    }
    let (scripts, not_read) = if catalog_mode {
        from_catalog(&all[1], &all[2])?
    } else {
        from_dir(&all[1])?
    };
    let loaded_guids: BTreeSet<u32> = scripts.iter().filter_map(|(g, _)| *g).collect();
    let super_unresolved = scripts
        .iter()
        .filter_map(|(_, s)| s.super_class.as_ref())
        .filter(|d| d.guid.is_none_or(|g| !loaded_guids.contains(&g)))
        .count();
    let mut vm = Vm::default();
    let mut target = None;
    let count = scripts.len();
    for (guid, s) in scripts {
        if s.class_name == args[0] && target.is_none() {
            target = Some((vm.load(s.clone(), guid), s));
        } else {
            vm.load(s, guid);
        }
    }
    let (class, s) = target.ok_or_else(|| format!("class {} not found", args[0]))?;
    println!(
        "loaded {count} scripts ({not_read} not read, {} GUIDs); superclass not loaded {super_unresolved}; class {}",
        loaded_guids.len(),
        args[0]
    );
    let list = functions(&s);
    if args.len() == 1 {
        for f in &list {
            println!(
                "  {}{}{} args={} code={}",
                f.name,
                if f.is_static { " static" } else { "" },
                if f.is_native { " native" } else { "" },
                f.arguments,
                f.instructions
            );
        }
        return Ok(());
    }
    let output = Path::new(&args[2]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let mut table = BTreeMap::<u32, String>::new();
    for t in &args[3..] {
        let bytes = std::fs::read(t).map_err(|e| e.to_string())?;
        let parsed = parse_translation_table(&bytes, TranslationLimits::default())
            .map_err(|e| format!("{t}: {e}"))?;
        for e in parsed.entries {
            if let Some(text) = e.text {
                table.insert(e.key, text);
            }
        }
    }
    let translate = |k: u32| table.get(&k).cloned();
    let names: Vec<String> = if args[1] == "all" {
        list.iter()
            .filter(|f| !f.is_native)
            .map(|f| f.name.clone())
            .collect()
    } else {
        vec![args[1].clone()]
    };
    let mut reports: Vec<RunReport> = Vec::new();
    for name in &names {
        let mut host = GooeyHost::new(&translate);
        let r = run(&mut vm, &mut host, class, &s, name, BUDGET);
        let c = &r.counts;
        println!(
            "{name}: {} after {} instructions; receiver {:?} arg offsets {:?}; natives {} \
             (distinct {}), translated {}/{}, frames unbalanced {} unclosed {}, \
             member reads {} writes {}",
            match &r.result {
                Ok(v) => format!("returned {v}"),
                Err(f) => format!("FAULT {f} at {}", r.fault_site.as_deref().unwrap_or("?")),
            },
            r.instructions,
            r.receiver,
            r.argument_offsets,
            c.native_calls,
            c.by_native.len(),
            c.translated,
            c.translated + c.untranslated,
            c.unbalanced_end,
            c.unclosed,
            c.member_reads,
            c.member_writes
        );
        if names.len() == 1 {
            for line in outline(&r.tree, 40) {
                println!("  {line}");
            }
        }
        reports.push(r);
    }
    let faults = reports.iter().filter(|r| r.result.is_err()).count();
    let with_gooey = reports
        .iter()
        .filter(|r| r.counts.by_native.keys().any(|k| k.starts_with("Gooey.")))
        .count();
    println!(
        "functions run {}; faulted {faults}; drew Gooey frames {with_gooey}",
        reports.len()
    );
    let doc = serde_json::json!({
        "schema_version": 1, "milestone": "U12",
        "scope": "one class's functions run headlessly with stub natives",
        "class": args[0], "budget_per_function": BUDGET,
        "functions_run": reports.len(), "faulted": faults, "drew_gooey": with_gooey,
        "runs": reports,
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    Ok(())
}
