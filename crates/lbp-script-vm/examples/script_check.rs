//! Runs `check()` over every `FSHb` file in a directory and writes the totals to a NEW
//! JSON file under ignored `local-analysis/` (docs/script-vm.md, "PC checks").
//! Inputs are the user's own scripts extracted locally; nothing here is committed.
use lbp_level::{script, Limits};
use lbp_script_vm::check::check;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

fn main() {
    if let Err(e) = run() {
        eprintln!("script_check: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: script_check SCRIPT-DIR NEW-JSON".into());
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
    let (mut read_ok, mut read_failed) = (0usize, 0usize);
    let (mut branches, mut from_current, mut from_next) = (0usize, 0usize, 0usize);
    let (mut unknown, mut no_terminal) = (0usize, 0usize);
    let mut scripts_all_current = 0usize;
    let mut scripts_all_next = 0usize;
    let mut opcodes = BTreeMap::<String, usize>::new();
    let mut rows = Vec::new();
    for p in &paths {
        let size = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
        if size > MAX_FILE_BYTES {
            return Err("file size budget".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let parsed = script::read(&bytes, Limits::default(), &|| false)
            .ok()
            .and_then(|r| r.result.ok());
        let Some(s) = parsed else {
            read_failed += 1;
            rows.push(serde_json::json!({"sha1": name, "status": "not_read"}));
            continue;
        };
        read_ok += 1;
        let c = check(&s);
        branches += c.branch.branches;
        from_current += c.branch.in_range_from_current;
        from_next += c.branch.in_range_from_next;
        unknown += c.unknown_opcodes;
        no_terminal += c.functions_without_terminal;
        scripts_all_current += usize::from(c.branch.in_range_from_current == c.branch.branches);
        scripts_all_next += usize::from(c.branch.in_range_from_next == c.branch.branches);
        for (op, n) in &c.opcodes {
            *opcodes.entry(format!("0x{op:02x}")).or_default() += n;
        }
        rows.push(serde_json::json!({
            "sha1": name,
            "status": "checked",
            "branches": c.branch.branches,
            "in_range_from_current": c.branch.in_range_from_current,
            "in_range_from_next": c.branch.in_range_from_next,
            "unknown_opcodes": c.unknown_opcodes,
            "functions_without_terminal": c.functions_without_terminal,
        }));
    }
    let doc = serde_json::json!({
        "schema_version": 1,
        "scope": "FSHb files in the given directory",
        "unit": "distinct content hash (one file per hash)",
        "files": paths.len(),
        "read": read_ok,
        "not_read": read_failed,
        "branches": branches,
        "in_range_from_current": from_current,
        "in_range_from_next": from_next,
        "scripts_all_in_range_from_current": scripts_all_current,
        "scripts_all_in_range_from_next": scripts_all_next,
        "unknown_opcodes": unknown,
        "functions_without_terminal": no_terminal,
        "opcode_counts": opcodes,
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
        "files {} read {} not_read {}; branches {} in range from current {} / from next {}; \
         scripts all-in-range current {} next {}; unknown opcodes {}; functions without terminal {}",
        paths.len(),
        read_ok,
        read_failed,
        branches,
        from_current,
        from_next,
        scripts_all_current,
        scripts_all_next,
        unknown,
        no_terminal
    );
    Ok(())
}
