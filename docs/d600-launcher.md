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

Open: the Pod, Popit (in the Pod, Z), Rookie Test, Sackboy on the Moon and the asset
explorer; game modes hide the panels. The game's 426 compiled scripts exist only in the
encrypted `patches/patch.edat` (D133), so the Pod uses the viewer's own resting camera when
none load (D411, Popit lane); its planets aren't placed yet. Story waits for a script-free
fallback (Story lane). No key is added to the repository.

## Fixes found on the way

- RPCS3's install holds the game's own `patches/cumulative_0133.farc`, whose table is not in
  hash order. The scanner saves tables sorted, so the attach check now sorts the parsed table
  the same way and finds candidates by their table index (`lbp-catalog/src/attachment.rs`).
  Before: "Source FARC entry differs from snapshot".
- Windows were shown only once placed on a second monitor; a one-monitor PC now shows them
  where they are (`place_on_second_monitor`).

## Checked

From the ISO plus the RPCS3 install only (no decrypted overlay): the Pod, Popit, Rookie
Test and the Moon render Sackboy in place; the explorer opens. Packaging: `local-analysis/friends-build/`
(ignored), not published.

## D601: plain-words setup errors (2026-10-19)

A friend reported "assets failed to load". Likely first-run causes, each now caught by Set up
with a message that names the problem and the fix (README "If something goes wrong" lists them):

- Wrong thing in the disc box: a .zip/.7z/.rar, a .pkg, a .rap/.edat/.sfo, the update folder,
  a folder without PS3_GAME, a quoted path pasted from "Copy as path" (quotes now stripped).
- A disc folder instead of an ISO (RPCS3 boots both): now accepted and read in place, no copy.
- Another region: the PARAM.SFO title id is named ("That ISO is BCES..., not the USA ...").
- An encrypted disc dump: the ISO 9660 tables read fine but the files are scrambled, so the
  old Set up said Ready and every mode loaded nothing. Set up now checks every `.farc` under
  PS3_GAME/USRDIR ends in the `FARC` tag and stops with "contents are scrambled" otherwise.
- Update box: empty, a file, USRDIR picked (now accepted), wrong title, not 01.33.
- Game modes run with `--qa-hide-ui`, which hid "Load failed" entirely: the viewer now shows
  a centred "The game files didn't load" window with the reason in hidden-UI modes.

Tests: `disc_box_explains_wrong_picks`, `title_check_names_the_other_disc` and the extended
`update_folder_is_found_from_rpcs3_or_its_dev_hdd0` (launcher.rs). Not tested on the PC
against the real ISO in this change (cloud thread); the FARC tag check assumes every
retail `.farc` ends in `FARC`, as lbp-archive already requires.

## D602: keep the voice-over bank (2026-10-19)

The Story lane's D397 times the Pod's voice-over waits from the headers of
`PS3_GAME/USRDIR/gamedata/audio/voiceover.fsb`, which sits loose and unencrypted on the disc
(about 19 MB). Set up skipped every `.fsb`; it now keeps that one file (`iso::KEPT_AUDIO`).
A copy made before this lacks it, so `ready()` is false and the menu asks for Set up again,
which copies once more. Test: `lists_and_filters_a_small_image` (kept bank, skipped music).
