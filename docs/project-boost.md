# Project boost research (2026-10-18)

What would make the rewrite faster to build, faster to run and cheaper to work on.
Builds on the context audit (2026-10-12), the token audit (PR 87, 2026-10-15), the
similar-projects survey (2026-10-15) and the ps3recomp research (2026-10-17); it repeats
none of their findings except where a recommendation is still open. Measured on the PC
checkout at commit 2d23495 (D324). Research only: nothing here was changed.

## Decide first

1. **Script fallback strategy** (the 2026-10-18 rule). Every script-driven mode (the
   scripted Pod, `--pod-menu`, `--ui-script`, `--level-scripts`, lbp-runtime) runs the
   426 FSHb scripts that exist only inside the encrypted `patch.edat`; the disc FARCs hold
   no scripts at all (docs/d133-patch-scripts.md), so there is no unencrypted source to
   fall back to and decrypting at runtime would need Sony keys. The only rule-compliant
   fallback is a **native Rust implementation of each script the shipped modes need**,
   with the VM demoted to a development oracle. Recommended shape, "VM in dev, native in
   the app":
   - Keep the decrypted overlay and the VM for research only (ignored `local-analysis/`).
   - Add a per-script registry keyed by script GUID: at load, if the script resource is
     readable, run the VM; otherwise call the native entry. Both implement one trait.
   - Generate golden traces with the VM in dev (inputs, native calls, globals per tick)
     and test each native implementation against them. The traces are derived data,
     small, and can be committed as fixtures without the scripts themselves.
   - Order the work by what the shipped modes call: `native_coverage` already counts
     which natives each script reaches, so the first slice is the Pod and main-menu
     scripts (D317-D324), not all 426.
   Effort: large but incremental (one script per task); it also removes interpretation
   cost from the runtime. The alternative, keeping the VM as the product and treating
   the fallback as "mode disabled", breaks the rule as written.

2. **Disk and build-tree policy.** Seven `target*` trees inside the checkout total about
   152 GB (`target` 40 GB, `target-pr83` 65 GB, `target-d320` 24 GB, three more at 6-8 GB)
   plus a dozen `lbp2-wt-target*` and `lbp2-perf-target*` trees outside it, against the
   standing rule of at most two live. Deleting them is the owner's call; the fix that
   stops regrowth is below (sccache plus one target per worktree).

3. **Use or lose the cloud credit** ($250, expires 2026-11-05, currently unused). The
   mechanical items in this list (doc splits, line-ending normalization, CI changes,
   parity-suite scaffolding) are exactly what a cloud session can do without touching
   RPCS3 or the PC lanes.

## Ranked recommendations

