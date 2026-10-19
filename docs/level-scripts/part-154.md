# Level scripts, part 154: the previous level (2026-10-19, D1007)

The Pod's `OnEnterPod__` sets `WantLevelCreatedDialogue` when the previous level's slot
is of type 2 and `Game.IsPreviousLevelValid__` answers true. Neither the previous slot
nor the two natives existed here, so D1005 always cleared it.

### Found

- `GetPreviousLevelSlotID__` is 0x602324; through 0x89a83c it reaches 0x16ca90, the twin
  of the current slot's 0x16cb18, which reads the game state's previous slot (Type at
  +0x1a8, SlotNumber at +0x1ac; the current slot is at +0x1a0/+0x1a4) and wraps it as a
  new script SlotID (0x50c2cc). Like `GetLevelSlotID__`, a type-2 slot in one state of
  the game (0x16c788 answering 0) gives the global default slot instead.
- `IsPreviousLevelValid__` is 0x5f87ac; through 0x89a80c it reaches 0x1aa8ec, which
  builds the previous slot and asks 0x1a6cd8. Type 0 needs a SlotNumber, then passes a
  last check (0x349bc); types 1 to 9 go through a jump table at 0xd97cdc; types 10 and
  above are never valid. The answer is a bool.
- 0x1ab508 (called from 0x89eca8) sets the slot being loaded: when it differs from the
  current one, the current slot becomes the previous one. Reloading the same slot keeps
  the previous one.
- `OnEnterPod__` reads the previous slot's type through the script `SlotID.GetType__`
  and asks `IsPreviousLevelValid__` only when the type is 2.

### Inferred

- The slot types, in order from 0: made by the developers, a user level on the server,
  a user level stored locally (2), a local group, a developer group, the Pod, a fake
  slot, a remote moon, a DLC level, a DLC pack, a playlist. So the dialogue follows a
  locally stored user level. The order is guessed from the type names.
- Types 1 to 9 need a SlotNumber too (the jump table was not decoded), and the last
  check of a type-0 slot passes (0x349bc was not read).
- The default slot of the special case above is not modelled.

### Code

- `LevelWorld::previous_level_slot_id` ((0, 0) unless the caller sets it) and a third
  SlotID object made ahead, which `Game.GetPreviousLevelSlotID__` hands out;
  `Game.IsPreviousLevelValid__` answers from the slot as above.
- A native port reads a SlotID the world made through `get_member` (`Type`,
  `SlotNumber`); the native `OnEnterPod__` uses it as the script does.
