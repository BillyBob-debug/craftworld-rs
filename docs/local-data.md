# Local data inventory

Evidence date: 2026-10-05. All measured statements in this document are
`CONFIRMED_FROM_LOCAL_DATA` unless another label is supplied.

## Build identity and accessibility

Input: `LittleBigPlanet 2 (USA) (EnFrEsPt).zip`, **19,079,937,829 bytes**.
One ISO member: `LittleBigPlanet 2 (USA) (En,Fr,Es,Pt).iso`, **19,919,929,344 bytes**
(18.55 GiB); ZIP member CRC32 `ac48339b` (metadata, not a whole-disc verification).
ZIP method 8 (DEFLATE); ISO member compressed size 19,079,937,224 bytes.
Readable ISO9660 primary volume `PS3VOLUME`, 2,048-byte sectors. No missing
directories in the bounded 40 MiB listing. Other filesystem views such as UDF were
not established; do not claim this is exclusively ISO9660. ZIP is not password encrypted.

`PS3_GAME/PARAM.SFO`:

- TITLE_ID `BCUS98245`; TITLE `LittleBigPlanet™2`.
- APP_VER `01.00`; VERSION `01.00`; CATEGORY `DG`; minimum PS3 version `03.5000`.
- NP_COMMUNICATION_ID `NPWR01210_00`.
- SHA-256 `b6491a83d66901a7fc18bb494dc2448811046d959de7a8cb9aa85ef4e9b99d39`.

This establishes the disc-declared base application version, not a full-disc
authentication, mastering revision, installed update, or exact executable source
commit. No update overlay/install/save directory was found outside the image.
The ZIP filename's language list is not exhaustive: many more localized files exist.

Executable: `PS3_GAME/USRDIR/EBOOT.BIN`, **5,729,888 bytes**, starts `SCE\0`;
`STRONG_INFERENCE`: PS3 SELF executable container. No executable decryption,
disassembly, or execution attempted. Plain ELF accessibility remains unestablished.
`DATA.SDAT` starts `NPD\0` (checked header only). Treat it as a protected container
and stop there. Packages and system updater are not decoded.

## Disc layout

```text
/
  PS3_DISC.SFB
  PS3_GAME/
    PARAM.SFO, ICON*, PIC1.PNG, PS3LOGO.DAT
    LICDIR/, PKGDIR/, TROPDIR/NPWR01210_00/
    USRDIR/
      EBOOT.BIN
      DATA.FARC
      DATA.SDAT
      OUTPUT/BLURAYGUIDS.MAP
      GAMEDATA/              loose audio, videos and fonts
  PS3_EXTRA/D000..D011/       bonus PKG/video content
  PS3_UPDATE/PS3UPDAT.PUP     system updater, not an LBP resource archive
```

4,101 directory records: **3,899 file records**, 202 directory records. One PKG
record has the ISO multi-extent flag. Counts are records; a future ISO reader must
merge consecutive extents of the same logical file before totals or extraction.
The tested FARC and MAP are single-extent. No other FARC, FAR4, or MAP is listed.

| Major file/group | Count | Bytes / role |
|---|---:|---|
| USRDIR/DATA.FARC | 1 | 1,023,990,298 bytes; resource payloads and footer index |
| USRDIR/OUTPUT/BLURAYGUIDS.MAP | 1 | 3,820,654 bytes; revision 0x100, GUID/path/hash metadata |
| USRDIR/DATA.SDAT | 1 | 1,910,672 bytes on disc; MAP-declared size differs (1,906,643); protected/unknown access, do not decode |
| Loose .fsb | 259 | 1,487,799,680 bytes; FMOD banks, content not decoded |
| Loose .mp3 | 3,038 | 204,326,211 bytes; localized dialogue/narration naming |
| .bik | 94 | 13,013,235,276 bytes; tutorials/videos dominate disc size |
| .mp4 | 10 | extras videos |
| .fnt | 15 | fonts; do not assume Toolkit FNT wrapper for all loose fonts |
| .m / .mp / .f | 160 / 222 / 5 | 18,434,048 / 26,852,535 / 2,515,840 bytes; recurring loose audio-bank naming, semantics UNKNOWN |
| .pkg | 3 records | bonus/install packages; one multi-extent; not investigated |
| PS3UPDAT.PUP | 1 | 268,435,456 bytes; excluded from explorer scope |

