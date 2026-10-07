# D6 native Material metadata tab — 2026-10-05

The Material tab uses the independently verified PS3 GMT prefix reader. It is
available for MAP resources and archive entries, including entries without aliases.
Attach a validated game directory, select a resource/candidate and open Material.
Plain clear GMTb heads920–951/ordinary branch0are the supported prefix contract;
other signatures/layouts and protected resources produce explicit outcomes.

The panel shows revision/serialization flags, expanded/prefix/unparsed byte counts,
collapsible raw scalars and IEEE bits, opaque shader interval metadata, all eight
texture slots in original descriptor order, raw wrap S/T values and metadata JSON.
Wrap positions not declared are marked "not declared". Unknown numeric enum values
are retained; raw zero identifiers remain visible as unresolved/raw null. A null
descriptor is different from an unresolved identifier. This is inspection metadata,
not proof of texture roles, graph semantics or original rendering behavior.

Each GUID/hash has a separate candidate menu. All MAP aliases are listed and
overlapping/conflicting candidates are preserved, with no automatic winner. Select
a candidate to follow existing navigation/history to Metadata; Texture can then
preview the supported resource. Menus use virtualized rows; the fixed eight slots
live in a vertically scrollable panel. Scalar/shader/JSON sections are collapsible.
No shader/blob/tail bytes are exposed by this tab or its diagnostic JSON.

## Architecture and provenance

UI depends on lbp-resource directly for metadata only; reader/decompression is
independent of eframe/egui. Format rules, pinned Toolkit source URLs, conflicts,
build applicability and parser tests remain in [material-prefix.md](material-prefix.md).
No new format inference or third-party code translation was made during D6.
Material candidate resolution shares the authored mesh GUID/hash grouping helper
through app/references.rs; no dependency between the material and mesh codecs.

One worker and one metadata cache per selected source/key. It reads at most32bytes
for magic preflight before allocating a larger input, then approved4096byte pages.
Stored1MiB/expanded8MiB limits are enforced by the reader. Known protected locations
are refused by attachment before opening input; unexpected protected magic is
blocked after bounded preflight. Workers only use validated read-only attachment.
Reload, cancellation, source/candidate/row changes and catalog replacement discard
old jobs/results. Dropping a job cancels its token; cancelled completion cannot
publish metadata. No game-file writes, extraction, shader execution or asset export.

## Verification and reproduction

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-viewer
cargo test --workspace --all-targets
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
```

74workspace tests,13viewer tests, fmt/Clippy/native build pass. Three new authored
viewer cases cover reset/cancellation/cache/navigation disposal; cancelled or failed
worker cannot publish; layout with NaN bits/unknown enums/wraps/null/both IDs,
alias/conflict and unresolved references. Existing every-tab test now includes
Material across all synthetic rows at minimum window size; existing source
protection/candidate/navigation/entry tests still pass.

Initial test used a nonexistent demo convenience API; corrected to Catalog::build
of authored demo_snapshot. Its first successful layout then failed on unapplied
egui font texture deltas; explicitly cleared for the headless test without a GPU.
Neither failure involved original inputs or parser results. Final checks pass.

Native QA on RTX3080/Vulkan: GUID000134dd garage_car.gmat, head935/flags7, eight
slots, three actualGUIDcandidate groups each resolving once plus raw null slots.
GUID00013051 mesh in Material reports unsupported after preflight. Captures inspected;
lower slots are accessible through scrolling. No manual menu click/scroll/cancel
input was delivered, other GPU/platform tested, or full material corpus scanned.
Tests cover resolver/state/history and layout; captures cover native rendering.

QA commands (new ignored output filenames required):

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 000134dd --qa-tab material --qa-capture local-analysis/NEW-material-tab.png
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013051 --qa-tab material --qa-capture local-analysis/NEW-material-unsupported.png
```

Actual ignored artifacts: local-analysis/d6-material-tab.png and
d6-material-unsupported.png plus separate stdout/stderr logs. Screenshots may contain
private paths/game-derived information; never publish them as fixtures. Track only
authored source/tests/docs. Inputs/fingerprints/read-only attributes unchanged;
reference source clean, root without Git, no commit or publication.

## Exact next bounded task — D7 graph metadata reconnaissance

Study pinned Toolkit MaterialBox/MaterialWire/BoxType/BrdfPort readers. Establish
ordinary PS3 revision/flag applicability and guarded external metadata oracle for
box/wire counts, raw kinds/ports, texture-slot links and bounds. Use documentation,
existing readers and synthetic structures first; compare bounded local samples.
Record conflicts/unknowns before implementing an independent graph metadata reader.
Do not infer sampler roles from dependency order, execute original shader code,
evaluate graphs, add textured shading or expand into game/runtime/animation scope.
