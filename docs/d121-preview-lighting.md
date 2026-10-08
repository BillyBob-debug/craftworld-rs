# D121: opt-in preview lighting (Bevy viewer)

Date: 2026-10-08 (America/Chicago). Scope: `lbp-bevy-viewer` material/lighting path,
`lbp-level` PLevelSettings field parsing, `lbp-preview::level_scene` carrying the
settings, oracle projection. Off by default: without the flag nothing is applied, so
existing captures and the default `--level-report` output are unchanged.

## What it does

- `--preview-lighting` (CLI) or the panel checkbox "Preview lighting (not game
  lighting)" turns on a Lambert key light plus ambient:
  `factor = ambient + key * max(dot(n, toward_light), 0)`.
- The factor multiplies the finished colour of every material mode in the authored
  shader (`crates/lbp-bevy-viewer/src/dual.rs`): single texture, two-sample product,
  D99 alpha MIX, third/fourth samples (D65/D67/D101), solid COLOR, neutral grey. Their
  colour logic is unchanged; lighting is one multiply after it, before Bevy's
  post-lighting step (fog). With lighting off (`light_dir.w = 0`) the shader path is
  the old one.
- Normals are the viewer's generated diagnostic normals (area-weighted face normals,
  `adapter::build_mesh`), never original game normals.
- Double-sided: the smooth normal is turned into the hemisphere of the screen-derivative
  face normal that points at the viewer, so back faces and mirrored (negative
  determinant) instances light like front faces. CPU mirror: `lighting::facing_normal`.
- Untrusted normals: `adapter::normal_trust` marks a referenced vertex weak when its
  face normals cancel (|sum| <= 1e-3 * sum of magnitudes, e.g. a front and a back face
  sharing the vertex). A mesh with any weak vertex stays unlit and is counted
  ("N of M draw entities unlit"). Synthetic fixtures are never lit (counted the same way).

## Light sources

| Mode | Direction | Key / ambient | Fog |
|---|---|---|---|
| Single mesh, or level without usable settings | generic: toward (-0.45, 0.75, 0.5) normalized (upper left, in front of the side-on view) | 0.65 / 0.35 grey | none |
| Level with PLevelSettings | `normalize(sunPosition * sunPositionScale - bounds centre)` (level units) | `sunColor.rgb * sunMultiplier` / `ambientColor.rgb`, both scaled so the fully lit peak channel is 1 | Bevy `DistanceFog` linear, `fogColor.rgb`, fogNear..fogFar times the scene normalization scale |

Labels: generic = "preview lighting (not game lighting)"; level = "preview lighting from
level settings (approximate, not game shading)". The panel material line and the QA log
(stdout, one block per apply) print the label, the values used, and the unlit count.
Unusable settings (non-finite, negative colour, zero sun vector) fall back to the generic
light with the reason in the log; an unusable fog range drops only the fog.

## PLevelSettings parsing (lbp-level)

`parts::LevelSettings` keeps the stored LevelSettings fields of the PLevelSettings part:
sunPosition v3, sunPositionScale, sunColor v4, ambientColor v4, sunMultiplier, exposure,
fogColor v4, fogNear, fogFar, rimColor v4, rimColor2 v4, plus the preset count. The
in-band layout was already walked (module doc in `parts.rs`); D121 only stores values.

- Source: Toolkit `cwlib/structs/things/components/LevelSettings.java` and
  `structs/things/parts/PLevelSettings.java` at pin
  d6a2ef80b5a09999f08f23a803b516096055e5c9 (`serialize`). Evidence:
  CONFIRMED_FROM_COMMUNITY_CODE for the layout, CONFIRMED_FROM_LOCAL_DATA by oracle
  parity below. Applicability: the reader's LBP2 band (heads 0x341..0x3f8, branch 0;
  the 0x325..0x330 extra floats and LBP3 sub-version fields are outside it).
- Holder: `LevelSettings::find` takes the world Thing's part, else the first Thing with
  one. In the 5 oracle scene levels exactly one Thing holds it: the world Thing in
  pod.bin, a separate POS Thing in the other four.
- Tests: `crates/lbp-level/tests/thing.rs::level_settings_fields_and_presets` (synthetic
  recipe with one preset, every truncation stops); viewer `lighting::tests` (mapping,
  fit, fallback, labels, facing), `adapter::tests::normal_trust_*`,
  `dual::tests::authored_shader_validates_with_every_branch_enabled` (naga 29 parse +
  validation of the authored WGSL with every `#ifdef` enabled and Bevy imports stubbed).

Oracle parity: `scripts/LevelOracle.java` now projects `level_settings` per Thing
(`"absent"` when the Thing has no part; older dumps lack the key and are skipped by
`level_parity`). Scene run on the D106 five-level manifest:

```powershell
scripts/run-level-oracle.ps1 -ProjectRoot <main> -SampleManifest local-analysis/d106-level-manifest-scene.json -Output local-analysis/d121-level-oracle-scene.json   # 5 parsed_exact
target/release/examples/level_parity.exe local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d106-level-oracle-all.json local-analysis/d121-level-oracle-scene.json local-analysis/d121-level-parity.json
# summary 236/236 equal; scene 4,180/4,180 Things all fields equal (5 settings Things, exact f32)
```

## tutorial_23 values used (gamedata/lbp2/th_tutorials/levels/tutorial_23.bin)

sunPosition (0.45429555, 0.42996562, 0.6072852) x 300000; toward light (0.556, 0.472,
0.685); sunColor (1.0104374, 0.7520584, 0.3271594) x sunMultiplier 1.5979382; ambient
(0.5810731, 0.72163016, 0.9099767); fit factor 0.455; exposure 1.0 (not applied); fog
(0.26878607, 0.5019598, 0.79259527) linear 11309.276..21525.771 level units (level span
about 3,700 units, so the home view sits mostly before the fog start); 3 presets
ignored; 0 of 39 draw entities unlit. main_01_rookie_test: 0 of 684 unlit.

Headless check: `lbp-bevy-viewer --snapshot local-analysis/d68-inventory-0133-verified.json
--game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path
gamedata/lbp2/th_tutorials/levels/tutorial_23.bin --level-report --preview-lighting`
prints the lighting block above; without the flag the report has no lighting lines.

## Not game-accurate (by design)

- LBP's lighting model is not reproduced: no PWorld/SPRITE_LIGHT lights, no rim light
  (rimColor/rimColor2 parsed, unused), no exposure, baked shadow/AO, DOF, zEffect, no
  presets/blending, no backdrop lighting, no shadows or specular.
- Whether the game treats sunPosition as a point (scaled) or a direction, and whether
  the colours are linear or sRGB, is UNKNOWN; the mapping above is a preview choice.
- The unit-peak fit and the generic light are preview choices.
- Fog distance meaning (camera distance vs depth, game falloff curve) is UNKNOWN; Bevy
  linear distance fog is used.
- Normals are generated (smooth per shared vertex), not the game's.

## Not tested

- No native window or capture in this milestone (Integrator captures on the second
  monitor). Bevy's runtime shader composition is exercised only when the window runs;
  the naga test covers the authored WGSL with stubbed imports.
- Toggling at run time (checkbox on/off restores materials; fog insert/remove) is not
  exercised by an automated test.
- Oracle parity covers the 5 scene levels only, not all 236 in-band levels.
