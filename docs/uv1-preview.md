> D45 now allows bounded static scale/offset with separate derived sampling UVs;
> the identity-only sampling restriction below is historical. Raw UV preservation,
> channel selection and other route guards remain. See [D45](affine-preview.md).

# D44 direct-diffuse UV1 selection — 2026-10-06

Shared preview preparation now selects the material's UV0 or UV1 channel explicitly.
Requested UV1 must exist; it never falls back to UV0. Selected coordinates are copied
bit-for-bit from the verified CPU arrays, without channel reordering, transformation
or V flipping. Both frontends display the original selected channel. All D43 graph,
sampler/descriptor and D42 group budgets remain in force.

## Per-rule provenance and limits

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`:

- `CONFIRMED_FROM_COMMUNITY_CODE`: [GfxAssembler.TEXTURE_SAMPLE case](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  uses sample parameter4 equal to one to select the second UV pair (`zw`), otherwise
  the first (`xy`). [MaterialBox constants](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java)
  identify the sampler channel/index parameters. This product supports raw selectors
  zero/one only; Toolkit's fallback for other values is not adopted. Intent/meaning
  of other selector values remains `UNKNOWN`, even when a mesh has more UV channels.
- `CONFIRMED_FROM_COMMUNITY_CODE`: [RMesh.getUVs(channel), serialize/init](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java)
  and MemoryInputStream.v2 establish the already verified interleaved pairs. The
  pinned nonzero-start getter bug remains avoided by full-channel reads. No disk
  parser/layout change is introduced by this milestone.
- `CONFIRMED_FROM_LOCAL_DATA`: existing 32 material sample routes/channel selectors
  match the external reader, zero mismatches; 15 graphs eligible (14 UV0 / one UV1).
  Separate first-48 linked sample set also matches, 14 eligible (eight UV0 / six
  UV1). Eligibility is not linked texture/mesh compatibility; the sets may overlap.
- `CONFIRMED_FROM_LOCAL_DATA`: 36 mesh samples / 605145 stored bytes, zero full-channel
  digest mismatches/failures against the pinned reader. All UV metadata is identical
  to D40. Valkyrie has 384 vertices and three UV channels with distinct digests;
  selected UV1 digest is `bfe84ad6717f0ddf2b8a99481891c5252eb80a104978a6c7469bf11f03433b2b`.

Applicability remains clear ordinary GMT heads920–951/branch0 and the existing
bounded mesh/texture contracts. Scale must be exact one and offsets zero (either
sign). Channel selectors >1, affine transforms, sampler inputs, animations/swizzles
and general graph operations remain unsupported. This is a diffuse-only projection,
not complete game material reproduction. Alpha, lighting/pose and pixel equivalence
remain `UNKNOWN`.

No decryption/executable investigation was necessary/performed. Community renderer
source was consulted as behavior evidence; no shader or recovered game code was
copied/adapted/translated. Product includes no original LBP code/assets/keys. Native
assets come only from the user's read-only game data; private oracle/capture outputs
remain ignored. This is not a strict clean-room-process claim.

## CPU and renderer boundary

DiffuseSelection records `uv_channel`. The loader decodes the established complete
CPU UV IR and checks/clones the requested array, retaining all-channel digest
metadata. The temporary original channel arrays are never mutated/reordered;
unused arrays may be released after preparation. Missing channels, count mismatch,
nonfinite selected values and unsupported selector reject explicitly. UV validation
now precedes texture resolution/read/decode, avoiding texture reads for a missing
requested channel. Source stability and final cancellation checks still precede
publication. Existing parsers also reject invalid unused channel data as before.

The renderer uses its usual single diffuse sampling attribute: selected original
UV1 coordinates are supplied to that attribute, with UI/CPU channel identity still
UV1. This does not rename/rewrite original mesh arrays or require a second texture
binding. Shared material cache and GPU draw pooling remain per resolved material.
No parser/UI dependency or new runtime/decryption machinery is introduced. The
copy adds at most 0.8 MB transient UV storage per attempt; D42's input/expansion/pixel
budgets and at most eight material attempts/nine draws remain unchanged.

Synthetic coverage: distinct UV0/UV1 values, negative-zero bit preservation, original
array immutability, missing UV1 with existing UV0 (no fallback), out-of-contract
selector, vertex-count mismatch, nonfinite values; graph predicate also verifies
UV1 acceptance and selector2 rejection. Existing interleaving, extent/truncation,
cancellation, alias/cache/budget and renderer correspondence tests continue to pass.

The private diffuse census envelope is version2 with selected-channel fields;
external helper evidence additionally retains `uv_channel_raw`. Older D43 oracle
reports without that optional field remain accepted, but do not assert new external
channel-field corroboration. The geometry diagnostic export stays unchanged and
contains no coordinates/pixels or selected-channel data. Eframe QA logs display
selected channel separately from complete UV metadata.

## Native results and reproduction

161 workspace tests (74 focused), fmt/Clippy and both native builds pass. Bevy
Valkyrie 00010e8d has 384 vertices / 693 triangles / two textured UV1 groups:
350 triangles with 256x256 BC3 and 343 with 128x128 BC3, Repeat, each ignoring one
node/wire. Inspected capture shows two distinct textured parts with consistent
whole-mesh framing. UFO 00020155 has 880 vertices /1168 triangles/six groups: one
56-triangle UV1 group with 32x256 BC1 is supported; five remain neutral. Stored pose
is untransformed and unrelated branches/alpha/lighting are not reproduced.

Eframe's single-primitive contract remains explicit: Valkyrie Direct diffuse reports
its limit and retains metadata/other diagnostic modes. The sampled UV1 meshes were
multi-primitive; no real single-primitive UV1 eframe render is claimed. Its shared
channel selection and display support are implemented/tested; native UV0 floor
regression works and export is byte-identical to D43. Manual mouse/keyboard/reload
stress, other GPUs/platforms, full corpus and game rendering fidelity are untested.

```powershell
. ./scripts/use-local-rust.ps1
./target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid 00010e8d
```

Left drag orbits, right/middle drag pans, wheel zooms, R resets, F frames. The two
status rows should say UV1. Try GUID 00020155 with Load / reload for supported plus
neutral groups, or 000150e5 for the known UV0 floor. Exact oracle/check commands,
resolved build lock and private evidence filenames: STATUS.md.
