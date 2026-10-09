# D188: primitives with an absent material identifier

Date: 2026-10-09 (America/Chicago). Outcome: **findings only, no code change.**

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

No implementation: existing behaviour (`Unresolved: absent resource identifier`,
neutral group, mesh stays viewable but not all-diffuse) is unchanged.

## Not tested / open

- No game data in this container; the 58 GUIDs and their primitives were not inspected.
- Not checked whether these primitives share a `region` listed in some costume's
  `regionIDsToHide`, have a nonzero `alternatives` descriptor, or sit in skinned/
  softbody meshes; any of these could point to a gameplay meaning.
- RPCS3 observation (whether the region visibly draws in-game) not done.

## PC verification (when the 10 example GUIDs arrive)

No code changed, so the census must reproduce the baseline exactly:

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
