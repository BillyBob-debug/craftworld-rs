# Level scripts, part 161: the unlock thread's group push (2026-10-19, D1014)

Part 127 read the end of `LevelUnlockingThread.Run__` (script ec688550): after its
clusters it pushes the search of the group under the cursor. The native thread
(part 130) ended at the cursor mode 4 instead.

### Found

- `Run__` 0192-0198 sets the cursor mode 4 after the last cluster, or at once when there
  are none. With `IsTarsierDLCEnabled__` it pops the state first and skips that set.
- 0199-0220: with a focus planet whose `GetCurResult__` has `GetType__` 1 and a
  `SlotID.Type` of 4 (a group), 0221-0241 make
  `SearchDetails(CreateGroup__Q6SlotIDb(slot, false))`, set the cursor mode 2 on it and
  `PushState__` it. Without a focus planet, or on any other result, the thread ends.
- 0242-0260 yield while the planets world's `ViewTransition` is under 1, the search is
  busy, or its decorations are not loaded. 0261-0278 then call
  `MoveCursorToResult__Q6SlotIDb` (no snap) with the focus planet's
  `PickDefaultResult__` and set the cursor mode 4.

### Inferred

- The branches out of 0204, 0220 and 0258 read as targets of 0183 (the cluster loop's
  `PlayVO__`), which cannot be meant; they are taken as the thread's end and the jump
  past the yield, as the code's shape says.
- The native group search is ready the tick after its push.

### Code

- The native thread's end asks for the group's push (`unlock_group_push`, cursor mode
  2) when the selected result is a group slot. The native pod computer's `push_group`
  takes it after the tick and pushes the group search over the top state.
- The thread waits for the push, then flies the cursor to the group's default level
  (`group_default_result`) and sets the cursor mode 4. With no pending links the Story
  search's default group (4, 27) is pushed this way.
