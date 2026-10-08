//! U1 PC parity helper: dump one `.trans` file to a new deterministic JSON file.
//! `cargo run -q -p lbp-resource --example translation_dump -- <file.trans> <local-analysis/new.json>`
//! The JSON is the input of `scripts/TranslationOracle.java`. Strings are written in
//! full, so the output stays under ignored `local-analysis/` (game text, not distributable).
use lbp_resource::translation::{parse_translation_table, TranslationLimits};
use serde_json::json;
use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: translation_dump <file.trans> <local-analysis/new.json>");
        return ExitCode::from(2);
    };
    let output = Path::new(output);
    if output.exists() || output.extension().is_none_or(|ext| ext != "json") {
        eprintln!("output must be a new .json file");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("read failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let body = match parse_translation_table(&bytes, TranslationLimits::default()) {
        Ok(table) => {
            let rows: Vec<_> = table
                .entries
                .iter()
                .map(|entry| {
                    json!({
                        "key": format!("{:08x}", entry.key),
                        "string_offset": entry.string_offset,
                        "tag": entry.tag,
                        "text": entry.text,
                    })
                })
                .collect();
            json!({"status": "parsed", "summary": table.summary(), "rows": rows})
        }
        Err(error) => json!({
            "status": "rejected",
            "reason": format!("{:?}: {}", error.kind, error.message),
        }),
    };
    let document = json!({
        "schema_version": 1,
        "scope": "u1_translation_table",
        "input_bytes": bytes.len(),
        "result": body,
    });
    let text = serde_json::to_string_pretty(&document).expect("json");
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
    {
        Ok(()) => {
            println!("translation_dump: {}", document["result"]["status"]);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("write failed: {error}");
            ExitCode::FAILURE
        }
    }
}
