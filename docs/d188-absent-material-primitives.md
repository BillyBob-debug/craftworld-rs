# D188: standalone preview defaults and material-less primitives

Date: 2026-10-09 (America/Chicago). Two parts:

- **Part 1 (research, below):** what an absent material identifier means. Outcome:
  game behaviour `UNKNOWN`; the descriptor shape is now confirmed on local data.
- **Part 2 (implementation, "D188 implementation" section):** (A) single-mesh preview
  and mesh censuses default to a white instance colour and animation base values;
  (B) material-less groups are classified, tallied and excluded from the all-diffuse
  denominator, without inventing a diffuse binding.

## Question

On 01.33, 58 mesh catalog GUIDs miss "all-diffuse" only because some nonempty
primitive group has no material identifier at all (`Primitive.material_kind == 0`,
so `material_guid` and `material_hash` are both `None`; `linked::resolve` returns
`Unresolved: absent resource identifier`). What does the game, or a pinned open
reader, do with such a primitive: draw it with a default material (which one), take
a material from the owning Thing / `PRenderMesh`, or not draw it?

## Sources checked

Pin for all Toolkit links: `ennuo/toolkit` `d6a2ef80b5a09999f08f23a803b516096055e5c9`
(read only; nothing copied or translated).

1. Repository docs: `docs/formats.md`, `docs/mesh-geometry.md`,
   `docs/material-navigation.md`, `docs/primitive-groups.md`. They record the
   primitive descriptor layout only; no rule for an absent descriptor.
2. [`cwlib/structs/mesh/Primitive.java`](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/mesh/Primitive.java)
   `serialize` (lines 72-92): `material` is a plain `GFX_MATERIAL` resource
   descriptor that may be null; the field comment (lines 17-20) says only "RGfxMaterial
   used to texture this primitive". `region` (lines 49-53) is the costume-hideable
   sub-mesh id. No default, fallback or "invisible" meaning is attached to a null
   material. `CONFIRMED_FROM_COMMUNITY_CODE` (layout only).
3. [`cwlib/resources/RMesh.java`](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java)
   `getPrimitives` (line 718), `getSubmeshes` (lines 1106-1130, groups by region,
   keeps every primitive), `regionIDsToHide`/`cullBones` (lines 222-228, 476-477).
   No primitive is dropped for a null material.
4. [`cwlib/structs/things/parts/PRenderMesh.java`](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/things/parts/PRenderMesh.java)
   `serialize` (lines 84-134): mesh, bone things, animation, editor colour, shadow
   type, RTT flag, visibility flags, poppet render scale, parent distances. **No
   material descriptor**, so there is no Thing-level material for a primitive to
   inherit from this part. `CONFIRMED_FROM_COMMUNITY_CODE`.
5. [`cwlib/io/exports/MeshExporter.java`](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/exports/MeshExporter.java)
   `GLB.FromMesh(RMesh)` (lines 608-800; material branch lines 751-777): every
   primitive of every region is exported. When `primitive.getMaterial() == null`
   the branch is skipped, `setMaterial` is never called, and the glTF primitive has
   no material index (glTF default material: white, no texture). The local
   `"DIFFUSE"` name is assigned but unused in that case. Same pattern for
   `RStaticMesh` (lines 556-582). Unresolvable non-null descriptors get a
   texture-less white material via `createMaterial(String)` (lines 1190-1205).
   `CONFIRMED_FROM_COMMUNITY_CODE` **for Toolkit export behaviour only**.
6. Other open readers listed in `docs/community-projects.md`: no mesh renderer with a
   null-material rule was found; a web search returned nothing LBP-specific.
   `ennuo/alear` (game patch project, no visible licence) was not inspected in
   detail. Executable investigation was not done in this milestone.

## Evidence label and rule

- Game behaviour for a null-material primitive: **`UNKNOWN`**.
- "Not drawn" (collision/occlusion/invisible): **no supporting source**; the only open
  consumer (Toolkit export) keeps and emits the geometry.
- "Inherit the Thing's material": **contradicted for `PRenderMesh`**, which carries no
  material (`CONFIRMED_FROM_COMMUNITY_CODE`). Other parts were not audited.
- "Default gmat GUID": no source names one.
- Toolkit converts such a primitive to geometry with the glTF default material. That
  is an exporter choice, not evidence of what the game draws, so it is not adopted as a
  diffuse rule: counting these groups as diffuse-bound would raise "all-diffuse" from
  a converter convention, against the Complete definition (all textures showing).

Part 1 alone changed no code. Part 2 below keeps the group neutral (no diffuse is
invented) but stops counting it against all-diffuse.

## Local descriptor shape (PC, 01.33, all 4,261 mesh GUIDs)

