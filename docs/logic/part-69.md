# Logic, Part 69

See docs/logic.md for Parts 1-49.

## Part 69: useLayers puts a received tag on the sent tag's z (D1168)

Part 68 left the receiver's useLayers (byte +0x2c) unmodelled.

FOUND (EBOOT 01.33):
- The sender 0x13187c writes its offset to *(sender+0x90)+0x10. Lanes x and y are the
  tag's offset from the sender along the sender's unit axes (Part 59). Lane z is the
  tag's world z itself (the translation row of 0x3becc(tag)), not a difference from the
  sender's z. Lane w is left as it was. The offset is written only when a tag is found.
- The receiver 0x131f88 builds its tag's matrix at 0x1320e0-0x132190. x and y are its
  point plus the offset times `scale` along its axes. For z it tests +0x2c: when it is 0
  the tag keeps the receiver's own z (its root's translation), otherwise it takes the
  offset's z lane, the sent tag's z. It then sets the tag Thing's matrix (0xf26bc).
- 0x131638 binds the receiver's tag Thing to +0x94 when it is missing and sets bit
  0x40000000 on it, which the sender search skips (INFERRED from the subagent's read).

lbp-sim:
- `SceneTag`, `TagSpot` and `TagMarks` carry each tag's world z.
- `SceneSync` and `SyncSpot` carry `use_layers` and their world z.
- `synced_tags` gives a received tag the sent tag's z with useLayers, else the
  receiver's z.

Nothing reads a tag's z yet: the follower search is 2D, and allowInOut's layer move
(Part 67) is not hosted.

Test: `d1168_a_receiver_with_use_layers_puts_its_tag_on_the_sent_tags_z`.

Census (local-analysis/logic/d1168/census2.json, new `receivers_use_layers` and
`receivers_layered`): 20 of the 82 receivers have useLayers, in 4 levels, none in the
Rookie Test. None of them has a sender of its key in its level, so offline the change
places no tag on a new layer. The route is unchanged and parity passes 5/0.
