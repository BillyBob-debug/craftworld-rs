# Level scripts, part 137: which links are wide (2026-10-19, D390)

Part 136 found that a link is drawn wide (80, head 9) or narrow (40, head 6) by its
record's byte +0x4c, set when each end's object has +0xc0 of 0, 5, 7 or 8. This part
names +0xc0 and the pending colour's source. The viewer now draws each link wide or
narrow from its slots' level types.

### Found (EBOOT)

- A link's ends are SlotIDs (type, number); 0x8953cc is a thunk to 0x1a6cd8, which
  resolves one to its slot.
- +0xc0 is the slot's `DeveloperLevelType`: the native
  `GetDeveloperLevelType__Q6SlotID` (registered at 0xd6b084, OPD 0xd8d2a8, code
  0x6403f8) reads it through 0x89d78c and 0x746594, and the serializer 0xab3848 reads
  the field "DeveloperLevelType" at 0xab3a5c.
- The pending colour's only caller is 0x433be4 in 0x433328 (OPD 0xd82930). Its r6 is
  the second output of 0x89599c, which goes to 0x18b040 with the player id: the
  per-player record (stride 0x6b0, key +0xdc), its sub-object +0x16c, the word at +0x50.
  Without one it is the BSS word 0xe90fc8; the slots start as -1 (0x433448).

### Inferred

- From the enum string table at 0xcbc3c8: 0 main path, 1 mini level, 2 mini game,
  3 tutorial, 4 random, 5 boss, 6 versus, 7 cut scene, 8 DLC hub. A wide link joins two
  slots of the main path, a boss, a cut scene or a DLC hub; a link to a mini level or a
  mini game is narrow.
- The pending colour is a per-player colour set at run time, so the viewer keeps part
  133's green.

### Code

- `LevelWorld::link_is_wide` (lbp-runtime `level_unlocking.rs`): both ends resolve
  and each slot's `developer_level_type` (none reads as 0) is 0, 5, 7 or 8.
- `link_ribbon` (viewer `story_world.rs`) takes `wide`; narrow links use
  `LINK_NARROW_WIDTH` (40 x 0.4) and `LINK_NARROW_HEAD` (6 x 0.4).

### Next

The 48 and 7.2 pair: what SlotID test +0x10 == 1 with +0x14 of 3 or 4 makes.
