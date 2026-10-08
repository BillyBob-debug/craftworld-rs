# D119 level backdrop in the Bevy level scene

2026-10-08 America/Chicago. Level mode (D113) now draws the level's backdrop: the
prebaked STATIC_MESH named by the backdrop Thing and the RENDER_MESH Things of the
backdrop plan. New readers: `lbp_mesh::static_mesh` (SMHb) and `lbp_level::plan`
(RPlan Thing array, including the four pre-0x341 backdrop plans).

## How a level references its backdrop

Sources: ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9 (local clean checkout,
grep only; nothing copied or translated).

| Rule | Toolkit location | Evidence | Applicability |
|---|---|---|---|
| PWorld `backdrop`, `backdropNew` = reference(Thing) after `selections`; `backdropPlan`, `backdropNewPlan` = plan descriptors after GlobalAudioSettings (head >= 0x321) | `structs/things/parts/PWorld.java` serialize | CONFIRMED_FROM_COMMUNITY_CODE; CONFIRMED_FROM_LOCAL_DATA | LBP2 band (subVersion 0; the backdrop offset fields are subVersion-gated, absent) |
| Below 0x321 the plan sits in a PRef on the backdrop Thing; at >= 0x321 Toolkit moves it to `backdropPlan` | `resources/RLevel.java` serialize (backdropPlan / PRef shuffle) | CONFIRMED_FROM_COMMUNITY_CODE | explains why no in-band level has a REF part (D107) |
| PLevelSettings `backdropMesh` = resource(STATIC_MESH) after `backdropAmbience` (head >= 0x2f3) | `structs/things/parts/PLevelSettings.java` | CONFIRMED_FROM_COMMUNITY_CODE; CONFIRMED_FROM_LOCAL_DATA | all heads in band |
| RPlan: i32 plan revision (ignored), bytearray thingData, then InventoryItemDetails; thingData = array(Thing, reference) with a fresh serializer at the resource revision/flags | `resources/RPlan.java` serialize, getThings | CONFIRMED_FROM_COMMUNITY_CODE; LOCAL_DATA parity below | plans 0x335..0x3f8 |
| Thing header below 0x341: isStamping bool (> 0x21a), planGUID (>= 0x254), hidden bool (>= 0x2f2), no flags byte | `structs/things/Thing.java` serialize (`THING_FLAGS` 0x341) | CONFIRMED_FROM_COMMUNITY_CODE; LOCAL_DATA parity | plans only (RLevel still refuses < 0x341) |
| PPos below 0x341: localPosition m44 before worldPosition | `parts/PPos.java` | same | plans |
| PGroup below 0x341: copyright bool, creator, plan desc, editable bool, emitter, lifetime, aliveFrames, pickupAllMembers bool, mainSelectableObject bool (>= 0x30f), no u8 flags | `parts/PGroup.java` | same | plans |
| PMetadata (head > 0x2ba): 4 u32 keys, primaryIndex, fluffCost, type, subType i32, creationDate u32, icon res, reference(PhotoMetadata), referencable, allowEmit bool | `parts/PMetadata.java`, `structs/inventory/PhotoMetadata.java`, `PhotoUser.java` | same | plans 0x335..0x3f8 (new SUPPORTED part) |
| SMHb: magic, head, dependency offset; no branch/flags/compressed bytes; StaticMeshInfo (fixed BE: 3 lightmap descriptors, primitive/node counts, index/vertex stream sizes, primitives, nodes, i32 "HELP") then chunk-compressed payload ending at the dependency table | `types/SerializedResource.java` process; `structs/staticmesh/StaticMeshInfo.java`, `StaticPrimitive.java` | CONFIRMED_FROM_COMMUNITY_CODE; CONFIRMED_FROM_LOCAL_DATA | heads 0x191..0x3f8 |
| Payload: vertices (stride 0x20: v3, packed normal, f16 UV0, packed tangent, f16 UV1, smooth normal) then u16 BE indices; primitive type 5 lists, 6 strips with 0xFFFF restart (odd triangles flipped) | `resources/RStaticMesh.java` fetchVertexData, getTriangles | same | same |
| Primitive indices are relative to `vertexStart` | `io/exports/MeshExporter.java` GLB.FromMesh accessor offsets, OBJ export `+ vertexStart` | CONFIRMED_FROM_COMMUNITY_CODE; LOCAL_DATA (bounds check) | same |

