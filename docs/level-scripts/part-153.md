# Level scripts, part 153: whether a video plays (2026-10-19, D1006)

`Video.IsVideoPlaying__` always answered false. The Pod's camera at rest (it skips the
players' average while a video plays) and the Pod's engage test (no engaging while a
video plays) read it, so neither saw the intro video D1003 starts.

### Found

- The native is 0x5f61e0, reaching 0x679534 through TOC tail calls; that loads the video
  manager singleton and calls 0x6793ec, which answers whether the manager's word at
  +0x400 is non-zero. It takes no handle.
- The manager holds one video slot (at +0x80, its handle at +0x318 of the slot), not a
  list. Bit 0 of +0x400 is "slot in use"; +0x410 counts handles.
- `StartVideo` (0x67c620 -> 0x67c45c -> 0x67a4a8) finishes any previous video
  (0x67a25c), then takes the slot (0x679494), setting bit 0. So a video counts as
  playing from the `StartVideo` call, before any `UpdateVideo`.

### Inferred

- The finish (0x67a25c, which ends in a call to 0x679978) frees the slot and clears
  bit 0; 0x679978 was not read. Only bit 0 of +0x400 was seen in use.

### Code

- `MediaState::video_in_use`: the one video slot is taken from `StartVideo` until
  `UpdateVideo` finishes the ended video or `StopVideo` stops it; the world answers
  `Video.IsVideoPlaying__` from it.
