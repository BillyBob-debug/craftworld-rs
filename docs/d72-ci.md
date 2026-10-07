# D72: GitHub Actions CI

Workflow: `.github/workflows/ci.yml`, runs on `ubuntu-latest` for pushes to `main` and all pull requests.

## What runs

Job `rust` (stable toolchain with rustfmt and clippy; Cargo registry and `target/` cached, key `cargo-<os>-<hash of Cargo.lock>`):

1. `cargo fmt --all --check`
2. `cargo test -q -p lbp-core -p lbp-io -p lbp-archive -p lbp-resource -p lbp-texture -p lbp-mesh -p lbp-catalog -p lbp-preview -p lbp-cli`
3. `cargo clippy -q <same crates> --all-targets -- -D warnings`

Job `python`: `python3 -m unittest discover -s tests`. The tests import only the standard library and the repo's own `scripts/` modules; no pip install is needed.

## Excluded: GUI crates

`lbp-bevy-viewer` and `lbp-viewer` are excluded entirely (not even build-only). They pull in Bevy/eframe (wgpu, x11, wayland), which need system packages (alsa, udev, wayland/x11 dev libraries) and long compiles, so the cost outweighs the value for a first CI. `cargo fmt --all` still covers their source formatting. Revisit later if a build-only job is wanted.

## Game data

No step reads `game-data/`, `references/` or existing `local-analysis/` contents. Two Python tests only `mkdir local-analysis` and write temporary files there (the directory is absent in a fresh clone); one test skips itself when the optional `references/` oracle checkout is missing.

## Local verification (Linux container, cargo 1.97.0, Python 3.13)

| Command | Result |
| --- | --- |
| YAML parse of ci.yml (PyYAML) | ok |
| `cargo fmt --all --check` | pass, no output |
| `cargo test -q` (9 non-GUI crates) | all `test result: ok`, 0 failures |
| `cargo clippy -q` (9 crates, `--all-targets -- -D warnings`) | pass, no output |
| `python3 -m unittest discover -s tests` | 34 tests OK (1 skipped: optional oracle) |

No Rust source changes were needed.

## Not tested

The workflow has not run on GitHub yet (first run happens on the PR). Cache behavior, action version resolution, and `ubuntu-latest` image differences are unverified. Not verified against disc data.
