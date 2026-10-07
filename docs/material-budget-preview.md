# D66 per-mesh material budget — 2026-10-07

Before D66 a mesh previewed at most 8 distinct materials (lbp-preview) and 9 draws
(Bevy adapter: 8 materials plus one neutral bucket). Further groups failed with
"eight distinct material preview budget". The count stood in for a memory bound:
8 materials × up to four bounded textures ≈ 512 MiB raw plus 128 MiB derived pixels.

## Rule

- lbp-preview: up to 64 distinct material loads per mesh, plus an aggregate
  **retained-bytes** budget of 640 MiB (the old implied total). Retained bytes are
  measured after each successful load: decoded primary/second/third/fourth/product
  pixels, derived tint/product pixels and sampling/raw coordinate arrays
  (`MaterialAsset::retained_bytes`). A load that would exceed the total fails for
  its groups with "mesh preview memory budget", is cached as failed and is not
  retained. Count check precedes any read; failures retain nothing.
- Bevy adapter: `MAX_DRAWS = 65` (64 materials + one neutral bucket). GPU image
  memory follows the retained-pixel budget; the 50 000-triangle cap is unchanged.
- Each load keeps its existing per-material read, decompression, pixel and
  cancellation limits. Worst-case cumulative stored reads and expansion therefore
  scale with the 64-load cap (8× the old worst case); retained memory does not.
- Legacy viewer unchanged.

No format knowledge changed; this is an authored resource policy.

## Measurement

Mesh dependency census (2716 mesh resources by hash): materials per mesh 1: 1187,
2: 721, 3: 401, 4–8: 383, more than 8: 22 (max 55 by dependencies).

Counts (distinct catalog GUID; "full diffuse" = all diffuse groups prepared):
- New cohort d66-many-material-mesh-samples.json (all 22 meshes with >8 materials,
  8013469 bytes): 22/0/0 → 22/5/0; groups 157 → 449 of 504; geometry identical.
- D65 third-texture cohort: 48/29/0 → 48/30/0; groups 134 → 212 of 235.
- D61 48 cohort 48/43/0 and D64 squared 26/23/0 unchanged.
- **+0 newly viewable / +0 newly Complete.** No mesh reached the 640 MiB budget.

Note: compatibility_census does not run the Bevy adapter's draw grouping, so before
the adapter change the census counted cosmos_room_default_lbp2 (0001d050) as
viewable while the Bevy viewer refused it ("material draw budget"). Found by native
capture; fixed in the same milestone. Native capture after the fix: 58/59 groups,
1 neutral (unresolved resource), 24 s including startup.
