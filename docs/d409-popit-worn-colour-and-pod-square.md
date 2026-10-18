# D409 / D410: the worn colour's check mark, and Square for the Popit in a Pod input script (Popit part 9)

Before: the Player Color page always ticked palette entry 0, and in the scripted Pod the
Popit could only be forced open from the first frame (`--popit-open`), drawn over the view
from inside the shell before the planets script places the camera. After: the tick follows
`--profile-records`, and an input script can walk Sackboy away from the pod computer and
press Square there, so the card is drawn where the game draws it.

## D409: worn colour
- `--profile-records PATH`: the profile's first colour (D342: the player's colour) picks
  the ticked swatch; its RGB is compared with each palette entry's primary. Without the
  flag it stays `DEFAULT_PLAYER_COLOUR` (palette entry 0).
- `sackboy_play::install` reads the file and passes the colour to
  `popit_play::load_assets`, which bakes the swatches.
- Check: with `colours 0xffceb424 ...` (entry 1) the tick sits on the second swatch.

## D410: Square from an input script in the Pod
- The `e` key of `--play-sackboy-input` already clicks the engage button on its step's
  first tick (D281). It now sets `PodSackboy::popit_square` too; `popit_input` takes it as
  Square, under the D403 rule (ignored while Square is the pod computer's).

## MEASURED (viewer vs the RPCS3 frame of 2026-10-11, play mode, own Pod)
- Viewer: `--level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --play-sackboy
  --level-scripts --play-sackboy-input "-:470,l:23,-:20,e:1,-:300"
  --level-scripts-hold-at 700 --popit-mode play` (planets camera at rest from tick 452).
- Same camera, Sackboy left of the computer, the five-button play card above him with its
  tail down to him; card size and place match within a few px.

## Open
- Tooltip pill: in the Pod frame (focus top-left of a two-column card) the pill sits above
  and to the right, its tail from the pill's left end down to the tile's top right corner;
  ours centres it over the tile (D235's top-row rule). The Moon create frame with focus on
  the top-right tile shows the pill to the tile's right. The game's rule is not yet found.
- Highlight sizing 264 / style 4 (D400) still not tied to the 68 px focus tile.