| # | Recommendation | Impact | Effort | What it changes |
|---|---|---|---|---|
| 1 | Enforce the doc budgets in CI: a Python unit test fails when STATUS-CURRENT.md exceeds 80 lines or contains "## Before:", and when any `docs/*.md` exceeds 1,500 lines. First move the 70 "Before:" sections (1,452 lines) into STATUS.md and split `docs/level-scripts.md` (8,919 lines). | High: every thread reads STATUS-CURRENT (3-16k tokens a read in the token audit); the rule has been open since 10-15 | Low | STATUS-CURRENT.md, STATUS.md, docs/level-scripts/*, tests/test_doc_budget.py, CLAUDE.md |
| 2 | Local `sccache` as `RUSTC_WRAPPER` with a 30-50 GB cache, one `target` per worktree (keeps cargo's lock contention away), and a `scripts/prune-targets.ps1` that deletes targets of merged worktrees. | High on disk, medium on time: dependency crates (Bevy 0.19.1 with ~25 features, avian, eframe, egui) compile once per machine instead of once per worktree | Low | `.cargo/config.toml` (new), scripts/, CLAUDE.md disk rule |
| 3 | Try `rust-lld` on the gnu toolchain (`-C link-arg=-fuse-ld=lld`) and `bevy/dynamic_linking` in a dev-only feature; measure with `cargo build --timings`. `cargo test --workspace --no-run` took 213 s after a warm check, which is link time for 17 test binaries, so linking is the lever. | Medium | Low (measure first; keep only what wins) | `.cargo/config.toml`, Cargo.toml dev feature |
| 4 | Automate the parity measurements. Every milestone since D300 reports "mean abs levels" against game captures by hand; a `tests/parity/manifest.json` of (viewer args, capture path under local-analysis, region, threshold) and one `scripts/run-parity.ps1` make each PR check a single subagent call that returns pass/fail. Captures stay uncommitted and are keyed by RPCS3 build. | High on tokens (no image reads, 1.3-1.5k each) and on regressions | Medium | scripts/, tests/, local-analysis/parity/ |
| 5 | Run the viewer tests in CI. The 173 lbp-bevy-viewer tests and 65 lbp-viewer tests never run on a PR (CRATES excludes both; `viewers` only checks on main). Add them to the `rust` job on ubuntu with the apt libs already listed, and swap `actions/cache` for `Swatinem/rust-cache` with `save-if` on main. | Medium: today a viewer test can break on main unnoticed (one lbp-bevy-viewer test fails in the current checkout) | Low | .github/workflows/ci.yml |
| 6 | Model mix per lane: capture, PR-check, census and Ghidra-lookup lanes on Sonnet or Haiku, design and renderer lanes on the top model. Anthropic's cost guidance: subagents with `model: haiku` for mechanical work, teams at roughly 7x a single session. | High on plan usage | Low | Lane briefs; `.claude/agents/*.md` model lines |
| 7 | A PostToolUse hook that truncates any tool result over 2,000 characters to its head and tail (cargo, git, probe JSON). The token audit's medians (1.2k per file dump, 8-10k per probe dump) make this the cheapest remaining per-call saving. | Medium | Low | `.claude/settings.json` hooks |
| 8 | Add benchmarks before optimizing the VM or renderer: a criterion bench for `lbp-script-vm` over the Pod scripts and a frame-time counter in the Bevy viewer (`--bench-frames N` printing mean and p95). There are no benches today, so no number says the VM is slow. | Medium (prevents guesswork) | Low | crates/lbp-script-vm/benches, viewer flag |
| 9 | VM fast path when a number does say so: fuse hot opcode sequences and keep locals in slot-indexed frames before changing dispatch style (fusevm's finding); stable Rust has no tail-call dispatch. | Low until item 8 reports | Medium | lbp-script-vm |
| 10 | Finish the line-ending normalization (docs/workflow.md §8, still no `.gitattributes`) in a quiet moment between batches; each Linux-side edit otherwise risks a whole-file diff. | Low-medium | Low | .gitattributes, .gitignore |
| 11 | Keep the 1-hour prompt cache warm: an idle lane that wakes after an hour re-reads its full context. Either keep lanes busy back to back or park them, and set `crossSessionInbound: hold` so relays do not wake idle threads (66 no-reply wakes at ~680 tokens each in the audit). | Low-medium | Low | Lane scheduling, settings |
| 12 | Trim the workspace graph: `lbp-preview` and `lbp-level` depend on each other, and `lbp-bevy-viewer` depends on 11 of 16 crates, so one edit in `lbp-resource` rebuilds nearly everything. Break the cycle and move shared types down into `lbp-core`; consider `cargo-hakari` once feature unification shows up in `--timings`. | Medium on incremental builds | Medium | Cargo.toml of three crates |

Not recommended: upgrading Bevy (0.19.1 is the latest release, 2026-06-19), nightly
Cranelift on the gnu toolchain (unverified on Windows), a GitHub merge queue (few PRs
a day, the app's auto-merge monitor already covers it), and changing the RPCS3 oracle
(the ps3recomp research keeps it; a recomp build is a later headless option).

## Measurements behind the ranking

- 17 crates, about 135k lines of Rust, 888 tests, no benches, no `.cargo/config.toml`,
  dev profile opt-level 1 with deps at 3, line-tables-only debug info.
- `cargo check --workspace` warm 0.5 s; `cargo test --workspace --no-run` 213 s.
- CI: 2-3 minutes a run (fmt, test, clippy on 15 crates; both viewers excluded).
- Docs: STATUS.md 5,550 lines, MODLOG.md 1,976, STATUS-CURRENT.md 1,452, about 251
  files under docs/.
- Threads: 99k-token floor, 45% of calls above 175k in the last audit, 20-35k tokens
  of orientation per rollover.

## Sources

- Bevy setup guide on compile time (dynamic linking, lld, Cranelift, dep opt-level):
  https://bevy.org/learn/quick-start/getting-started/setup/
- Bevy 0.19 release notes (2026-06-19): https://bevy.org/news/bevy-0-19/
- rust-lld default status on Windows (still opt-in):
  https://github.com/rust-lang/rust/issues/71520 and
  https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable
- Worktrees sharing one target dir contend on the cargo lock:
  https://users.rust-lang.org/t/announcements-cargo-worktree-fix-build-isolation-in-git-worktrees/139192
- cargo-hakari workspace-hack: https://docs.rs/cargo-hakari
- cargo-nextest: https://nexte.st/
- Interpreter dispatch findings: https://docs.rs/crate/fusevm/0.13.0 and
  https://www.slideshare.net/RReverser/building-fast-interpreters-in-rust
- Emulator oracles have their own bugs (155,642 inconsistent ARM streams in QEMU):
  https://arxiv.org/pdf/2105.14273v2
- Differential testing of a decompiled DLL under an emulator with replayed inputs
  (2026-03-05): https://jmason.ie/2026/03/05/104259a.html
- Agents gaming a byte-exact verifier; keep thresholds outside agent reach:
  https://www.remio.ai/post/modern-warfare-2-ai-decompilation-worked-only-after-the-agents-tried-to-cheat
- Claude Code best practices (CLAUDE.md size, /clear, subagents, headless fan-out):
  https://code.claude.com/docs/en/best-practices
- Claude Code costs (haiku subagents, 7x for teams, hooks that filter output, 1-hour
  cache, `crossSessionInbound`): https://code.claude.com/docs/en/costs
- Effective context engineering (subagents return 1-2k summaries):
  https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Swatinem/rust-cache (`shared-key`, `save-if`): https://github.com/Swatinem/rust-cache
- GitHub merge queues need `on: merge_group`:
  https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue
