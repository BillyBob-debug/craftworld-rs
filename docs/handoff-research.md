# Handoff method research (D900, 2026-10-18)

Question: is the current PC-thread handoff method (write `docs/handoff-pc-*.md` at
~150k context, stop at 170k, coordinator starts a fresh thread from it) the best way to
carry lane work forward, or is there a better one?

Answer: the method is sound on quality but not ideal. The handoff file itself is cheap;
what costs is everything around it: a fresh thread re-orients for 10-20 calls, the user
allows a card for every rollover (about 30 a day on 2026-10-17), and the manual stop is
missed 40% of the time. The recommended change keeps the handoff file as a per-lane
state file and lets Claude Code's own auto-compaction do the rollover in place.

## Measured (40 most recent PC sessions, transcripts via scripts/ctx_audit.py)

| Metric | Median |
|---|---|
| Context at call 1 (baseline) | 67.5k (47-70k; was 99k in the 10-15 audit) |
| Peak context | 159k (17 of 40 sessions above 170k; max 258k) |
| API calls per session | 77 |
| Wall clock per session | 37 min |
| PRs opened per session | 2 (7 sessions opened none) |
| Context growth per call | 1.19k |
| Orientation: growth by call 10 / call 20 | +16k / +32k (1.6k per call, above the average) |
| Handoff write: content / tail after it | ~0.5k / ~3.4k over 5 calls, under a minute |
| Cache-read tokens per session | 8.8M (baseline is 52% of all cache reads) |

Rollover cadence from `git log` on 2026-10-17: 40 merged PRs against 30 handoff files;
the Pod lane rolled every ~80 min (parts 36-44), the Popit lane every ~55 min (parts 1-11).

Biggest context growth classes: Bash file dumps (1.10M over 819 calls), Read (443k),
python scripts (285k), Grep (224k), cargo (145k). File dumps are 2.5x the next class.

What this says about the current method:

- The write is not the cost. 4k tokens and five calls per rollover is noise.
- Orientation is the cost. 16-32k tokens and 10-20 of 77 calls (13-26% of the thread)
  go to re-reading what the previous thread already knew, every 37 minutes.
- The stop is unreliable. Threads cannot see their context; even with the audit script
  the rule is a manual check, and 17 of 40 sessions ran past 170k (five past 220k).
- User friction is the largest unmeasured cost: each rollover is a card the user must
  allow, plus a coordinator relay of the standing rules. On a 30-rollover day the user
  is the bottleneck, and a lane sits idle until the card is clicked.
- Quality is good. The "Read first / State / Facts / Next" template carries exact
  addresses, flags and open items, and it is committed, so it is auditable. The
  downside is 52 `handoff-pc-*` files in docs/ after one week.

## Alternatives (current Claude Code docs, fetched 2026-10-17)

| Option | Quality of carried state | Tokens | User friction | Reliability |
|---|---|---|---|---|
| A. Current: fresh thread per handoff file | High, explicit, committed | Baseline cold write + 16-32k orientation per rollover | One allow card + rule relay per rollover | Manual stop, missed 40% |
| B. Auto-compaction in place (`autoCompactWindow`) | Medium alone: summary is lossy; CLAUDE.md, MEMORY.md and 5 recent files survive, hook/relay context does not | Summary from the warm cache; no re-orientation reads | None | Deterministic trigger |
| B+. Compaction plus a per-lane state file re-injected after compaction | High: the file is at most one PR old and exact | As B plus one small file read | None | Deterministic |
| C. Per-lane state file updated each PR (replaces part-N files) | High | Lowers orientation for any rollover style | Same as the style it is paired with | Same |
| D. Lane state in auto memory (MEMORY.md) | Low: 200-line/25KB cap shared by every lane, bloats every baseline | Adds to every thread's floor | None | Fragile |
| E. Subagent-heavy main thread | Same as paired style; fewer rollovers | Cuts per-call growth (file dumps are the top class) | Same | Same |
| F. `--resume` the old transcript | Full history | Cold re-read of the whole transcript, the costliest path | Card | Loses nothing, pays for everything |
| G. No cap, run to the 1M window | Full history | Each call costs its whole context; cache hits are 0.025x base on Fable, so cheaper than raw token counts suggest, but plan usage counts tokens | None | Degrades late; the docs still compact near 967k |

