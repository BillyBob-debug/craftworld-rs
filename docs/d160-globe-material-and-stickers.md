# D160: Pod globe sea material and sticker decals

Date: 2026-10-08 (America/Chicago). Two parts, committed separately.

## Part 1: the Pod globe's grey sea

### Finding

New read-only probe `crates/lbp-preview/examples/mesh_group_probe.rs` (prints each
primitive's material GUID, index range and preview outcome; for neutral groups also the
material's texture slots, boxes and wires as numbers) on `cloth_earth` 000027bd:

| Prim | Material | Indices | Before | After |
|---|---|---|---|---|
| 0, 2 | 0001f14e, 0001d256 | 30,507 / 6,794 | texture x11 | unchanged |
| 1, 3 | 0001d254 | 12,050 / 4,766 | independent two-texture sampling (x11 / x4.4) | unchanged |
| 4-6, 8 | 000204ed..f0 | 6,351 total | texture (1, 2) | unchanged |
| **7** | **0001d257** | **4,832** | **neutral: "Unsupported: texture product pixel budget"** | **independent two-texture sampling (x12 / identity)** |

Material 0001d257 graph (numbers from the probe): TEX slot 0 (scale 12, 12) and TEX slot 1
(scale 1, 1) into MULTIPLY (type 10), MULTIPLY into output port 0 (diffuse); TEX slot 2
(scale 12) into output port 3 (not diffuse; stays unrendered and listed). All slots Repeat.

Both diffuse samples have integer scales, so the D50 nearest-grid product was selected;
its LCM grid is 512 x 12 = 6,144 texels wide, over the 4,096 per-axis / pixel budget, and
the route failed instead of falling back. Non-integer pairs (prim 1, x4.4) already used
the independent dual path (D50/D99).

### Rule and change

`lbp-preview` `linked.rs`: a **plain** two-texture product (no MIX, BLEND, ADD or
constant BLEND) whose grid bake would exceed the product grid or pixel budget now uses the
independent dual path (both textures sampled with their own transform on the GPU and
multiplied), the same output function as the grid bake per texel; only filtering differs
(linear rather than nearest-grid). Malformed extents still fail; MIX/BLEND/ADD products
still require the grid (unchanged "requires Repeat grid" errors).

| Rule | Evidence label | Source |
|---|---|---|
| MULTIPLY of two TEX samples is the per-texel product | CONFIRMED_FROM_COMMUNITY_CODE (existing D50 rule) | Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, material box types as recorded in docs/dual-preview.md; no code copied |
| Over-budget grid -> dual sampling is display-equivalent up to filtering | STRONG_INFERENCE | product is commutative per texel; both samplers Repeat |

Unit test `product_grid_over_budget_falls_back_only_on_budget` (synthetic extents: 512 x 12
vs 512 x 1 falls back; x2 does not; malformed extent and x4.4 do not).

### Counts (unit: primitive groups / meshes; scope: mesh 000027bd and level 00018fdb)

- 000027bd: neutral groups 1 -> 0 (9 of 9 groups select a diffuse route).
- Newly viewable: 0 meshes (000027bd already drew since D156, with one grey group).
- Newly Complete: 0. 000027bd still has "Other required texture slots unrendered"
  (non-diffuse channels such as port 3), so it cannot be called Complete.
- Corpus effect not measured (no census rerun); other meshes whose plain integer product
  exceeded the budget would also move from neutral to the dual route.

Capture: `local-analysis/d164-pod-menu.png` (`--level-guid 00018fdb --pod-camera 1
--pod-menu PodStateMainMenu --qa-hide-ui`; log "Preview window positioned on non-primary
monitor 214v0 before showing"; "Drawn with neutral (unsupported material) groups: 0
Things"). Visual check: the globe's oceans now show a blue woven fabric with the land
patches, no grey band. Not compared against the game.
