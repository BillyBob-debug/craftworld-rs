# Pod, menus, HUD and Popit in Rust/Bevy: research and plan

2026-10-08 America/Chicago (written ~15:30 UTC). Cloud research only: no game data was
read. Repo state: main `c4e4998` (level research merged; asset work paused at D105).
Format evidence: Toolkit/cwlib (MIT) at the project pin
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, read from a local checkout; paths below are
under `lib/cwlib/src/main/java/cwlib/`. Labels follow docs/formats.md: Toolkit facts
are CONFIRMED_FROM_COMMUNITY_CODE; everything else is labelled where it appears.
Repo copy of the project file `lbp2/research/ui-pod-popit.md`. Sibling plan: `docs/level-research.md` (L-items). This plan depends on it in places,
called out as "needs L…".

## 1. Short answer

LBP2's interface is **not stored as layout data**. The menus, Popit, pause menu and
probably the Pod computer screens are drawn by a native immediate-mode widget system
("Gooey": frames, text, buttons, images, carousels keyed by u64 UIDs) and **driven by
compiled Fish scripts** (`.ff`, resource type SCRIPT, magic `FSH`). Evidence for that
architecture is the unlicensed LBP1-engine project ennuo/alear (behaviour reference
only, see §8); for LBP2 it is STRONG_INFERENCE, not confirmed against the LBP2 ELF.

What *is* data, and readable from Toolkit's formats:
- every item the Popit shows (plan `InventoryItemDetails`: icon texture, type bits,
  subtype, LAMS title/description keys, location/category keys, tool type, colour),
- the starter item sets (`PAL` palettes), pins (`PIN`), story planets and badges
  (`SLT` slot lists: slot icon, position on the planet, lock state, translation tag),
  DLC packs (`PCK`),
- all strings (`.trans` tables keyed by the LAMS hash), fonts (`FNTb`, header + glyph
  table already read by D79), icons and UI textures (already decodable: all 23,247),
- the Pod itself: an ordinary `LVLb` level referenced from the player profile.

So a faithful 1:1 port of the menus needs a **Fish VM plus the engine's native API**,
which nobody has publicly documented. Recommended route: two tracks.
1. **Data track (now):** read the real data (strings, fonts, slots, items, palettes,
   pins, scripts-as-data) and build native Rust/Bevy views over it: a translation
   browser, a story-planet/slot browser, a Popit-style inventory browser. These fit the
   current Asset Explorer scope and give visible results early.
2. **Script track (research, later decision):** an RScript reader and disassembler,
   then a PC census of which native functions the UI scripts call. That census sizes
   a Fish VM honestly before anyone commits to building one. A VM is the only route to
   original menu behaviour and is shared with level logic (switches, microchips).

Runtime HUD, Popit editing tools and pause/scoreboard behaviour need gameplay and are
out of the explorer scope; they are sized here but scheduled last.

## 2. How the interface is built (evidence)

### Gooey + Fish (ennuo/alear, LBP1 engine, no licence: behaviour evidence only)
- Gooey is native C++ (`CGooeyNodeManager`, `CGooeyNodeContainer`, `CGooeyImage`,
  `CGooeyCarousel`). Immediate-mode API: `StartFrame/EndFrame`, `SetFrame*` (sizing,
  layout mode, borders, style, highlight, scroll, corner radius), `DoText`, `DoTitle`,
  `DoButton`, `DoImage`, `DoIcon`, `DoSpacer`... returning input flags; `PerFrameUpdate`
  and `RenderToTexture`. Style enums: text GTS_*, container GCS_* (chalky, spikey,
  bubbly, cloudy), bubble GBS_*, icons GI_*. Files: `code/CWLib/gooey/GooeyNodeManager.h`,
  `GooeyContainerStyles.inl` at https://github.com/ennuo/alear (branch main, pushed
  2026-08; no pinned commit recorded yet).
- Scripts drive it: `CScriptObjectGooey` wraps a per-thing Gooey manager; natives are
  bound by class and mangled name (e.g. `Start_Menu.DoConfigSubmenu__Q5Gooey`,
  `Poppet.ReloadFromInventory__i`). The pause menu is a script on a world Thing; the
  Popit Gooey has a "needs kick script" state (`code/Alear/AlearStartMenu.cpp`,
  `code/CWLib/poppet/PoppetGooey.h`, `gooey/ScriptedGooeyWrapper.h`).
