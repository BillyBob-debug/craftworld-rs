# Metadata explorer B1 + B2 — 2026-10-05

B2 adds validated source attachment, bounded hex inspection and independent archive
entry browsing. See [source validation and usage](source-attachment.md). The B1
verification below remains a historical record of snapshot-only browsing.

Native metadata browser over scanner schema 2. Original-format parsing remains in
the scanner libraries. No game data, textures, models, or proprietary fixtures are
bundled. The six-row Demo is entirely authored metadata with fictional identifiers.

## Usage

```powershell
. ./scripts/use-local-rust.ps1
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json
cargo run -p lbp-viewer -- --demo
cargo run -p lbp-viewer -- --check-snapshot local-analysis/a2-inventory-verified.json
```

The last command validates/indexes without a window and prints counts plus an
explicitly nondeterministic load duration. `--query TEXT` sets the initial filter;
`--select-guid HEX` selects the first matching MAP row. Exact GUID search displays
all matching aliases. Copy buttons copy full identifiers, including the SHA1 that
is shortened on screen. Archive filters select MAP rows with candidates in that
archive; B2's Archive entries mode includes entries without MAP aliases.
Inspection/type totals describe locations; the resource list counts MAP rows.

Select a row for Metadata, Depends on, Referenced by, or Raw metadata. Reference
buttons navigate to target/source MAP rows; Back returns to earlier selections.
Direct edges retain duplicate dependency entries and ambiguous candidates. Incoming
edges union GUID/hash targets without counting one raw edge twice. No recursive
graph walk occurs, so cycles cannot trigger traversal. Raw metadata is a read-only
diagnostic projection, not original bytes or a lossless re-export of the snapshot.

Open snapshot loads an editable path. Reading/validation/indexing run on one worker;
Cancel, a failed load, or closing the window cancels the job. A previous successful
catalog remains available. B2 adds source/hex workers; a new catalog detaches any
old source. Native file picker, live scanning, texture/mesh preview and payload
decompression remain deferred.

## Architecture, provenance, and bounds

`lbp-catalog` uses only serde/serde_json and standard library I/O/indexes, with
tempfile for tests. B2 adds shared io/archive/resource/core and SHA-256 dependencies
for attachment validation. It owns a typed schema-2 projection, identifier/alias indexes,
cached search terms, and forward/reverse indexes. Wire provenance is the project's
A2 schema in `docs/resource-metadata.md` and scanner output types; no community
format reader is copied into this layer. It ignores unused envelope fields and
rejects unknown schema/milestone combinations. Schema-2 tables-only snapshots are
supported; the historical schema-1 A1 inventory requires regeneration.

`lbp-viewer` uses eframe/egui 0.36.2, wgpu 30.0.1 via eframe, and image 0.25.10 for
local QA PNGs only, pinned in Cargo.lock. Selection/filter state belongs to the UI;
game identifiers/relationships are independent of graphics. Versions/API were
checked against the official [eframe docs](https://docs.rs/eframe/0.36.2/eframe/)
and installed crate source. GUI configuration disables eframe persistence and
uses the wgpu backend. There is no network path. B2 delegates validated read-only
original-byte access to the catalog attachment/shared io libraries.

Inputs must be regular files, at most 256 MiB. Reader checks cancellation at each
64 KiB buffer refill and enforces a total byte budget even if the file grows.
Catalog validation caps MAP rows/locations/archive entries at one million each,
dependency links and target references at two million each, and raw dependencies
per location at 65,536. It validates identifiers, declared principal counts, table
ranges, row/link indices and duplicates. These are input/work bounds, not a hard
heap limit or authentication of snapshot claims. Trusted scanner-produced snapshots
are the intended input. Lists virtualize fixed-height rows; filters/search terms
and type choices are cached. The whole accepted catalog stays in memory.

## Verification

On this Windows GNU Rust 1.99.0 setup, the GUI built and rendered on NVIDIA GeForce
RTX 3080 / Vulkan. The 119,338,719-byte existing inventory loaded/indexed in 4.434 s
headlessly and approximately 4.6 s in the window (debug build). Counts matched:
36,854 MAP rows, one archive, 31,534 inspected locations, 116,739 raw reference
links. Timings are observations, not a performance guarantee.

Authored tests cover aliases and exact identifiers, missing/protected/unknown states,
reference unions, duplicate edges and cycles, invalid schema/counts/indices/ranges,
read-only input preservation, byte budgets and cancellation, UI navigation/history,
and preservation of the previous catalog on cancellation/error. Native screenshot
QA inspected the synthetic dependency view and full local overview, dependencies,
reverse references, metadata, and raw metadata. Screenshots stay ignored. No manual
mouse/keyboard interaction, other GPU/backend/platform, release performance,
screen reader accessibility, or adversarial fuzzing was verified.

Reproduction commands (captures need new names):

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p lbp-viewer -- --demo --select-guid 00001001 --qa-tab dependencies --qa-capture local-analysis/b1-demo-new.png
```

QA capture uses egui's viewport screenshot command and exits after saving. It
refuses existing output and accepts only PNG paths inside this workspace's ignored
local-analysis directory. It does not capture other applications. The compiler and
headless state tests verify behavior; rendered screenshots verify layout, not
interactive click delivery or payload correctness.

## B2 and C2 complete

B2 implements these source/hex/entry features using existing format readers, with
verification in source-attachment.md. C1 evidence now supports C2's plain 2D GTF
BC1 Texture tab. See [texture-preview.md](texture-preview.md) for supported scope,
channel/alpha/zoom views, validated workers, native QA and CPU pixel parity.

D6 adds the Material tab for bounded clear PS3 GMT prefixes and separate raw
GUID/hash texture-slot candidate navigation; [usage and verification](material-viewer.md).
Mesh/texture/material code remains separate from rendering and game simulation.
