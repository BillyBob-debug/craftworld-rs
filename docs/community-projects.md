# Community projects ranked for an LBP2 Asset Explorer

Reviewed 2026-10-05 using GitHub APIs, pinned source files, local reference checkouts,
and public web searches. Activity timestamps and full commits are listed below.
Confidence is scoped: high for inspected layout rules, not blanket retail-build support.
No public repository's assets or binary templates are presumed redistributable.

## 1. Craftworld Toolkit / cwlib — primary reference and verified metadata oracle

[ennuo/toolkit](https://github.com/ennuo/toolkit), Java, **MIT**, actively updated
(default branch 2026-09-20). Mainline LBP1/2/3; code also contains Vita/PS4/branch
variants, which are not substitutes for LBP2 tests. It covers FARC and save archives,
MAP/FileDB, GUID/SHA1 descriptors, versioned wrappers, dependency resolution,
TEX/GTF textures, MSH/SMH, graphics/physical materials, ANM, LVL/PLN, slots/packs,
localization, saves, and script representations. Its GUI previews textures and its
exporter emits OBJ/glTF and resource JSON. Best fit across scanner and viewer milestones.

Study `lib/cwlib/src/main/java/cwlib/`: `types/archives/FileArchive.java`,
`types/databases/FileDB.java`, `types/SerializedResource.java`, `types/data/ResourceInfo.java`,
`ResourceDescriptor.java`, `enums/ResourceType.java`, `SerializationType.java`,
`resources/RTexture.java`, `RMesh.java`, `RStaticMesh.java`, `RGfxMaterial.java`,
`RAnimation.java`, `RLevel.java`, `RPlan.java`, `RTranslationTable.java`,
`util/Compressor.java`, `DDS.java`, `io/exports/MeshExporter.java` and `structs/`.
GUI/reference-renderer module `lib/cwlib-gl`; tools include jsoninator, descriptor,
meshimporter. Use read-only entry points only; some constructors can decode encryption.

Reuse/adaptation/translation of MIT-covered source is possible with retained license
and attribution; audit per-file inherited third-party notices separately. Avoid
copying binary templates under resources. **Confidence: high** archive/MAP/header
rules (locally corroborated); **medium-high** payload readers until local oracle parity.
The old FarcTool ancestry does not automatically resolve all inherited-file licensing.

C1 corroborated 22 clear texture top mips and 191 fields at the pinned commit.
Confidence is high for those sampled top mips, not full texture coverage. The
synthetic DDS mip-1 test fails; use a different established layout oracle before
higher-mip work. DDS::convertSwizzleGtf explicitly cites RPCS3 at a GPLv2 revision;
do not assume MIT permission to translate that inherited helper. DDSReader's own
MIT notice is separate. Detailed evidence and limits: [textures.md](textures.md).

## 2. Python lbptoolspy — useful independent experiments

[Zhaxxy/lbptoolspy](https://github.com/Zhaxxy/lbptoolspy), Python, **GPL-3.0**,
updated 2026-08-19. Mainline game/save workflows; exact variant support must be read
per function, not inferred from the package description. Includes FAR4/save operations,
MAP generation/limited reading, TEX DDS wrapping, image conversion, JSON helpers and
mod installation. Study `src/lbptoolspy/{far4_tools,tex_tools,jsonify_lbp_files}.py`.
JSON functions may delegate to Toolkit; they are not proof of a second complete parser.

[Zhaxxy/lbptoolspy-lite](https://github.com/Zhaxxy/lbptoolspy-lite), Python,
**GPL-3.0**, last default commit 2025-06-21; removes Java/ImageMagick-dependent
features. Its MAP reader explicitly accepts only the Vita-labelled 0x3A8 revision,
so it cannot oracle our 0x100 MAP. TEX converter accepts TEX wrappers, not GTF.
Source-only clone added to ignored references. Isolated `tex_tools.py` successfully
round-tripped an authored 2x3 DDS/TEX synthetic image in the tests; local GTF pixels
have not been decoded with it. Do not invoke L0 encryption/save manipulation.

Useful as an **external oracle**, not a default vendored Rust dependency. Copying or
translating GPL source requires GPL obligations and project licensing review; do not
assume language translation removes them. **Confidence: medium**, narrow synthetic
TEX path verified, clear support exclusions and stored-chunk caveat.

## 3. LBP Asset Studio (formerly Background Maker) — viewer experience reference

[MindOfBog/lbp_bgm](https://github.com/MindOfBog/lbp_bgm), Java/OpenGL,
updated 2026-07-15. Mainline LBP1/2/3: archive browsing, 3D construction/preview and
plan/level export, relying heavily on cwlib. Study `src/main/java/bog/`, embedded
`cwlib/resources/{RMesh,RTexture,RGfxMaterial}.java` and scene/renderer integration.
[Author documentation](https://mindofbog.github.io/LBP_AS/) explains archive loading
and viewer interaction. Great evidence that useful 3D inspection is achievable
without recreating gameplay. The older [ennuo/lbp_bgm](https://github.com/ennuo/lbp_bgm)
fork is less current (2023-06-09); prefer MindOfBog's source for current features.

No repository-wide license was found in metadata; inherited cwlib and third-party
files may have their own licenses. Treat application code as behavioral reference
unless explicitly licensed. **Confidence: medium-high** application scope, **medium**
individual parser behavior because it inherits cwlib and was not run here.

## 4. FeTetra/cwlib-rs — existing Rust effort, but not a ready foundation

[FeTetra/cwlib-rs](https://github.com/FeTetra/cwlib-rs), Rust, updated 2026-03-01,
no license file/declaration found. README explicitly calls it minimal: MAP and FARC
(de)serialization and rudimentary hash-based extraction. Study `src/io/farc.rs`,
`src/io/filedb.rs`, `src/types/{farc,filedb}.rs`, `src/io/serdes.rs`,
`src/tests/testfarcmapserdes.rs`. No texture/mesh/level reader established.
Do not vendor/adapt/translate without permission; independently use public format
knowledge from licensed Toolkit and keep cwlib-rs as a reference comparison lead.
**Confidence: medium** scope, **low-to-medium** correctness/coverage until built.

## 5. FarcTool2 — historical cross-check, superseded by Toolkit

[hellfire3d/farctool2](https://github.com/hellfire3d/farctool2) (historical
philosophofee upstream now redirects here), Java, archived, last commit 2026-06-09
mostly historical documentation. Mainline LBP1/2/3 archive/MAP manipulation, extraction
and limited resource utilities. Study
`farctool2/src/com/philosophofee/farctool2/{FarcUtils,MapParser,BRGMapFile,MapEntry,ZlibUtils}.java`.
README explains early community discoveries and Toolkit ancestry. No explicit
repository license found; reference behavior/factual layouts only, no copied code.
**Confidence: medium-high** historical archive layout, **medium/low** payload coverage.

[LBPModding/FarcTool2](https://github.com/LBPModding/FarcTool2), Java fork,
last default commit 2019-10-17; FARC/FAR4/MAP editing, no explicit license found.
Inspect the same FarcUtils/MapParser family. GitHub upstream fork list showed this
one direct fork; broad searches did not establish a newer maintained FarcTool reader
that improves on current Toolkit. Other forks may exist outside that network.
**Confidence: medium**; do not spend time porting obsolete GUI code.

## 6. Refresh / Lighthouse — focused auxiliary readers, not game runtimes

[LittleBigRefresh/Refresh](https://github.com/LittleBigRefresh/Refresh), C#,
**AGPL-3.0**, updated 2026-10-04, active custom server. Mainline games plus other
platform-specific paths. Useful files: `Refresh.Core/Importing/TexStream.cs`,
`Importing/Gtf/{GtfHeader,GtfMetadata,GtfDecoder,GtfFormat,GtfPixelFormat}.cs`,
`Helpers/ResourceValidationHelper.cs`, asset relation/validation types. Texture
streaming is a useful second reference for chunk handling; network endpoints do
not implement full mesh/level playback. Encryption helpers are excluded from use.
**Confidence: medium-high** narrowly inspected texture stream; other paths not run.

[LBPUnion/ProjectLighthouse](https://github.com/LBPUnion/ProjectLighthouse), C#,
**AGPL-3.0**, updated 2026-09-02. Mainline custom-server project, relevant to resource
identification, dependency validation and resource APIs. Study
`ProjectLighthouse/Types/Resources/{LbpFile,LbpFileType}.cs`,
`ProjectLighthouse.Servers.GameServer/Helpers/GameResourceHelper.cs` and serialization
dependency tests. `LbpFile` is metadata around SHA1/GUID/type, not a full payload parser.
Do not reuse its bundled test assets. **Confidence: medium** auxiliary semantics.

Use both as external references/oracles; copied/adapted/translated AGPL code carries
obligations including applicable network-source requirements, not a permissive
library grant. This pass does not implement or connect to their servers.

## 7. Public documentation and small specialized tools

[LBPModding/Documentation](https://github.com/LBPModding/Documentation), archived
Markdown repository, last commit 2019-02-08, no license found, intended game-format/
network research. Actual main tree contains only README and a Discord invitation;
the locally available wiki checkout contains only Home.md. **Confidence: low** as a
format corpus; do not pretend it supplied missing specs.

[Imagisphere format page](https://wiki.imagisphere.me/Filetype:File_Archive) and
[modding guide](https://wiki.imagisphere.me/Help:Modding_Resources/Basic_Modding_Guide)
are public community-maintained documentation for mainline workflows. FARC page
last edited 2026-05-27; prose/diagram disagree about footer order. Useful leads,
**medium confidence** after source/local cross-check. No wiki text or images copied;
content reuse licensing was not established by this pass.

[LittleBigRefresh/Docs](https://github.com/LittleBigRefresh/Docs), Markdown/Writerside,
updated 2026-09-20, no license found; `Docs/topics/lbp/{labels,pins,community-tab-data,
recent-activity}.md` and security topic. Good public context around server-side data,
not a complete offline asset specification. **Medium confidence** scoped documentation.

[LittleBigRefresh/GenderFluid](https://github.com/LittleBigRefresh/GenderFluid), C#,
last commit 2023-10-13, no license found; `.trans` <-> JSON converter. Study
`GenderFluid/Trans.cs::ReadTransFile`. Potential string delimiter condition issue
means cross-check with Toolkit before implementing. **Medium confidence** table shape;
no code reuse without permission.

[Pear0533/LittleBigPlanet-Dependencies-Parser](https://github.com/Pear0533/LittleBigPlanet-Dependencies-Parser),
JavaScript, default commit 2023-02-04 (pushed 2024-02-16), no license found.
`de-net.js::getResourceDependencies` reads dependency offset/count and chooses SHA1
or GUID refs. Assumes a fixed header and only flags 1/2; lacks robust combined-flag,
revision and bounds handling. **Low-to-medium confidence**; corroborating lead only.

## 8. Decompilations and long-term engine leads — segregate from parser implementation

[MasonLeeBack/yellowhead](https://github.com/MasonLeeBack/yellowhead), C++, active
2026-06-11, no license found, **LittleBigPlanet Deploy Build**, not verified LBP2
retail. `README.md` describes symbol/DWARF/disassembly recovery and matching-workbench
progress; requires local ELF/SDK inputs. Source-tree leads: `src/CWLib/src/Fart.cpp`,
`src/CoreLib/src/{GUIDHash,Serialise,SerialiseRevision,CompressionJob}.{cpp,h}`.
Only metadata/tree/README inspected; recovered source not used to design our parsers.
**Low applicability to LBP2 viewer**, no code adaptation/translation permitted here.

[ArataTamafuchi/Orrible](https://github.com/ArataTamafuchi/Orrible), README/license
only at b257dec0, top-level **GPL-3.0**, 2026-07-26, announces LBP1 update 01.30
decompilation. No working parser/runtime evidenced by tree. **Low confidence** as an
implementation resource. Top-level license would not establish rights to recovered
proprietary source anyway; no source use.

[hellfire3d/OpenCW](https://github.com/hellfire3d/OpenCW) and
[hellfire3d/Mussqr](https://github.com/hellfire3d/Mussqr), project descriptions for a
mainline-engine reimplementation and LBP2/3 music sequencer editor respectively.
GitHub commit endpoints returned **409 empty repository** on inspection; no code,
language/license or usable modules established. Pushed timestamps 2026-07-12/13
are not evidence of an implementation. Keep on watch list manually; **low confidence**.

## Toolkit forks: evaluated against current upstream

| Fork | Default commit / activity | Comparison with current upstream | License / useful difference |
|---|---|---|---|
| [Manoplay/toolkit](https://github.com/Manoplay/toolkit) | e4e8f463, 2026-07-21; pushed Aug 8 | 0 ahead, 3 behind | MIT; no extra default-branch parser improvement |
| [Pacomatic1/craftworld-foolkit](https://github.com/Pacomatic1/craftworld-foolkit) | a07f4301, 2026-06-17 | 10 ahead, 36 behind | MIT; inspected commit descriptions show comments/docs/build scripts and getter cleanup, not established extra format coverage |
| [MetricCepheid/toolkit](https://github.com/MetricCepheid/toolkit) | 44a7c920, 2025-05-17; archived | 1 ahead, 36 behind | MIT; GitHub Actions Maven workflow change |
| [uhwot/toolkit](https://github.com/uhwot/toolkit) | 3facf2a8, 2022-09-17; pushed 2026-09-23 | 0 ahead, 484 behind | API license null; old default branch, recent push alone misleading |

All are Java Toolkit-lineage projects aimed at mainline resources, except uhwot API
language null for current metadata; source lineage is Java. Important modules remain
cwlib; inspect fork-specific commits before importing any claimed improvement.
Fork coverage claims are not newly verified format support. **Medium-high confidence**
comparison metadata, **medium** code differences (not a full fork audit).
Other direct forks found: qwrwed, mrf4nta, jwfeniello/toolkit-cb, mkarpiarz/lbp-toolkit,
sgdc3, DreamyAyu/toolkittesting, tamagofox, RealRedcodi, users-things/toolkit-re-release,
cualquiercosa327/farctool3, Xewy1, FridiNaTor1. Their listed pushes were older and no
current parser improvement was established; do not rank by fork name or push alone.

## Platform distinctions and excluded leads

- [ennuo/cambridge](https://github.com/ennuo/cambridge), Java, 2026-01-08, no license
  found: PSP asset tooling; `cambridge/resources/Archive.java`, Biff/model/texture
  readers and `craftworld/ResourceConverter.java`. Low priority for LBP2; no source reuse.
- [ennuo/lbparc](https://github.com/ennuo/lbparc), JavaScript/Node, 2022-06-19,
  no license found: PSP archives/BIFF/model/texture scripts under `src/resources/`.
  Different platform, not LBP2 FARC evidence; low LBP2 applicability.
- [uhwot/arcunpack](https://github.com/uhwot/arcunpack), Rust, pushed 2026-04-22:
  PSP-alpha PSARC unpacker search lead; not inspected deeply, license UNKNOWN.
- [ennuo/ufg.js](https://github.com/ennuo/ufg.js), JavaScript, archived 2022-09-22,
  no license found: UFG/Karting formats, not mainline Craftworld; low applicability.
- [ennuo/cwml](https://github.com/ennuo/cwml), C/C++ sources, MIT, 2025-10-07,
  modloader with CWLib header leads. Runtime patching outside current scope; source
  ancestry needs separate audit before any reuse. Not needed as a reader oracle.
- [ennuo/alear](https://github.com/ennuo/alear), C++, 2026-08-03, no license found:
  modified-game headers/resources plus patch assets; potentially decomp-derived.
  Only tree inspected, no code/assets downloaded; **low** applicability within scope.
- Workbench is referenced in tutorials, but no separate current standalone canonical
  source repository was established by searches. Toolkit's current tools and shared
  cwlib provide the traceable source route; do not attribute an unknown Workbench version.
- `LittleBigRefresh/LbpData` returned 404; no library established under that name.
- Archive downloaders, save/server patchers, fan recreations, music-sequencer projects,
  and game-asset repositories appeared in searches; they are not evidence of a
  reusable retail LBP2 parser. No game assets were downloaded from them.

No license is selected for this project's code yet. The reuse recommendations above
are engineering provenance policy, not a guarantee of legal clearance for every file.

## Full source revision ledger

Dates are default-branch commit timestamps in UTC, distinct from push dates.

| Repository | Full commit | Commit date UTC |
|---|---|---|
| [ArataTamafuchi/Orrible](https://github.com/ArataTamafuchi/Orrible/tree/b257dec00ff2c1282fb27bdd90ca069076a3e4e2) | `b257dec00ff2c1282fb27bdd90ca069076a3e4e2` | 2026-07-26T05:03:32Z |
| [FeTetra/cwlib-rs](https://github.com/FeTetra/cwlib-rs/tree/e796cf7046a933024a2a075bff0b3c2fbdb03f91) | `e796cf7046a933024a2a075bff0b3c2fbdb03f91` | 2026-03-01T18:49:50Z |
| [LBPModding/Documentation](https://github.com/LBPModding/Documentation/tree/07902c62d081d18aa68b0076b47e5fda67e8c6da) | `07902c62d081d18aa68b0076b47e5fda67e8c6da` | 2019-02-08T00:49:29Z |
| [LBPModding/FarcTool2](https://github.com/LBPModding/FarcTool2/tree/4736687e221c2d287f92538f4c6f3a413f6792b7) | `4736687e221c2d287f92538f4c6f3a413f6792b7` | 2019-10-17T15:04:29Z |
| [LBPUnion/ProjectLighthouse](https://github.com/LBPUnion/ProjectLighthouse/tree/ac9dc7bcbf87bf5478d3f53cd7e4d2567cb7414e) | `ac9dc7bcbf87bf5478d3f53cd7e4d2567cb7414e` | 2026-09-02T14:21:00Z |
| [LittleBigRefresh/Docs](https://github.com/LittleBigRefresh/Docs/tree/82989476c9959ab02c76f192f504dc32256fecd6) | `82989476c9959ab02c76f192f504dc32256fecd6` | 2026-09-20T08:26:10Z |
| [LittleBigRefresh/GenderFluid](https://github.com/LittleBigRefresh/GenderFluid/tree/c37c887b2cdc2f60d88828f81097f684273dbfbb) | `c37c887b2cdc2f60d88828f81097f684273dbfbb` | 2023-10-13T03:46:45Z |
| [LittleBigRefresh/Refresh](https://github.com/LittleBigRefresh/Refresh/tree/25f03e40416ebe73016b9e9972e0fa7b11a7f0b0) | `25f03e40416ebe73016b9e9972e0fa7b11a7f0b0` | 2026-10-04T19:46:38Z |
| [Manoplay/toolkit](https://github.com/Manoplay/toolkit/tree/e4e8f4631624753536dd06ee37e79fdfaf18df9d) | `e4e8f4631624753536dd06ee37e79fdfaf18df9d` | 2026-07-21T02:03:46Z |
| [MasonLeeBack/yellowhead](https://github.com/MasonLeeBack/yellowhead/tree/fcad235bbc0fd4d01bba993f802063f062c9968e) | `fcad235bbc0fd4d01bba993f802063f062c9968e` | 2026-06-11T02:42:16Z |
| [MetricCepheid/toolkit](https://github.com/MetricCepheid/toolkit/tree/44a7c920c7463de3bb59b524182dd3555eb9508a) | `44a7c920c7463de3bb59b524182dd3555eb9508a` | 2025-05-17T14:47:01Z |
| [MindOfBog/lbp_bgm](https://github.com/MindOfBog/lbp_bgm/tree/f015b676dc0938ed88b94b6d9b473fd9d25b49e0) | `f015b676dc0938ed88b94b6d9b473fd9d25b49e0` | 2026-07-15T06:37:35Z |
| [Pacomatic1/craftworld-foolkit](https://github.com/Pacomatic1/craftworld-foolkit/tree/a07f43019cc0abc5407d0c395372e05a7e04df43) | `a07f43019cc0abc5407d0c395372e05a7e04df43` | 2026-06-17T02:38:01Z |
| [Pear0533/LittleBigPlanet-Dependencies-Parser](https://github.com/Pear0533/LittleBigPlanet-Dependencies-Parser/tree/fb23c2b0316c668e5620803996fe7359ef1e32ad) | `fb23c2b0316c668e5620803996fe7359ef1e32ad` | 2023-02-04T20:01:09Z |
| [Zhaxxy/lbptoolspy](https://github.com/Zhaxxy/lbptoolspy/tree/5b2588f92e7b353331e64303c199cb602622f494) | `5b2588f92e7b353331e64303c199cb602622f494` | 2026-08-19T20:33:03Z |
| [Zhaxxy/lbptoolspy-lite](https://github.com/Zhaxxy/lbptoolspy-lite/tree/9e044e0f068d9f3f4db3608e8a4305e1f2f76ce9) | `9e044e0f068d9f3f4db3608e8a4305e1f2f76ce9` | 2025-06-21T17:58:08Z |
| [ennuo/alear](https://github.com/ennuo/alear/tree/b651c4402c821c848474c889b12a4453ac55d5b4) | `b651c4402c821c848474c889b12a4453ac55d5b4` | 2026-08-03T12:21:47Z |
| [ennuo/cambridge](https://github.com/ennuo/cambridge/tree/59c98e459e80867f5bb6831709a7915ddd78633d) | `59c98e459e80867f5bb6831709a7915ddd78633d` | 2026-01-08T17:23:50Z |
| [ennuo/cwml](https://github.com/ennuo/cwml/tree/fc1aa2b67f48340ad70c207138c91082cc3b9a83) | `fc1aa2b67f48340ad70c207138c91082cc3b9a83` | 2025-10-07T14:30:35Z |
| [ennuo/lbparc](https://github.com/ennuo/lbparc/tree/16ad36aa7f4eae2f7b406829e604082750f16fe1) | `16ad36aa7f4eae2f7b406829e604082750f16fe1` | 2022-06-19T21:51:39Z |
| [ennuo/toolkit](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9) | `d6a2ef80b5a09999f08f23a803b516096055e5c9` | 2026-09-20T02:03:07Z |
| [ennuo/ufg.js](https://github.com/ennuo/ufg.js/tree/2da9b486627bdb4b82306d5b46dbb1b2e289bb65) | `2da9b486627bdb4b82306d5b46dbb1b2e289bb65` | 2022-09-22T19:23:27Z |
| [hellfire3d/farctool2](https://github.com/hellfire3d/farctool2/tree/b3d6c186c1b79677a6ef68cda6d4c91783a521a0) | `b3d6c186c1b79677a6ef68cda6d4c91783a521a0` | 2026-06-09T01:29:03Z |
| [kvalls/hl2-rs](https://github.com/kvalls/hl2-rs/tree/b4b1731530a9ca146f9d45532f00ef7f9fc69e7d) | `b4b1731530a9ca146f9d45532f00ef7f9fc69e7d` | 2026-10-04T20:32:32Z |
| [muffinmxn/gang-beasts-rust](https://github.com/muffinmxn/gang-beasts-rust/tree/815ffbcb4d68a080feb975cf214f14c9a41f1f51) | `815ffbcb4d68a080feb975cf214f14c9a41f1f51` | 2026-10-04T14:37:21Z |
| [samwhosung/benilla](https://github.com/samwhosung/benilla/tree/ffdc206ac2e87e3164e31caf26862c43a5180152) | `ffdc206ac2e87e3164e31caf26862c43a5180152` | 2026-10-05T15:28:59Z |
| [uhwot/toolkit](https://github.com/uhwot/toolkit/tree/3facf2a8470d0f6aea8bba6faa4bb632283f6927) | `3facf2a8470d0f6aea8bba6faa4bb632283f6927` | 2022-09-17T18:09:32Z |

C4 oracle caveat: pinned Toolkit DDSReader applies BC1 color ordering to BC3.
Documented synthetic conflict and independent corroboration: [bc3-preview.md](bc3-preview.md).


D27 deeper pinned Toolkit study: RLevel::serialize immediately uses Serializer::reference
for recursive worldThing. RPlan::serialize instead stores inner revision and opaque
thingData byte array before InventoryItemDetails. Revisions/MemoryInputStream define
field gates and integer widths. This favors a bounded external plan-prefix oracle;
no code reuse or fulllevel compatibility claim. [Study](level-plan-recon.md).
