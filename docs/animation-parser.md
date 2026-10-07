# D24 independent animation prefix reader

2026-10-06 America/Chicago. CPU-only lbp_resource::animation::inspect reads modern
ordinary ANMb diagnostic prefix metadata. Independent code; no community source
translation, new dependency, keyframe/locator decoding, UI change or playback.

## Contract and provenance

Field order, optional revision gates, raw signedness conflict and oracle scope are
documented in [D23 evidence](animation-prefix.md). All rules derive from pinned
[RAnimation::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RAnimation.java)
and MemoryInputStream at the same commit, with references beside implementation.
Evidence: CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA on33prefixes
across20local modern revisions. Synthetic-only gate heads are not local build proof.

Scope: branch/subrevision0, heads888..951, serialization flags0/7, clear ANMb.
Older/head844, other branches, unsupported flags/nonbinary magic return Unsupported;
protected markers return Blocked. Shared binary::payload validates header/dependency
boundaries, source length, strict raw/stored/zlib envelopes and cancellation.
Input2MiB, expanded16MiB; no unbounded allocation. Decompression precedes prefix
revision rejection but remains capped/cancellable. Prefix data bounds are strict.

AnimationInspection retains compression flags, expanded bytes and AnimationPrefix:

- raw16-bit frame count/fps/loop start and raw8-bit morph count;
- six counts: bones, rotation/position/scale/morph tracks and locator keys;
- optional raw IEEE32 position offset/scale bits and raw fat byte;
- opaque animation-data length/start, parsed-prefix and unparsed byte counts.

Absent optional fields remain null. Counts/raw scalar values, zero fps, high-bit
shorts, unknown fat and nonfinite float bits remain diagnostic; no semantic validity,
duration, transform evaluation or default values invented. Array length is bounded
and verified inside expanded payload, but opaque animation bytes/tail are not
decoded, returned or serialized. parsed_prefix_bytes ends after its length word;
unparsed_bytes includes the whole opaque blob and any locator/tail data. Missing
locator records do not make the prefix malformed or prove an empty animation.

The existing authored Material unsigned-integer decoder is now shared privately
under binary::integer, used by both prefixes. BE32/ULEB32 behavior unchanged; exact
overflow/truncation guards retained. Full material/graph/tail synthetic tests and
32local material prefixes validate this small refactor. No parser library depends
on GUI/rendering or filesystem input paths.

## Checks and local parity

After `. ./scripts/use-local-rust.ps1`:

```powershell
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
cargo run -p lbp-catalog --example animation_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d23-animation-oracle-fields.json local-analysis/NEW-animation-parity.json
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d4-material-prefix-oracle-final.json local-analysis/NEW-material-regression.json
```

119workspace tests (37viewer) pass, including four new animation cases. Recipes
cover heads888/907/908/945/946/951; flags0/7; raw/zlib/stored/zero-count envelopes;
raw extremes/NaN/nulls, all prefix/blob and compressed-wire truncations, uninterpreted
tail, old/new/branch/flag/protection rejection, ULEB overflow, input/expansion budgets
and cancellation before/after inflation. No failed experiment in this milestone.

All33modern oracle prefixes match exactly, including optional bits and byte offsets;
two sampled old layouts are explicitly unsupported.205,629archived bytes/run plus
shared source-validation metadata; reports repeat byte-identically (30,715bytes).
32material prefixes regress with0mismatches/0failures. No fresh Java oracle needed:
the pinned validated D23 evidence is reused. No whole-animation corpus compatibility,
payload validation, new float semantics or native animation UI claim.

The parity example validates oracle schema/pin/scope, caps48samples, rejects absent/
ambiguous hash locations instead of silently choosing a candidate, reads<=4096-byte
pages with source checks, caps64MiB total and writes exclusive private JSON. Supported
oracle expectations must match; only expected old-layout Unsupported is counted as
an intentional skip. Unexpected unsupported/error outcomes fail the comparison.

SFO/MAP hashes unchanged, DATA.FARC ReadOnly/size unchanged, pinned checkout clean,
default-deny audit intact, no root Git/commit/publishing. Only authored source/tests/
docs trackable. Reports ignored: d24-animation-parity/repeat and material-regression.

Next D25: native Animation prefix inspector for MAP aliases/archive entries, using
validated source and bounded cancellable worker/cache. Show raw fields, optional
bits/counts and explicit unparsed/protected/unsupported outcomes. Integrate cache
reset with existing route/detach/snapshot transitions and add synthetic lifecycle/
layout tests plus selected native oracle-field verification. No posing/playback.
