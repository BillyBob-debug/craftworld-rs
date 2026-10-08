# D93 measured corpus counts in the app — 2026-10-07

## Question

AGENTS.md asks for viewable / all-diffuse / Complete distinctions and measured counts
(with baseline, unit and scope, unknown totals explicit) to be displayed in the app.
The corpus censuses (D81 mesh, D85 opt-in mesh, D90 texture) wrote JSON only; the
Bevy browser said "Corpus totals remain unknown". How can the viewers show them?

## What changed

- `lbp_catalog::census_summary` (new, UI-free): reads the project's own census JSON
  read-only, refuses files over 64 MiB before reading (`MAX_CENSUS_FILE_BYTES`, the
  01.33 texture census is ~12 MB), then validates strictly and builds display lines.
  No game data is read and no counts are in source; every number shown comes from the
  files given on the command line.
- Accepted inputs (anything else is rejected with an explicit message):
  - `tool` = `corpus_mesh_census`, `schema_version` 1 (default options; no
    `preview_options`; scope `corpus_all_mesh_map_rows` or `cohort_manifest:NAME`)
    or 2 (opt-in; scope suffix `+animation_base`, `+instance_rgba` or both, which must
    agree with `preview_options`; `verified_game_instance` must be false).
    `asset_unit` must be `distinct_catalog_guid`. Totals must satisfy
    Complete <= all-diffuse <= viewable <= catalog GUIDs and hash counts within
    `mesh_hashes`.
  - `tool` = `corpus_texture_census`, `schema_version` 1, scope
    `corpus_all_texture_map_rows`, units `distinct_catalog_guid` + `distinct_texture_hash`,
    `by_outcome` keys from the schema's eight outcomes, outcome hashes summing to
    `totals.texture_hashes`.
  - Unknown tool / schema / scope / unit / outcome, missing fields and inconsistent
    totals are errors; the viewer exits with the message at startup.
- Large per-asset arrays (`meshes`, `textures`, GUID lists) are skipped by the
  deserializer; only summary fields are kept.

## Viewer flags (both viewers)

```
--mesh-census FILE       repeatable; one file per mode (default, animation base,
                         instance color, both); the mode comes from the file's scope
--texture-census FILE    one corpus_texture_census file
--census-baseline FILE   an earlier mesh (per mode) or texture census to diff against
--print-census           Bevy viewer only: print the panel text and exit (no window,
                         no snapshot needed)
```

A second file for the same mode or the wrong tool for a flag is an error.

## What is shown

Bevy viewer: window "Corpus counts (census files)" (open by default, scrollable);
the browser's source-session note now points to it. Legacy viewer: collapsible
"Corpus counts (census files)" at the end of the snapshot overview. Same text in both:

- Per mesh mode: viewable / all-diffuse / Complete (visually verified) "of N mesh
  GUIDs"; unit (distinct catalog GUID, with content hashes, failed loads and excluded
  unmapped hashes); scope string and scope note from the file; file name, tool and
  schema; supplied instance RGBA with "not a verified game instance".
- Baseline: the `--census-baseline` file of the same mode (deltas, or "scope or GUID
  total differs; no delta computed"); otherwise, for opt-in modes, the default-options
  census given in the same run; otherwise "none supplied".
- The file's own definitions of viewable / all_diffuse / complete.
- Texture: decoded "of N texture hashes (of M GUIDs)", unsupported (always shown) and
  any other nonzero outcome; units; mixed-outcome GUIDs and excluded unmapped hashes;
  the file's `decoded` definition; scope, file, schema and milestone; baseline.
- No file: "Not measured: corpus mesh/texture totals unknown (pass --... FILE)"; a
  missing mode when others are given: "Not measured for this mode; totals unknown".
- The census never certifies Complete; the files record 0 and the panel shows that.

Headless output with the real 01.33 files (`b90-corpus-*.json`,
`d90-texture-census-0133.json`) matches STATUS-CURRENT: 4,257 viewable of 4,261;
all-diffuse 3,669 default, 3,721 animation base (+52), 3,899 instance color (+230),
3,967 both (+298); Complete 0; textures 22,973 of 23,247 hashes decoded (23,195 of
23,477 GUIDs), 274 unsupported (282 GUIDs).

D93 changes display only: +0 newly viewable / +0 newly Complete.

## Verification

```
cargo fmt --all --check                                                    -> clean
cargo test -q -p lbp-catalog -p lbp-bevy-viewer -p lbp-viewer              -> pass (8 new census_summary tests)
cargo clippy -q -p lbp-catalog -p lbp-bevy-viewer -p lbp-viewer --all-targets -- -D warnings -> clean
cargo build -q -p lbp-bevy-viewer -p lbp-viewer                            -> ok
LBP_CENSUS_FILES="<5 local census files>" cargo test -q -p lbp-catalog local_census -- --nocapture -> parses all 5
lbp-bevy-viewer --mesh-census ...default.json --mesh-census ...both.json --texture-census ...d90... --census-baseline ...default.json --print-census -> counts above, exit 0
lbp-bevy-viewer --print-census                                              -> "Not measured ... unknown" lines
lbp-bevy-viewer --mesh-census d90-texture-census-0133.json --print-census   -> rejected, exit 1
```

The `local_census_files_parse_when_present` test is a no-op unless `LBP_CENSUS_FILES`
is set (`;`-separated paths); census files stay under ignored `local-analysis/`.
The legacy viewer's two diagnostics tests need an existing `local-analysis/` folder
in the checkout (pre-existing; they fail in a fresh worktree without it).

## Not tested

- No native window was opened: the Bevy window and the legacy overview section were
  not viewed (layout, scrolling, overlap with the asset-preview window unverified).
- Legacy viewer has no headless census print; its flag parsing shares the tested
  library code only.
- Cohort-scope (`--hashes`) mesh census files were tested with synthetic JSON only.
- No in-app way to load census files after startup (command line only).