Other disc extensions: `.dat` 2, `.fev` 1, `.pam` 3, `.png` 18, `.sfx` 12,
`.swf` 2, `.trp` 1, `.txt` 1; 44 records have no extension. Full extension counts
remain in the ignored JSON. `.sfx` here belongs to extras metadata, not necessarily
a sound effect resource. Paths include `GAMEDATA/AUDIO/DIALOGUE/{AVALON,CLIVE,
DAVINCI,EVE,...}` and localized variants; `VIDEOS/LBP2/TUTORIALS/TUTORIAL_*.BIK`
and `VIDEOS/SCEJ/...`. Naming suggests content roles, not a verified decoder.

## MAP/FARC facts

MAP SHA-256 `51de580f2badf74db647999b212afbc3fb509d4c389adeb0ef14008cb4398350`.
36,854 rows, 35,867 distinct hashes, zero duplicate GUIDs. FARC has **31,534**
entries and distinct hashes; index starts at byte **1,023,107,338**, occupies
882,952 bytes, followed by 8-byte count/magic footer. Every FARC hash has a MAP row.
All FARC entry ranges passed the probe's payload-before-table bound check; payload
hashes were verified for 19 samples only.

32,232 MAP rows resolve to a FARC hash. The remaining **4,622** are not archive
payloads in this index. Of those, **3,689** match an ISO path by case-insensitive
`PS3_GAME/USRDIR/` + MAP-path comparison; matching paths alone do not verify bytes.
**933** do not match: `.ff` 393, `.mp3` 382, `.sbu` 102, `.fsb` 49, `.fnt` 7.
Missing references could be packed elsewhere, regional/build leftovers, or otherwise
absent. Their cause is **UNKNOWN**. Do not assume all belong to SDAT or are corrupt.

| MAP extension | Rows | Interpretation confidence |
|---|---:|---|
| .tex | 13,697 | texture naming; four sampled distinct resources have GTF headers |
| .plan | 7,342 | Toolkit PLAN; sampled PLNb |
| .gmat | 5,536 | graphics material; sampled GMTb |
| .mol | 2,731 | mesh; sampled MSHb |
| .anim | 854 | animation; sampled ANMb |
| .bin | 282 | level candidates; sampled LVLb; extension alone is ambiguous |
| .mat | 80 | physics material; sampled MATb |
| .smh | 21 | static mesh; sampled SMHb |
| .trans | 40 | translations; sampled raw table prefix, no payload parse |
| .ff | 393 | Toolkit SCRIPT/FSH naming; none resolves in FARC |
| .fpo / .vpo | 145 / 39 | shader naming; payloads not investigated |
| .sbu | 102 | SPU program naming; none resolves in FARC |
| .raw / .smp / .rinst / .mus | 791 / 131 / 42 / 116 | raw/audio/instrument/music candidates |

Counts include path aliases. No full resource type census has been performed.
The sampled GTF UI atlases are 512x512 with 10 mips and format byte 0x88;
`sponge_diffuse` is 512x512, 10 mips, 0x86; the minimal SDF sample is 2x2, 2 mips,
0x81. Toolkit maps these codes to DXT5, DXT1 and B8 respectively
(`CONFIRMED_FROM_COMMUNITY_CODE`); only raw header values are locally measured.
First-known family samples include `gamedata/data/ui.tex`, player-character `.mol`
paths, themed mesh libraries, LBP2 level/palette directories, `languages/*.trans`,
and `gmat/*.gmat`. Resource header revisions in tested binary samples range **306–940**.
They are not interchangeable with disc APP_VER or MAP revision.

## Input recommendation

Milestone A accepts an already accessible extracted game directory, either disc root
or `PS3_GAME` root. Read FARC in place using seeking; never bulk-extract its resources.
Minimal asset scanner inputs are PARAM.SFO, DATA.FARC, OUTPUT/BLURAYGUIDS.MAP;
include loose GAMEDATA later for external-reference resolution. On this workspace,
selective copying of these three clear, verified files would be roughly 1.03 GB and
can be done later into ignored `game-data/` without decrypting anything. It was not
done in this pass. ISO convenience support follows after the directory scanner.

Deflated ZIP/ISO random seeking repeatedly replays decompression. The probes work by
streaming forward through approximately the first 1.06 GB of a 19.92 GB ISO, never
writing that stream to disk. Direct ISO support should use a bounded read-only
filesystem library, handle extents correctly, and explicitly reject protected data.