Facts the ranking rests on (docs: context-window, model-config, hooks, memory,
prompt-caching, sub-agents under https://code.claude.com/docs/en/):

- Auto-compaction triggers at a configurable window (`autoCompactWindow` setting,
  `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, 100k-1M), not at a percentage. On a 1M model the
  default is ~967k, which is why today's threads never compact.
- It can be steered: a "Compact instructions" section in CLAUDE.md, `/compact <focus>`,
  `PreCompact` (can block) and `PostCompact` hooks, and a `SessionStart` hook with the
  `compact` matcher that re-injects content after the summary.
- The summary reads the warm prefix, so compaction costs far less than a cold restart of
  the same context; `/clear` is free; resuming an old transcript after the 1h cache TTL
  is the costliest path.
- Subagents run in their own context and return only their report; non-fork subagents
  start fresh.

## Ranked recommendation

1. **Compaction in place with a per-lane state file (B+ with C).** In the thread folder
   (`Documents\Projects\LBP2 Rust Rewrite` under the user profile, where the PC threads run):
   - `.claude/settings.json`: `autoCompactWindow` at ~170k (the current stop line).
   - Thread-folder `CLAUDE.md`: a "Compact instructions" section that is the handoff
     template (Read first, State, Facts, Next; keep addresses, flags, PR numbers, open
     items verbatim), and a rule that the lane state file is rewritten after every PR.
   - A `SessionStart` hook with matcher `compact` that prints the lane's state file
     back into context (the thread knows its lane from its brief; one file per lane,
     `docs/lanes/<lane>.md`).
   - Pilot on one lane for a day. Measure with `ctx_audit.py`: peak context, PRs per
     hour, and whether the first calls after compaction re-read anything the state file
     already held. Compare the compaction summary against the state file for lost facts.
   Expected effect: no allow cards, no rule relays, 10-20 orientation calls saved per
   rollover (about 2.5-5 lane-hours per 30-rollover day), deterministic stop. Risks: the
   setting and hooks are unverified in Remote Control sessions (the pilot checks this);
   a hook-free fallback is the CLAUDE.md section alone, which still improves the summary.
2. **Per-lane state file regardless of 1 (C).** Replace `handoff-pc-<date>-<lane>-partN.md`
   with one `docs/lanes/<lane>.md` rewritten at each PR; git history is the part series.
   Fresh starts read one file; docs/ stops accumulating; the file is never more than one
   PR stale, which also protects against a bad compaction or a crashed thread.
3. **Keep cutting per-call growth (E).** The thread-folder rules already push exploration
   to subagents; the measured top class is still in-thread file dumps. Boost item 7 (a
   PostToolUse hook truncating results over 2,000 chars) is the concrete lever and halves
   the rollover rate on its own.
4. **If 1 fails the pilot, keep A** and only add 2 and 3. The method is not broken; it is
   paying orientation and user attention it does not need to.

Not recommended: lane state in auto memory (D), `--resume` (F), uncapped 1M runs (G).

## Cost note

In raw tokens the baseline dominates (52% of cache reads), which is what plan usage
appears to count. In API dollars Fable's cache hits are 0.025x base, so a 150k-context
call reads for the price of ~3.75k fresh tokens and the new tokens, outputs and cache
writes are a comparable share. Both views favour the same change: fewer cold restarts
and fewer orientation reads, not a smaller per-call window.

## Implementation (owner-approved 2026-10-18)

Done in this PR: `docs/lanes/` with README (rules, template) and seeded state files for the
pod/Story, popit, sackboy, level1 and launcher lanes, each copied from the lane's latest
handoff part. The level-globals lane is finished and gets no file.

Left for the user, because sessions cannot edit CLAUDE.md or `.claude/settings.json`
(auto-mode classifier). Paste these by hand.

1. Thread folder (`Documents\Projects\LBP2 Rust Rewrite` under the user profile),
   `.claude\settings.json`, merged into the existing object. The window is a plain token
   count (settings-reference: 100000-1000000 or "auto"); the hook's stdout is added to the
   context after each compaction. Note the setting applies to every PC thread in that
   folder at once; the "pilot" is therefore watching the Popit lane closely for a day (it
   rolls over most often) and reverting the key if facts get lost.

   ```json
   {
     "autoCompactWindow": 170000,
     "hooks": {
       "SessionStart": [
         {
           "matcher": "compact",
           "hooks": [
             {
               "type": "command",
               "command": "echo Context was compacted. Before anything else, re-read your lane state file docs/lanes/LANE.md in your worktree (the summary names the lane and worktree), then continue its Next list."
             }
           ]
         }
       ]
     }
   }
   ```

2. Thread-folder `CLAUDE.md`: replace the line "Plan the handoff at ~150k, stop near ~170k."
   with the two blocks below.

   ```markdown
   - Context rollover is auto-compaction in place (autoCompactWindow 170k). Do not stop
     at a context size and do not write docs/handoff-pc-*.md; rewrite docs/lanes/<lane>.md
     after every PR in that PR. After a compaction, re-read it before continuing.

   # Compact instructions
   Keep, under these headings and verbatim where they are facts: Lane and worktree path;
   Read first (at most 5 files or sections); State (open PR number, branch, merge status,
   decision-number range and next free number, anything running in the background); Facts
   (every address, flag, command line, measured value and owner rule stated so far, copied
   exactly, never paraphrased or rounded); Next (ordered). Drop tool output, file contents
   already in the repo, and reasoning that led to a committed result.
   ```

3. Repo `CLAUDE.md`, standing rule "Context rollover: plan at ~150k, stop near ~170k
   tokens": replace with "Lane state instead of handoff parts: each PC lane keeps one
   docs/lanes/<lane>.md (template in docs/lanes/README.md), rewritten after every PR in
   that PR. Rollover is auto-compaction in place at ~170k in the thread folder; after a
   compaction re-read the lane file. A fresh thread starts from that file plus the open
   PR's branch. Do not write new docs/handoff-pc-*.md files." The "Handoff writing
   budget" heading's first sentence becomes "Lane state files are rewritten every PR and
   reread after every compaction, so keep them under ~40 lines."

4. Coordinator briefs: name the lane and `docs/lanes/<lane>.md` as the only "Read first";
   drop "plan the handoff at 150k" and the per-PR `ctx_audit.py` check (keep it as an
   optional measurement); stop relaying standing rules on rollover, since there is no
   rollover; on a crashed or user-restarted thread, brief from the lane file on main plus
   the open PR's branch.

## Sources

- Transcripts: `~/.claude/projects/<thread folder>/*.jsonl`, 40 most recent, via
  `scripts/ctx_audit.py` and two scratch scripts (medians, not committed).
- docs/token-audit-2026-10-15.md (99k floor, 45% of calls above 175k then).
- docs/project-boost.md items 6, 7 and 11 (model mix, truncation hook, cache TTL).
- Claude Code docs: context-window, model-config, hooks, hooks-guide, sessions,
  checkpointing, memory, sub-agents, prompt-caching, costs.
