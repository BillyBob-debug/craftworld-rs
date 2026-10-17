# D402: Popit page titles from the game's text table, and the play card's Pod button (Popit part 3)

Date 2026-10-17. Before: the viewer's inventory page titles were hand-typed English strings,
and the play-mode card always had Retry (3) and never the Pod Decoration button (10). Now
each title is the open page's own LAMS key looked up in the game's English table (the old
strings stay as the fallback), and the play card follows `Poppet.DoMainMenuPlay`: in our
own Pod as party host the card is `[[6, 10], [4, 0], [7]]`, elsewhere `[[6], [4, 0], [7, 3]]`.

## FOUND
- `InventoryPage.GetTitleKey` is native: it returns a LAMS key stored on the page. The page
  comes from the profile's inventory collections; a new profile gets them from a native
  default builder (EBOOT 01.33, around 0x465688), which creates each collection with its own
  title key and then its pages, each with (type, subtype, title key). Read as data only.
- Default pages per collection (title via the 0000799b English table):
  - 0 Customise Character: Costumes, Head, Body, Materials, Player Colour.
  - 1 Stickers & Decorations: Stickers, Decorations, PlayStation Eye Camera, My Pictures,
    Community Photos.
  - 2 Goodies Bag: two shared pages (Materials, Collected Objects), My Objects, Community
    Objects.
  - 3 Tools Bag: Tools, Gadgets, Gameplay Kits, Music, Sounds, plus shared pages.
  - 4 Heart Items: Tools and Used Objects, Heart Items.
  - 8 Pod Decoration: My Pods.
- Keys used by the viewer: Body 0x9843f40a, Stickers 0xdae7f79a, Community Objects
  0xfead43d3, Tools 0x3bd3366a, Tools and Used Objects 0x5036ae03, My Pods 0x15f8630f,
  Sounds 0xea9b0d51, Heart Items 0x4d45f1f1.
- Which page is open when a collection opens is the profile's (saved current page), so the
  measured titles (Body, Stickers, Community Objects, Tools / Sounds, Tools and Used
  Objects, Heart Items in play mode, My Pods) stay MEASURED; they are all pages of the
  collections above.
- `DoMainMenuPlay` (Poppet listing): button 6; then 10 only when
  `CurrentLevelIsOurPod()` and `AmIOverallPartyHost(player)`; break; 4, 0; break; 7; then 3
  only when not `CurrentLevelIsOurPod()`.

## Built
- `lbp_ui_script::popit::answers(edit, our_pod)` answers `Game.CurrentLevelIsOurPod`;
  `run_card` and `run_responses` take `our_pod`. The viewer passes "playing in the Pod"
  (`--pod-shell`), and its fallback play rows follow the same rule. The viewer's Pod has no
  Popit yet (D281: Square is the pod computer's engage button there), so the Pod card is
  only seen through `ui_trace` for now; a capture of the blank level's play card is
  unchanged.
- `ui_trace SNAPSHOT GAME-DIR Poppet popit-pages` prints the card rows and the
  button-to-collection map for create, play, and play in our Pod.

## Result
- `ui_trace ... Poppet popit-pages`: create `[[6, 1, 8], [4, 2, 0], [7, 5, 3]]`, play
  `[[6], [4, 0], [7, 3]]`, play in our Pod `[[6, 10], [4, 0], [7]]`; the button-to-collection
  maps are D401's in all three.

## Open
- Popit in the viewer's Pod (when Square is not on the computer).
- Page switching inside a collection (the other default pages) is not drawn.
- Highlight sizing 264 / style 4 vs the 68 px focus tile (from D400).
