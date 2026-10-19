# Level scripts, part 143: a group's lock (2026-10-19, D396)

Part 141 read the state query 0x1a7efc, which both the active test and the finish's
lock test use. This part gives the runtime its group case and names the SlotID types
from the toolkit oracle's `SlotType` (3 LOCAL_GROUP, 4 DEVELOPER_GROUP, 9 DLC_PACK).

### Found (EBOOT, 0x1a7efc)

- A key of type 4 (a developer group) or 9 (a DLC pack) gives 0 only when every slot
  in it gives 0; a level's 0 is part 141's locked case. So a group or a pack is locked
  while all its slots are, and open once any one of them is.

### Code

- `World::slot_unlocked` (lbp-runtime `pod.rs`): for types 4 and 9 with slots whose
  `Group` is the key, any of them open; else the slot's own lock as before. The finish
  (`finish_story_level`) and the links' active look both use it.

### Inferred

- A group with no slots in it keeps its own slot's `InitiallyLocked`; the loop's empty
  case was not read. Whether a story level's `PrimaryLinkLevel` ever names a group was
  not counted.

### Next

The line shader's program and the active band's motion (a capture of an unlock).
