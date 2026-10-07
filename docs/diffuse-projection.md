> D45 now allows bounded static scale/offset with separate derived sampling UVs;
> the identity-only sampling restriction below is historical. Raw UV preservation,
> channel selection and other route guards remain. See [D45](affine-preview.md).

# D43 diffuse-only graph projection — 2026-10-06

A material can now preview its direct texture-to-diffuse route even when other
nodes/wires are present. This is a bounded diffuse-only projection: other material
outputs and branches are deliberately ignored and visibly counted. It does not
reproduce the complete material. Multiply/mix/color/ThingColor paths, transformed
UVs, sampler inputs, animation and swizzles remain unsupported.

## Evidence and provenance

Pinned Toolkit commit `d6a2ef80b5a09999f08f23a803b516096055e5c9`:

- `CONFIRMED_FROM_COMMUNITY_CODE`: [RGfxMaterial.getOutputBox,
  getWireConnectedToPort, getBoxConnectedToPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java)
  select the output and its incoming wire by output port. BrdfPort.DIFFUSE is zero.
- `CONFIRMED_FROM_COMMUNITY_CODE`: [GfxAssembler.setupPath, TEXTURE_SAMPLE case,
  getWithSwizzle](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  resolves each output path separately. Texture sample parameters retain D40's
  scale/offset/channel/slot interpretation. Subtype-zero sampler inputs can modify
  UVs, so this subset rejects all incoming wires to the selected sampler. Numeric
  offset comparison with zero treats either IEEE zero sign as no offset; Rust
  checks the raw zero magnitude bits and preserves the original parameter bits.
- `CONFIRMED_FROM_COMMUNITY_CODE`: [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java)
  OUTPUT=0/TEXTURE_SAMPLE=1. Other node kinds are not evaluated by this extension.
- `CONFIRMED_FROM_LOCAL_DATA`: the existing 32 sampled GMTs / 232,687 archived
  bytes have 17 direct texture diffuse routes. The selection restrictions accept
  14, including ten graphs with more than two nodes. Fourteen is graph eligibility,
  not a claim that all linked textures or meshes are renderable. All 32 route
  indices/source kinds/texture slots match the pinned reader's methods; graph
  metadata remains identical to D7's established oracle records.

The authored Java adapter stores bounded reader-produced MaterialBox/MaterialWire
objects in an RGfxMaterial container solely to call selection helpers. It does not
deserialize a full resource or invoke shader generation/evaluation. Other existing
external field/animation/truncation guards remain in force. Raw shader bytes and
pixels are not exported by the oracle. Source was inspected as behavior evidence;
no community/recovered shader implementation was copied, adapted or translated.
No decryption/executable research was needed or performed.

Applicability is unchanged: clear ordinary PS3 GMT heads 920–951 / branch zero;
existing mesh/texture variants, descriptor and wrap restrictions still apply.
Original game rendering, lighting, alpha, normal/specular/emission and other branch
semantics remain `UNKNOWN`. The product uses the user's own runtime data only.

## Independent selection policy

`lbp-preview::select_diffuse` requires exactly one output node and exactly one wire
to its diffuse port; that wire must come directly from an unanimated subtype-zero
texture sample, source port zero and zero swizzle. Scale must be exact one, offsets
must be either zero sign, channel must be UV0, and the selected sampler must have
no incoming wires. Output subtype/animation also remain conservatively restricted.
All wire endpoints must be in range, including ignored wires. Existing parser
bounds (1024 boxes / 4096 wires) remain; the public selector checks them as well.

Conflicts: Toolkit chooses the first output/first matching wire when duplicates
exist. This product explicitly rejects that ambiguity. This strict policy is
independent, not evidence that the original game rejects duplicates. No arbitrary
recursive traversal is added. Valid unrelated cycles, unknown nodes, wires and
animations are ignored; their evaluation is outside this projection. Malformed
endpoints still reject the material. Ignored node count excludes only the selected
sample/output; ignored wire count excludes only the direct diffuse wire.

The CPU Asset records selected indices/slot and ignored counts. Both frontends
label the result as diffuse-only and display ignored counts; unsupported groups
retain neutral geometry. Existing candidate resolution, cancellation, source
stability, first-eight distinct material attempts, budgets and GPU lifecycle stay
unchanged. No extra graph branch resource reads are introduced. Geometry metadata
exports remain unchanged and contain no UV arrays, pixels or selection data.

Synthetic tests cover ignored branches/animations/cycles, selected sampler inputs,
duplicate diffuse wires/outputs, missing diffuse wire, malformed endpoints,
nonzero/NaN/infinite offsets, signed zero, unknown/color/multiply/mix sources,
subtype, channel, source port, animation and swizzle rejection. Existing parser,
linked resolver, cache/budget and scene lifecycle checks remain in force.

## Reproduction

```powershell
. ./scripts/use-local-rust.ps1
./scripts/run-material-oracle.ps1 -Diffuse -Output local-analysis/NEW-diffuse-oracle.json
cargo run -p lbp-preview --example diffuse_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/NEW-diffuse-oracle.json local-analysis/NEW-diffuse-parity.json
./target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00013051
```

Use new ignored output names. The parity example compares the route indices/kind/
slot to external helper calls and independently reports which graphs pass the
projection's strict policy. It is not an external shader or pixel oracle. Exact
verification/native results and remaining limits are recorded in STATUS.md.

## Additional controlled evidence and native results

First 48 distinct material GUIDs in D42's mesh primitive order, with strict single
hash/FARC-location selection: 404,629 bytes, 48 matching external routes / zero
mismatches, eight projection-eligible. This sample set may overlap the earlier 32;
no combined unique total or full-corpus coverage is claimed.

Inspected Bevy bottle 00003114: 536 vertices / 531 triangles, 256x256 BC1 Repeat,
three ignored nodes/wires, clearly visible colored cap. Eframe shows the same new
projection and its label. Garage light 00013edb: 431 textured housing triangles /
128x128 BC1 Repeat / one ignored node/wire, plus 28 neutral unsupported center
triangles. Board 00013051 loads one eligible group's texture but its default pose
largely occludes it. Floor still renders; eframe floor JSON is byte-identical to D42.
160 workspace tests, fmt/Clippy and both native builds pass. Manual interaction,
other GPU/platform/corpus and original-game pixel/shader/pose fidelity are untested.

Interactive bottle launch:

```powershell
. ./scripts/use-local-rust.ps1
./target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00003114
```

Left drag orbits, right/middle drag pans, wheel zooms, R resets, F frames. Try
00013edb with Load / reload to inspect the mixed supported/neutral garage light.

## D44 channel extension — 2026-10-06

Historical UV0-only restriction now permits UV1 identity sampling. All other graph/
transform/animation restrictions remain. Selected raw array is copied bit-for-bit,
requested channel must exist, channel>1 rejected, both UIs label original identity.
[Evidence and native results](uv1-preview.md). Affine transforms remain next.
