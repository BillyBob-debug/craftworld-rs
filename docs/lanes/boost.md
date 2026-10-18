# Lane state: PC boost items (2026-10-18)

## Read first
1. docs/project-boost.md, items 2, 3, 6 and 7
2. scripts/prune-targets.ps1 and scripts/use-local-rust.ps1
3. local-analysis/boost/time_link.py (link-step A/B, ignored folder)

## State
- PR 147 (parity manifest and runner, D800) merged as 982539f.
- This PR (branch `boost-build`, worktree E:\Projects\wt-boost): prune script, the sccache switch
  and the worktree-aware toolchain lookup in use-local-rust.ps1. D801.
- Decision numbers D800-D899; next free D802.
- The lane pauses at 06:00 UTC 2026-10-18 (owner: one thread at a time after that).

## Facts
- Owner said yes to deleting 19 trees: 16 E:\Projects\lbp2-*target* folders plus the canonical checkout's
  target-pr83, target-lineprobe and target-planets. All were deleted, freeing 280 GB net (E: had 341 GB free afterwards).
  Kept: target-d312, target-d319 (the shot scripts' viewer), target-d320, target-globals and lbp2-wt-target-d336probe.
- sccache: the owner said yes to installing it. `cargo install sccache` fails with and without
  `--no-default-features`, because zstd-sys and aws-lc-sys need a C compiler and the PC has none (the bundled
  mingw gcc lacks cc1). Asked for the prebuilt sccache-v0.18.0-x86_64-pc-windows-msvc.zip
  (9.7 MB, mozilla/sccache releases), with no answer yet. Once sccache.exe is in local-analysis/toolchain/cargo/bin,
  use-local-rust.ps1 sets RUSTC_WRAPPER (cache local-analysis/sccache, 40G).
- Linking: the viewer dev bin links in 10.7-13.9 s on a quiet machine (765 MB exe) and up to 74 s while other
  lanes build, so linking is not the bulk of the 213 s test build. `-C link-arg=-fuse-ld=lld` is
  ignored by the gnu toolchain's gcc 14.2 (`-Wl,--version` still reports GNU ld 2.44), and stable has no
  supported way to select rust-lld for the gnu target. No linker change made. Bevy dynamic_linking was not tried.
- Model routing and output truncation guidance is in the PC thread CLAUDE.md. Settings snippet for the
  owner to paste (not sent yet): `"env": {"BASH_MAX_OUTPUT_LENGTH": "8000", "CLAUDE_CODE_SUBAGENT_MODEL": "sonnet"}`.
  BASH_MAX_OUTPUT_LENGTH is documented for Bash only.

## Next
1. If the owner says yes: unzip sccache.exe into the toolchain bin and measure a cold dependency build in a second worktree.
2. Try bevy/dynamic_linking in a dev-only feature, which may hit mingw's 65535-export limit.
3. Send the owner the settings snippet.
