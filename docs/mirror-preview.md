# D60 MIRROR and independent product axes — 2026-10-06

Static single textures accept MIRROR2 on either axis. Bevy stores independent
Repeat/MirroredRepeat/ClampToEdge U/V modes. Legacy accepts uniform Mirror via
explicit epaint wrapping; mixed axes remain rejected. Two-source diffuse MULTIPLY
preserves each sampler's typed axes and derived UVs on the existing GPU path.
Integer LCM baking still requires Repeat on both axes of both sources. Constant
product MIX and three/four-source shared-coordinate chains remain Repeat-only.
General BORDER and modes5–8 remain unsupported. No disk parser or shader change.

## Evidence, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [TextureWrap::getValue/fromValue](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java)
  assigns MIRROR2; [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java)
  preserves independent wrapS/wrapT. MaterialBaker/MultiMaterialBaker wrap mapping
  uses GL_MIRRORED_REPEAT. Evidence only; no reader code reused.
- `INFERRED`: authored frontend policy maps2 to pinned Bevy0.19.1
  [ImageAddressMode::MirrorRepeat](https://docs.rs/bevy_image/0.19.1/bevy_image/enum.ImageAddressMode.html)
  and epaint0.36.2 [TextureOptions::LINEAR_MIRRORED_REPEAT](https://github.com/emilk/egui/blob/0.36.2/crates/epaint/src/textures.rs).
  Locally inspected APIs support explicit mapping. Boolean compatibility helpers
  returnNone for Mirror; uniform-only callers use typed enum. No UV folding.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00; five selected clear GMT931 branch0
  materials. Bounded metadata census5163locations/5536MAP rows reads44060339stored
  bytes; all prefix/graph/tail stages parse, no cancellation. Five full raw wrap
  arrays match pinned oracle;5diffuse routes compared,0mismatches. Four materials
  have Mirror only in unused/other branches. Candlestick material00002883 selects
  UV0 Repeat/Repeat times UV1 Mirror/Repeat. Mesh00002487,
  hashd232581653b546e593a2543cdbf5b9ee04cef46b, gains primitive0/1278triangles;
  3/4 to4/4diffuse groups,1032vertices/1888triangles unchanged. This mesh is
  catalog-linked; no external geometry oracle was added.
- `UNKNOWN`: original game filtering/mips/alpha/full shader, bump/normal branches,
  geometry/pose correctness and corpus asset totals. Toolkit's known extreme
  ZigZag synthetic conflict remains recorded, not adopted. No new raw-array
  conflict found. API mapping does not certify original visual fidelity.

Tests cover all nine1/2/3axis combinations, missing/unknown modes, uniform-only
rejection, Bevy sampler axes/sRGB/filter/extent and Repeat-grid rejection when
either sample mirrors. Existing route/UV/cache/cancel/budget/completeness checks
remain. Census schema2 adds existing full raw wrap arrays at all successful prefix
stages; compatibility schema5 records secondary axes and marks dual products
Bevy-only. No proprietary fixtures, new shader or parser UI dependency.

## Counts and native check

Original39hash/39GUID cohort before/after single Mirror mapping stays39viewable,
35full-diffuse,0verifiedComplete. Extended40hash/40distinctGUID baseline was taken
after that mapping but before independent product addressing:40viewable,
35full-diffuse,0Complete. Final40viewable/36full-diffuse/0Complete.
**Newly viewable0; newly Complete0; one already-viewable asset gains full diffuse
coverage.** Unit: distinct catalog GUID; scope: these40static samples on Bevy.
Full diffuse coverage is preparation, not all-texture certification. Corpus asset
totals unknown; five Mirror materials are not five newly supported assets.

One hidden-start native test logs non-primary165v0 placement before showing.
One image inspection resolves new texture/UI questions: textured candlestick,
4/4diffuse groups, explicit sampler labels,1source-session viewable/0Complete,
manual texture/geometry checks unchecked. No shader errors. This does not establish
correct original geometry or all textures. App Viewable/Complete definitions and
measured distinct GUID session counts remain, verification resets on reload.
No automatic Complete classification. Legacy native Mirror not tested.
Commands, failed attempts and limits:STATUS.md D60.
