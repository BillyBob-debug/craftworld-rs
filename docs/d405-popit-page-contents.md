# D405: Popit contents for every page of a collection (Popit part 6)

Before: stepping to another page with L1 / R1 showed that page's title, counter and arrows
over an empty panel; only the profile's open page had items. After: every page with an
inventory type shows the game's plans of that type, grouped by category in the same grid
as the open page.

## FOUND (EBOOT 01.33, read as data only)
- The default profile builder `0x465688` gives each page an inventory type mask and, for
  costume pages, a sub-type mask. Read as `InventoryObjectType` bits:
  - Costumes 0x100010 (costume, user costume); Head 0x10 / 0x1fd and Body 0x10 / 0x3e02
    (costume, by piece); Materials 0x20 (costume material); Player Colour 0x80000.
  - Stickers 8; Decorations 4; PlayStation Eye Camera 0x4000; My Pictures and Community
    Photos 0x400 (user sticker).
  - Collected Objects 2 (readymade); My and Community Objects 0x80 (user object).
  - Tools 0x10000; Gadgets 0x8040 (gadget, joint); Gameplay Kits 0x200; Music 0x20201000
    (sequencer, music, instrument); Sounds 0x400000; Backgrounds 0x100.

## INFERRED
- The costume sub-type masks follow the piece bits: 0x1fd = beard, eyes, glasses, mouth,
  moustache, nose, hair, head; 0x3e02 = feet, neck, torso, legs, hands, waist.
- The Goodies Bag's Materials page is primitive material (bit 0); the type list above
  gives only the costume collection's Materials (0x20).
- Gameplay Controls, Global Controls, Tools and Used Objects, Heart Items and My Pods have
  no plan type (custom or profile-only pages) and stay empty.

## Built
- `COLLECTION_PAGES` carries each page's type names and sub-type mask;
  `plan_sections` fills a page from the game's plans (first 36 with an icon and a
  category, sorted by category and title, as the open page already was).
- `PageData::others` holds every other page's sections; the open page keeps its own
  list, function tiles and kits. Grid focus moves on every page.

## Open
- The game's pages list the profile's own inventory, not every plan in the game data, so
  which items a player sees (and whether the game hides an empty page) is the profile's.
- The slide between pages is still not animated.
