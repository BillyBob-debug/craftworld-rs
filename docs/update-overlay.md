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

## D69 — 01.33 is the working input (2026-10-07)

The user chose to move the project to 01.33 outright. Implemented:

1. **Layered source** (`lbp-io::source`): `--game-dir` is an OS path list, base first
   (`"game-data/a1-disc;game-data/a1-update-0133"`). A later layer replaces the same
   case-insensitive relative path; replaced files are kept in `shadowed`
   (`shadowed_filesystem_files` in JSON; only `PARAM.SFO` here). Overlapping layers are
   rejected. Lookups are indexed (the overlay has 1,744 files). Rule: STRONG_INFERENCE
   from installed-update semantics; tested with synthetic layers.
2. **Patch MAP** (`lbp_archive::overlay_map`): with two or more layers the scanner reads
   `USRDIR/output/brg_patch.map`; each patch row replaces every disc row with the same
   non-null GUID; patch rows are renumbered after the disc rows. Replaced rows are kept
   as `shadowed_map_rows`. JSON adds `patch_map` and `counts.shadowed_map_rows`;
   `source_scope` becomes `disc_with_installed_update_overlay`. Attach re-verifies the
   patch MAP fingerprint and the merged rows. STRONG_INFERENCE (no reader handles the
   patch MAP); synthetic test in `lbp-archive/tests/tables.rs`.
3. **Identical copies**: the same SHA-1 in several FARCs (1,360 duplicate rows across
   patch archives) is one asset, not an ambiguity. The scanner lists one candidate
   (update layer, then latest archive name) and keeps the rest in
   `identical_archive_copies`. Ambiguous rows 1,445 -> 0.
4. **Revisions to LBP2_MAX**: resource header gate 951 -> 0x3f8 (1016), and the
   material (920..), plan (893..) and animation (888..) payload gates likewise.
   CONFIRMED_FROM_COMMUNITY_CODE: Toolkit `Revisions.LBP2_MAX = 0x3f8`;
   `SerializedResource.process` has no header condition after 0x297; RGfxMaterial,
   MaterialBox, RMesh, RAnimation, RPlan and their structs have no branch-0 condition
   above 0x3b7 (Vita/LBP3 checks need other branches). Branch 0x4431 (116 resources) and
   `FNTb` fonts (word at +8 is not a dependency offset; one local sample) stay unsupported.
5. **Read cost**: per-read stability checks now cover the SFO/MAP files plus the file
   being read; `check_stable` sweeps identity files and archives (loose stamps are
   checked whenever opened, and all at attach). Before this, each 4 KiB read reopened
   ~2,040 files and a 26-mesh census took 33 s instead of 2 s.
6. **Oracle runners** take `-Archive`; Java guards accept heads to 1016; manifests may
   name any `USRDIR/...` archive matching the file read (`TextureOracle.sameArchive`).
7. Launcher (`scripts/launch-viewer.ps1`) prefers the 01.33 snapshot and layered dir,
   falling back to the disc.

Results (`local-analysis/`, all CONFIRMED_FROM_LOCAL_DATA):

| Check | Result |
|---|---|
| `d68-inventory-0133-verified.json` scan | 58,124 MAP rows (01.00: 36,854); 33 FARCs; 0 ambiguous; partial false |
| Header statuses | parsed 27,624 (01.00: 16,515); unsupported 117 (branch 0x4431 116, font 1) |
| Toolkit oracle, revisions 952..1016 | material 29, mesh 18, plan 14, animation 5: all parsed |
| Parity vs oracle | material 29/0, mesh 18/0, plan 14/0, animation 5/0, diffuse route 29/0 mismatches |
| Material census | 8,159 of 8,159 locations: prefix, graph and tail parsed (budget now 128 MiB) |
| Diffuse route census | 8,094: texture 6,828, solid 603, instance-only 207, animation-only 60, unwired 10, unsupported 386, no MAP row 65 |
| Mesh census, new cohort `d68-new-mesh-samples.json` (48 post-951 meshes) | 48 viewable / 45 full diffuse / 0 Complete (01.00 baseline: 0, absent) |
| Existing cohorts on 01.33 | D61 48/43/0, D65 48/38/0, D66 22/7/0, D64 26/23/0: unchanged from D67 |
| Native Bevy, `--select-guid 0002e3db` (artist_vangogh_to.mol, 01.33 DLC costume) | non-primary 165v0; 8/8 primitives; 1 viewable / 0 Complete / 0 failed; capture written, not viewed |

Counting units: distinct catalog GUIDs (mesh census), material locations (route census).
Corpus-wide viewable/Complete mesh totals remain unknown (census is cohort-bounded).

## Not tested

EBOOT 01.33; EDAT contents; text resources (`..t`); legacy viewer on the overlay;
game pixels; whether the patch removes disc content beyond GUID replacement; corpus-wide
mesh census. Superseded disc copies (2,116 archive hashes without MAP rows) are not
offered as assets.
