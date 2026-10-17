# D407: Popit Backgrounds order and the worn colour's check mark (Popit part 8)

Before: the Backgrounds page listed every background plan in the game data by title, so it
opened on Toy Story's Andy's House, repeated titles (three Avalonia tiles) and showed DLC
and Pod copies. The Player Color page had no check mark. After: Backgrounds opens on
Avalonia and its first six tiles match the game frame tile for tile; the colour Sackboy
wears carries the game's white check mark.

## MEASURED (RPCS3 frames 2026-10-17, 1280x720, kept in ignored local-analysis)
- Backgrounds, top to bottom: Avalonia, Blank Background, Blank Level Background, Da
  Vinci's Hideout, Eve's Asylum, The Canyons (matched by icon and border colour against
  the decoded plan icons).
- Player Color: a white tick over the secondary square of the worn colour (entry 1 on the
  owner's profile), from the square's left middle down to its lower centre and out past
  its top right corner, about 15% of the square wide.

## INFERRED
- The page is sorted by title ("The" is not skipped: The Canyons follows Eve's Asylum)
  and holds one tile per owned item; the owner's profile has no DLC backgrounds (no
  Caribbean Island, DC Comics, Edna Mode's Home, Monster Kit Cave, Spaceship ...).
- Without a profile reader the viewer stands in for the profile's inventory with the
  story and LBP1 theme backgrounds: it drops plans with a `dlc` path, "Kit" or "Pack" in
  the title or location, a location naming a pack rather than the place itself (Toy
  Story for Andy's House), the Pod's copies and plans titled by file name, then keeps the
  first plan of each title.
- Without a profile the worn colour is palette entry 0, the default a new profile holds.

## Built
- `owned_backgrounds` picks the Backgrounds plans; `plan_sections` lays them out as before.
- `swatch` takes `worn` and draws the tick (`tick_cover`, two round-capped strokes).

## Open
- Reading the profile's inventory would replace the DLC heuristic for every page.
- `--profile-records` colours do not reach the Popit yet, so the check stays on entry 0.
