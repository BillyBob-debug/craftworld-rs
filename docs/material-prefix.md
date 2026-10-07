# D4 material prefix evidence — 2026-10-05

Current result: D5 independently implements this narrow prefix in
`lbp-resource::material`, with32/32canonical metadata matches. D4's external-only
evidence and historical limits below remain recorded; D5 verification is appended.

This step establishes a bounded external metadata oracle before a Rust material
parser. It does not add an application codec, material graph evaluator, textured
renderer or original shader execution. The authored Java harness calls pinned
Toolkit stream helpers and RGfxMaterial count methods; it never invokes full
RGfxMaterial deserialization. Original shader bytes are skipped, not exported.

## Provenance and applicability

All community-code claims below use Toolkit commit
`d6a2ef80b5a09999f08f23a803b516096055e5c9`. Public reader source is studied as format
evidence and used externally; no source was copied/adapted/translated into product
code. Toolkit's MIT license and inherited file notices remain in ignored references.
The project's distribution license remains undecided.

| Rule | Evidence and exact reader | Applicability / coverage |
| --- | --- | --- |
| Prefix sequence: flags, alpha test, alpha layer, conditional alpha mode, shadow byte, bump/cosine/reflection/refraction floats | CONFIRMED_FROM_COMMUNITY_CODE: [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java), [Revisions::GFXMATERIAL_ALPHA_MODE](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/Revisions.java) = 0x2fa | Oracle accepts ordinary branch-zero heads920–951 only; all observed heads include alpha mode. LBP3 Fresnel/fuzz, Vita and Toolkit branches excluded. IEEE bit patterns preserved, without inferring appearance. |
| Integer fields follow resource compression flags; floats remain fixed-width IEEE bits | CONFIRMED_FROM_COMMUNITY_CODE: [MemoryInputStream::i32/f32](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/streams/MemoryInputStream.java) | Synthetic flags0/7 prefixes compare semantic fields, every truncation rejected. Actual32samples all flags7. |
| Shader offset count is revision/branch/material-flag dependent | CONFIRMED_FROM_COMMUNITY_CODE: RGfxMaterial::getBlobOffsetCount/getShaderCount; offset and byte-array reads in serialize | Count methods invoked directly. Current32samples have10offsets/shaders. Current guard requires monotonic nonnegative ends within blob; this is accepted sample scope, not a universal validity claim. Old (<0x34f) start-offset convention unsupported. |
| Eight texture descriptors follow opaque blob | CONFIRMED_FROM_COMMUNITY_CODE: RGfxMaterial::MAX_TEXTURES/serialize and [Serializer::resource](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/io/serializer/Serializer.java) | Current flags/kind/GUID/hash encoding matches mesh descriptor contract. Raw nulls retained. Synthetic0/1/2/3 descriptor presence checked; actual200null/56GUID. |
| S/T wrap arrays have integer count then byte enums | CONFIRMED_FROM_COMMUNITY_CODE: MemoryInputStream::enumarray and [TextureWrap](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java) | Oracle budget8perarray; every selected array has8entries. Raw values preserved; actual510value1(WRAP),2value4(BORDER). Unknown enum values remain numeric. |
| Boxes/wires and other material tails follow wraps | CONFIRMED_FROM_COMMUNITY_CODE: RGfxMaterial::serialize | Not traversed. Local residual163–2911bytes remain UNKNOWN in this contract. Texture semantic roles, UV association, shader meaning, alpha/blending/render fidelity UNKNOWN. |

`CONFIRMED_FROM_LOCAL_DATA`: 32 selected samples, heads920,928,931,932,933,935,
936,937,940,941,944,945,946,947,949,950 from BCUS98245 APP_VER01.00. Archived reads
total232,687bytes. Opaque blobs864–23,328bytes; bytes after final shader end64–9,528
remain uninterpreted. All eight texture slots are retained including nulls. All56
normalized references agree with the prior independently scanned and Toolkit-verified
header/dependency tables. This is cross-consistency of external prefix evidence with
existing header evidence, **not independent Rust material parser parity**. It does
not establish payload compatibility for all5,163materials or any other game build.

