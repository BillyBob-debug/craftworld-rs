//! U12: runs functions of one UI script class headlessly on the VM with the Gooey host
//! and writes the native-call trace and frame tree to a NEW JSON file under ignored
//! `local-analysis/` (docs/u12-gooey-trace.md). Inputs are the user's own scripts
//! extracted locally (the same `.ff` directory `script_check` reads); nothing is committed.
//!
//! Usage:
//!   ui_trace SCRIPT-DIR CLASS                         list the class's functions
//!   ui_trace SCRIPT-DIR CLASS FUNCTION|all NEW-JSON [TRANS-FILE...]
//! `all` runs every non-native function of the class, each on a fresh instance and host.
//! TRANS-FILEs are `.trans` tables (later files win per key) for `LAMS.Translate`.
use lbp_level::{script, Limits};
use lbp_resource::translation::{parse_translation_table, TranslationLimits};
use lbp_script_vm::Vm;
use lbp_ui_script::{functions, outline, run, GooeyHost, RunReport};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const BUDGET: u64 = 5_000_000;

fn main() {
    if let Err(e) = run_main() {
        eprintln!("ui_trace: {e}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 && args.len() < 5 {
        return Err(
            "usage: ui_trace SCRIPT-DIR CLASS [FUNCTION|all NEW-JSON [TRANS-FILE...]]".into(),
        );
    }
    let mut paths: Vec<_> = std::fs::read_dir(&args[1])
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ff"))
        .collect();
    paths.sort();
    if paths.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let mut vm = Vm::default();
    let mut target = None;
    let mut not_read = 0usize;
    for p in &paths {
        if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("file size budget".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let Some(s) = script::read(&bytes, Limits::default(), &|| false)
            .ok()
            .and_then(|r| r.result.ok())
        else {
            not_read += 1;
            continue;
        };
        if s.class_name == args[2] && target.is_none() {
            target = Some((vm.load(s.clone(), None), s));
        } else {
            vm.load(s, None);
        }
    }
    let (class, s) = target.ok_or_else(|| format!("class {} not found", args[2]))?;
    println!(
        "loaded {} scripts ({not_read} not read); class {}",
        paths.len() - not_read,
        args[2]
    );
    let list = functions(&s);
    if args.len() == 3 {
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
    let output = Path::new(&args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let mut table = BTreeMap::<u32, String>::new();
    for t in &args[5..] {
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
    let names: Vec<String> = if args[3] == "all" {
        list.iter()
            .filter(|f| !f.is_native)
            .map(|f| f.name.clone())
            .collect()
    } else {
        vec![args[3].clone()]
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
                Err(f) => format!("FAULT {f}"),
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
        "class": args[2], "budget_per_function": BUDGET,
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