Strictness vs Toolkit: the "HELP" marker is required (Toolkit ignores it); count
mismatches, stream-size/payload mismatches, out-of-range vertices, restart indices in
lists and unknown primitive types are explicit errors. Normals, tangents and the three
lightmap textures are not interpreted.

## Local data (01.33, snapshot d68-inventory-0133-verified.json)

Unit: distinct level content hash, the 236 levels the D107 reader completes.

| Field | Levels |
|---|---:|
| PWorld backdrop Thing set | 235 (1 null) |
| backdrop Thing planGUID == PWorld backdropPlan | 235 |
| backdropMesh set on the backdrop Thing | 185 (13 distinct SMH) |
| backdropNew Thing / plan set | 0 |
| backdrop Thing parts: POS+LEVEL_SETTINGS / +GROUP / BODY+POS+LEVEL_SETTINGS+GROUP | 127 / 107 / 1 |

The plan's other Things are not in the level (the backdrop Thing has no children); they
come from the plan resource. 42 distinct backdrop plans: heads 0x335 (1), 0x336 (3),
0x397..0x3f6 (38). All 42 now read completely with 0 trailing thingData bytes (6,427
Things). Typical pattern: a baked SMH plus a small plan with lights and animated props
(e.g. rookie_test: SMH 00015317 + plan 000195ef with 43 Things, 12 RENDER_MESH);
51 levels have no backdropMesh and draw the plan's props only (up to 771 RENDER_MESH).

Static meshes: all 27 local SMH resources decode with stream sizes matching the payload,
every primitive triangulates, and every used vertex lies inside its primitive's stored
min/max box (27/27 `decoded_consistent`; 39k..589k vertices, 409..1,989 primitives,
heads 0x335..0x3f4).

Toolkit parity (new authored oracle `scripts/PlanThingOracle.java`, runner
`scripts/run-plan-thing-oracle.ps1`, manifest `scripts/plan_thing_manifest.py`): 42/42
plans `parsed_exact`; `plan_parity` compares per Thing in tag order (UID, planGUID, parts,
POS world bit-exact, mesh/anim GUID, bone count, backdropMesh, consumed bytes): 42 agree,
0 differ, 6,427 Things. No Toolkit oracle covers SMH payloads; the bounds check is the
local evidence.

## Scene (AGENTS rule 5)

