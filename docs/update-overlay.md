# D68 — official update 01.33 as a separate overlay input (2026-10-07)

Question: what does the newest official LBP2 update add to the user's BCUS98245 01.00
disc, and what would the scanner need to read disc + update together?

Scope: research and data acquisition only. No scanner, viewer or crate code changed.
The disc tree `game-data/a1-disc` and the original ZIP/ISO were not modified.

## Provenance

- Update list: `https://a0.ww.np.dl.playstation.net/tpl/np/BCUS98245/BCUS98245-ver.xml`,
  fetched 2026-10-07. 32 packages 01.01..01.33 (no 01.19), 4,047,336,256 bytes total,
  latest requires system 4.20. Copy kept at `local-analysis/lbp2-update-lists/ver.xml`.
- Packages downloaded from the XML URLs (Sony `b0.ww.np.dl.playstation.net`) to
  `local-analysis/lbp2-update-pkgs/` (ignored). All 32 verified.
- Checksum rule, `CONFIRMED_FROM_LOCAL_DATA`: the XML `sha1sum` is SHA-1 of the PKG
  without its last 32 bytes, and footer bytes 0..20 hold the same digest. A whole-file
  SHA-1 never matches (first attempt failed on that).
- PKG layout, `CONFIRMED_FROM_DOCUMENTATION` (public PS3 PKG documentation, psdevwiki
  "PKG files"): big-endian header, retail revision 0x8000 / type 1, data area at
  header 0x20 (size 0x28), IV at 0x70, AES-128-CTR over the data area, 32-byte item
  records (name offset/size, data offset/size, flags; flags low byte 4 = directory).
  Corroborated: all 32 packages parse with in-bounds tables and sane UTF-8 names.
- Extractor: one-off research tool `local-analysis/lbp2-update-lists/pkgx.cs`
  (.NET Framework csc, built-in AES). Contains the public retail PKG key, so it stays
  ignored and is not product code. Rejects non-retail headers, out-of-range tables and
  unsafe paths. Download script `dl.py`, analysis `compare.py`, `revisions.py`, and
  per-package item lists `A0101.lst..A0133.lst` live beside it.

## Result

`game-data/a1-update-0133/` (ignored) = all 32 packages extracted in version order,
later files overwriting earlier ones, like an installed HDD game directory:
1,742 files, 3,707,092,937 bytes. Patched `PARAM.SFO`: APP_VER 01.33, VERSION 01.00,
CATEGORY GD (disc: 01.00 / 01.00 / DG).

Each package carries `PS3LOGO.DAT`, `PARAM.SFO`, icons, `USRDIR/EBOOT.BIN`,
`USRDIR/output/brg_patch.map`, `USRDIR/patches/patch.edat` and **its own**
`USRDIR/patches/patch_NNNN.farc`. FARCs therefore accumulate (32 files, none replaced);
the MAP, EBOOT, SFO and EDAT are replaced by each later package. Loose content
(`USRDIR/gamedata/**`: 1,661 audio, 19 video, 17 under `gamedata/lbp2`, 1 font, 1 data)
was added mainly by 01.06 (657 items) and 01.28 (959 items).

All counts below are `CONFIRMED_FROM_LOCAL_DATA` from `compare.py` / `revisions.py`.

| Measure | Value |
|---|---:|
| Disc MAP (`OUTPUT/BLURAYGUIDS.MAP`) rows, rev 0x100 | 36,854 |
| Final `brg_patch.map` rows, rev 0x100, no trailing bytes | 23,229 |
| Patch GUIDs not on disc MAP | 21,270 |
| Patch GUIDs on disc MAP with a different hash (replaced) | 1,014 |
| Patch GUIDs on disc MAP with the same hash | 945 |
| Patch FARC entries (32 archives) / unique hashes | 22,797 / 22,353 |
| Unique patch hashes absent from disc `DATA.FARC` | 21,437 |
| Patch MAP hashes found in patch FARCs | 20,778 |
| Patch MAP hashes found only in disc FARC | 0 |
| Patch MAP rows unresolved by any FARC | 2,243 rows (2,220 hashes) |
| … of which are loose patch files under `USRDIR/gamedata` | 1,715 |
| … remaining: 426 `.ff`, 102 `.sbu`, 1,962,359 bytes | 528 |

New or changed patch rows by extension (top): .tex 9,954; .plan 4,866; .gmat 2,899;
.mp3 1,558; .mol 1,536; .ff 424; .cpv 125; .fsb 109; .bin 107.

`STRONG_INFERENCE`: `brg_patch.map` is cumulative (the final one indexes content from
all 32 FARCs). The 528 unresolved `.ff`/`.sbu` rows (shader/SPU programs, 1.96 MB)
are inside `patch.edat` (NPDRM EDAT, 1,975,808 bytes; sizes nearly match). The EDAT
was not decrypted; it holds no texture/mesh/material/plan rows.

Resource revisions reachable through the patch MAP (first 8 bytes, binary `..b`
magic): 9,665 entries have revisions 952..1016 (0x3B8..0x3F8); the most common are
952 (1,478), 994 (1,124), 1014 (1,099), 1006 (925). Existing readers are bounded to
heads ≤ 951 (e.g. `lbp-resource` animation 888..951), so most patch resources would be
reported unsupported until those revisions are researched. Disc MAP rows whose hash is
missing from `DATA.FARC` (4,333) are not supplied by the patch either.

## What a disc + update overlay needs (not implemented)

1. Input: a second root (installed update dir) beside the disc root; paths compared
   case-insensitively (disc `OUTPUT`/`GAMEDATA`, patch `output`/`gamedata`).
2. MAP: read disc `BLURAYGUIDS.MAP` and the latest `brg_patch.map`; for each GUID the
   patch row wins; keep the shadowed disc row as an alias/conflict record (AGENTS rule 7).
3. Payloads: FARCs are content-addressed, so look a hash up in every
   `patches/patch_*.farc` plus `DATA.FARC`; order does not change bytes. Keep the source
   archive per entry for diagnostics.
4. Loose files: 1,715 patch rows point at loose `USRDIR/gamedata/**` files, not FARCs.
5. Treat `patch.edat` like disc `DATA.SDAT`: protected container, explicit outcome.
6. Report patched counts separately from the 01.00 baseline; extend revision support
   (952..1016) as its own milestone with oracle comparison.

## Not tested

Scanner/viewer on the overlay; EBOOT 01.33; EDAT contents; whether any disc content is
removed or redirected by the patch beyond GUID replacement; text resources (`..t`).
Asset-compatibility counts: +0 newly viewable / +0 newly Complete (no code change).
