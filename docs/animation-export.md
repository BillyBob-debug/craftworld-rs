# D26 cached Animation diagnostic JSON

2026-10-06 America/Chicago. Animation inspector now exports its completed cached
metadata to a new `.json` under ignored `local-analysis/`. The UI export section
and `--qa-animation-json` use the same production saver. No extra source reads,
parser calls, animation payload bytes, locator records, keyframes or playback.

## Contract and provenance

Version 1 envelope: `schemas/animation-diagnostic.schema.json`. Explicit projection
in `crates/lbp-viewer/src/app/animation/export.rs` includes compression flags,
expanded length, raw prefix scalars, six counts, optional IEEE bits/fat and opaque
byte offsets/lengths. Absent fields remain null, raw unsigned values remain raw.
`source_state` is `validated_at_read_not_rechecked_for_export`: this is a report of
cached inspection, not a fresh source-integrity guarantee. Source-relative location
is normalized; error categories exclude host paths/messages. No timestamp or timing.

No new disk-format knowledge. All field provenance, pinned Toolkit commit/class,
revision/branch limits and evidence labels remain in [animation-parser.md](animation-parser.md)
and [animation-prefix.md](animation-prefix.md). Native lifecycle in
[animation-inspector.md](animation-inspector.md). This exporter is independently
authored project code; no external parser code was copied or translated.

Shared `app/diagnostics.rs` enforces ordinary workspace `local-analysis`, existing
contained parent, JSON extension, exclusive new-file creation and 8MiB serialized
budget before opening output. Busy, unselected, incomplete or reset inspections
are refused. Completed unsupported/blocked/cancelled/failed outcomes have null
metadata and stable categories. Source attachment remains mandatory for native
inspection, although projecting an existing cache itself requires no source handle.
Reload clears export messages; route/source/snapshot reset clears cached result and
export state. Existing writer limits remain: ancestor races are not fully covered;
a partial I/O failure may leave a file, and subsequent overwrite is refused.

## Verification

Local Rust toolchain loaded with `. ./scripts/use-local-rust.ps1`:

```powershell
cargo test -p lbp-viewer animation
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

All pass: 126 workspace tests / 43 viewer tests. Two new authored tests cover
explicit cached field projection, absent/unknown/raw values, excluded payload,
normalized location, busy/incomplete/reset refusals, cancellation and sanitized
failure categories. Shared exclusive-path/budget tests also pass.

Native QA command (new output names required):

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013d70 --qa-tab animation --qa-capture local-analysis/new-animation.png --qa-animation-json local-analysis/new-animation.json
```

Four hidden native auto-exit runs, all exit 0, RTX3080/Vulkan:
- `d26-animation-891`: GUID00017d6a, head891; JSON1143bytes.
- `d26-animation-946`: GUID00013d70, head946/fat1; JSON1310bytes.
- `d26-animation-old`: GUID00015dd2, head844; unsupported/null, JSON530bytes.
- `d26-animation-946-repeat`: same supported resource; JSON byte-identical.

Three supported reports' complete metadata objects equal the corresponding D23
pinned external oracle records (compression flags, expanded bytes and all prefix
fields). All four pass Draft202012Validator with ignored jsonschema4.23.0; schema
self-check passes. Five mutated reports rejected: extra payload field, raw16 overflow,
wrong count-array length, unsupported status with metadata, schema version2.
Inspected the head946 PNG: export section and saved confirmation visible. Native
captures exercise production saver, not manual editing, click/scroll or file dialogs.
No new full animation reader/visual correctness coverage claimed.

SFO/MAP SHA256 unchanged; FARC1023990298bytes retains ReadOnly. Toolkit pin
`d6a2ef80b5a09999f08f23a803b516096055e5c9` clean. No root Git, commits or publication.
All native JSON/PNG/logs are ignored; authored Rust/schema/docs remain allowlisted.

## Next bounded task

D27: census clear level/plan resource headers from the existing inventory, compare
with pinned community RLevel/RPlan readers/specs, and choose a bounded metadata-only
slice useful for level inspection. Preserve aliases, revisions, dependency evidence
and explicit unknowns. No full Thing graph, rendering, recovered source or gameplay.
Before implementation establish reader/oracle coverage and allocation preflight;
if a safe prefix cannot be isolated, document that boundary and choose another
useful inspector improvement.