## Conflicts and limits

Synthetic truncation133 exposed MemoryInputStream::bytes using Arrays.copyOfRange,
which pads a short final fixed-width read. The authored oracle now rejects a final
offset beyond actual buffer length; every truncation of both fixed-width and packed
synthetic prefixes fails. Toolkit remains unchanged. This is an oracle safety guard,
not evidence that a truncated material is valid. Earlier failed run emitted no report.

Only clear GMTb with ordinary branch0, known compression flags0/7 and heads920–951
enters SerializedResource. Read-only FARC seeks, SHA1 identifier checks, maximum48
samples,1MiB/sample,16MiB total stored reads,8MiB accepted expanded payload,
4096dependency count,24offsets and8wrap entries bound this experiment. Existing
shared strict chunk guard has32MiB temporary inflation budget before the8MiB accepted
limit. JVM heap256MiB. No guarantee of arbitrary hostile-input safety for Toolkit;
no protected constructors or decryption paths were used in this experiment. Current
research permissions are in AGENTS.md. Graphs/tails are skipped by this prefix API.

The unknown bytes after shader intervals are not automatically material constants,
nor are dependency order or texture slot number proof of diffuse/normal roles.
Shader bytes never enter diagnostic JSON, fixtures or tracked files. No shader
translation, recovered source or executable investigation was used.

## Reproduction and verification

```powershell
./scripts/run-material-oracle.ps1 -Output local-analysis/NEW-material-prefix-oracle.json
python scripts/material_prefix_report.py local-analysis/NEW-material-prefix-oracle.json local-analysis/d3-material-header-oracle.json local-analysis/NEW-material-prefix-consistency.json
python -m unittest discover -s tests -p test_material_prefix_report.py
```

Final oracle:32/32 prefixes accepted;32/32 prior header identities/revisions match,
56normalized slot/dependency references match. Java authored tests cover flags0/7,
all prefix truncations, malformed interval and descriptor presence0/1/2/3. Three
Python cases cover both IDs, raw null, unknown wraps, malformed intervals/lengths,
descriptor mismatches, dependency conflicts and duplicate/missing identities/pins.
No Rust code changed; previous67Rusttests/fmt/Clippy/native build remain the latest
product validation and were not rerun for this diagnostic-only step. No new GUI QA.

Final ignored reports: local-analysis/d4-material-prefix-oracle-final.json and
d4-material-prefix-consistency-final.json. Earlier successful reports retained as
local evidence. Authored scripts/tests/docs allowed; reports/assets/reference jar
ignored. Original input and reference tracked source remain unchanged.

## Exact next bounded task

Implement an independent Rust PS3 GMT prefix reader for the corroborated contract,
first factoring shared bounded binary-wrapper inflation into lbp-resource without
coupling materials to meshes. Export scalar bits, offsets/counts, raw descriptors
and wraps only; retain opaque tails. Use strict checked reads rather than padded
oracle semantics; synthetic revisions/flags/counts/offsets/descriptor modes/truncations,
budgets/cancellation and canonical metadata parity against these32samples are required.
Keep parser/CPU IR/UI separate. No graph evaluation, original shader execution,
texture-role guesses, new renderer, level/gameplay/animation/physics/network work.

## D5 independent reader complete — 2026-10-05

`lbp-resource::material::inspect` now returns MaterialInspection/MaterialPrefix:
fixed IEEE scalar bits, unsigned raw scalar/enums, ten end offsets and blob byte
counts, eight descriptors retaining kind/flags/GUID/hash/raw nulls, wrap bytes and
remaining-tail count. No opaque byte slice, shader bytes or material graph is exposed
or serialized. Wrap lengths0–8are retained as declared; unknown wrap/shadow values
are numeric. This is diagnostic metadata, not a renderer material representation.