- Toolkit corroborates the script side only: `ScriptObjectType.POPPET(17)`
  (`enums/ScriptObjectType.java`) makes the Popit a script-visible object. Toolkit has
  **no** Gooey, front-end, Pod or Popit script GUIDs and no VM.

### RScript (`resources/RScript.java`, `enums/InstructionType.java`, `structs/script/`)
Standard wrapper + dependency table, then: `upToDateScript` descriptor only when
head <= 0x360 (Revisions says 0x33a: conflict, PC check), `className`, superclass
SCRIPT descriptor, modifiers (i16 from head >= 0x3d9, else i32), type/field/function
reference rows, field/property/function definitions (from 0x3d9 the function ranges
are u16 begin + u16 length), shared arguments, **bytecode = u64 per instruction,
opcode in the low byte** (203 opcodes 0x00..0xca, 18 instruction classes; operand bit
fields per class in `structs/script/instructions/*`), line numbers, local variables,
ASCII and UTF-16BE string tables, s64 constants (head >= 0x30c; Revisions says 0x30b:
conflict), f32 constants, depending GUIDs (head >= 0x1ec). Native functions are rows
with the NATIVE modifier: the **names** are in the string table, the **semantics** are
in the executable only.

### Popit state and pages
- Runtime: `PYellowHead.poppet` → `Poppet` (mode stack, edit state, sticker/decoration
  placement, marquee selection, tweak object) (`structs/things/components/poppet/*`).
  Mode/submode values are UNKNOWN in Toolkit; alear lists LBP1 values (NORMAL, EDIT,
  CURSOR, MENU, TWEAK...; ~56 submodes) as behaviour evidence.
- Pages: `RLocalProfile.inventoryCollections` (head > 0x316): collections of
  `InventoryPage` (title LAMS key, sort mode), current page; plus `stringTable`
  (`StringLookupTable`, interned location/category strings). `resources/RLocalProfile.java`.
- Items: `InventoryItem` (plan descriptor + `InventoryItemDetails`, compact layout from
  head 0x37d). Type bits `enums/InventoryObjectType.java` (LBP2 meanings differ from
  LBP1: decode by revision), subtypes incl. POD_CONTROLLER/POD_MESH, `ToolType` 0..64,
  `InventorySortMode`, flags incl. RESTRICTED_POD. The repo already reads the scalar
  prefix of this from plans (D33) but stops before CreationHistory and the icon.
- `PAL` (`resources/RPalette.java`): plan list + location/description keys; converted
  plans from head 0x323. Likely the default Popit contents (STRONG_INFERENCE).
- `POP` POPPET_SETTINGS, `CON`, `TUT`, `LMT`: type known, **layout UNKNOWN**.

