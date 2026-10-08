# D106 — Toolkit level oracle, L0 census and L1 scene dump (2026-10-08)

Implements backlog items **L0** (story level census + Toolkit RLevel oracle) and **L1**
(deterministic oracle scene dump) from [level-research.md](level-research.md) sections
5-7. Oracle tooling only: it runs the pinned Toolkit/cwlib jar locally and is never part
of the product. No Rust change, no Toolkit code copied into Rust, no game bytes or
oracle output committed (all under ignored `local-analysis/`).

## Files

| File | Role |
|---|---|
| `scripts/LevelOracle.java` | Authored adapter: independent wrapper preflight, then Toolkit `RLevel` deserialize, projection to versioned JSON (schema 1, scope `lvlb_toolkit_rlevel_summary_and_scene_v1`) |
| `scripts/run-level-oracle.ps1` | Pinned-Toolkit and clean-checkout check, `javac`/`java` from `local-analysis/toolchain/java`, `--self-test`, then the run over DATA.FARC + every `patch_*.farc`. `-ProjectRoot` selects the data checkout (worktree scripts, main-checkout data) |
| `scripts/level_report.py` | `manifest`: snapshot → sample manifest (every confirmed `level`, `--scene` flags, `--only-scene`). `summary`: oracle JSON + snapshot + mesh corpus census + diffuse-route census → L0 report |
| `.gitignore` | Allowlists `scripts/LevelOracle.java` like the other authored oracles |
| `tests/test_level_report.py` | Synthetic snapshot/oracle metadata only: ordering, scene flag, ambiguous candidates, overwrite refusal, pin check, counts, byte-identical repeat |

## Evidence

- CONFIRMED_FROM_COMMUNITY_CODE, Toolkit `d6a2ef80b5a09999f08f23a803b516096055e5c9`
  (local clean checkout; upstream links as in [level-plan-recon.md](level-plan-recon.md)):
  `resources/RLevel.java` serialize (cross-play hash array when head > 0x3e6, then
  `worldThing` reference), `structs/things/Thing.java` serialize (part order, flags),
  `enums/Part.java`, parts `PPos`, `PRenderMesh`, `PShape` + `components/shapes/Polygon`,
  `PGeneratedMesh`, `PRef`, `PWorld`; `io/serializer/Serializer.java` (`reference` tag
  table `visited`, Thing stack `things`, overridable `log`); `types/SerializedResource.java`
  wrapper (head ≥ 0x271 branch u16+u16, flags byte at head ≥ 0x297 or Leerdammer 0x272
  rev ≥ 2, compressed bool) and `util/Compressor.decompressData` (u16 flag, i16 chunk
  count, u16 pairs). `PPos.fixup` recomputes `localPosition` for head ≥ 0x341, so the
  dumped local matrix is a Toolkit derivation, not stored data; world matrices are stored.
