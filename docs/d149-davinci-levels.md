# D149: Da Vinci's Hideout story levels in Bevy level mode

Date: 2026-10-08 (America/Chicago). Input: 01.33 (`a1-disc` + `a1-update-0133` +
`local-analysis/d133-edat-overlay`), snapshot `local-analysis/d133-inventory-0133-edat.json`.

## Finding the levels

`lbp2story_slots.slt` (82 slots) groups the Da Vinci's Hideout planet as group slot 22.
New opt-in flag `--slot-browser-slots` (with `--slot-browser`) prints one line per slot:
list index, slot number, root level, group, name. Group 22 members:
Rookie Test g124806, Grab and Swing g124819, Final Test g124814, Hedge Hopping g124916,
Gripple Grapple g124808, Bravery Test g124821, Tower of Whoop g124810. The planet slot
itself (22) has no root level.

Note: the d133 snapshot records `USRDIR/patches/patch_edat.farc`, so the game-dir list
must include `local-analysis/d133-edat-overlay`; without it attach fails ("required input
... absent").

## Per-level counts (`--level-report`, unit: Things in the level's tag table)

| Level | GUID | Things | RENDER_MESH placed / total | Distinct meshes | Neutral-group Things | Shapes extruded+placed / SHAPE | Flat-fallback shapes | Decorations not drawn | Backdrop primitives |
|---|---|---|---|---|---|---|---|---|---|
| Rookie Test (`main_01_rookie_test`) | 0001e786 | 3,247 | 455 / 455 (122 from JOINT endpoints) | 49 loaded, 0 failed | 130 | 599 / 857 | 34 | 51 | 1,310 / 1,310 (mesh 00015317) |
| Grab and Swing (`main_02_grab_and_swing`) | 0001e793 | 2,163 | 439 / 440 (109 from JOINT) | 42 / 0 | 69 | 589 / 1,061 | 32 | 84 | 1,310 / 1,310 |
| Hedge Hopping (`side_02_hedge_hopping`) | 0001e7f4 | 875 | 163 / 163 (31 from JOINT) | 28 / 0 | 83 | 171 / 318 | 17 | 15 | 1,310 / 1,310 |
| tutorial_23 (baseline) | 0001db4f | 91 | 14 / 14 | 8 / 0 | 2 | 9 / 36 | 1 | 3 | 409 / 409 |

Level reader: complete for all three. Backdrop plans 000195ef / 00019316 / 000179f6 (12,
12, 15 RENDER_MESH Things drawn). Shear dropped by TRS: 0 in all three.

All three already loaded with no code change: no parse stops, no missing resource kinds.
**+0 newly viewable / +0 newly Complete** (assets unchanged; levels are not an asset count).

## Unsupported / not drawn (recorded, not guessed)

- Grab and Swing: 1 RENDER_MESH Thing has neither POS nor JOINT; skipped (placement
  source unknown).
- Shapes without GENERATED_MESH (collision only): 258 / 466 (+6 edit_only) / 147.
- Flat fallback: "diffuse multiply requires two direct textures or one texture and static
  COLOR" (33 / 32 / 16), "GENERATED_MESH without gfx material GUID" (1 / 0 / 1).
- Backdrop (Grab and Swing): 4 primitives flat, "sampler must be static, unwired UV0/UV1".
- DECORATIONS entries not drawn (51 / 84 / 15). JOINT-placed Things: no skinning,
  angle/renderScale not applied (D120 approximation).
- `--preview-lighting` overview: the stored fog range (Rookie Test: linear -1,062..45,237
  level units) is meant for the gameplay camera; the overview camera frames the whole
  level (~50,000 units wide, wider with backdrop) and fog covers almost everything
  (capture `d149-davinci-rookie-test.png`). Not changed: remapping fog to an overview
  camera would be a guess. Use the unlit view for overviews.

## Visual check

`local-analysis/d149-davinci-rookie-test-unlit.png` (window on non-primary monitor 213v0):
recognisable Da Vinci workshop, scaffolds, flying machine, house backdrop, upright and
placed sensibly; grey neutral groups on unsupported-material Things. Not committed.

## Commands

Env: main-checkout toolchain; `GD = a1-disc;a1-update-0133;local-analysis/d133-edat-overlay`.

- `lbp-bevy-viewer --slot-browser --snapshot SNAP --game-dir GD --slot-browser-report --slot-browser-slots`: exit 0, 164 slots.
- `lbp-bevy-viewer --snapshot SNAP --game-dir GD --level-guid {0001e786,0001e793,0001e7f4,0001db4f} --level-report`: exit 0 each.
- From the main checkout (capture must be under `./local-analysis`):
  `--level-guid 0001e786 --qa-hide-ui [--preview-lighting] --qa-capture local-analysis/d149-davinci-rookie-test[-unlit].png`: exit 0.

## Not tested

Grab and Swing and Hedge Hopping in a window (headless counts only); the other four group-22
levels; `--shape-bevels` on these levels; gameplay/logic; legacy viewer.
