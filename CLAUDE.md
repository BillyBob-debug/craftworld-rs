# CLAUDE.md

Project rules live in AGENTS.md and are authoritative; they are imported here so
Claude Code loads them automatically. Do not duplicate or override them in this file.

@AGENTS.md

## Start of every session

1. Read the latest entry at the end of `STATUS.md` (it is large; read the tail, not
   the whole file), then `RESEARCH.md` and the docs for the area you are touching
   (`docs/architecture.md`, `docs/formats.md`, the milestone doc under `docs/`).
2. Continue from the "Next:" line of the latest STATUS entry unless the user asks
   for something else.

## Build and test (Windows, PowerShell)

The toolchain is project-local (Rust stable, x86_64-pc-windows-gnu, no MSVC).
Activate it once per shell:

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check
cargo test -p <crate>            # or --workspace
cargo clippy -p <crate> --all-targets -- -D warnings
cargo build -p lbp-bevy-viewer -p lbp-viewer
python -m unittest discover -s tests -v
```

Viewer: `target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid <guid>`
(`--qa-capture local-analysis/<new>.png` for a capture). Oracle checks use
`scripts/run-*-oracle.ps1` and the `lbp-catalog` / `lbp-preview` examples; exact
commands for each milestone are recorded in STATUS.md.

## Reminders

- The project root is not a Git repository. No init/commit/push unless asked.
- Output filenames under `local-analysis/` must be new; tools refuse to overwrite.
- Write docs as UTF-8 explicitly (a cp1252 default broke a write before).
- Native test windows go on the non-primary monitor; verify placement first.
- After meaningful work, append to STATUS.md and MODLOG.md.
