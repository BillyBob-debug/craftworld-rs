# Lane state files

One file per active PC lane, `docs/lanes/<lane>.md`, rewritten (not appended) after every
PR in the same PR. It replaces the `docs/handoff-pc-<date>-<lane>-partN.md` series; the
part history is the file's git log. Rationale and measurements: docs/handoff-research.md.

Rules:

- Rewrite the whole file after each PR, before the PR is handed to the coordinator. Keep
  it under ~40 lines; no copied logs or JSON.
- A fresh thread on the lane reads this file first (plus the open PR's branch), nothing
  else until the "Read first" list says so.
- When a thread's context is compacted in place, the compaction summary follows the same
  four headings (see "Compact instructions" in the thread-folder CLAUDE.md) and the thread
  re-reads this file before continuing.
- A finished lane deletes its file in the PR that finishes it.

Template:

```markdown
# Lane state: <lane> (<date>)

## Read first
1. <file or section, at most 5 lines>

## State
- PR <n> (branch `<b>`, worktree `E:\Projects\<wt>`): <one line>; merge status.
- Decision numbers: <range>, next free D<nnn>.

## Facts
- <exact addresses, flags, commands, measured values; verbatim, never paraphrased>

## Next
1. <ordered list>
```