`lbp_preview::level_backdrop` (no Bevy): `backdrop_refs`, `add_backdrop_mesh` (primitives
merged per gfx material into batches within the 100,000-vertex / 50,000-triangle adapter
limits, vertices compacted, placed at the backdrop Thing's POS matrix),
`add_backdrop_plan` (plan Things through `build_scene`, placed by backdrop world x
inverse(plan settings-Thing world) x Thing world; identity on the tested levels),
`BackdropCounts` (shown in the panel and QA log via `SceneCounts::lines`). Materials load
with `linked::load_material` and the new `UvInput::Static` (stored UV0 and UV1).
`MeshSource::Backdrop` / `ScenePayload::Backdrop` are the new variants. Viewer change:
one payload arm, backdrop items counted separately, and the home view frames the level
items without the backdrop (the backdrop spans about 20x the level; it stays inside the
far plane at the default zoom). Skipped/fallback counts are per primitive or per plan
Thing with reasons. Plan SHAPE Things are counted, not drawn.

## Counts (headless `--level-report`)

| | tutorial_23 (0001db4f) | main_01_rookie_test |
|---|---:|---:|
| backdrop source | Thing 9, SMH 000190ca, plan 000195eb | Thing 1173, SMH 00015317, plan 000195ef |
| SMH primitives drawn / total | 409 / 409 | 1,310 / 1,310 |
| SMH triangles drawn | 33,755 | 102,750 |
| material batches | 75 | 203 |
| primitives in flat fallback colour | 0 | 4 (animated sampler) |
| plan Things / RENDER_MESH / drawn | 2 / 0 / 0 | 43 / 12 / 12 (6 meshes, 0 failed) |
| backdrop scene items | 75 | 215 |
| draw entities (whole level) | 114 (was 39) | 933 (was 684) |

Level counts (RENDER_MESH, shapes) are unchanged from D113/D114. Also run:
side_01_tower_of_whoop (plan-only backdrop 000178f7): 257 plan items drawn, 0 fallback.

Newly viewable / Complete: +0 newly viewable and +0 newly Complete catalog assets
(counting unit: catalog GUID; the 27 SMH resources are now decodable geometry but not a
browser route). Level-scene unit: 2 tested levels now show their backdrop; 0 levels
Complete (bones, bevels, decorations, lighting missing; visual check pending).

## Commands

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check                                                     # ok
cargo test -q -p lbp-level -p lbp-mesh -p lbp-preview -p lbp-bevy-viewer   # all ok
cargo clippy -q -p lbp-level -p lbp-mesh -p lbp-preview -p lbp-bevy-viewer --all-targets -- -D warnings   # ok
cargo build -q -p lbp-bevy-viewer -p lbp-viewer                             # ok
target/debug/examples/backdrop_census SNAPSHOT GAMEDIRS local-analysis/d119-backdrop-census-5.json
target/debug/examples/static_mesh_probe SNAPSHOT GAMEDIRS local-analysis/d119-static-mesh-probe.json   # 27 decoded_consistent
python scripts/plan_thing_manifest.py SNAPSHOT local-analysis/d119-backdrop-census-5.json local-analysis/d119-plan-manifest.json
./scripts/run-plan-thing-oracle.ps1 -Output local-analysis/d119-plan-thing-oracle.json -SampleManifest local-analysis/d119-plan-manifest.json   # 42 parsed_exact
target/debug/examples/plan_parity SNAPSHOT GAMEDIRS local-analysis/d119-plan-thing-oracle.json   # 42 agree, 6427 Things
target/release/examples/level_census ... local-analysis/d119-level-census-0133-b.json   # identical to d107-level-census-0133.json
target/release/examples/level_parity ... local-analysis/d119-level-parity-0133.json     # 236/236 summary, 4180/4180 scene Things
target/debug/lbp-bevy-viewer.exe --snapshot SNAPSHOT --game-dir GAMEDIRS --level-path lbp2/th_tutorials/levels/tutorial_23.bin --level-report
target/debug/lbp-bevy-viewer.exe --snapshot SNAPSHOT --game-dir GAMEDIRS --level-path main_01_rookie_test/main_01_rookie_test.bin --level-report
```

(SNAPSHOT = `local-analysis/d68-inventory-0133-verified.json`, GAMEDIRS =
`"game-data/a1-disc;game-data/a1-update-0133"`; the examples run from the checkout root.)

## Not tested

No native window or capture (Integrator). Visual placement of the backdrop relative to the
level, the front-side choice, whether the plan placement formula matters for non-identity
backdrop matrices (none locally), lightmap/rise/fall maps, backdrop animation and
`backdropNew` swaps are unverified or not implemented. `python -m unittest discover -s
tests`: 36 OK (1 skipped); the new manifest helper has no unit test. Only three levels were run
through `--level-report`; the other 233 levels' backdrops were checked at the reader level
(refs, 13 SMH decode, 42 plans parse), not through material loading.
