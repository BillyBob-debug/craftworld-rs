# Level scripts, part 155: the viewer's level slots (2026-10-19, D1008)

D1007 gave the world a previous level slot, but the viewer never set it, nor the slot
of the level it launches, so `OnEnterPod__` always saw (0, 0).

### Found

- 0x1ab508 (called from 0x89eca8) sets the slot being loaded; when it differs from the
  current one, the current slot becomes the previous one (part 154).

### Inferred

- The Pod's own slot is not known, so the Pod keeps (0, 0), as it has at the start.

### Code

- A level switch hands the new world its slot and the previous one: a story level
  started from the Pod gets its story slot, and back in the Pod that slot is the
  previous one. Reloading the same slot keeps the previous one.
- Story slots are made by the developers (type 0), so the level-created dialogue stays
  off after them; it needs a locally stored user level (type 2), which the viewer does
  not launch yet.
