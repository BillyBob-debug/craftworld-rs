# P2g: closing the first three Pod gaps vs RPCS3 (2026-10-08)

Gap list: docs/pod-rpcs3-comparison.md (PC, D165/D166 captures beside RPCS3).

## 1. Side-panel art (pod.mol 00002f6c primitive 2, runtime slot 0)
Both candidate textures drew wrong with UV0 untransformed (000027fb black, 00004277 flat
lavender), which fits a sampler that uses another UV channel or a scale/offset.
- `runtime_texture::slot_sampler` finds the material graph's own sample box for the slot
  (type 1, parameter 5 = slot) and decodes it with the diffuse routes' static-sampler
  rule (`linked::sampler`): UV channel and scale/offset or matrix transform
  (CONFIRMED_FROM_LOCAL_DATA per material once the PC run prints it).
- The override samples that channel through that transform. If the sampler cannot be
  read it falls back to UV0 untransformed and says why.
- The status adds one line per rule: the sampler and the box types on the path from
  the sample to the output, so tinting (still none, INFERRED) can be decided from the
  graph rather than guessed.
- `linked::read_material_graph` is factored out of `load_material` (unchanged
  behaviour) so the override can read the material.

## 2. Open view: tent still in frame
RPCS3 shows no tent once the Puter is open; ours kept the arch and panels because the
whole shell shares the DualShock-fitted eye.
- pod.mol (mesh 00002f6c) now hangs off its own root (`PodTentRoot`,
  `PodRig::tent_root`). Its eye matches the shell's when closed and ends just past the
  shell's far side when open, so the tent sweeps out of frame during the dolly
  (STRONG_INFERENCE from the RPCS3 end states; the mid-dolly frame will check it).
- The DualShock, cords and Sackboy stay on the shell root.

## 3. Closed view: window about 0.06 h too high
The window measured 0.21-0.64 h (RPCS3 0.28-0.69) with the DualShock within 0.01.
- The tent eye is tilted up by `TENT_TILT_CLOSED` = 0.06 of the frame height
  (atan(0.06 x 2 tan(fov/2)), about 2.8 degrees at 45). This is a correction fitted to
  that one measurement; a tilted game camera is the INFERRED cause.
- `--pod-shell-camera` replaces the tilt with the preset's own orientation.

## Tests
- `slot_sampler_reads_the_slots_sample_box_and_its_output_path` (UV1, scale 0.5, path
  [3, 0], missing slot, loop skip, bad wire refused).
- `apply_replaces_shared_solid_once_and_counts_groups` now samples UV1 at scale 2.
- `p2b_rig_keeps_each_layer_where_its_reference_puts_it`: the tent shows
  `TENT_TILT_CLOSED` lower than the shell closed and is wholly behind the eye open.

## PC check
B = `lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f --play-animation 0001a697 --pod-hold-animation 000042b2 --animation-frame 0 --qa-hide-ui`
1. B `--runtime-texture 00002f6c:0:000027fb` closed; 2. same with 00004277;
3. B `--pod-open`. Report the "material sampler" line, the window fractions closed, and
whether any tent shows open.