### Pod, planets and slots
- `RLocalProfile.podLevel` is a LEVEL descriptor: the Pod is a normal level stored per
  profile, built from pod items (USER_POD type bit, POD_CONTROLLER/POD_MESH subtypes).
  TCRF names the LBP2 pod room mesh `gamedata/mesh_library/cosmos/cosmos_room/
  cosmos_room_default_lbp2.mol` (https://tcrf.net/LittleBigPlanet_2, community).
- `RSlotList` (`SLT`) holds `Slot`s: id (SlotType: DEVELOPER, POD=5, ...), root level,
  icon texture (badge), `location` v4 (badge position on the planet), translationTag
  (`_NAME`/`_DESC` hashed at lookup), group, initiallyLocked, isSubLevel,
  showOnPlanet, collectabubbles, livesOverride, planetDecorations
  (`structs/slot/Slot.java`). Badge meshes: `enums/CommonMeshes.java` (g16006 level
  badge). `RPacks` lists DLC packs; `RPins` the pins (title/description keys, icon).
- Profiles (`IPR`, `BPR`, `PRF`) are **save data**, not disc data. Whether a default
  profile/pod ships on the disc is a PC check.

### Strings and fonts
- `.trans` (`resources/RTranslationTable.java`): headerless BE, i32 count, sorted
  {u32 key, i32 offset}, FEFF-prefixed UTF-16 strings; legacy text form too. LAMS key:
  base-27 fold over the tag padded to 32 chars, high part seeded 0xC8509800 for tags
  <= 32 chars, `(v0 + v1*0xDEADBEEF) mod 2^32` (`makeLamsKeyID`). Known GUIDs: English
  table g54800, patch table g69910 (`tools/unlockall/RestlessPopulator.java`).
- `FNTb` (`resources/RFontFace.java`): header + glyph table already implemented in
  `crates/lbp-resource/src/font.rs` (D79). Missing: per-glyph zlib inflate
  (compressed form) and the meaning of the 2-byte uncompressed pixels (UNKNOWN), line
  metrics/kerning (UNKNOWN). alear names Omnes Black (GUID 16710) and Helvetica faces.
  Note: one research agent wrongly reported "no Toolkit parser"; RFontFace exists at
  the pin.

### HUD and in-level UI parts (LBP2 range)
`PHudElem` and the other HUD-ish parts are LBP3-only (subVersion-gated), so the LBP2
HUD is native/scripted, not Thing data. Data that feeds it: `PGameplayData` (score,
prize, key bubbles; plan/mesh GUIDs in `structs/things/Thing.java`), `PCheckpoint`
(spawns/lives), `PWorld` (isPaused, scoreLocked, rewards, deathCount),
`PCreature.directControlPrompt`, `PMetadata`. All parse with the level reader (L-items).

## 3. What the repo already has

| Need | State on main c4e4998 |
|---|---|
| Textures for icons, badges, UI art | All 23,247 texture hashes decode (D98) |
| Fonts | FNTb header + glyph table (D79); no glyph pixels |
| Translations | Raw prefix only (formats.md); no reader, no LAMS hash |
| Plan inventory details | Scalar prefix (D33); no icon/creation history |
| Serializer core (descriptors, compressed ints/vectors, references) | Planned as L3; needed by slots, pins, palettes, profiles, scripts |
| Slots, pins, palettes, packs, profiles, scripts | Nothing |
| Bevy UI | bevy_egui 0.42 dev panels only; `bevy_ui`/`bevy_text` features not enabled |

## 4. Gaps and risks

1. **Behaviour lives in code.** Menu layout, flow and Popit logic are in Fish bytecode
   plus native functions. Without a VM, every screen is a native reimplementation
   guided by observed behaviour, and its fidelity has to be reported as such.
2. **VM size is unknown.** 203 opcodes are cheap; the native API (Gooey, Poppet,
   Thing, World, Pad, Audio...) is the real cost and is only discoverable from the
   scripts' native rows (names) and the executable (semantics). The U-census sizes it.
3. **Provenance.** alear and the yellowhead / ps3decomp decomps carry no licence and
   are derived from the game binary; behaviour evidence only, never source. Native
   semantics recovered from the user's own ELF follow AGENTS.md (findings, not code).
4. **Profiles are saves.** Popit page layout and the player's pod level live in the
   profile. Without a profile, the inventory view must be built from plans and
   palettes (and labelled as such).
5. **Scope.** AGENTS.md rule 6 keeps GUI/gameplay out of scanner work. Explorer views
   (string browser, slot browser, inventory browser) fit the current product; a
   runtime HUD/Popit is a scope change the user should approve when it comes.
6. **Fonts.** Glyph pixel format partly UNKNOWN; until decoded, use a stand-in system
   font and label it.

## 5. Phased path

**Phase 0, PC census (no product code).** Toolkit oracle over SCRIPT, TRANSLATION,
FONTFACE, SLOT_LIST, PALETTE, PINS, PACKS, POP/CON/TUT/LMT on 01.33 (§7). Output:
counts, heads, paths, script class names and native function names. Decides the size
of the script track and which data readers matter.

**Phase 1, strings and fonts.** `.trans` reader + LAMS hash, then glyph decode to a
font atlas IR. Translation browser in both viewers. Every later view shows real names.

**Phase 2, front-end data readers** (needs L3 serializer core). Slot/SlotList, Pin,
Palette, Packs, then full InventoryItemDetails (icon, creation history). Toolkit oracle
parity per reader.

**Phase 3, explorer views in Bevy.** A small Gooey-shaped UI facade over `bevy_ui`
(frames, text, image buttons, scroll, style tokens; pad navigation via
`bevy_input_focus`), then: story planet/slot browser (badge icons, translated names,
lock state, links to the level once levels load), and a Popit-style inventory browser
(pages by type, tabs by location/category, icons, titles, sort modes). Dev overlays
stay in bevy_egui.

**Phase 4, Pod scene** (needs Lb, and Lx for shaped parts). Load the Pod level found
in Phase 0 with the level scene mode; pod computer screen as UI rendered to a texture
(Bevy render-to-texture example, or the `bevy_world_space_ui` crate after a licence
check).

**Phase 5, script track.** RScript reader + disassembler (data only), native census,
then a decision with the user: build a Fish VM subset with stub natives (run one menu
script headless and compare its Gooey call trace with expectations) or keep native UI.

**Phase 6, runtime UI** (scope change). HUD (score, lives, prize bubbles), pause menu,
Popit cursor and edit tools, scoreboard. Needs gameplay, input and the level runtime.

## 6. Candidate backlog items (unnumbered; Integrator assigns D-numbers)

| # | Item | Where | Needs | Done when |
|---|---|---|---|---|
| U0 | UI census: Toolkit oracle over SCRIPT/TRANSLATION/FONTFACE/SLT/PAL/PIN/PCK/POP/CON/TUT/LMT on 01.33; script class names, superclasses, native function names (all native rows), dependencies of scripts; `.trans` paths/languages; default pod level candidates | PC | — | Report in `local-analysis/`, summary in a milestone doc |
| U1 | `.trans` reader + `make_lams_key` (binary and text forms, budgets, sorted-key check) | cloud-ok (synthetic) + PC oracle parity | — | All tables on 01.33 parse; key/string parity with Toolkit |
| U2 | Font glyph decode: per-glyph zlib inflate under budgets, coverage bitmap IR, atlas packing (no GPU in lbp-resource) | cloud-ok + PC check of pixel meaning | D79 | Every glyph in every FNTb decodes or is labelled |
| U3 | Translation + font browser in both viewers (string search by key/tag, glyph sheet) | cloud-ok + PC QA | U1, U2 | Counts shown in the app |
| U4 | Slot + RSlotList, Pin + RPins, RPalette, RPacks readers | cloud-ok + PC parity | L3 | All such resources on 01.33 parse byte-exact |
| U5 | Full InventoryItemDetails (icon, creation history, user details, photo/eyetoy, LBP2 type-bit decode) extending D33 | cloud-ok + PC parity | L3 | All plans' details parse; icon GUIDs resolved |
| U6 | Bevy UI facade (Gooey-shaped frames/text/image buttons/scroll, style tokens, pad focus navigation); synthetic content | cloud-ok + PC QA | — | Demo screen with pad and mouse navigation |
| U7 | Story planet/slot browser (badges at Slot.location, icons, translated names, lock and group) | PC QA | U1, U4, U6 | Every story slot listed with icon and name; count in app |
| U8 | Popit-style inventory browser from plans + palettes (pages by type, location/category tabs, icons, sort modes) | PC QA | U1, U5, U6 | Item count browsable with icon+name vs total plans |
| U9 | Pod scene: pod level in the level scene mode; pod computer screen rendered to texture | PC QA | U0, Lb (Lx for shapes), U6 | Pod visible; fidelity labelled |
| U10 | RScript reader + disassembler listing (data only, budgets, both 0x3d9 layouts) | cloud-ok + PC parity | L3 | All scripts on 01.33 parse; listing deterministic |
| U11 | Native API census from U10: native function names by class, call counts, which scripts are UI (Gooey/Poppet/Start_Menu) | PC | U10 | Sized list; decision memo for the user |
| U12 | Fish VM spike (decision gate after U11): subset interpreter, stub natives, headless Gooey call trace for one menu script | cloud-ok + PC | U10, U11, user decision | Trace for one script; cost estimate for the rest |
| U13 | Runtime HUD / pause / Popit tools | later, scope change | levels runtime, U12 or native | — |

Counting units: strings (distinct LAMS keys per table), glyphs (per font), slots
(distinct SlotID on 01.33), items (distinct plan GUIDs with inventory details), scripts
(distinct SCRIPT GUIDs). "Viewable" = shown with real icon and translated name;
"Complete" for a view is manual comparison with the game, reported separately.

Suggested first batch, independent of the level batch already running: U0 (PC), U1
and U2 (cloud-ok), U6 (cloud-ok). U4/U5/U10 wait for L3.

## 7. PC checks for the Integrator

1. Paths, heads, branches and counts of every SCRIPT (`.ff`) resource on 01.33; Toolkit
   RScript parse success; split by head < / >= 0x3d9.
2. RScript gate conflicts: any script with head in 0x33b..0x360 (upToDateScript present
   by code, absent by Revisions) and any head 0x30b (s64 table).
3. From all scripts: class names, superclasses, and every NATIVE function row (class,
   mangled name). Which classes mention Gooey, Poppet, Start_Menu, Pod, HUD, Scoreboard.
4. All `.trans` files: paths, languages, entry counts; check that g54800/g69910 are
   the English base/patch tables on 01.33; binary vs text form.
5. Every FNTb on 01.33 (D79 found `icon_patch_font.fnt`; others?): compressed flag,
   glyph counts, and a decoded sample glyph to settle the 2-byte pixel meaning.
6. SLT/PAL/PIN/PCK resources: counts, heads, Toolkit parse success; which slot list
   holds the story planets; slot count by SlotType/LevelType.
7. Default pod: LEVEL resources or plans with POD subtypes / USER_POD type, the
   `cosmos_room_default_lbp2` mesh, and whether any default profile (IPR/BPR) ships on
   the disc or update.
8. POP/CON/TUT/LMT/GSB presence and sizes (layouts unknown; just census).
9. UI texture folders (by MAP path prefix) and how many already decode (expected all).

## 8. Provenance and licensing

- Toolkit/cwlib (MIT) at the pin: format specification and oracle, as for D27-D33 and
  the level plan. Independent Rust implementation, oracle comparison, no translation.
- ennuo/alear (no licence, LBP1 engine, reconstructed from the binary): Gooey/VM/Popit
  behaviour evidence only. Never copy names-as-code or structure; record findings.
  Pin a commit before citing it in repo docs.
- MasonLeeBack/yellowhead, ps3decomp/littlebigplanet1 (decomps, no licence):
  behaviour evidence only, same rule.
- MindOfBog/lbp_bgm (no licence found): behaviour reference only.
- Servers LBPUnion/ProjectLighthouse, LittleBigRefresh/Refresh: AGPL-3.0; not used.
- Wikis (LittleBigPlanet fandom: The Pod, My Moon, Popit Menu; TCRF LittleBigPlanet 2;
  TheSixthAxis 2010-06-16 LBP2 features): behaviour description, unverified, LBP1-era
  where noted. Several fandom/miraheze pages were not reachable from cloud.
- Bevy 0.19 (https://bevy.org/news/bevy-0-19/): `bevy_ui` + parley text,
  `bevy_input_focus` directional navigation, render-UI-to-texture example. Crates to
  licence-check before use: `bevy_world_space_ui` 0.19 (MIT/Apache per docs.rs). Skip
  `bevy_fluent` (stale, Bevy 0.15); game strings come from `.trans`.
- Native semantics recovered from the user's ELF (if ever needed for U11/U12): allowed
  by AGENTS.md as research; raw output stays in `local-analysis/`; implement from
  written findings.

Sources read: Toolkit `RScript`, `InstructionType`, `structs/script/*`,
`ScriptObjectType`, `ScriptInstance`/`InstanceLayout`, `RLocalProfile`,
`RBigProfile`, `RSyncedProfile`, `InventoryItem`, `InventoryItemDetails`,
`InventoryObjectType`, `InventoryItemFlags`, `ToolType`, `RPalette`, `RPins`, `Pin`,
`RSlotList`, `Slot`, `SlotType`, `RPacks`, `RTranslationTable`, `RFontFace`,
`ResourceType`, `ResourceInfo`, poppet components, `PGameplayData`, `PCheckpoint`,
`PWorld`, `PMetadata`, `RLevel` (script GUIDs), `RestlessPopulator`; alear
`GooeyNodeManager.h`, `GooeyContainerStyles.inl`, `AlearStartMenu.cpp`,
`LooksMenu.cpp`, `PoppetGooey.h`, `ScriptedGooeyWrapper.h`, `PoppetEnums.inl`,
`ResourceGFXFont.cpp`; repo `docs/d79-fntb-font.md`, `plan-inventory-parser.md`,
`formats.md`, `level-research.md`, `STATUS-CURRENT.md`.
