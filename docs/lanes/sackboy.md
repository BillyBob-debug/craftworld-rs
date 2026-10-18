# Lane state: sackboy model (2026-10-18, seeded from handoff-pc-2026-10-18-sackboy-part3.md)

## Read first
1. `docs/d700-sackboy-model.md`: the Open list (eyes, zip pull, program decoding).
2. `local-analysis/results/2026-10-18-ghidra-sackboy-eyes.md` (the EBOOT pass on the eye
   bones; may be missing if it did not finish).

## State
- Branch `sackboy-model-3` (worktree `E:\Projects\wt-sackboy-model`): the `eye_probe`
  example and the D702 notes. No visual change.
- The owner said the eyes look off all the time (2026-10-18). Nothing visual was committed:
  the 720p Pod frame can't separate a 10-20% eye size difference.
- Decision numbers: D700 series; D702 used.

## Facts
- Eye measurements (Pod frame `d156-rpcs3-pod-default.png` vs ours `d700-podplay-stand.png`):
  game eyes 7-8 px wide and 8-9 px tall, ours 10 x 10 at a 1.14x larger Sackboy. Ours look
  wider and rounder.
- `eye_probe ... 0000043f 0001a697 0000053c`: eye vertices follow only `bone_eye_left/right`
  (bones 27, 28; local scale 0.596/0.600 = bind). No morph touches an eye vertex.
- Pull gap: knit and pull programs end alike (`light^2 x P + 2 x spec`, tint, per-channel
  sqrt), so look elsewhere: the pull's light inputs (aomap u8 / `aotweak`, rim terms) or the
  texture data.
- The local `cgb.py` prints dst scales now (SRC1 bits 28-30). Its old `|abs|` on `w[2]`
  bit 29 is probably that scale field, not abs.

## Next
1. A close Customize Character game frame needs an RPCS3 launch, which needs the owner's
   fresh ask through the coordinator.
