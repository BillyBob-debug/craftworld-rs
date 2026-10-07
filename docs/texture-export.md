# D18 cached texture diagnostic JSON export

Completed 2026-10-05 (America/Chicago). The Texture tab now offers an explicit
destination and Save action under "Export cached texture diagnostic JSON".
It exports only cached header, expanded byte count, top-mip block byte count and
unparsed byte count. An explicit field whitelist excludes decoded RGBA pixels,
encoded blocks, images and other asset data. Channel, fit, zoom and GPU state do
not affect the report. Export invokes no source reader, decoder or parser.

Version 1 schema: `schemas/texture-diagnostic.schema.json`; scope
`cached_texture_diagnostic_metadata_not_asset_export`. It preserves relative
source location, prior-read scope, request outcome and texture stage availability.
Failed, unsupported, blocked and cancelled results have null metadata and a
stable error category, never guessed empty headers. Busy, unselected and
uncompleted previews cannot export. The current preview API supplies no partial
header on decode failure, so the exporter does not invent one. Raw DDS mip zero
remains zero rather than becoming one.

## Shared safeguards and architecture

The second consumer justified moving D17's private output guard, bounded writer,
relative location validation and stable stage/error helpers to `app::diagnostics`.
Material and Texture envelopes stay separate, and parser libraries remain
independent of UI and filesystem export. No product dependency or codec change.

Safeguards remain: canonical ordinary workspace local-analysis root and contained
existing parent, new `.json` only, exclusive create, no overwrite, deterministic
serialization and an 8 MiB budget before file creation. Arbitrary OS error strings
and absolute host paths are excluded from report bodies. Prior-read metadata is
explicitly not a fresh source-stability assertion. Disk-write failures can leave
partial files; they remain protected from overwrite. Directory replacement races
are not claimed to be fully prevented. See material-export.md for the policy.

## Evidence and provenance

No new format rule. CONFIRMED_FROM_COMMUNITY_CODE: existing Toolkit texture header/
envelope rules at pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`, including
CellGcmTexture, SerializedResource and RTexture. CONFIRMED_FROM_DOCUMENTATION: BC1/
BC3 and DDS fields retain Microsoft provenance in texture-preview.md,
tex-preview.md and bc3-preview.md and near lbp-texture's implementation. Higher
mips and unimplemented layouts remain UNKNOWN/unsupported, not inferred by export.

CONFIRMED_FROM_LOCAL_DATA: native exports from existing C4 samples:

| GUID | Wrapper / dimensions | Resource SHA1 | JSON bytes / SHA256 |
|---|---|---|---|
| 00017440 | GTF BC3, 512x512, 10 declared mips | 69824fc9c2eab333e57dca0fa7800ad6a4acec7e | 903 / 89213115383cfcb3de6aa59d2d333dfd80cddf2aac31b604e338aa915108c1ae |
| 0000c9cd | TEX/DDS BC3, 128x128, raw mip count 0 | 2189a80a2ee12e1f3d31bf515ef3d46be6b7e185 | 1305 / 5d743a8d3e29edbbee88a41a11dc11f17b10c5db645face025c285ff9484e1d7 |

Each source location matches the private C4 sample manifest. Exported metadata
equals every field of the previously oracle-validated Rust C4 parity report's
texture object. Independent checks against the pinned external oracle also match
dimensions, mip count, no_srgb, raw GTF base format and all supplied DDS fields.
The external oracle does not emit every Rust cached count/raw header field; direct
oracle equality is claimed only for those shared fields. No fresh pixel digest,
decoder oracle run or new visual fidelity claim was needed for this projection.

Repeated GTF export is byte-identical across native RGBA and red-channel views.
Material regression export is byte-identical to D17's 19,153-byte report, SHA256
`e1cfe4f258d0f31626f3f56381ff79f2b0a0c2c8d8fc538bf16daccf5a398016`.
Both texture schemas validate with the existing ignored jsonschema4.23.0 tool;
four mutations per report are rejected: pixel payload, nested header pixels,
unsupported stage with available metadata, wrong scope. Material schema remains
valid. Schema copies shared stage definitions but strictly whitelists texture
metadata and header properties; it does not replace parser validation.

## Verification and reproduction

Two new authored texture cases verify metadata whitelist and independence from
channels/zoom/pixel contents, raw DDS fields and mip zero, failure categories,
busy/reset refusal and host-path exclusion. Two existing private-output cases
moved to the shared module and now also prove over-budget serialization creates
no file. Existing Material cache/fallback checks remain green. All 101 workspace
tests (27 viewer tests), fmt, Clippy and native build pass. Log:
local-analysis/d18-workspace-tests.log (ignored).

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
$qaProcess = Start-Process -FilePath target/debug/lbp-viewer.exe -ArgumentList @('--snapshot','local-analysis/a2-inventory-verified.json','--game-dir','game-data/a1-disc','--select-guid','00017440','--qa-tab','texture','--qa-texture-json','local-analysis/d18-gtf-diagnostic.json','--qa-capture','local-analysis/d18-gtf-export.png') -WindowStyle Hidden -PassThru -RedirectStandardOutput local-analysis/d18-gtf-export.stdout.log -RedirectStandardError local-analysis/d18-gtf-export.stderr.log
$qaProcess.WaitForExit(30000)
```

TEX capture uses GUID0000c9cd, `--qa-channel a` and new tex output paths; GTF repeat
uses `--qa-channel r` and new repeat paths. Material regression uses D17's QA recipe
with new d18 paths. All four native runs exited zero. Inspected GTF RGBA and TEX
alpha screenshots on RTX3080/Vulkan: saved status, metadata and previews fit the
pane. QA uses the same production save function as the GUI button. No manual
destination editing/Save click, native error interaction, other GPU/platform,
directory-race test or full-corpus export claimed. Python sources/tests unchanged;
prior 27 Python tests were not repeated. No new format failure or approval rejection.
Original SFO/MAP hashes, FARC size/ReadOnly, clean pinned reference source and
default-deny audit unchanged. All images, JSON and tools remain ignored; authored
Rust and schema remain trackable. No Git initialization, commits/publication or
licensing decision.

## Next useful task: D19 cached mesh diagnostic export

Inspect the actual mesh cache/API before designing the report. Extend shared
private output to mesh metadata and summaries: existing validated prefix/counts,
topology, bounds and material references where available. Exclude vertex/index/
bone data arrays, geometry payload and images. Preserve failures and source-relative
location without additional reads or new parser work. Use a versioned envelope,
synthetic cache/output checks and one native report compared against existing
mesh oracle/parity evidence. Keep visual wireframe behavior unchanged.
