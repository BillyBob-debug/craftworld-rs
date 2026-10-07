# D45 static diffuse UV scale/offset — 2026-10-06

D48 extends sampling to bounded static subtype-one matrices; see
[matrix-preview.md](matrix-preview.md). References below to unsupported matrices
describe the original D45 scope. SamplingTransform now retains either four
scale/offset bits or six row-major matrix bits.

Direct, unanimated subtype-zero diffuse texture samplers now accept finite bounded
scale and offset on UV0 or UV1. Sampling uses component-wise `raw UV * scale +
offset`. Negative scale reflects a channel; zero scale produces a constant
coordinate. Identity axes copy raw bits, including signed zero. No V flip, channel
fallback or mesh-coordinate mutation is introduced.

## Rule provenance, applicability and conflicts

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [MaterialBox sampler constants/constructor](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java):
  parameters0/1 are scale X/Y, parameters2/3 are offset X/Y; values are raw
  IEEE-754 float bits. Existing independent graph reader supplies these fields.
  Coverage: raw parameter-bit comparison against both existing external datasets;
  finite/nonfinite boundary tests.
- `CONFIRMED_FROM_COMMUNITY_CODE`: same pin,
  [GfxAssembler TEXTURE_SAMPLE, unanimated subtype-zero branch](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  static scale precedes additive offset. This is independently expressed arithmetic,
  not copied/adapted shader assembly. Animated branches use other conventions;
  those branches and subtype-one matrices remain unsupported. No wired sampler
  input is accepted. Coverage: authored asymmetric order/reflection/constant cases,
  raw UV1 preservation, authored native quadrant fixture.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, established clear ordinary GMT
  heads920–951/branch0. Existing32 and48 external material records produce zero
  route/channel/raw scale-offset bit mismatches. Eligibility increases15→16 and
  14→16 respectively; sets may overlap. Newly eligible transforms include
  `(1,0.5)` scale and 2×/3× scaling. These counts establish selection evidence,
  not renderability of every linked mesh or texture.
- `CONFIRMED_FROM_LOCAL_DATA`: Toro mask GUID00002635 displays its61-triangle
  UV0 strap with2× scaling; Clive GUID00016c87 has50/16-triangle UV1 groups using
  `(1,0.5)` scaling. Other unsupported groups stay neutral. Native captures and
  detailed diagnostics remain private under ignored local-analysis.
- `UNKNOWN`: exact original GPU rounding, original pose, lighting, alpha and shader
  fidelity. The community assembler formats floats for shader text; this preview
  uses full decoded f32 values with separate multiplication/addition. No original
  renderer or generated-shader pixel parity is claimed. Nonzero offset is covered
  by authored checks, not a confirmed local game-material render comparison.

## Authored bounds and ownership

The preview policy rejects nonfinite parameters or magnitudes above1,000,000;
nonidentity derived coordinates must also be finite with magnitude at most1,000,000.
These are diagnostic bounds, not game-format limits. Unchanged identity coordinates
retain the previous finite-coordinate behavior. At most100,000 pairs are allocated;
cancellation is checked every256 vertices and before returning. Overflow, budget or
missing-channel outcomes leave that material binding unsupported without removing
the parent geometry. Existing descriptor, graph, source-stability and eight-binding
budgets remain unchanged.

`Asset.uv` retains selected raw coordinates; `Asset.sampling_uv` is a separate
derived array. `DiffuseSelection.uv_transform.bits` retains four raw parameter bits.
The Bevy and eframe display adapters consume sampling_uv and label the transform.
CPU mesh/UV metadata and geometry exports remain unchanged. The additional array
costs at most800,000 bytes per binding (at most6.4MB for eight bindings).
No parser/UI dependency or new disk layout is added.

## Verification and use

Focused preview/Bevy/eframe tests cover selection, affine order, negative/zero scale,
signed-zero identity, UV1, raw preservation, finite/overflow/length/cancellation
guards, existing material failures and renderer correspondence. The known floor
geometry export retains SHA256
`636b78a5cbf37ec2650069d2c0f4a553f37783008a403919fb8a06d000460be1`.

Use the existing clickable launcher and browse to GUID00002635 or00016c87.
Transform diagnostics appear in the scrollable primitive status area.
`lbp-bevy-viewer --synthetic-affine` is an authored native fixture: left raw
quadrants, middle Repeat with2× scale and offset `(0.25,0.25)`, right Clamp with
horizontal reflection `(-1,1)` scale and `(1,0)` offset. Original raw arrays are
unchanged. Exact commands/results and capture filenames: latest STATUS.md.

No decryption/decompilation needed, recovered code used, or proprietary bytes/keys
bundled. Pinned community source is behavior evidence, not implementation source;
no strict clean-room-process claim. Animation/matrix/general graph work remains
deferred. Next gather evidence for one common unsupported diffuse route before
adding further operations.
