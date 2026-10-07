# LBP2 multi-agent workflow

Written 2026-10-07 (America/Chicago). Builds on `docs/cloud-sessions.md` (credit scope,
cloud/PC split, branch rules); AGENTS.md stays authoritative. Lives in the repo as
`docs/workflow.md`; cloud sessions read it from there.

## 0. Does running N agents cost more than N times?

No, roughly N times, if the tasks are independent. Each agent pays its own cold start
(about 5k tokens with STATUS-CURRENT.md) plus its own work. It goes above N times only
when agents touch the same files (conflicts, rebases, re-reads) or when one agent has
to read the others' output. Parallelism buys wall-clock time, not savings.

What matters more here is *who pays*:

| Runs as | Paid by |
|---|---|
| Cloud session started at claude.ai/code or `claude --cloud` on the GitHub repo | $250 credit (expires 2026-11-05) |
| claude.ai Project threads, Remote Control, local `claude -p`, Routines | Plan limits |

So: push as much token-heavy work as possible into cloud sessions, and keep the PC
session (the one thing that must run on plan limits) short and mechanical.

## 1. Roles

| Role | Where | Count | Owns |
|---|---|---|---|
| **Integrator** | PC, scheduled `claude -p` (§7) | 1 | Oracle/parity runs, viewer QA, milestone counts, merges, `main`, STATUS.md, MODLOG.md, STATUS-CURRENT.md, `docs/backlog.md`, `docs/batch-report.md` |
| **Planner** | Cloud batch driver | 1 per batch | Turning `docs/backlog.md` + Toolkit research into GitHub issues; reserving D-numbers |
| **Implementer** | Subagents of the cloud batch driver | 2-3 per batch | One issue: branch, code, synthetic tests, PR |
| **Reviewer** (optional) | Cloud session | 0-1 | `/code-review` on a PR before it reaches the PC |

Only the Integrator ever sees game data. Every cloud result is "unverified on real
data" until the Integrator's run.

## 2. Task flow

```
backlog.md --Planner--> Issue "D70: ..." (label: ready)
   --Implementer--> branch cloud/D70-<slug> -> PR "Closes #n" (Linux fmt/test/clippy green), label pc-verify
   --Integrator--> PC fmt/test/clippy + PC verify command from the PR body
         pass -> merge into main, counts recorded in the batch handoff
         fail -> docs/batch-report.md -> next day's batch driver fixes the branch
```

- **Queue = GitHub Issues.** Labels: `ready`, `in-progress`, `pc-verify`, `blocked`,
  plus `cloud-ok` or `needs-pc`.
- **D-numbers**: only the Planner assigns them, sequentially from max(latest STATUS
  milestone, highest open issue) + 1. One batch driver at a time, so no collisions.
- **Batching**: the Integrator verifies all unmerged `cloud/*` branches in one run,
  merges in D-number order, then writes one STATUS/MODLOG entry set for the batch.

## 3. Parallelism

- 2 implementers per batch; 3 once a batch merges without conflicts. More just piles
  PRs up behind the single PC verifier.
- Never run two tasks that edit the same parser module at once. Each issue has a
  `Touches:` line; only non-overlapping issues go into one batch.
- Shared docs: implementers write full detail only in `docs/<milestone>.md`. Edits to
  `docs/formats.md` / `architecture.md` are limited to the task's own section.
  STATUS/MODLOG/STATUS-CURRENT/backlog/batch-report are Integrator-only.

## 4. Credit pacing

$250 over ~29 days is about $8.60/day. Check usage after the first batch and adjust
batch size. Levers: Sonnet for tests/refactors/CI, Opus for format research and new
parsers; lean prompts so rules are read from the repo, not pasted.

## 5. Task card (issue body)

```markdown
**D70** · <one-line goal>
Labels: ready, cloud-ok | needs-pc
Touches: lbp-resource/src/material/*.rs, docs/d70-<slug>.md
Evidence: <Toolkit path @ d6a2ef80, method/section; evidence label from docs/formats.md>
Done when (Linux): <tests that must exist and pass>
PC verify: <exact command; expected count change>
Out of scope: <what not to touch>
Backlog ref: docs/backlog.md#<anchor>
```

## 6. Role steps

**Implementer:**
1. Read the issue, STATUS-CURRENT.md, and only the doc sections the card names.
2. Label the issue `in-progress`. Branch `cloud/D<NN>-<slug>` from `main`.
3. Implement; synthetic fixtures only; checked arithmetic on untrusted lengths.
4. Linux checks, output capped: `cargo fmt --all --check`, `cargo test -q -p <crate> 2>&1 | tail -n 30`,
   `cargo clippy -q -p <crate> --all-targets -- -D warnings 2>&1 | tail -n 40`.
5. Open PR "D<NN>: <goal>", `Closes #<n>`, label `pc-verify`. Body: evidence label +
   Toolkit path @ pinned commit, Linux tests run, "Not verified against disc data",
   the PC verify command, STATUS draft (≤15 lines), MODLOG draft (≤3 lines).
6. Failures found on the PC come back through `docs/batch-report.md` (§7).

**Integrator:** verify unmerged `cloud/*` branches in D-number order: fetch, checkout,
fmt/test/clippy (CLAUDE.md, capped output), run each PR's PC verify command. Pass:
merge into main. Fail: ≤10 lines of output in docs/batch-report.md, skip. Then one
batch STATUS entry (≤40 lines), MODLOG (≤6), overwrite STATUS-CURRENT.md, refresh
docs/backlog.md (ranked, counts only, no asset bytes), commit, push main.

## 7. Hands-off daily loop (what actually runs)

The user does nothing after setup. Task Scheduler task `LBP2-DailyBatch` runs
`scripts/daily-batch.ps1` once a day:

1. **Integrator, local** (`claude -p`, plan limits): fetch, verify every remote
   `cloud/*` branch not yet merged into `main` (`git branch -r --no-merged origin/main`)
   per §6, overwrite `docs/batch-report.md` (merged, failed with ≤10 lines each, new
   counts), commit, push `main`. GitHub marks a PR merged when its commits reach `main`.
2. **Batch driver, cloud** (`claude --cloud`, paid by the credit), started after
   step 1 so it sees fresh `main`. One fresh session per day:
   - Read STATUS-CURRENT.md, `docs/batch-report.md`, `docs/backlog.md`.
   - Fix each failed branch from the report first (push to that branch).
   - Create up to 3 non-overlapping issues (§5, D-numbers §2).
   - Implement each with a subagent on its own `cloud/D<NN>-<slug>` branch; open
     PRs per §6. Stop when done.

Logs: `local-analysis/daily-batch/<date>.log` (ignored). Disable with
`Disable-ScheduledTask -TaskName LBP2-DailyBatch`.

## 8. First batch

1. GitHub Actions CI on ubuntu-latest (fmt, non-GUI test/clippy, Python unittest).
2. Synthetic tests for each revision gate added for 01.33 (952-1016).
3. Bounds/fuzz tests: FARC, MAP, SFO, resource headers.
4. The Integrator's first run produces `docs/backlog.md` from the 01.33 census
   (386 unsupported materials, two-texture x COLOR first).
