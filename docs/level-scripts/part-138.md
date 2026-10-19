# Level scripts, part 138: links between groups (2026-10-19, D391)

Part 137 left open the drawer's third size, 48 wide with a 7.2-degree head. This part
traces when it applies. The viewer now draws links between two groups at that size.

### Found (EBOOT, the drawer 0x426484)

- The link record's two badges are at +0x40 and +0x44 (0x4265e8, 0x4265ec). Each
  badge's key is at +0x10: a kind byte, then a word at +0x14.
- After the wide or narrow pick (+0x4c, 0x426870), both paths test the badges' keys
  (0x426680 for wide, 0x426880 for narrow): when both have kind 1 and a word of 3 or 4
  (0x426894-0x4268bc), the width is 48 (0xda269c) and the head 7.2 (0xda26a0), loaded
  at 0x4268c0 and not scaled by 0.4 (the branch to 0x42669c skips 0x426694).
- 0x420890, which picks a badge's group radius (part 134), makes the same test and
  also accepts kind 2.

### Inferred

- Kind 1 is a slot badge whose word is its SlotID type; 3 and 4 are the developer and
  local group types, so the size is for a link between two groups. Kind 2 is another
  kind of group badge that the link drawer does not count.

### Code

- `LevelWorld::link_joins_groups` (lbp-runtime `level_unlocking.rs`): both ends'
  SlotID types are 3 or 4.
- `link_size` (viewer `story_world.rs`) gives 48 and 7.2 for such a link, else the
  wide or narrow size; `link_ribbon` takes the pair.

### Next

What badge kind 2 is, and whether a story unlock ever links two groups.