Evidence categories remain per-rule above. `CONFIRMED_FROM_LOCAL_DATA`: independent
Rust output agrees exactly with all32external prefixes, compression flags and
expanded sizes, including the56GUID slots. No float decimal normalization is needed:
IEEE bits are canonical. Actual revision cohort and scope remain D4's16heads, flags7,
branch0. Flags0, scalar NaN/signed-zero bits, descriptors0/1/2/3 including both IDs and
raw zero identities, unknown wraps/enums, and all compression modes are synthetic
coverage; do not mistake them for observed local variants. Head951is within the
established ordinary PS3 header/source layout but not represented in these32samples.

Application limits:1MiBstored/8MiBexpanded, checked chunk totals before allocation,
strict zlib end/length/trailing checks, strict five-byte ULEB32 with overflow checks,
ten monotonic nonnegative offsets within blob, eight slots/wrap budget and checked
slice arithmetic. Unknown revisions/branches/descriptors, encrypted magic, malformed
structures, budgets and cancellation are distinct outcomes. Default header/dependency
limits remain in force. A blob is skipped as a checked slice; no shader copy occurs
inside prefix parsing. Expansion transiently holds bounded clear payload as required
for compressed inputs; it is dropped when only metadata returns.

Shared `lbp-resource::binary` factors the project's own previously verified mesh
envelope/chunk implementation; both mesh and material use it. Mesh Error is a public
alias of the shared error enum with unchanged variants. No third-party source was
translated. Zero-chunk/raw payload budgets are checked before copy, and completion
checks cancellation. `lbp-texture` wrapper handling is unchanged: its GTF/TEX layout
is a separate envelope, not forced through the ordinary binary API.

No new crate is needed for a small metadata prefix. There is no material geometry,
texture-role mapping or GPU IR yet. A future material graph codec can split into a
dedicated crate once it has substantial responsibilities and actual consumers.

Verification commands (local Rust activated first):

```powershell
. ./scripts/use-local-rust.ps1
cargo test -p lbp-resource -p lbp-mesh
cargo test -p lbp-resource --test material
cargo run -p lbp-catalog --example material_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d4-material-prefix-oracle-final.json local-analysis/d5-material-prefix-parity.json
cargo test --workspace --all-targets
cargo run -p lbp-catalog --example mesh_parity -- local-analysis/a2-inventory-verified.json game-data/a1-disc local-analysis/d2-mesh-oracle.json local-analysis/d5-mesh-geometry-regression.json
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lbp-viewer
```

Four new integration tests cover fixed/packed fields; raw, stored, zero-chunk and
zlib wrappers; all wire truncations and short payloads inside rebuilt valid envelopes
(so header rejection cannot mask an overrun or padded read); protected markers;
revisions/branches; offset,
count, ULEB overflow, descriptor and wrap failures; input/expanded budgets; early
and subsequent cancellation; trailing chunk bytes.71workspace tests pass, material
parity32/32and mesh geometry regression36/36pass, fmt/Clippy/native build pass.
Initial sandbox formatter/native parity launch returned OS access denied; approved
normal tool execution completed without approval-review rejection. No parser or
oracle mismatch occurred. Full material corpus, native UI integration/QA, graphs,
shader semantics or original render fidelity were not tested in D5.

Read-only attachment and existing fingerprints/ranges constrain parity source access;
diagnostics create a new file only under ignored local-analysis. Root remains outside
Git; no publication. Original fingerprints/size/attributes unchanged.

Next bounded task: add a native Material metadata tab using this reader, existing
validated/cancellable source workers and candidate navigation. Present scalar bits,
raw values, shader interval metadata, all eight descriptor slots and wrap arrays;
keep null/unresolved/alias/conflict states explicit. No shader/tail byte export,
graph evaluation, guessed sampler roles or textured shading. Verify synthetic tab
states, cancellation/reset and one bounded native sample before concluding.
