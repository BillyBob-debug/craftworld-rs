# Logic, Part 68

See docs/logic.md for Parts 1-49.

## Part 68: a receiver's second source is online-only (D1167)

Part 59 left a tag synchroniser receiver's second offset source, 0x2a8538, unmodelled,
and also the other caller of the sender search 0x130c90, 0x29ff3c.

FOUND (EBOOT 01.33):
- 0x29ff3c (called from 0x2a0298 at 0x2a03cc) takes a per-sender state. It calls
  0x130c90 at 0x29ff70 and, with a sender, packs the offset at *(sender+0x90)+0x10/+0x14
  /+0x18 through 0x891f1c into a quantised 64-bit key with the top bit set. It writes
  the key to the state's +0x10/+0x14 and keeps a counter (bytes +1 and +2) that marks
  the offset stable after 5 or 6 ticks. INFERRED: this is the network send of a
  sender's offset.
- 0x2a8538(ctx, holder, out) returns 0 unless the network context (from 0x2a1388) has
  bit 0x4000 at +0x8. Otherwise it looks up the holder's key (+0x70) in the tree at
  *(*(ctx+0x18)+0x9fe0), decodes the node with 0x891ebc into 3 floats and returns 1.
- The receiver 0x131f88 calls 0x2a8538 at 0x132068 and 0x132498. With no sender in the
  level it uses only 0x2a8538, and with none from there it switches its tag off
  (**(receiver+0x90) = 0). With a local sender it also asks 0x2a8538, and the shorter
  offset wins.

So offline, a receiver needs a sender of its key in its own level, which is what
`synced_tags` already does. Nothing changes in the model; the doc of `synced_tags`
says why. Still not modelled: the useLayers z (byte +0x2c picks the z lane source; its
meaning is not clear yet).

Test: `d1167_offline_a_receiver_needs_a_sender_of_its_key_in_the_level` in
crates/lbp-sim/tests/sandbox.rs.

Census (local-analysis/logic/d1167/census1.json, new top-level and per-level
`receivers_unsent`): 79 of the 82 receivers have no sender of their key in their level,
so they work only online. They are in 7 levels, none in the Rookie Test. The
`level_physics` example now raises `recursion_limit` to 256, since its summary
`json!` outgrew the default. The route is unchanged and parity passes 5/0.
