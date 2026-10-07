# D25 native Animation prefix inspector

2026-10-06 America/Chicago. Select Animation on a MAP alias or archive entry after
attaching/validating the source. Uses the verified [D24 CPU reader](animation-parser.md)
without new format rules, dependencies, keyframes, posing or playback.

## Behavior

Native view displays raw frame/fps/loop/morph fields, six counts, optional position
IEEE bits/fat, expanded/opaque/start/prefix/unparsed byte counts. Position fields
expand; byte metadata is in the scrollable area. Counts do not establish valid
tracks, an empty animation or valid payload. No duration/defaults/semantics invented.
Absent optional fields remain explicitly absent; fat meaning remains unknown.

Candidates are listed individually using existing MAP/loose/archive location
construction, preserving aliases/conflicting choices. Source attachment required;
source pointer change/detach resets worker/cache. Route transitions and successful
snapshot replacement also reset Animation, preserving the selected tab in history
where supported. Cached source bytes are not exported.

Worker preflight reads<=32bytes with approved source callback and rejects protected,
non-ANMb, old/unknown revision, branch/flag variants before reading the body. Stored
budget2MiB, pages<=4096bytes, expansion16MiB and cancellation inherited from the
verified reader. Short callback responses explicitly fail. Reload discards cached
prefix and cancels prior work; Cancel prevents queued successful results publishing.
Empty/failed/disconnected workers produce no fabricated metadata. Supported loading
remains off the UI thread. Capture waits for Animation jobs to finish.

Native QA flag --qa-tab animation uses the production path. With capture enabled,
successful cached metadata prints as one QA JSON log line for exact field comparison,
without another source read. Ordinary viewing logs only revision/raw frames/raw fps.
This is QA evidence, not an Animation export command yet.

## Protection classification correction

A new synthetic worker test caught an existing contract mismatch: protection helper
matched exactly4bytes, but several callers passed a longer preflight/full buffer.
This could report a protected signature as Unsupported rather than Blocked. No
decryption was attempted. The helper now classifies only the leading4bytes for any
buffer length; short buffers false, interior markers ignored. Shared binary payload
and Animation pass an explicit prefix too. Existing callers get consistent blocked
classification without new format/protection guesses or weakened guards. A new
cross-signature synthetic test and full-length protected ANM cases prove it.

## Verification

After `. ./scripts/use-local-rust.ps1`:

```powershell
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

124workspace tests, including41viewer, pass. Four new controller/worker/layout cases
cover protected/old/nonmagic/budget preflight, bounded/short pages and cancellation,
cancelled/failed/disconnected results, route/snapshot reset, candidate reset/tab
retention, raw unknown/optional fields and detached UI. Existing minimum-size/all-tab
test now includes Animation. New protection helper test covers short/long/interior
signatures; existing metadata/material/mesh/source tests remain green. Initial
closure-error type inference failure fixed with explicit Result type; protected
worker classification failure investigated and fixed, not normalized away.

Final scroll-area change received narrow Animation tests, fmt/Clippy/build and native
QA; full workspace tests ran immediately before it. No manual scrolling, Reload,
Cancel, candidate editing or other GPU/native-key dispatch claim.

Hidden native capture commands (choose new output names):

```powershell
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00017d6a --qa-tab animation --qa-capture local-analysis/NEW-animation-891.png
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013d70 --qa-tab animation --qa-capture local-analysis/NEW-animation-946.png
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-entry 0:25720 --qa-tab animation --qa-capture local-analysis/NEW-animation-entry.png
target/debug/lbp-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00015dd2 --qa-tab animation --qa-capture local-analysis/NEW-animation-old.png
```

Four runs exit0, captures inspected on RTX3080/Vulkan. Both modern aliases and the
archive entry match every cached field in d23-animation-oracle-fields.json, including
position/fat/nulls, raw counts and byte offsets. Head844 visibly Unsupported and no
cached-prefix log. All use validated31,534-entry source; metadata validation reads
4,704,862bytes. No fresh Java oracle needed or full animation corpus claim.

Original SFO/MAP hashes unchanged; FARC1,023,990,298bytes/ReadOnly unchanged, pinned
Toolkit clean, default-deny audit intact, no root Git/commit/publishing. All captures/
logs remain ignored local-analysis/d25-animation-*; synthetic recipes only trackable.

Next D26: explicit cached Animation diagnostic JSON export using shared private
writer, versioned schema/status/error metadata and synthetic guard tests. No opaque
bytes/keyframes, source reread or parser change; native pinned-oracle/repeat/schema
checks. This completes diagnostic parity with Material/Texture/Mesh inspectors.
