# D21 keyboard history navigation

Authored UI behavior, 2026-10-05 America/Chicago. Alt+Left invokes Back; Alt+Right
invokes Forward through D20's checked route transition. Visible help appears below
the detail toolbar. Shortcuts run before the panels bind their selected resource.

Modifiers must be exactly Alt. Plain, Shift, Ctrl, Ctrl+Alt and Shift+Alt arrows
remain available to widgets. Matching pressed events are consumed even when history
is empty; held-key repeats do not navigate. At most one fresh matching press acts
per frame, in input order. Key releases and text events are untouched. Alt history
shortcuts remain available while a text field is focused; ordinary caret and word
movement are not reserved. No extra keyboard/UI dependency.

The existing egui0.36.2 InputState::consume_key implementation includes repeats and
matches modifiers logically, allowing extra modifiers. This project instead filters
the event list explicitly for this narrower shortcut policy; no external code was
copied. Source attachment, parser rules and resource budgets are unchanged.

## Checks

After `. ./scripts/use-local-rust.ps1`, these passed:

```powershell
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

111 workspace tests, including37 viewer tests. Three new cases supply egui keyboard
events and verify consumption, both directions, repeated/empty-frame suppression,
release/repress, disabled history, exact modifiers, and focused TextEdit insertion
after moving its caret with an ordinary arrow. Existing toolbar pointer and route
cancellation/cross-mode tests remain green. An initial let-chain syntax error was
corrected to a match guard to retain the project's Rust2021 edition.

Native hidden/auto-exit QA:

```powershell
target/debug/lbp-viewer.exe --demo --qa-navigation-roundtrip --qa-capture local-analysis/d21-navigation-shortcuts.png
```

Exit zero, expected synthetic route/tab logs, screenshot inspected on RTX3080/
Vulkan: shortcut help and both enabled history buttons visible, entry/Hex selected.
This native recipe uses production route methods; keyboard dispatch is covered by
the headless event tests, not manual operating-system input. Capture/logs ignored.
SFO/MAP hashes unchanged, DATA.FARC size/ReadOnly unchanged, pinned Toolkit clean,
no root Git or publishing. No other GPU/native keyboard-layout claim.

## Next useful format task

D22: add independently specified plain32-bit TEX/DDS top-mip support after checking
the existing C1 pinned oracle result and documented masks/pitch. The stored census
contains one clear256x256 TEX with32bits, pixel_flags65, red/green/blue/alpha masks
0x00ff0000/0x0000ff00/0x000000ff/0xff000000 and326,413 stored bytes. Begin with
these exact masks; preserve raw DDS metadata, bounds, cancellation and unsupported
outcomes. Use synthetic distinctive channel/pitch/truncation recipes and pixel
oracle parity before native preview. Do not infer higher-mip offsets.

The census also has three linear GTF0xa5 textures (two256x1 pitch1024; one16x1
pitch64). Treat them as separate follow-up evidence; no swizzle implementation or
translation of the GPL-origin community helper is authorized by this task.
