# Level scripts, part 162: no push without links (2026-10-19, D1015)

Part 161 had the unlock thread push the group under the cursor also when no link was
pending, so every Story search with nothing to unlock opened its default group. The
listing says otherwise.

### Found

- `Run__` 0064-0071 (script ec688550), between building the clusters and the cluster
  loop: the cursor mode 4 and `RETURN`. This is the end part 127 gives for no cluster;
  the group push (0199-0278) is reached only after the cluster loop.

### Code

- The native thread ends with the cursor mode 4 when it finds no cluster, as before
  D1014. The D1014 test now reaches the push through a link into a group (the cursor
  rests on the group's badge) and checks that no links push nothing.
