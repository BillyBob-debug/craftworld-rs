# D3 material navigation and header census — 2026-10-05

D3 complete within its bounded metadata scope. Native Mesh tab lists each
primitive's graphics-material descriptor. GUID and SHA1 candidate groups remain
separate; every MAP alias can be selected, and conflicting groups are never silently
merged into one winner. A selected target navigates through existing catalog/history
to Metadata tab, where its dependency references can be followed to textures.
Unresolved, zero GUID and zero SHA1 remain visible as unresolved/raw null.
No material resource is executed and no original shader code is rendered/exported.

## Evidence and scope

`CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, Primitive::serialize/GetMaterial and
Serializer::resource define graphics-material references; current raw descriptor
layout/canonical geometry evidence is in [mesh-geometry.md](mesh-geometry.md).
Catalog reference normalization follows existing A2 GUID/hash/null behavior while
keeping raw metadata. Navigation resolves identities, not inferred texture roles.

`CONFIRMED_FROM_LOCAL_DATA`: existing validated snapshot has 5,163 graphics material
locations, all ordinary metadata_parsed, in18revision/flag/branch cohorts. Raw
material dependency tables contain13,426texture references (including duplicates;
not a count of unique textures or named sampler slots). Metadata-only selection
chooses32smallest/median resources across16cohorts. Pinned Toolkit header/dependency
oracle agrees on all32samples and56normalized references, plus complete existing
36,854MAP/31,534FARC table fields. Actual raw multiplicity remains in the scanner.
This is header/dependency parity, not a material payload/shader oracle.

`CONFIRMED_FROM_COMMUNITY_CODE`: [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java)
is the next reader lead: versioned alpha/shadow/bump/refraction fields; shader
source-offset tables and code blob; resource texture descriptors; wraps/material
boxes/wires and other tails. This pass only studies the sequence. Shader meaning,
slot roles, material graph evaluation and original shading are `UNKNOWN` locally.
No speculative material payload parser, shader translation or full renderer added.

## Verification and reproduction

Final workspace67Rusttests pass; workspace Clippy/fmt/native build pass. Five
Python report tests (four texture and one material) pass. New authored resolver
case checks GUID/hash alias groups with overlapping/conflicting candidates, missing
references and raw nulls. Existing navigation/history integration remains tested.
Native mesh-with-material-controls capture inspected on RTX3080/Vulkan; four
real GUID reference groups each resolve to one candidate in the selected sample.
No manual menu click/navigation input delivered; resolution and route code are
covered by synthetic tests, rendered controls by capture.

The first source edit used Windows default encoding and introduced an invalid
UTF-8 middle-dot byte; attempted normalization doubled CRLF. Corrected with UTF-8
and single line endings, then all checks passed. A synthetic material sample of
10bytes was below the12byte selection threshold; corrected recipe16bytes before
the deterministic selection test passed. No original input was affected.

```powershell
. ./scripts/use-local-rust.ps1
python scripts/material_report.py local-analysis/a2-inventory-verified.json local-analysis/NEW-material-samples.json
./scripts/run-toolkit-oracle.ps1 -SampleManifest local-analysis/NEW-material-samples.json -Output local-analysis/NEW-material-header-oracle.json
python scripts/verify_toolkit_oracle.py local-analysis/a2-inventory-verified.json local-analysis/NEW-material-header-oracle.json --output local-analysis/NEW-material-header-parity.json
cargo run -p lbp-viewer -- --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc
```

Use Mesh -> Material references -> candidate -> Metadata/Depends on. Full inventories,
oracle outputs and screenshot/log stay ignored under local-analysis/d3-*.
No game assets, reference binaries or unknown-provenance fixtures are publishable.
Existing oracle input1MiB/sample and Java512MiBheap bounds remain; selected resources
are clear GMTb only. This does not promise arbitrary hostile-input safety of Toolkit
or authorize protected constructors/decryption. Our Rust readers remain bounded.

Future bounded task: material payload metadata contract and guarded external oracle,
then an independent parser only for corroborated fields. Keep shader/code bytes
uninterpreted and excluded from diagnostic serialization. Textured material preview
requires separate slot/color-space/UV evidence; no full-level/gameplay expansion.
