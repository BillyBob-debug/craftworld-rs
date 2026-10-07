# Cloud sessions setup (LBP2 Asset Explorer)

Written 2026-10-07 (America/Chicago). Purpose: let several Claude Code cloud sessions
work on this repo in parallel while game-data work stays on the Windows PC.
AGENTS.md stays authoritative; nothing here relaxes it.

## 1. Credit and what it pays for

- $250 cloud-session credit (Max promo), claimed 2026-10-07. Expires 2026-11-05
  01:59 CST (2026-11-04 23:59 PT). Unused credit is lost; afterwards normal plan
  limits apply. Forfeited if the subscription is cancelled or downgraded to free.
- Pays for: Claude Code sessions running on Anthropic's servers, started from
  claude.ai/code (web, desktop, CLI `--remote`/teleport, or mobile) against a
  connected GitHub repo. While credit remains, that usage does not count against
  plan limits.
- Does NOT pay for: claude.ai Projects (including the "LBP2 Rust Rewrite" project
  threads), Routines, Remote Control sessions on the PC, Chat, Cowork.
- Inferred, not confirmed by the terms: subagents and workflows started inside a
  cloud session bill as that session's cloud usage.
- Sources: https://support.claude.com/en/articles/17152539-cloud-sessions-bonus-credit-promotion
  and https://www.anthropic.com/legal/promotion-credit-terms

## 2. Where each kind of work runs

| Work | Cloud session (Linux, no game data) | Windows PC (game data) |
|---|---|---|
| Parser code from public Toolkit/cwlib source | yes | |
| Synthetic-byte unit tests, fuzz/bounds tests | yes | |
| Refactors, docs, clippy/fmt cleanups | yes | |
| Linux build/test of all crates (no viewer launch) | yes (verified, see section 4) | |
| Oracle parity runs against DATA.FARC (`scripts/run-*-oracle.ps1`, census/parity examples) | no | yes |
| Viewer QA, captures, second-monitor checks | no | yes |
| Milestone counts (newly viewable / newly Complete) | no | yes |
| 01.33 update reading, revisions 952-1016 checked against real files | write the reader in cloud | verify on PC |

Cloud sessions never see `game-data/`, `local-analysis/`, `references/`, the ZIP, or
the local toolchain. A cloud result is "unverified on real data" until the PC run.

## 3. Repo prerequisites (one-time, done by the user)

1. Private GitHub repo, empty (no README/license/.gitignore from GitHub).
2. On the PC: `git remote add origin <url>` then `git push -u origin main`.
   The commit author email is the personal Gmail; if GitHub email privacy is on,
   switch to the noreply address first (`git config user.email <id>+chilladept@users.noreply.github.com`).
3. Audit done 2026-10-07: 239 tracked files, all authored source/docs/scripts/
   schemas/Cargo files. Default-deny `.gitignore` excludes game data, analysis
   output, reference checkouts, archives, asset extensions and `target/`. No test
   embeds disc bytes (`include_bytes!` unused; no long byte/hex blobs; every
   DATA.FARC in tests is a tiny synthetic archive in a temp dir).
4. Before every push: `git status --short` and `git ls-files` must show nothing from
   ignored roots; never `git add -f`.
5. Connect GitHub at claude.ai/code and pick the repo.

## 4. Linux status (checked 2026-10-07 in a cloud container)

- `cargo fmt --all --check`: clean.
- `cargo test -q` on lbp-core, lbp-io, lbp-archive, lbp-resource, lbp-texture,
  lbp-mesh, lbp-catalog, lbp-preview, lbp-cli: all pass.
- `python3 -m unittest discover -s tests`: passes after the 2026-10-07 fix that
  creates ignored `local-analysis/` in `setUpModule` (it is absent in a fresh
  clone). 1 test skips without `references/lbptoolspy-lite` and Pillow; expected.
- `lbp-io/src/source.rs` has `#[cfg(windows)]` branches; Linux runs the other path.
- GUI crates: `cargo test -q -p lbp-viewer -p lbp-bevy-viewer` compiles and passes
  (65 + 11 tests) with no extra apt packages (container had only pkg-config).
  Bevy here has no audio/gamepad features, so no ALSA/udev dev libs are needed.
  No display in cloud: never launch the viewers there.
- `cargo clippy -q --all-targets -- -D warnings` on the non-GUI crates: clean.

## 5. Cloud environment

Create a cloud environment at claude.ai/code for this repo
(docs: https://code.claude.com/docs/en/cloud-environments).

- Network: Limited/default is enough for crates.io. Add `github.com` and
  `codeload.github.com` if sessions should clone Toolkit for format research.
- Setup script: none required (the default image already has cargo, rustfmt,
  clippy, Python 3 and Java). Optional: `rustup component add rustfmt clippy || true`.
- Toolkit for format research: inside the session, from the repo root
  (lands in ignored `references/`, never committed):

```bash
git clone --filter=blob:none https://github.com/ennuo/toolkit references/toolkit
git -C references/toolkit checkout d6a2ef80b5a09999f08f23a803b516096055e5c9
```

Linux commands (cap output, per CLAUDE.md):

```bash
cargo fmt --all --check
cargo test -q -p <crate> 2>&1 | tail -n 30
cargo clippy -q -p <crate> --all-targets -- -D warnings 2>&1 | tail -n 40
python3 -m unittest discover -s tests
```

## 6. Multi-agent rules

- One task = one cloud session = one branch `cloud/<D-number>-<slug>` = one PR.
  Never push to `main`; the user merges after PC verification.
- Reserve D-numbers before fanning out so parallel sessions don't collide
  (e.g. note "D64 reserved: <task>" in the session prompt).
- Shared handoff files conflict under parallel edits. Cloud sessions write full
  detail only to their own `docs/<milestone>.md` and put a short STATUS/MODLOG
  draft in the PR body. The PC integrator appends `STATUS.md`, `MODLOG.md`, and
  overwrites `STATUS-CURRENT.md` once per merged batch.
- Each PR body states: evidence label (docs/formats.md), Toolkit source path +
  pinned commit, what was tested on Linux, and "Not verified against disc data"
  plus the exact PC command that must run before merge.
- Format rules: read Toolkit source as reference, implement independently; keep
  the MIT notice rules in THIRD-PARTY-NOTICES.md. No GPL/AGPL translation.
- Synthetic fixtures only: authored bytes built in the test, never copied from
  any game file, decompiler output or reference repo asset.
- Untrusted lengths: checked arithmetic and budgets (AGENTS.md rule 7).
- Token budget: follow CLAUDE.md (read big docs by section, capped output).

## 7. PC verification loop (not credit-covered)

1. `git fetch origin && git checkout cloud/<branch>`
2. `. ./scripts/use-local-rust.ps1`, then fmt/test/clippy as in CLAUDE.md.
3. Run the oracle/parity command named in the PR body against `game-data/`.
4. Record counts and results in STATUS.md/MODLOG.md, merge, push `main`.

## 8. Good first cloud tasks

- Revisions 952-1016: diff Toolkit's revision-gated fields (pinned commit) for each
  resource type the explorer reads; write the table into docs/formats.md style
  with evidence labels; add reader branches + synthetic tests.
- GitHub Actions CI on ubuntu-latest: fmt, test, clippy for non-GUI crates, Python
  unittest (needs the user's OK for a workflow file; `.gitignore` must allow
  `.github/workflows/*.yml`).
- Bounds/fuzz tests for FARC, MAP, SFO and resource headers.
