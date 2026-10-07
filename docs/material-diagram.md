# D16 native diagnostic material graph

Completed 2026-10-05 (America/Chicago). The Material graph inspector now draws
boxes and directed wires with linked selection, hover diagnostics, bounded drag
pan/zoom, Fit and a diagram visibility toggle. Metadata lists remain available.
Resource/reload reset clears the camera; deeper reads preserve the earlier view.

The layout is an authored index grid. It never interprets serialized positions or
traverses/evaluates a shader. Nodes show index/raw type; selected and hovered wires
show signed endpoints, raw ports and swizzle bytes. No port or texture roles are
invented. Curved routes and arrowheads convey the recorded endpoint direction.
Fit includes backwards and self-link routes. Boxes and wires use the existing
shared parser caps (1,024/4,096); these constants are now publicly reexported.
Pan is clamped to +/-10,000 pixels, relative zoom to 0.25..16. Painter clipping and
bounded routing/hit testing keep large synthetic graphs finite. Invalid endpoints
are not cast to unsigned indices; a virtualized warning list preserves them and
allows selection without inventing a connection. Original metadata is unchanged.

## Evidence and scope

CONFIRMED_FROM_COMMUNITY_CODE: endpoint/port/swizzle layout comes from the existing
independent reader, with pinned Toolkit `MaterialWire::serialize` as external
oracle. Source:
[MaterialWire.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialWire.java),
commit `d6a2ef80b5a09999f08f23a803b516096055e5c9`. Ordinary PS3 branch-zero heads
920-951. Per-field rules, conflicts and sample validation remain in
material-graph.md and material-census.md. D16 introduces no format rule or source
translation. Layout and drawing are independently authored GUI code.

CONFIRMED_FROM_LOCAL_DATA: inspected native RTX 3080/Vulkan captures show
garage_car.gmat GUID 000134dd (head 935, 5 boxes/4 wires) and
dlc_potc_cannon_metal.gmat GUID 000140c1 (head 931, 19 boxes/18 wires) fitting the
1320x850 window. Node labels and selected raw wire fields are visible; remaining
metadata is scrollable. Final captures have correct text encoding and controls.
No new parser oracle run was needed; D15's complete field parity remains evidence.
Shader/sound/animation meaning, original editor layout and rendering fidelity
remain UNKNOWN.

## Verification

Four new authored Rust cases check deterministic bounded/nonoverlapping index
layout and fit at 0/1/19/1,024 nodes; cycle/self-link routes and signed endpoint
extremes; actual egui pointer events selecting a box and a wire without mutating
metadata; empty/invalid/max/over-budget wires and pan clamping. Existing reset,
cancellation, fallback, aliases and native UI tests remain green.

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

95 workspace tests passed (including diagnostic examples; ignored log
local-analysis/d16-workspace-tests.log). Final encoding/control wrapping refinement
was followed by all 21 viewer tests, fmt, Clippy and build. Workspace tests preceded
that UI-only refinement and were not needlessly repeated.

Native capture recipe (repeat for GUID 000140c1 with new output paths):

```powershell
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','000134dd','--qa-tab','material-graph','--qa-capture','local-analysis/d16-material-graph-small-final.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d16-material-graph-small-final.stdout.log -RedirectStandardError local-analysis/d16-material-graph-small-final.stderr.log
$qaProcess.WaitForExit(30000)
```

Both final runs exited zero; images/logs remain ignored. Manual native pan/zoom,
Fit/visibility toggles, metadata-row selection, invalid-endpoint interaction,
maximum-size GPU performance and other GPUs/platforms were not tested. Selection
events are verified by headless egui tests, not claimed as manual native interaction.

Resolved development failures: initial imports exposed that parser cap constants
were private (reexported); headless test texture deltas needed explicit clearing;
Clippy required the test module at the file end. A Python edit using the Windows
default encoding introduced mojibake; explicit UTF-8 repair plus refreshed native
captures verified the fix. No game-format failure, weakened bounds or approval
rejection. Original SFO/MAP hashes, FARC size/ReadOnly and clean reference source
unchanged. Default-deny audit allows authored Rust while excluding private captures
and game input. No Git initialization, commits, publication or license decision.

## Next useful task: D17 cached material diagnostic JSON export

Add an explicit export of already loaded prefix/graph/tail metadata from the native
Material inspector. Use deterministic versioned JSON with selected relative source
location, stage status/error information and metadata; no payload or shader bytes,
absolute paths or timestamps. Export only to a new file under ignored local-analysis,
validate containment and extension, and refuse overwrite. Retain absent/failed/
unrequested distinctions and cached earlier stages. Do not read game data again or
start new parser work. Test exclusive output, path escapes, determinism and stage
fallback; verify one real private report against the existing oracle metadata.
