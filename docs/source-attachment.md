# B2 source attachment and hex inspection — 2026-10-05

This completes Basic Asset Explorer B for metadata and bounded original-byte
inspection. No texture, mesh, level or gameplay implementation was added.

## Usage

```powershell
. ./scripts/use-local-rust.ps1
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --entries
cargo run -p lbp-viewer -- --check-snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-entry 0:0
```

The window accepts an extracted disc root or PS3_GAME directory in Game directory.
Attach source validates on a cancellable worker. Detach clears cached bytes and
cancels pending source/hex jobs. Failed/cancelled replacement attachment retains a
previous valid source; its actual root is in the Attached indicator's tooltip.
Switch MAP resources / Archive entries to browse either list. Entry rows preserve
table indices and duplicate/hash conflicts, show hash-alias counts, and link to MAP
rows. Archive filter applies to both modes. Entry search matches archive, hash or
decimal entry number; MAP type/status filters do not filter archive entries.

For a resource, select Hex bytes and choose an explicit archive/loose candidate.
Archive entries expose hex directly. Reads use 4 KiB pages, stop at candidate EOF,
accept decimal or 0x-prefixed offsets, and show relative byte positions plus the
page's absolute file offset. Prev/Next page within that candidate. Horizontal
scrolling reveals ASCII columns. No byte export is offered. Cached pages are
labeled as such; Read validates the source again. Source/byte workers are cancellable.
Headless checking emits counts/byte counts only, never raw bytes.

## Shared source access and evidence

`lbp-io` now owns the existing Source and SFO modules moved out of lbp-cli. CLI and
catalog attachment share discovery, MAP path normalization, root containment,
symlink/reparse rejection, regular-file and size checks. Source.open checks every
path component, including links introduced after discovery, even when their target
stays inside the game root. Caller-supplied filesystem keys must be strict relative
slash paths, never absolute/parent/device-stream paths.

Attachment uses the original A1 `read_map`/`read_farc` and shared SFO identity reader.
Their pinned community provenance remains beside those implementations and in
docs/formats.md: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, FileDB::process,
FileArchive constructor and ParamSFO::build. This pass introduced no new format
rules, no speculative decoding and no copied community implementation.

Binding, stability stamps, approval keys and hex pagination are project-authored
policies, not claims about the game's engine or undocumented formats.

## Validation and boundaries

Attachment requires the scanner's build_fingerprint and map records. Historical
schema-1 files and authored Demo metadata cannot attach; schema-2 table-only input
can attach. Verify, in order:

1. Read SFO under 1 MiB and compare size, SHA-256 and all emitted identity fields.
2. Read MAP under 64 MiB, compare SHA-256, revision/count and every row's path,
   timestamp, declared size, GUID and indexed SHA1 against the catalog projection.
3. Read each listed FARC footer/table with existing checked bounds. Compare file
   size, table offset, table count and every entry's index/hash/offset/size.
4. Check resource candidate and inspection links against those validated tables.
   Check each loose candidate against its normalized MAP path, discovered file and
   size. An arbitrary snapshot path/inspection cannot grant extra byte access.
5. Record sizes, modification and creation timestamps on all bound files; recheck
   them after validation and before/after every page. Reject changes or links.

Metadata validation has a 128 MiB total read budget and checks cancellation during
discovery, buffer reads and comparisons. The scanner's per-table and entry bounds
remain in force. File stamps are metadata checks, not payload authentication or
atomic protection against a malicious concurrent filesystem. Keep source files
stable; no watcher, file locking or complete payload SHA1 verification is claimed.
Changes before attachment that preserve identity/tables can escape payload checks.
Extra source files do not expand the snapshot inventory; regenerate to discover them.
Only a single page is cached in the viewer.

FARC ranges with identical locations and differing indexed hashes retain all claims;
the viewer does not choose archive precedence or claim payload-hash verification.
Zero-length/short entries remain bounded. Each page checks at most four signature
bytes at the resource start before any requested payload-range read. The shared A2
protection rule recognizes NPD/SCE/package signatures and known encrypted resource
wrappers. A protected result returns no preview. A snapshot location already marked
blocked returns before any original-file I/O. This is the established read boundary,
not an encryption detector for every unknown proprietary format.

## Verification and limitations

The existing BCUS98245 01.00 source bound successfully using **4,704,862 metadata
bytes**: SFO/MAP plus FARC prefix/footer/table, no full-archive scan. All **31,534**
entries matched the snapshot; the selected first entry yielded a 4,096-byte page
at absolute offset 39,023,490. The existing local FARC has zero hashes without MAP
aliases; orphan browsing is verified using authored synthetic tests, not claimed
as a local-disc discovery.

Eight new tests cover immutable authored inputs, conflicting co-located hash claims,
aliases/orphans, candidate EOF and invalid offsets/page lengths, forged fingerprints
and table/candidate paths, protected prefixes/pre-blocked locations, cancellation,
changed files, case-changed extracted paths, post-discovery ancestor junctions, and
entry/alias UI transitions. Existing CLI tests pass after source extraction.
The 35-test workspace run and updated catalog suite pass (36 tests total); Clippy, formatting
and native build pass. Rendered archive/resource hex views were inspected on
RTX 3080/Vulkan; captures remain ignored under local-analysis/b2-*. Manual click
delivery, other platforms, hostile races and decoded visual output remain untested.

Commands:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-entry 0:0 --qa-capture local-analysis/b2-entry-new.png
```

No new extraction, executable investigation, protected decoding, game writes,
commits or publishing occurred. Next is C1: identify local clear texture layouts
and run a pinned Toolkit oracle before choosing the first pixel decoder.
