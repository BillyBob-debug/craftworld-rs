# Level scripts, part 148: where a key's kind 2 comes from (2026-10-19, D1001)

Part 140 said no code stores a literal 2 into a key's kind byte. That was wrong: a
search result for a user stores it. This part reads the key and its builder.

### Found (EBOOT)

- A search-result key is a 0x30-byte entry in a table at owner+0x264 (count at +0x268).
  Its byte at +0 is the kind. Kind 1 holds a SlotID: type (i32) at +4, number at +8.
  Kind 2 holds a 20-byte NetworkOnlineID at +0xc.
- The matcher 0x42630c compares kind 1 by type and number; kind 2 goes to 0x89569c, a
  memcmp of 0x14 bytes (0x27e7c). Keys are copied by 0x894e7c (0x8b861c). A badge holds
  its key at badge+0x10 (0x42ae24).
- 0x50c74c builds a key from a script SearchResultHandle: when 0x6223c8 answers 1 it
  stores 1 and a SlotID, when it answers 2 it stores 2 and copies a NetworkOnlineID
  (0x895e7c, 0x894e7c; the store is beside 0x50c8fc). Its callers are the natives
  FindResult (0x60d648), SetPublishCursorBadge (0x6189a8), MoveCursorToResult
  (0x618ab4), SetDefaultResult (0x618bc8), SetDesiredResult (0x618cec) and
  FindResultIndex (0x618e18).
- The key test 0x420890 (callers 0x429f8c, 0x42ae8c, 0x42e514) is true for kind 2, and
  for kind 1 when type - 3 is at most 1 (types 3 and 4, the groups).

### Inferred

- Kind 2 is a user result (a creator's or profile planet, or a player's moon). What
  0x6223c8 reads to choose 1 or 2 was not resolved.
- Disc story slots only give kind 1, so the Story path keeps kind 1 with types 3 and 4
  as the groups; no code change.

### Next

A key for a user result, when searches other than Story are ported.
