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
