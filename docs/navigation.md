# D20 bounded route navigation

Authored UI behavior, 2026-10-05 America/Chicago. No format rule, community-source
translation, parser change or new dependency.

Back, Forward and Overview now cover individual MAP rows (aliases remain distinct),
archive table entries and both list-mode overviews. Each visit remembers its tab.
Entry destinations normalize unsupported tabs to Hex. History is bounded to 128
visits in each direction; a new destination clears Forward, repeated selection does
not push a duplicate, and successful snapshot replacement clears both directions.
Invalid destinations are rejected before modifying state; stale invalid history
visits are discarded. Back/Forward clear filters so the restored row can be shown.
Switching list mode opens that mode's overview as a history destination.

One checked transition cancels/discards texture, mesh, material and hex jobs/caches,
clears pending inspector jumps, updates selection/raw metadata and queues a list
scroll target. The validated source attachment remains available. History retains
route and tab, not decoded assets, candidate selection, graph selection, camera,
scroll position or filters. Returning to an inspector can require another bounded
read. A repeated route with a changed tab does not invalidate its resource cache.

Toolbar transitions return before binding the detail body, preventing an old detail
from starting another inspector job on the navigation frame. Archive alias jumps
also return before rendering the old entry inspector.

## Verification

After `. ./scripts/use-local-rust.ps1`:

```powershell
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

108 workspace tests pass, including 34 viewer tests. Five new authored cases cover
cross-mode/tab restoration, scroll targets, rejected/duplicate/branch destinations,
history caps and snapshot replacement, actual egui pointer press/release events
for all three toolbar buttons, and route-driven material cancellation/cache discard.
The pointer test verifies the navigation frame contains only the toolbar, before
the next destination's detail is rendered. Existing individual inspector reset
tests continue to verify their cancellation and cache disposal.

The first headless pointer test failed because unapplied font texture deltas were
dropped; the harness now explicitly clears them. Initial strict Clippy found two
nested mode-button conditions; they were collapsed and the check passed. Neither
failure required weakening product validation.

Native QA, hidden launch with automatic screenshot/exit:

```powershell
target/debug/lbp-viewer.exe --demo --qa-navigation-roundtrip --qa-capture local-analysis/d20-navigation-roundtrip.png
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 000150e5 --qa-tab mesh --qa-capture local-analysis/d20-mesh-regression.png
```

Both runs exited zero; screenshots were inspected on RTX3080/Vulkan. The fixed
synthetic QA recipe uses production route methods, verifies Overview/Hex,
Entry(0,1)/Hex, MAP(0)/Raw and Forward-to-Entry/Hex, then renders an entry with both
history buttons enabled. It does not simulate native mouse clicks. The headless
pointer test supplies actual egui events separately. The real-data regression
validates 31,534 entries using 4,704,862 metadata bytes and previews 24 vertices /
8 triangles. Logs and captures remain ignored under local-analysis.

SFO/MAP SHA256 fingerprints are unchanged; DATA.FARC remains 1,023,990,298 bytes
and ReadOnly. Pinned Toolkit remains clean at
d6a2ef80b5a09999f08f23a803b516096055e5c9. No root Git repository, publishing or
original writes. No manual native interaction, other GPU, concurrent filesystem
race, restored candidate/camera state or new format compatibility is claimed.

Next bounded task: D21 keyboard Back/Forward using the same transitions, with
visible shortcut help, consumed events and tests preventing repeated/disabled
actions. Keep typing behavior intact; no parsing or dependency changes.
