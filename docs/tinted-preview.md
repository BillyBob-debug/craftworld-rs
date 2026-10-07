# D46 static texture × color diffuse routes — 2026-10-06

Both frontends now support one MULTIPLY node between the diffuse output and two
inputs: one existing static TEXTURE_SAMPLE and one static unwired COLOR. Either
input order is accepted. Direct texture routes retain their prior behavior.
The app labels the static tint alongside UV selection, tiling and texture extent.
Other outputs remain explicitly ignored; this is a diffuse projection.

## Rules, provenance, applicability and conflicts

Toolkit source remains pinned at
`d6a2ef80b5a09999f08f23a803b516096055e5c9`; no reference source was modified.
Existing independently parsed graphs cover clear PS3 GMT heads 920–951, branch 0,
locally BCUS98245 01.00. These rules add behavior, not a new serialized layout.

| Rule | Evidence and source | Coverage / limits |
|---|---|---|
| OUTPUT/DIFFUSE=0, TEXTURE_SAMPLE=1, COLOR=3, MULTIPLY=10 | `CONFIRMED_FROM_COMMUNITY_CODE`: pinned [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java), [BrdfPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BrdfPort.java) | Synthetic route tests; existing parser/oracle field agreement. |
| Diffuse multiplication takes inputs on ports 0/1 and multiplies their values component-wise | `CONFIRMED_FROM_COMMUNITY_CODE`: pinned [GfxAssembler.resolve, MULTIPLY case](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java) | Both input orders tested; other output ports can have different semantics, particularly BUMP, and are not evaluated. |
| Static COLOR has four IEEE float parameters, indices 0–3 | `CONFIRMED_FROM_COMMUNITY_CODE`: same pinned `GfxAssembler.resolve`, COLOR case | Four newly eligible local material graphs retain exact tint bits. Named/keyed animations stay unsupported. |
| Texture/color/multiply route indices and tint bits | `CONFIRMED_FROM_LOCAL_DATA` / `CONFIRMED_FROM_COMMUNITY_CODE`: external [RGfxMaterial.getWireConnectedToPort/getBoxConnectedToPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java) queried by authored Java adapter | 32 and 48 distinct material sample sets: zero route/channel/transform/tint mismatches; each gains two eligible materials (four new material hashes combined). No shader assembly or game pixel oracle executed. |
| sRGB bytes decode through a linear segment below 0.04045 and a power segment above it | `CONFIRMED_FROM_DOCUMENTATION`: [Khronos glTexImage2D sRGB conversion](https://wikis.khronos.org/opengl/GLAPI/glTexImage2D) | Preview converts RGB to linear, multiplies by tint, then mathematically inverts that transfer function and rounds to nearest 8-bit value. Alpha is linear. This is authored display policy, not established LBP color calibration. |
| Tinting original texture data on these assets | `CONFIRMED_FROM_LOCAL_DATA`: matching before/after source-bound 36-mesh census; native vulture preview outside that baseline | Six additional nonempty primitive groups on three previously viewable sample meshes. Vulture displays a non-white 0.14901961 tint with 4× tiling. |
| Original GPU rounding/filtering, no_srgb calibration, pose/lighting/alpha/full material fidelity | `UNKNOWN` | Community shader text rounds floats for formatting; preview retains full f32 bits. Baking tint into 8-bit display pixels adds quantization and may differ from multiplying after GPU filtering. No exact game-pixel parity or newly complete asset certification. |

The graph traversal/pixel arithmetic is independently expressed from these findings.
No community/recovered source or shader templates were copied/adapted/translated.
The externally executed MIT reader remains an oracle. No strict clean-room claim.
Web search exposed the Khronos conversion rule; direct wiki access returned 403,
and the larger DataFormat HTML exceeded the browsing fetch limit. Pinned local
Toolkit source provided the material behavior evidence without further downloads.

## Bounds and ownership

Authored policy requires one output and exactly one diffuse incoming edge. The
selected multiply has exactly two incoming edges on distinct ports 0/1; all three
selected edges use source port 0 and zero swizzle. Its children must be exactly one
texture and one COLOR; no graph recursion is introduced. Duplicate/extra inputs,
cycles, texture×texture, nested multiplication, animated/ThingColor/Mix paths,
sampler wiring and color wiring stay unsupported. Selected nodes are subtype zero
and unanimated. Existing UV0/UV1/affine/descriptor/source/texture bounds still apply.

Tint components must be finite in [0,1]. HDR/negative colors are explicit unsupported
outcomes, not silently clamped. Raw tint bits remain in DiffuseSelection; raw UVs
and original texture RGBA remain unchanged. Nonidentity color creates an optional
separate display buffer, at most 16 MiB per binding and 128 MiB for eight bindings.
Identity needs no second allocation. RGB conversion uses 3×256 lookup tables;
the pixel pass checks cancellation every 4096 pixels and at its boundaries.
Both renderers consume the same derived pixels. Bevy retains its opaque unlit
policy; full alpha/rendering behavior is not established.

## Asset counts and reproduction

The same pinned D42 selection of 36 distinct mesh hashes maps to 36 catalog GUIDs:

| Measurement | Before D46 | After D46 |
|---|---:|---:|
| Geometrically viewable GUIDs | 36 | 36 |
| GUIDs with every nonempty diffuse group loaded | 15 | 15 |
| Newly visually verified complete GUIDs | 0 | 0 |

**Newly viewable: 0. Newly complete: 0.** Three previously viewable assets gain
six textured groups, measured on this same set:

- Victorian frame, GUID `0001e458`: 1→3 supported nonempty groups out of 4.
- Nurse costume, GUID `000201dc`: 1→3 out of 8.
- Thor head, GUID `000156e7`: 1→3 out of 6.

The vulture costume `00027987`, outside that fixed baseline, also displays its
single newly supported tinted diffuse group (1200 vertices/1713 triangles,
512×512 BC1). It has no before/after corpus-count claim. Fully loaded diffuse
bindings alone do not establish all textures/geometry correctness; the app's
Complete confirmations remain unset. Whole-corpus totals remain unknown.

The new `compatibility_census` example produces versioned metadata reports for up
to 48 mesh hashes, with a 64 MiB aggregate mesh-input limit and existing per-mesh
eight-binding/texture/expansion bounds. It records content hash rows, distinct GUID
sets, neutral reasons and group coverage. It refuses existing/outside-analysis
outputs. The before report was actually run before extending the selector.

```powershell
. ./scripts/use-local-rust.ps1
./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/d46-linked-material-oracle.json -SampleManifest local-analysis/d43-linked-material-samples.json
./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/d46-diffuse-oracle.json
cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d46-linked-material-oracle.json local-analysis/d46-linked-parity.json
cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d46-diffuse-oracle.json local-analysis/d46-diffuse-parity.json
cargo run -p lbp-preview --example compatibility_census -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d42-groups-oracle.json local-analysis/d46-compatibility-after.json
```

These outputs already exist; use new names when repeating. Exact checks/native
command, initial failures and remaining limits are in STATUS.md. Test windows select
the non-primary monitor before showing; one native vulture capture was inspected.
All original game inputs remain read-only; diagnostics stay under local-analysis.
