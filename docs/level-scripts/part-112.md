# Level scripts, part 112: the launched level starts at its entrance (2026-10-18, D365)

After the switch (part 111) the viewer framed the whole launched level from outside it,
which with the level lighting's fog was one flat colour. A level the Pod launches now
starts at its entrance, as the game's camera does when the level begins.

### What changed

- `lbp-bevy-viewer` (`main.rs`): `entrance_orbit` puts the view at the game's default
  camera (`FOLLOW_EYE`, eye (296, 168, 1218) from Sackboy, straight on along -z, vfov
  60 deg; part of D208's memory read) over Sackboy's spawn: the level's first
  checkpoint plus `SPAWN_OFFSET` (0, -208, 250), as `--play-sackboy` spawns him. The
  viewer's camera keeps its 45 deg vertical FOV, so the distance is 1218 x tan 30 deg /
  tan 22.5 deg = 1698, the same view height. R still resets to the whole-level view.
- `--level-entrance-camera` asks for the same start on a level loaded directly;
  `level_play::switch_level` sets it for every level the Pod launches.

MEASURED (the part 109 runs, scripted and native, hold at 900, `--preview-lighting`):
both switch after tick 761 and start Trash Valley (0001f333) at checkpoint Thing 33,
spawn (-11560, 19306, 200): the entrance pipe on the junk floor, drawn (101807 colours,
was one fog colour in part 111).

INFERRED: the first checkpoint in reader order is the entrance (as `--play-sackboy`);
the spawn's floor rule and the game's camera lead while walking are left out. Not done:
the game's entrance animation (Sackboy coming out of the pipe), the loading screen, and
a Sackboy to play after the switch.

### Next

Sackboy in the launched level (the play mode at the entrance spawn), the warnings
(0x402b24) and the select sound; then the cursor's flight and the other searches.
