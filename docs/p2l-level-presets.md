# P2l follow-up: lighting presets, the shell's own light, globe extent probe

Date: 2026-10-09 (Central). Follows docs/p2l-lbp-lighting.md and the PC's D195/D199
measurements (local-analysis/d195-pod-settings.json, d199-pod-open-lbplight.png; not committed).

## Stored presets (`--lighting-preset N`)

The reader keeps up to 16 preset `LevelSettings` per PLevelSettings
(`ThingData::level_settings_presets`, `LevelSettings::find_presets`, the scene's
`settings_presets`). They were read and dropped before. Same field order as the own
settings (Toolkit PLevelSettings, CONFIRMED_FROM_COMMUNITY_CODE; D195 read both Pod files to
the end with 0 trailing bytes, CONFIRMED_FROM_LOCAL_DATA).

`--lighting-preset N` lights a level (and the Pod shell) from preset N instead of the
main block. Default: main block, unchanged.

Why: pod_planets' main block holds defaults (sun (1,0.9,0.8), ambient (0.2,0.3,0.4)),
while preset 0 holds the tuned values (warm sun (0.97,0.54,0.09), teal ambient
(0.21,0.73,1.0), exposure 1.22). Which block the renderer applies, and how PWorld factors
(all 0 in pod_planets, `hasLevelLightingBeenSetup` false) blend them, is UNKNOWN. Using
preset 0 is INFERRED from the D199 colour gap (RPCS3 shows a saturated sea and an
olive Moon).

## The Pod shell keeps its own light

Before: `spawn_level` set the viewer light from the last level spawned, so with
`--pod-shell` the shell's settings (pod.bin: ambient ~0.9, multiplier 1.23, exposure 1.84) lit
the planets too, turned by the planets root. That matches D199's grey-white globe.

After: planets entities use pod_planets' light turned by the planets root; shell and
figure entities use pod.bin's light turned by the shell root (`LightingState::shell`).
One camera fog (the planets') is kept.

## Globe extent probe (`pod_globe_extent` example)

Tests the size hypothesis: we place every vertex by POS x invSkinPose(bone 0) (D120), but
the Earth's vertices are skinned 1:1 to bones 1 (sea) and 2 (land), whose boneThings
(Things 2, 3) have scale (0.9756, 0.9992, 0.961) and whose skin scales are
(0.7215, 0.8579, 0.7107), not bone 0's uniform 0.7116. The probe prints the world
half-extent and radius under both rules.

```
cargo run -q -p lbp-preview --example pod_globe_extent -- local-analysis/a2-inventory-verified.json game-data/a1-update-0133;game-data/a1-disc <pod_planets path> 1
```

Not run from the cloud (no game data). If the per-bone radius is about 1/1.26 of the
bone-0 radius, the fix is per-bone rigid placement for level models with boneThings.
