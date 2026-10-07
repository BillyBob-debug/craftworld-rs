# D85 opt-in corpus mesh census (01.33) - 2026-10-07

## Question

D81's `corpus_mesh_census` measured only the default preview route. How many catalog
mesh assets in 01.33 are all-diffuse when the existing opt-ins (animation base values,
supplied instance color) are enabled, alone and together?

## Method

- `corpus_mesh_census` accepts the same opt-ins as `compatibility_census`:
  `--animation-base` and `--instance-rgba R,G,B,A`. Parsing lives in
  `lbp_preview::census` (`parse_instance_rgba`, `apply_preview_flag`,
  `preview_scope_suffix`); `compatibility_census` now calls the shared RGBA parser
  (same values and error messages). Repeating an opt-in flag in `corpus_mesh_census`
  is an error.
- Without opt-ins the output format is unchanged (`schema_version` 1, same fields).
  With any opt-in the report is `schema_version` 2: `scope` gets the suffix
  `+animation_base` and/or `+instance_rgba` (fixed order), `scope_note` says opt-ins
  are on, and a `preview_options` object records `animation_base_enabled`,
  `animation_time` (null: base values, no motion), `supplied_instance_color_bits`,
  `supplied_instance_rgba` and `verified_game_instance: false`.
- Scope, definitions, budgets and unit are D81's (docs/d81-corpus-census.md):
  every MAP row with confirmed type `mesh`, one load per content hash; viewable =
  loads with a nonzero-area triangle; all-diffuse = viewable and every nonempty group
  has a loaded diffuse binding (binding coverage, not visual verification).
  Complete = 0 (manual visual check, not done).
- Unit: distinct catalog GUID (hash counts alongside). The instance color
  0.25,0.5,0.75,0.5 is an arbitrary supplied test value, not a real game instance.
- Unit tests (synthetic): RGBA parsing bounds/arity/NaN/inf, flag application,
  repeat and missing-value rejection, scope suffix order.

## Counts (01.33, disc + update 0133, snapshot d68-inventory-0133-verified.json)

Baseline: D81 corpus census, 4,257 viewable / 3,657 all-diffuse GUIDs of 4,261
(default route). Opt-in totals were unknown before D85.

| Mode | Viewable GUIDs | All-diffuse GUIDs | All-diffuse hashes | Supported / nonempty groups |
|---|---|---|---|---|
| default | 4,257 | 3,657 | 3,641 | 8,509 / 9,229 |
| `--animation-base` | 4,257 | 3,709 (+52) | 3,693 | 8,598 / 9,229 |
| `--instance-rgba 0.25,0.5,0.75,0.5` | 4,257 | 3,872 (+215) | 3,856 | 8,773 / 9,229 |
| both | 4,257 | 3,932 (+275) | 3,916 | 8,871 / 9,229 |

- No mode loses an all-diffuse GUID relative to default; opt-ins only add.
- Failed (2, triangle budget), no-geometry (2) and excluded unmapped (37) hashes are
  the same in every mode. Complete = 0 in every mode.
- Largest remaining not-all-diffuse reasons with both opt-ins (mesh hashes, counted
  once per distinct reason): absent material identifier 59, product grid budget 51,
  product pixel budget 47, BLEND phase 1 input shape 40, sampler not static/unwired 32.
- These are measured opt-in totals, not a change to the default route: +0 newly
  viewable / +0 newly Complete from D85; default all-diffuse unchanged (3,657).

## Default-mode identity

- Default totals equal D81 (4,261 / 4,257 / 3,657). The D85 default JSON differs from
  the D81 file only in neutral reason text on 59 mesh rows (D83/D84 merged after D81
  split reasons, e.g. "sampler must be static" 261 -> 215 plus new BLEND/MIX reasons);
  totals and GUID lists are equal.
- Byte identity of the flag change itself: origin/main's `corpus_mesh_census` built
  against the same library and run on the same inputs writes a file whose SHA-256
  equals the D85 default output (byte-identical).

## Cross-check

`corpus_mesh_census --hashes local-analysis/d61-mesh-samples.json --animation-base
--instance-rgba 0.25,0.5,0.75,0.5` gives 48 viewable / 48 all-diffuse, identical to
`compatibility_census` with the same flags on the same manifest; viewable and
all-diffuse GUID sets and per-hash nonempty/supported group counts are equal.

## Commands (outputs run from the main checkout; binaries built in the D85 worktree)

```
cargo build -q --release -p lbp-preview --example corpus_mesh_census --example compatibility_census
corpus_mesh_census SNAPSHOT "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d85-corpus-default.json   -> 4261/4257/3657, 325 s
  ... local-analysis\d85-corpus-animation-base.json --animation-base   -> 4261/4257/3709
  ... local-analysis\d85-corpus-instance-rgba.json --instance-rgba 0.25,0.5,0.75,0.5   -> 4261/4257/3872
  ... local-analysis\d85-corpus-both.json --animation-base --instance-rgba 0.25,0.5,0.75,0.5   -> 4261/4257/3932
corpus_mesh_census ... local-analysis\d85-d61-cohort-corpus-both.json --hashes local-analysis\d61-mesh-samples.json --animation-base --instance-rgba 0.25,0.5,0.75,0.5   -> 48/48/48
compatibility_census ... local-analysis\d61-mesh-samples.json local-analysis\d85-d61-cohort-compat-both.json --animation-base --instance-rgba 0.25,0.5,0.75,0.5   -> 48/48/0
origin/main corpus_mesh_census (temporary example, not committed) ... local-analysis\d85-corpus-default-base-tool.json   -> 4261/4257/3657, SHA-256 identical to d85-corpus-default.json
cargo fmt --all --check   -> clean
cargo test -q -p lbp-preview   -> 35 passed (2 new census tests)
cargo clippy -q -p lbp-preview --all-targets -- -D warnings   -> clean
```

## Not tested

- No visual check; Complete stays 0. All-diffuse is binding coverage only.
- Only one supplied instance color was measured; other values were not. Real game
  instance colors remain unknown.
- Animation base values only (no motion/time). Single-opt-in cross-checks against
  compatibility_census were not run; only the combined mode was cross-checked.
- `static_mesh`, the 01.00-only corpus, legacy/Bevy display paths; census not shown
  in the app.
