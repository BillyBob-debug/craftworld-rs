//! Rs: how many native call sites in the user's scripts have a binding
//! (docs/script-natives.md, "PC checks"). Loads every `FSHb` file in a directory into one
//! VM, treats a call as native when its owner class has no script code for the function,
//! and writes the totals to a NEW JSON file under ignored `local-analysis/`.
//! Usage: native_coverage SCRIPT-DIR NEW-JSON
//! Counting unit: call instruction (CALL, CALLVo, CALLVsp) in a distinct script file.
//! "Test" = a script that calls a `Test.*` native (the game's own test scripts).
//! "Gameplay" = not a test script, class name without UI words and no call to a Gooey or
//! HUD native (the G10 export's split, minus test scripts).
//! Natives the VM handles itself (`System.Yield__`) count as bound.
use lbp_level::script::{self, class, Class, Script};
use lbp_level::Limits;
use lbp_script_vm::{natives, signature, Vm};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const UI_WORDS: [&str; 8] = [
    "gooey",
    "poppet",
    "popit",
    "start_menu",
    "pod",
    "hud",
    "scoreboard",
    "frontend",
];

fn main() {
    if let Err(e) = run() {
        eprintln!("native_coverage: {e}");
        std::process::exit(1);
    }
}

/// (owner class, mangled name) of every call instruction in a script.
fn call_sites(s: &Script) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for bits in &s.bytecode {
        let op = (bits & 0xff) as u8;
        if class(op) != Some(Class::Call) {
            continue;
        }
        let Some(fref) = s.function_refs.get(((bits >> 32) & 0xffff) as usize) else {
            continue;
        };
        let name = s.name(fref.name).unwrap_or("").to_owned();
        let owner = s
            .type_refs
            .get(fref.type_ref as usize)
            .and_then(|t| s.name(t.type_name))
            .unwrap_or("")
            .to_owned();
        out.push((owner, name));
    }
    out
}

#[derive(Default)]
struct Tally {
    native_calls: usize,
    bound_calls: usize,
    natives: BTreeMap<String, usize>,
}

impl Tally {
    fn add(&mut self, key: &str, bound: bool) {
        self.native_calls += 1;
        self.bound_calls += usize::from(bound);
        *self.natives.entry(key.to_owned()).or_default() += 1;
    }
    fn json(&self, bound_keys: &[String]) -> serde_json::Value {
        let mut ranked: Vec<_> = self.natives.iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let unbound: Vec<_> = ranked
            .iter()
            .filter(|(k, _)| !bound_keys.contains(k))
            .take(100)
            .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
            .collect();
        serde_json::json!({
            "native_calls": self.native_calls,
            "bound_calls": self.bound_calls,
            "distinct_natives": self.natives.len(),
            "distinct_bound": self.natives.keys().filter(|k| bound_keys.contains(k)).count(),
            "top_unbound": unbound,
        })
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: native_coverage SCRIPT-DIR NEW-JSON".into());
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
    let mut vm = Vm::default();
    let mut scripts = Vec::new();
    let mut not_read = 0usize;
    for p in &paths {
        if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("file size budget".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        match script::read(&bytes, Limits::default(), &|| false)
            .ok()
            .and_then(|r| r.result.ok())
        {
            Some(s) => {
                let sites = call_sites(&s);
                let name = s.class_name.clone();
                vm.load(s, None);
                scripts.push((name, sites));
            }
            None => not_read += 1,
        }
    }
    let bound_keys: Vec<String> = natives::bound()
        .chain(natives::VM_HANDLED)
        .map(|(c, m)| format!("{c}.{m}"))
        .collect();
    let (mut all, mut gameplay, mut tests) = (Tally::default(), Tally::default(), Tally::default());
    let (mut calls, mut script_calls, mut gameplay_scripts) = (0usize, 0usize, 0usize);
    let mut test_scripts = 0usize;
    let mut unparsed = 0usize;
    let mut unknown_codes = BTreeMap::<String, usize>::new();
    for (class_name, sites) in &scripts {
        let mut native_sites = Vec::new();
        for (owner, name) in sites {
            calls += 1;
            let has_code = vm
                .class_by_name(owner)
                .is_some_and(|c| vm.has_code(c, name));
            if has_code {
                script_calls += 1;
                continue;
            }
            match signature::parse(name) {
                None => unparsed += 1,
                Some(sig) => {
                    for p in sig.params {
                        if let signature::Param::Unknown(c) = p {
                            *unknown_codes.entry(c.to_string()).or_default() += 1;
                        }
                    }
                }
            }
            let key = format!("{owner}.{name}");
            let bound = bound_keys.contains(&key);
            native_sites.push((key, bound, owner));
        }
        let lower = class_name.to_lowercase();
        let is_test = native_sites.iter().any(|(_, _, o)| o.as_str() == "Test");
        test_scripts += usize::from(is_test);
        let is_gameplay = !is_test
            && !UI_WORDS.iter().any(|w| lower.contains(w))
            && !native_sites
                .iter()
                .any(|(_, _, o)| o.starts_with("Gooey") || o.to_uppercase().contains("HUD"));
        gameplay_scripts += usize::from(is_gameplay);
        for (key, bound, _) in &native_sites {
            all.add(key, *bound);
            if is_gameplay {
                gameplay.add(key, *bound);
            }
            if is_test {
                tests.add(key, *bound);
            }
        }
    }
    let doc = serde_json::json!({
        "schema_version": 1,
        "scope": "FSHb files in the given directory",
        "unit": "call instruction in a distinct script file",
        "files": paths.len(),
        "read": scripts.len(),
        "not_read": not_read,
        "bindings_in_table": bound_keys.len(),
        "calls": calls,
        "script_calls": script_calls,
        "unparsed_native_names": unparsed,
        "unknown_signature_codes": unknown_codes,
        "all": all.json(&bound_keys),
        "gameplay_scripts": gameplay_scripts,
        "gameplay": gameplay.json(&bound_keys),
        "test_scripts": test_scripts,
        "test": tests.json(&bound_keys),
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "files {} read {}; calls {} (script {}); native calls {} bound {}; \
         gameplay scripts {} native calls {} bound {}; test scripts {} native calls {} bound {}; \
         unparsed names {}; unknown codes {:?}",
        paths.len(),
        scripts.len(),
        calls,
        script_calls,
        all.native_calls,
        all.bound_calls,
        gameplay_scripts,
        gameplay.native_calls,
        gameplay.bound_calls,
        test_scripts,
        tests.native_calls,
        tests.bound_calls,
        unparsed,
        unknown_codes
    );
    Ok(())
}