- CONFIRMED_FROM_LOCAL_DATA (01.33 overlay snapshot `d68-inventory-0133-verified.json`,
  user's BCUS98245 disc + update): every count below. Applicability: these 398 resources only.
- STRONG_INFERENCE: `gamedata/lbp2/th_*` (minus `th_tutorials`) are the six LBP2 story
  themes; `gamedata/levels/00_developer_levels_episode_*` and `goty_levels` are LBP1
  levels (heads 529..622 are LBP1-era revisions). Directory names only; no slot/pack
  resource was read to confirm (check 1 second half: not answered).
- UNKNOWN: visual correctness of anything; what a "tutorial" level looks like in game.

Guards (authored, not Toolkit's): stored ≤ 16 MiB, declared expansion ≤ 64 MiB summed
from the chunk table before any inflation, chunk bytes within payload, dependency count
≤ 65,536, head 0x109..0x3f8, only `LVLb` (encrypted `LVLe`/text blocked), SHA-1 check of
every read, ≤ 1,024 samples, ≤ 1 GiB per run, 1 h time budget, 1 GiB dedicated thread
stack (inline references recurse), `StackOverflowError` caught per level, new output only
under `local-analysis/`. Toolkit wrapper fields must agree with the preflight.

## What the oracle reports

Per level: snapshot identity (MAP path, GUID, SHA-1, archive, stored bytes), status
(`parsed_exact` = Toolkit consumed every expanded byte, `parsed_trailing_bytes`,
`rejected` = guard, `failed` = Toolkit exception), preflight header (head, branch,
branch revision, compression flags, expansion), Toolkit revision string and subVersion,
expanded/consumed/trailing bytes, cross-play hash count, reference tag count, Thing count
(distinct Things in the tag table), max Thing nesting depth (Serializer Thing stack,
sampled at every `log` call), world summary (UID, PWorld presence, listed things,
backdrop refs, world parts), part histogram (Things per Part name), counts (POS,
RENDER_MESH, SHAPE, GENERATED_MESH, SHAPE+GM, RM+SHAPE, shape vertices), and descriptor
histograms for render meshes, shape (physics) materials, gfx materials, bevels, REF
plans, plus boneThings length and polygon vertex-count histograms.

Scene dump (`"scene": true` samples, L1): per Thing in tag order: tag, UID, parent /
group head / old emitter (tag + UID), plan GUID, flags, present parts; POS world and
Toolkit-recomputed local m44 (column-major, JOML order), bone-of ref, anim hash;
RENDER_MESH mesh descriptor (GUID/SHA-1), bone count and bone refs, anim descriptor,
editor colour, visibility, shadow type; SHAPE vertex count, `requiresZ`, loops, all
vertices, material/oldMaterial, thickness, bevel size, flags, colour, brightness,
behaviour, static, zBias, LBP2-shape bit; GENERATED_MESH gfx material, bevel, uvOffset,
plan GUID, visibility, uvMode, textureScale, noBevel, sharded, includeSides; REF plan.

## L0 results (section 7 checks)

Unit: distinct level resource (MAP row, confirmed type `level`) on the 01.33 overlay.
398 rows, each with exactly one archive candidate (DATA.FARC 221, 11 patch archives 177).

1. **Prefixes**: lbp2/dlc 110, lbp2/th_tutorials 43, levels/00_developer_levels_episode_1
   87, episode_2 26, levels/goty_levels 21, lbp2/th_cosmos 17, th_da_vinci 15,
   th_eve_assylum 12, th_henry_garage 11, th_popov_factory 11, th_victorian_lab 10,
   lbp2/pod 16, levels/palettes 14, levels/pod 3, levels/hud 1, lbp2/psndemo 1.
   LBP2 story (six themes) = 76 levels.
2. **Headers**: heads 350..1015; branch 0 except 0x4c44 (20, head 626) and 0x4431
   (20 `.cpv`, head 994); flags 0 for heads ≤ 622, 7 above. 51 levels have head > 0x3e6
   (1005..1015) so the cross-play array is read; 15 of them have a non-empty array.
3. **Toolkit parse**: 385/398 `parsed_exact` (0 trailing bytes), 0 rejected, 13 failed.
   All 13 failures are Vita cross-play `.cpv` (branch 0x4431, `gamedata/lbp2/dlc/crossplay`,
   patch_0120/0121): 5 bad test marker, 6 array errors, 2 ClassCast; 7 other `.cpv` parse.
   680,391 Things in the parsed levels (max 8,048, fight_of_the_bumblebees); max nesting
   depth 29 (main_03_crazy_inferno). 40 part types occur; top by Things: POS 434,047,
   SHAPE 251,810, GENERATED_MESH 182,742, GROUP 163,782, SWITCH 148,806, BODY 122,364,
   RENDER_MESH 119,514, STICKERS 72,732, REF 65,867, SCRIPT 62,466. Full histograms
   (Things and levels per part) are in `d106-level-report-all.json`.
4. **REF plans**: 652 distinct REF plan descriptors; 385 resolve to catalog plans, 254
   GUIDs are not in the catalog, 12 hash-only. Plan Thing parsing was **not** run.
5. **RENDER_MESH vs SHAPE** (LBP2 story, 76 levels): 229,248 Things; RENDER_MESH 29,068,
   SHAPE 68,519 (all 48,375 GENERATED_MESH Things also have SHAPE; 20,144 shapes have no
   GENERATED_MESH; STRONG_INFERENCE: collision/invisible or mesh-carrying shapes), RM+SHAPE 15,054. 240 distinct
   mesh GUIDs, **240 viewable** and 159 all-diffuse per `b105-corpus-default.json`;
   29,060/29,068 RM instances use a viewable mesh. All levels: 501 mesh GUIDs, 497
   viewable, 368 all-diffuse, 2 not in catalog; 119,438/119,514 instances viewable.
6. **boneThings length** (all RM Things): 0: 29,008; 1: 68,099; 2: 4,517; 3: 15,124;
   5: 1,870; 8: 421; 44: 199; 20: 115; 43: 82; 7: 75; 4: 4. Mesh bone counts not projected.
7. **Bevels**: 144 bevel resources in the catalog (heads 306..1014; 10 on branch 0x4c44).
   134 distinct bevel descriptors used, 131 resolve to catalog bevels. GENERATED_MESH
   uses 439 distinct gfx materials: route classes (b105 census) texture_route 347,
   instance_color_only 33, solid_color 30, unsupported 11, no_route_row 9,
   animation_base_only 7, unwired_black 1, non-GUID 1. The one unsupported material in
   almost every story level is `gmat/proc_base_hologram.gmat` (diffuse multiply).
8. **Streaming**: 0 STREAMING_DATA / STREAMING_HINT parts in any parsed level.

## L1 scene dump and candidates

Scene manifest: five candidates. Two runs byte-identical (SHA-256 `E63C5C12…95B3`,
7,228,236 B); the 398-level summary run is byte-identical too (`57296B47…6EB0`), and two
`level_report.py summary` runs match.

| Level (MAP path) | Stored B | Head | Things | Depth | Part types | RM (viewable inst.) | Mesh GUIDs viewable / all-diffuse | SHAPE | GM | Gfx mats (route classes) |
|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---|
| `gamedata/levels/pod/pod.bin` | 2,948 | 919 | 19 | 3 | 14 | 4 (4) | 3/3 / 3 | 9 | 1 | 1 (solid 1) |
| `gamedata/lbp2/th_tutorials/levels/tutorial_23.bin` | 11,798 | 926 | 91 | 8 | 20 | 14 (14) | 8/8 / 6 | 36 | 9 | 4 (texture 2, solid 1, unsupported 1) |
| `gamedata/lbp2/th_da_vinci/final_levels/cutscene_intro_theme1/cutscene_intro_theme1_b.bin` | 25,646 | 944 | 237 | 7 | 22 | 11 (11) | 8/8 / 3 | 103 | 97 | 4 (anim-base 1, instance 2, unsupported 1) |
| `gamedata/lbp2/th_henry_garage/final_levels/comp_01_chick_flick/comp_01_chick_flick.bin` | 78,426 | 945 | 586 | 12 | 30 | 195 (195) | 35/35 / 16 | 329 | 197 | 20 (texture 10, solid 5, instance 3, unsupported 1, non-GUID 1) |
| `gamedata/lbp2/th_da_vinci/final_levels/main_01_rookie_test/main_01_rookie_test.bin` | 277,551 | 950 | 3,247 | 18 | 32 | 455 (455) | 49/49 / 30 | 857 | 599 | 19 (texture 11, solid 5, instance 1, unsupported 1, non-GUID 1) |

Observations from the dumps: every RM Thing in these five has POS except 2 in pod.bin, 2
in chick_flick and 122 in rookie_test (JOINT-attached meshes: placement must come from
bones). Most RM Things carry `bone_of`/boneThings, so "own POS" placement is a first
approximation. `PWorld.things` lists more entries than distinct Things (e.g. 352 vs 237),
so the list holds repeats or nulls; Thing count uses the tag table.

**Recommendation: `gamedata/lbp2/th_tutorials/levels/tutorial_23.bin`** (GUID 0001db4f,
SHA-1 e2534f8d…002a, DATA overlay) as the first level for L2. Smallest LBP2-era level
that still exercises every visible route: 91 Things, 20 part types (tied for the fewest
among LBP2 non-pod levels, so a later Rust reader can reach byte-exact early), depth 8,
all 14 RM instances on viewable meshes (6 of 8 GUIDs all-diffuse), and 36 shapes / 9
generated meshes with only 4 gfx materials. pod.bin is too thin (one generated
mesh) to test shapes. Next target after it: **main_01_rookie_test** (first story level
of the first theme; all 455 RM instances viewable, 857 shapes). Tutorial levels' in-game
appearance is UNKNOWN, so the L2 visual check needs a user screenshot or video reference.

## Commands (from the main checkout data, scripts from the worktree)

```powershell
python scripts/level_report.py manifest local-analysis/d68-inventory-0133-verified.json local-analysis/d106-level-manifest-all.json
scripts/run-level-oracle.ps1 -ProjectRoot <main> -SampleManifest local-analysis/d106-level-manifest-all.json -Output local-analysis/d106-level-oracle-all.json   # 398: 385 parsed_exact, 13 failed; repeat -run2 identical
python scripts/level_report.py summary local-analysis/d106-level-oracle-all.json local-analysis/d68-inventory-0133-verified.json local-analysis/b105-corpus-default.json local-analysis/b105-diffuse-route-shapes-0133.json local-analysis/d106-level-report-all.json   # repeat identical
python scripts/level_report.py manifest local-analysis/d68-inventory-0133-verified.json local-analysis/d106-level-manifest-scene.json --only-scene --scene <5 paths above>
scripts/run-level-oracle.ps1 -ProjectRoot <main> -SampleManifest local-analysis/d106-level-manifest-scene.json -Output local-analysis/d106-level-scene-run1.json   # and -run2: byte-identical
python -m unittest discover -s tests   # OK (skipped=1)
```

`LevelOracle --self-test` (run by the script each time) passes: header fields, every
truncation, encrypted/wrong magic, head above 0x3f8, negative dependency count, bad
compressed bool, 2 GiB declared expansion, chunk overrun, Leerdammer flags byte.

## Limits and not tested

- No plan (`RPlan` thingData) parsing, so REF backdrops/emitted objects are unexpanded.
- Mesh bone counts, RBevel contents, decorations/stickers, PLevelSettings not projected.
- The 13 Vita `.cpv` failures are a Toolkit limitation at the pin, not investigated.
- No synthetic full-RLevel round trip in the self-test (Toolkit write path not exercised).
- No viewer, no native window, no Rust build (no Rust change). "Viewable" is the b105
  mesh census definition; no level asset is Complete.
