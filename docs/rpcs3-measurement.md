# RPCS3 measurement setup (runtime decision 4)

2026-10-08 America/Chicago. The user approved measuring gameplay constants by running
their own LBP2 01.33 copy in RPCS3 on their PC. This doc is the PC Integrator's
checklist. Results are local observations: numbers and methods go in a milestone doc,
captures and memory dumps stay under ignored `local-analysis/`. No game or emulator
code is copied into the product.

## Rules that apply

- Game inputs stay read-only. RPCS3 boots the disc from a folder or ISO and writes only
  to its own directories (`dev_hdd0`, config, caches). Do not point RPCS3 at
  `game-data/` trees with write access; if a step needs a writable game folder, use a
  copy under RPCS3's own directory.
- RPCS3 and the PS3 system firmware are user-installed tools, outside the repository
  and never product dependencies.
- Test windows go on the second (non-primary) monitor.

## 1. Install (once)

1. Download the current Windows build from the official site, rpcs3.net, and unpack it to
   a folder outside the repository (for example `%USERPROFILE%\Emulators\rpcs3`).
   Record the build number.
2. Download the PS3 system software (`PS3UPDAT.PUP`) from PlayStation's official PS3
   system-update page and install it with *File → Install Firmware*.
3. Add the game:
   - Disc: *File → Boot Game* (folder with `PS3_GAME/` and `PS3_DISC.SFB`), or open the
     ISO directly if the build supports it. Keep the original read-only.
   - Update 01.33: install the update package with *File → Install Packages/Raps/Edats*
     if a `.pkg` is available. Otherwise copy (not move) the extracted 01.33 update
     folder into RPCS3's `dev_hdd0/game/<TITLE_ID>/`, where the title ID comes from the
     disc's `PARAM.SFO`. Confirm the game reports version 01.33 in RPCS3's game list.
4. Boot to the Pod once to confirm it runs. Record GPU renderer, frame limit, and any
   per-game settings changed from the defaults.

## 2. Measurement tools

- **Video:** record the RPCS3 window at a fixed 60 fps (OBS or similar) with the
  frame-rate overlay on. Count frames between events. This is the cross-check for
  everything below.
- **Pause and memory view:** RPCS3's debugger (*Utilities → Debugger*) pauses emulation
  and shows guest memory. Values are big-endian.
- **Memory search over time (optional):** a host memory scanner attached to RPCS3, with a
  big-endian float type, can find Sackboy's position and log it each frame. RPCS3 maps
  guest memory at a fixed host base address; record the base and the guest address of
  every value found so a later session can reuse them.

## 3. First measurements (runtime checks X1 and S9)

Use one story level with flat ground, for example the first tutorial level.

| Check | What to record |
|---|---|
| X1 tick rate | Game update rate: frames per in-game second. Video a timer or piston with a known stored duration and compare. Inferred 30 Hz is unverified. |
| S9 walk / run | Ground speed in level units per second: find Sackboy's x in memory, walk for 3+ s, sample x per frame. |
| S9 jump | Jump height and air time from a standing jump on flat ground (y per frame); fit a parabola for gravity. |
| S9 fall | Terminal fall speed, if any, from a long drop. |
| S9 layer shift | Frames from input to arrival on the next layer; z values of the three layers. |

Record the method, level, RPCS3 build, and the raw per-frame samples (CSV under
`local-analysis/`). The milestone doc gets the fitted numbers with the evidence label for
local observation and an error estimate.

## 4. Later uses

- G-checks for logic timing (gate delay, evaluation order) with small test rigs built in
  the game's create mode on a local save.
- Script VM checks (Phase RS): compare scripted gadget behaviour (scoreboard, magic
  mouth) with our VM in the same level.

## 5. Status and movement-measurement plan (2026-10-08, D133)

Done on the user's PC:
- RPCS3 v0.0.43-20247-222754bf in `%USERPROFILE%\Emulatorspcs3`; firmware 4.93
  (`--installfw`); the 32 update packages A0101..A0133 (`--installpkg`, game reports 01.33).
- The extracted `game-data/a1-disc` folder does not boot (no disc root RPCS3 accepts). A
  read-only copy of the user's ISO in `%USERPROFILE%\Emulators\lbp2-iso` boots with
  `rpcs3.exe --game-screen 1 <iso>` (second monitor). Vulkan, default settings
  (per-game database config applied); ~26 FPS during the intro on this PC.
- patch.edat decrypted (D133); no X1/S9 samples taken yet.

Because the emulator runs below full speed, measure in game ticks, not wall time: count
frames between samples with a game-side counter found in memory, or RPCS3's frame
counter, and never fit against host seconds.

Proposed route (no human needed if it works):
1. Turn on RPCS3's local IPC server (`config/ipc.yml`: server enabled, port 28012). A
   local script reads guest memory over it each frame (big-endian floats).
2. Send keyboard input to the game window. Set up a keyboard pad profile in RPCS3 and
   record its key map in the milestone doc.
3. Get to a flat story level. Find Sackboy's x/y/z by scanning while walking and
   standing still (values that change only while moving). Record the guest addresses.
4. Run the section 3 table: walk 3+ s, standing jump, long drop, layer shift. One CSV
   per run in `local-analysis/` (new names): tick, x, y, z, input state.

Steps that may need the user at the controls: first-run prompts (profile, save data,
network), and anything the keyboard profile cannot reach. If keyboard input does not
reach the game, the whole run needs the user playing while the script samples.
