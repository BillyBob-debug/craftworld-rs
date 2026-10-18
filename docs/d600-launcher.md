# D600: one exe for friends (launcher, ISO reading)

Date: 2026-10-18. Scope: the viewer executable as the only file a friend needs.

## What it does

- Double-clicking `lbp-bevy-viewer.exe` with no arguments (or `--launcher`) opens a small
  menu (`crates/lbp-bevy-viewer/src/launcher.rs`).
- The only inputs are the player's own LittleBigPlanet 2 ISO (USA, BCUS98245) and the 01.33
  update installed by RPCS3 (`dev_hdd0/game/BCUS98245`; RPCS3's own folder or its
  dev_hdd0 is accepted and the usual RPCS3 places are guessed). Owner rule (2026-10-18):
  nothing decrypted is ever required; other regions later.
- Set up copies the files the viewer reads out of the ISO (`launcher/iso.rs`: ISO 9660,
  PARAM.SFO title check, everything under PS3_GAME except videos, audio and packages;
  462 files, 1.2 GB) and builds the index with the `lbp-cli` scanner in-process (now a
  library, `lbp_cli::run`). Here: 25 s in all. Kept in `%LOCALAPPDATA%/craftworld-rs`.
- Each mode starts as a child process of the same exe; an early exit shows the end of its
  log in the menu. `--launcher-setup ISO UPDATE` does Set up without the window.

## Modes

Open: Rookie Test, Sackboy on the Moon, the asset explorer (game modes hide the panels).
Not yet: the Pod, Popit and Story. Their behaviour comes from the game's 426 compiled
scripts, which exist only in the encrypted `patches/patch.edat` (D133); without them the
Pod's camera is never placed. They switch on when the viewer has its own fallback (Popit
lane for the Pod/Popit, Story lane for Story). No key is added to the repository.

## Fixes found on the way

- RPCS3's install holds the game's own `patches/cumulative_0133.farc`, whose table is not in
  hash order. The scanner saves tables sorted, so the attach check now sorts the parsed table
  the same way and finds candidates by their table index (`lbp-catalog/src/attachment.rs`).
  Before: "Source FARC entry differs from snapshot".
- Windows were shown only once placed on a second monitor; a one-monitor PC now shows them
  where they are (`place_on_second_monitor`).

## Checked

From the ISO plus the RPCS3 install only (no decrypted overlay): Rookie Test and the Moon
render Sackboy in the level; the explorer opens. Packaging: `local-analysis/friends-build/`
(ignored), not published.
