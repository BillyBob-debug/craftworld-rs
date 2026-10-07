# D81 corpus-wide mesh census (01.33) — 2026-10-07

## Question

STATUS-CURRENT listed corpus-wide viewable / Complete mesh totals as unknown:
`compatibility_census` is capped at 48 samples per manifest. How many catalog mesh
assets in 01.33 open (viewable) and have every diffuse material routed (all-diffuse)?

## Method

New read-only example `crates/lbp-preview/examples/corpus_mesh_census.rs`; the
48-sample `compatibility_census` is unchanged. Shared, unit-tested logic lives in
`lbp_preview::census` (geometry check, per-mesh coverage, corpus tally).

- Scope: every snapshot MAP row with confirmed type `mesh`, grouped by content hash
  (BTreeMap, deterministic order), one `load_mesh_groups_with_options` per hash with
  default options (no animation base, no supplied instance color). `static_mesh`
  rows (27) are a different type and out of scope. Mesh-typed inspections whose hash
  has no MAP row are excluded and counted.
- Definitions (same as compatibility_census): viewable = loads and has a
  nonzero-area triangle (geometry availability, not visual fidelity); all-diffuse =
  viewable and every nonempty primitive group has a loaded diffuse binding (binding
  coverage, not visual verification). Complete = 0, not certified (manual visual check).
- Unit: distinct catalog GUID; a hash outcome applies to every GUID naming it
  (aliases preserved). Hash counts are reported alongside.
- Budgets: at most 20,000 mesh hashes, 1 GiB aggregate resolved mesh bytes (checked
  add), per-mesh limits of the existing loader (2 MiB input, 50,000 triangles,
  64 material loads, 640 MiB retained), `--max-seconds` (default 7200, max 86400).
  Resolve/load failures are recorded by reason, not fatal. Output: versioned JSON
  (`schema_version` 1, `scope` field), new files under ignored `local-analysis/` only.
- `--hashes MANIFEST` restricts to a sample manifest for cross-checks.

## Counts (01.33, disc + update 0133, snapshot d68-inventory-0133-verified.json)

Baseline: before D81 corpus totals were unknown; tested cohorts only (D61 48/43/0).

| Measure | GUIDs | Hashes |
|---|---|---|
| Mesh MAP rows in scope | 4,261 | 4,245 |
| Viewable | 4,257 | 4,241 |
| All-diffuse | 3,657 | 3,641 |
| Viewable, not all-diffuse | 600 | 600 |
| Complete (visually verified) | 0 | — |

- Failed loads: 2 hashes, both "more than 50,000 triangles" preview budget.
- Loaded without a nonzero-area triangle: 2 hashes.
- Excluded: 37 mesh-typed archive hashes without MAP rows.
- Groups: 8,509 of 9,229 nonempty primitive groups have a loaded diffuse binding.
- Mesh hashes not all-diffuse, counted once per distinct group reason (a mesh may
  appear under several): sampler not static/unwired UV 261; multiply shape outside
  two-texture / texture x COLOR 180; absent material identifier 59; product grid
  budget 51; product pixel budget 47; alpha MIX shape 10; unresolved material GUID 7;
  other reasons (11 kinds) <= 6 each.
- Runtime: 333 s (release build), 151,661,079 resolved mesh bytes.

These are newly *measured* totals, not newly viewable through a code change:
+0 newly viewable / +0 newly Complete from D81 itself.

## Cross-check

`--hashes local-analysis/d61-mesh-samples.json` gives 48 viewable / 43 all-diffuse /
0 Complete, identical to `compatibility_census` on the same manifest; the viewable
and all-diffuse GUID sets and per-hash nonempty/supported group counts are equal.

## Commands (run from the main checkout; binaries built in the D81 worktree)

```
cargo build -q --release -p lbp-preview --example corpus_mesh_census --example compatibility_census
corpus_mesh_census SNAPSHOT "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d81-corpus-mesh-census-0133.json   -> 4245/4261 above, exit 0
corpus_mesh_census SNAPSHOT GAME-DIRS local-analysis\d81-d61-cohort-corpus-census.json --hashes local-analysis\d61-mesh-samples.json   -> 48/48/43
compatibility_census SNAPSHOT GAME-DIRS local-analysis\d61-mesh-samples.json local-analysis\d81-d61-cohort-compat-census.json   -> 48/43/0
cargo fmt --all --check   -> clean
cargo test -q -p lbp-preview   -> 25 passed (3 new census tests)
cargo clippy -q -p lbp-preview --all-targets -- -D warnings   -> clean
```

## Not tested

- No visual check; Complete stays 0. All-diffuse is binding coverage only.
- Opt-in routes (animation base, supplied instance color) not counted.
- `static_mesh` resources, the 01.00-only corpus, and the legacy/Bevy display paths
  were not exercised. No external geometry oracle for the corpus.
- The census is not shown in the app yet.
