//! Runs the game's own test scripts in the VM (docs/script-natives.md, "PC checks").
//! A test script is one that calls a `Test.*` native. Every function of it that takes no
//! arguments and has code is run once with the bindings over a world that only records
//! `Test.*` checks. A non-static function runs on a fresh instance of its class, placed in
//! register 0 as `this` (as the U12 Gooey host does, INFERRED from the D134 trace). A yield resumes on the next "tick", up to a tick limit. Writes the
//! totals and one row per function (class and function names, outcome, checks) to a NEW
//! JSON file under ignored `local-analysis/`.
//! Usage: script_tests SCRIPT-DIR NEW-JSON
//! Counting unit: function run; checks are counted per `Test.*` call executed.
use lbp_level::script::{self, class, Class, Script};
use lbp_level::Limits;
use lbp_script_vm::natives::{Bindings, World};
use lbp_script_vm::{Fault, Step, Value, Vm};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const BUDGET: u64 = 1_000_000;
const MAX_TICKS: usize = 600;

#[derive(Default)]
struct Recorder {
    passed: usize,
    failed: usize,
    /// First failures (message, line), capped.
    failures: Vec<(String, i32)>,
}

impl World for Recorder {
    fn test_check(&mut self, passed: bool, message: &str, line: i32) {
        if passed {
            self.passed += 1;
        } else {
            self.failed += 1;
            if self.failures.len() < 20 {
                self.failures.push((message.to_owned(), line));
            }
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("script_tests: {e}");
        std::process::exit(1);
    }
}

fn calls_test(s: &Script) -> bool {
    s.bytecode.iter().any(|bits| {
        class((bits & 0xff) as u8) == Some(Class::Call)
            && s.function_refs
                .get(((bits >> 32) & 0xffff) as usize)
                .and_then(|f| s.type_refs.get(f.type_ref as usize))
                .and_then(|t| s.name(t.type_name))
                == Some("Test")
    })
}

/// Functions with code and no argument rows, with whether they are static (modifier bit 0).
fn entry_points(s: &Script) -> Vec<(String, bool)> {
    s.function_defs
        .iter()
        .filter(|f| f.modifiers & 2 == 0 && f.arguments.begin == f.arguments.end)
        .filter_map(|f| s.name(f.name).map(|n| (n.to_owned(), f.modifiers & 1 != 0)))
        .collect()
}

/// Short outcome label for the totals; the full fault goes in the row.
fn fault_kind(f: &Fault) -> String {
    let debug = format!("{f:?}");
    match f {
        Fault::Native(m) if m.starts_with("unbound native") => "unbound_native".into(),
        Fault::Native(m) if m.contains("world does not provide") => "world_feature".into(),
        _ => debug
            .split(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: script_tests SCRIPT-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[2]);
    if !output.starts_with("local-analysis") {
        return Err("output must be under local-analysis/".into());
    }
    if output.exists() {
        return Err("output exists".into());
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
    // Load every script so test scripts can call into the others.
    let mut vm = Vm::default();
    let mut tests = Vec::new();
    for p in &paths {
        if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("file size budget".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        if let Some(s) = script::read(&bytes, Limits::default(), &|| false)
            .ok()
            .and_then(|r| r.result.ok())
        {
            let entries = calls_test(&s).then(|| entry_points(&s));
            let id = vm.load(s, None);
            if let Some(e) = entries {
                tests.push((id, e));
            }
        }
    }
    let mut host = Bindings::new(Recorder::default());
    let mut outcomes = BTreeMap::<String, usize>::new();
    let mut rows = Vec::new();
    let (mut passed, mut failed) = (0usize, 0usize);
    for (id, entries) in &tests {
        for (name, is_static) in entries {
            host.world = Recorder::default();
            let mut budget = BUDGET;
            let mut ticks = 0usize;
            let mut step = if *is_static {
                vm.start(&mut host, *id, name, &[], &mut budget)
            } else {
                vm.instantiate(*id).and_then(|this| {
                    vm.start_registers(
                        &mut host,
                        *id,
                        name,
                        &[(0, Value::Object(Some(this)))],
                        &mut budget,
                    )
                })
            };
            let outcome = loop {
                match step {
                    Ok(Step::Done(_)) => break ("done".to_owned(), None),
                    Ok(Step::Yielded(task)) if ticks < MAX_TICKS => {
                        ticks += 1;
                        step = vm.resume(&mut host, task, &mut budget);
                    }
                    Ok(Step::Yielded(_)) => break ("tick_limit".to_owned(), None),
                    Err(f) => break (fault_kind(&f), Some(format!("{f:?}"))),
                }
            };
            *outcomes.entry(outcome.0.clone()).or_default() += 1;
            passed += host.world.passed;
            failed += host.world.failed;
            rows.push(serde_json::json!({
                "class": vm.class_name(*id),
                "function": name,
                "static": is_static,
                "outcome": outcome.0,
                "fault": outcome.1,
                "ticks": ticks,
                "checks_passed": host.world.passed,
                "checks_failed": host.world.failed,
                "first_failures": host.world.failures,
            }));
        }
    }
    let functions: usize = tests.iter().map(|(_, e)| e.len()).sum();
    let doc = serde_json::json!({
        "schema_version": 1,
        "scope": "scripts in the given directory that call Test.* natives",
        "unit": "function run (no-argument functions with code)",
        "files": paths.len(),
        "test_scripts": tests.len(),
        "functions": functions,
        "outcomes": outcomes,
        "checks_passed": passed,
        "checks_failed": failed,
        "unbound_natives": host.unbound,
        "rows": rows,
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "test scripts {} functions {}; outcomes {:?}; checks passed {} failed {}",
        tests.len(),
        functions,
        outcomes,
        passed,
        failed
    );
    Ok(())
}
