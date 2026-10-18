# Handoff: Sackboy model lane, part 1 (PC, 2026-10-18)

## Read first
1. `docs/d700-sackboy-model.md` (D700 and its Open list).
2. `crates/lbp-preview/src/linked.rs`: `load_material` (D338 and D700 blocks), `alpha_from_colour_sampler`.
3. This file's Facts.

## State
- PR #142 (branch `sackboy-model`, worktree `E:\Projects\wt-sackboy-model`): D700, stitch
  and zip overlays blend by their texture's alpha. rust and python checks are green, so the
  coordinator merges it. This handoff is a second commit on the same branch.
- RPCS3 is closed. A subagent's run using `rpcs3_launch.py` got only white frames, and the
  window took focus once after a minimise/restore (the watchdog killed it). Any new RPCS3
  launch needs a fresh ask from the owner.

## Facts
- The play-mode Pod view matches the RPCS3 frame `local-analysis/d156-rpcs3-pod-default.png`
  (Sackboy crop 590,480..690,600). Command (disc and update only): `--snapshot local-analysis/d79-inventory-0133.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --play-sackboy --level-scripts --play-sackboy-input "-:470" --level-scripts-hold-at 700 --window-16-9 --qa-hide-ui --qa-capture local-analysis/NEW.png`; ours crop 590,495..690,615.
- Run the viewer from the canonical repo folder (the snapshot paths are relative), using the worktree's exe.
- Sackboy's materials: knit `000025e2`; stitches `00001d92`; zips `00001d94`; zip lining
  `00001d91` (pink knit `cd7` x AO `427f`); metal zip pull `00000439` (brushed `684` x AO `e93`,
  with port 2 a product); eyes `0000053c` (black, specular 1.81). 29 morphs, see `morph_probe`.
- Comparison sheets (ignored): `local-analysis/d700-stand-compare.jpg` (main, PR, game).

## Next
1. Eyes: in the matched view their size and highlight already agree with the game. Ask the owner
   where they look wrong (close-up, Popit customise, the static preview with big glossy eyes).
2. The upper chest zip is dark grey in ours and light in the game: probably the metal pull `00000439`
   drawn matte (no specular from its product port 2) or the lining `00001d91`. Hide each group to see which.
3. Decision numbers D701 and up.
