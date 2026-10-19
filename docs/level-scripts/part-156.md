# Level scripts, part 156: the Pod's music files (2026-10-19, D1009)

Part 151 named the Pod's music bank `I_POD.FSB` on the USA disc and gave its size. That
size was measured on LBP1's copy (NPUA80479); LBP2 does not ship the bank.

### Found

- The disc's map (`BLURAYGUIDS.MAP`) names GUID 42103 `gamedata/audio/i_pod.fsb`
  (14,787,392 bytes, the size of LBP1's file), but the file is not in DATA.FARC nor
  loose on the disc, in the 01.33 update or in RPCS3's BCUS98245; the update's map has
  no row for it. 304 of the disc map's 417 `.fsb` rows are missing so.
- LBP1's bank holds two 6-channel MPEG loops (`amb_pod_A_01`, `amb_pod_A_02`, 121.9 s
  each at 44100 Hz). LBP2's `PS3_MAIN.FEV` names no `i_pod`.
- GUID 42108 is `gamedata/audio/i_pod.mus`, which the 01.33 update replaces (897 bytes,
  in `patches/patch_0110.farc`, not encrypted). It is text: a `MUSt` line, then
  `Key<TAB>Value` lines: `MasterVolume`, `TrackTranslatedTag`, `IsCopyrighted`,
  `StereoPairs 6`, and six `[i]` blocks with `MusicStemTranslatedTag`, `SliderIndex`
  (0 to 5), `Fade`, `FadeIn`, `FadeOut`, `ForwardSend`, `RearSend` and `Trigger`.

### Inferred

- With no bank, the game has nothing to play for the Pod's interactive music; what it
  does when the bank is missing was not read. The Pod's sound in LBP2 then comes from
  elsewhere (`musicbank.fsb` holds ambience loops), not from this music.

### Code

- `parse_music_settings` reads a `.mus` file into `MusicSettings` (the master volume and
  each stereo pair's slider, fades and sends). Nothing loads it yet.
- Part 151's claim is corrected in the comments of `POD_MUSIC` and
  `InteractiveMusic::guid`.