PC probe over the 59 mesh GUIDs whose blocker key was `none` (temporary example,
deleted; numbers from the PC batch report, not re-measured here): every one has
exactly one nonempty primitive with no material GUID and no hash, and that primitive
has `material_flags=0`, `material_kind=0`, `alternatives_flags=0`,
`alternatives_kind=0`, no alternatives GUID/hash. It is primitive 0 in 58 meshes and
primitive 1 in `00017471`; index counts 4-1164. All 59 are viewable; 58 have it as
their only blocked group. Label: **`CONFIRMED_FROM_LOCAL_DATA`** (descriptor shape).

## D188 implementation

### A. Standalone defaults (single-mesh preview and mesh censuses)

When no Thing supplies inputs (single-mesh preview in `lbp-bevy-viewer`,
`corpus_mesh_census`, `compatibility_census`):

| Input | Default | Evidence |
|---|---|---|
| Instance colour (THING_COLOR) | white linear (1, 1, 1, 1) | Value: `CONFIRMED_FROM_LOCAL_DATA`, instance_color_census found editorColor `ffffffff` on 65,746 / 68,618 placed RENDER_MESH Things in levels and 20,753 / 21,571 in plans (docs/instance-color-census.md); `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit `PRenderMesh.editorColor` default -1 (docs/d80-thing-color.md row E7). Link editorColor -> THING_COLOR shader input: **`INFERRED`** (no capture shows it). |
| Animated material parameters | stored base values, t = base (existing D61 snapshot) | Preview policy; game animation time `UNKNOWN`. |

- Code: `lbp_preview::StandaloneDefaults::apply` (crates/lbp-preview/src/instance.rs)
  fills only inputs the caller left unset and returns `AppliedDefaults`.
  `PreviewOptions::default()` is unchanged (source-only), so level scenes and every
  other caller keep their output.
- Flags (`lbp_preview::census::apply_preview_flag`, shared by both censuses):
  `--instance-rgba R,G,B,A` overrides the white default; `--animation-base` still
  accepted (explicit); `--no-instance-default` and `--no-animation-base` opt out.
  Repeats and contradictions (`--instance-rgba` with `--no-instance-default`,
  `--animation-base` with `--no-animation-base`) are errors.
- Viewer (`lbp-bevy-viewer`): the same defaults for single-mesh loads only (levels
  keep explicit inputs). `--no-instance-default` / `--no-animation-base` and two
  checkboxes turn them off. Defaulted inputs are labelled in the details list
  ("D188 default: ...") and the review panel; as before, an instance input or an
  animation snapshot blocks the Complete check. `lbp-viewer` loads a source-only
  single-primitive slice and has no instance or animation inputs, so it is unchanged
  (it shows the new census lines through `census_summary`).

### B. Material-less groups

- Predicate `lbp_preview::is_materialless` (crates/lbp-preview/src/lib.rs): material
  and alternatives descriptors entirely absent (no GUID, no hash, kind 0, flags 0).
  `load_mesh_groups_with_options` gives such a group the neutral reason
  `MATERIALLESS_REASON` ("Material-less: no material descriptor (game draw UNKNOWN;
  drawn neutral, no diffuse invented)") instead of
  `Unresolved: absent resource identifier`. No binding is invented. Any other
  unresolved descriptor (GUID/hash not found, nonzero flags or alternatives) is
  unchanged and still blocks.
- Census (`lbp_preview::census::GroupBinding::Materialless`): such a group is neither
  supported nor blocking. `MeshCoverage.nonempty_groups` excludes it;
  `materialless_groups` counts it. All-diffuse now needs at least one nonempty
  non-material-less group, all bound; a mesh whose groups are all material-less is
  counted in `viewable_only_materialless_hashes`, not all-diffuse.
- Labels: descriptor shape `CONFIRMED_FROM_LOCAL_DATA`; what the game draws
  `UNKNOWN`; Toolkit exports them with the glTF default white material
  `CONFIRMED_FROM_COMMUNITY_CODE` (export behaviour only, not adopted as a diffuse).
- Viewer: drawn neutral (unchanged geometry path), labelled "Neutral: Material-less:
  ...", counted outside the "diffuse groups supported" total, and Complete stays
  unconfirmable while any material-less group is present (game draw unknown).

### corpus_mesh_census schema 3

Every run now writes `schema_version` 3 (meaning changed: defaults and material-less
exclusion). Changes from schema 2:

- `scope` = `corpus_all_mesh_map_rows` or `cohort_manifest:NAME`, then in order:
  `+animation_base` (flag) or `+animation_base_default`; `+instance_rgba` (flag) or
  `+instance_white_default`; `+materialless_blocking` (see below). Default run:
  `corpus_all_mesh_map_rows+animation_base_default+instance_white_default`.
- `preview_options` is always present: `animation_base_enabled`,
  `animation_base_source` (`default` / `flag` / `off`), `animation_time` (null),
  `supplied_instance_color_bits`, `supplied_instance_rgba` (effective value, white
  when defaulted), `instance_color_source` (`default` / `flag` / `off`),
  `verified_game_instance` (false).
- `evidence`: the labels above (`instance_white_value`,
  `editor_color_to_thing_color`, `animation_base`, `materialless_group`).
- `materialless_policy`: `excluded (D188)` or `blocking (pre-D188 ...)`.
- `definitions.all_diffuse` reworded; `definitions.materialless_group` added.
- `totals` add `materialless_groups`, `materialless_hashes`,
  `viewable_only_materialless_hashes`; mesh rows add `materialless_groups`;
  material-less groups are never in `blocked_groups` / `blocking_materials`.
- `--materialless-blocking` (census only) restores the pre-D188 classification, so
  `--no-instance-default --no-animation-base --materialless-blocking` reproduces the
  pre-D188 counts (reason text for those groups is the new one).
- `lbp_catalog::census_summary` reads schemas 1-3. A schema 3 file with both defaults
  goes in the new `D188 standalone defaults` slot; an opt-out file with no inputs is
  the `default options` slot; mixed files go to the slot of their effective inputs
  and say which input was defaulted. Notes name the white default and its `INFERRED`
  link, the animation default and the material-less exclusion count.

`compatibility_census` (schema 13 -> 14): same flags and defaults; adds
`instance_white_default_applied`, `animation_base_default_applied`, `evidence`; group
status `materialless` with `materialless_groups` per row, excluded from all-diffuse.

### PC commands (01.33)

```powershell
cargo build -q --release -p lbp-preview --example corpus_mesh_census
$c = "target\release\examples\corpus_mesh_census.exe"
$g = "game-data\a1-disc;game-data\a1-update-0133"
& $c local-analysis\d68-inventory-0133-verified.json $g local-analysis\d188-corpus-default.json --blockers
& $c local-analysis\d68-inventory-0133-verified.json $g local-analysis\d188-corpus-sourceonly.json --no-instance-default --no-animation-base --blockers
& $c local-analysis\d68-inventory-0133-verified.json $g local-analysis\d188-corpus-pre-d188.json --no-instance-default --no-animation-base --materialless-blocking
python scripts\summarize_blockers.py local-analysis\d188-corpus-default.json
```

Expected (expectations from the PC batch numbers, not measured here):

- `d188-corpus-default.json`: 4,259 viewable; all-diffuse about 4,155 + up to 58
  (the white + animation-base run gave 4,155, and 58 meshes had the material-less
  group as their only blocker in the both-opt-in run; the 59th has another blocker),
  so at most 4,213; `materialless_groups` 59; no `none` key in `blocking_materials`;
  0 Complete.
- `d188-corpus-pre-d188.json`: exactly the pre-D188 default, 4,259 viewable /
  3,798 all-diffuse.
- `d188-corpus-sourceonly.json`: 3,798 + the meshes whose only source-only blocker
  was the material-less group (not measured; at most 59).

## Not tested / open (D188 implementation)

- No game data in this container: no PC census, viewer capture or count above was
  run here. Tests are synthetic (flag parsing, defaults, predicate, coverage, tally,
  census_summary schema 3, viewer evidence).
- Whether the game draws material-less primitives, and with what, stays `UNKNOWN`
  (RPCS3 look not done). If they turn out to be hidden, nothing changes in
  all-diffuse; if they draw with some material, the exclusion must be revisited.
- The editorColor -> THING_COLOR link remains `INFERRED`.

## Not tested / open (Part 1)


- No game data in this container; the 58 GUIDs and their primitives were not inspected.
- Not checked whether these primitives share a `region` listed in some costume's
  `regionIDsToHide`, have a nonzero `alternatives` descriptor, or sit in skinned/
  softbody meshes; any of these could point to a gameplay meaning.
- RPCS3 observation (whether the region visibly draws in-game) not done.

## PC verification (Part 1, superseded by the D188 implementation commands above)

Part 1 alone changed no code, so at that point the census had to reproduce the baseline exactly:

```powershell
cargo build -q --release -p lbp-preview --example corpus_mesh_census
corpus_mesh_census SNAPSHOT "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d188-corpus-default.json --blockers
corpus_mesh_census SNAPSHOT "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d188-corpus-both.json --animation-base --instance-rgba 0.25,0.5,0.75,0.5 --blockers
```

Expected: 4,259 viewable; all-diffuse 3,798 (default) and 4,155 (both), i.e. **no
change**; the absent-identifier blocker still covers the same 58 GUIDs.

To move the question forward, for each of the 10 example meshes record per primitive:
`material_kind`, `alternatives_kind`/flags, `region`, index count, vertex range, and
whether the mesh has `regionIDsToHide` / costume data or is a sackboy/costume piece
(`mesh_group_probe` covers most fields). If all absent-material primitives turn out to
be, for example, costume-hideable regions or zero-area geometry, record that as
`CONFIRMED_FROM_LOCAL_DATA` and reopen; an in-game RPCS3 look at one example would
settle "drawn vs not drawn".
