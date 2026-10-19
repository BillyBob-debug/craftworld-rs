# craftworld-rs

An independent, experimental asset explorer and runtime for *LittleBigPlanet 2*,
written in Rust with [Bevy](https://bevyengine.org). It reads **your own** game
copy, browses its resources, previews them, loads levels, and can run a playable
Sackboy and the game's scripts on a reimplemented runtime.

This is an unofficial fan project, unaffiliated with Media Molecule or Sony.
Despite the similar name, it is also not part of or affiliated with
[Craftworld Toolkit / cwlib](https://github.com/ennuo/toolkit), the community's
long-standing LittleBigPlanet modding toolkit, whose format work this project
gratefully builds on as prior art. It was prepared with AI assistance.

> **Status: experimental research software.** Many formats are only partly
> understood, rendering and physics are approximations, and nothing here is a
> complete or compatible replacement for the game. Expect gaps and breaking changes.

**Just want to try it?** Go to [Play it](#play-it-no-programming-needed).

## What it does today

- **Asset scanner** (`lbp-cli`): indexes a game directory (disc plus an optional
  installed update) into deterministic JSON: archives, MAP rows, resource headers
  and dependencies.
- **Asset explorer** (`lbp-bevy-viewer`, plus a legacy `lbp-viewer`): browse and
  search resources by path, GUID or hash, follow references, view hex, and preview
  textures (DDS/GTF BC1/BC3/BGRA8) and meshes with their materials and animations.
- **Level viewing**: loads level plans and draws their things, shapes, decorations,
  backdrops and lighting.
- **Playable Sackboy**: `--play-sackboy` puts a controllable Sackboy with his own
  animations, moods and a Popit overlay into a loaded level.
  Movement is measured against the original running under an emulator, and is close
  but not exact.
- **RScript VM** (`lbp-script-vm`, `lbp-runtime`, `lbp-sim`): an interpreter for the
  game's compiled scripts, with a native-function host and a small physics simulation.

Not done: complete material and shader evaluation, full level scripting, audio,
online features, save games, and a standalone distribution. See
[STATUS-CURRENT.md](STATUS-CURRENT.md) for the exact current state and open blockers.

## Play it (no programming needed)

> No game files come with this app. It reads **your own** copy of LittleBigPlanet 2,
> the one you already play in the [RPCS3](https://rpcs3.net) emulator. Nothing is
> downloaded, decrypted or changed; your ISO and RPCS3 folders are only read.

### What you need

1. **A Windows 10 or 11 PC** (64-bit) with a graphics card that RPCS3 runs on, and
   about **2 GB of free space** on the drive that holds your user folder.
2. **RPCS3 with LittleBigPlanet 2 that already plays.** It must be the **USA** disc,
   serial **BCUS98245**. Other regions (BCES, BCJS, BCAS...) don't work yet.
3. **The game fully updated in RPCS3, to version 01.33.** In RPCS3's game list, the
   Version column for LittleBigPlanet 2 should say 01.33. If it doesn't, install the
   update packages with RPCS3's *File > Install Packages/Raps/Edats* first.

### Steps

1. **Get the app.** Download the `craftworld-rs-preview` zip (from the
   [Releases page](https://github.com/chilladept/craftworld-rs/releases) once one is
   posted, or from whoever shared it with you).
2. **Unzip it.** Right-click the zip, *Extract All*, and open the extracted folder.
   Don't run the app from inside the zip.
3. **Start `lbp-bevy-viewer.exe`** (double-click). If Windows says it protected your
   PC, click *More info*, then *Run anyway*: the app is new and not code-signed.
4. **First box, the disc:** click *Browse...* and pick your LittleBigPlanet 2 **.iso**
   file, the same one RPCS3 boots. If your game is a folder instead of an .iso, type or
   paste the path of the folder that has `PS3_GAME` inside it.
5. **Second box, the update:** pick RPCS3's `dev_hdd0\game\BCUS98245` folder. It's
   inside the folder where `rpcs3.exe` is: open `dev_hdd0`, then `game`, then
   `BCUS98245`. The app often fills this box in for you. Picking RPCS3's own folder
   works too.
6. **Press *Set up*.** The first time, it copies about 1.2 GB out of your ISO and
   builds an index; that takes a minute or two. Wait for "Ready".
7. **Pick something to play**: the Pod, Popit, Rookie Test, Sackboy on the Moon, the
   Story screen, or the asset explorer.

### If something goes wrong

The app says what went wrong in red under the *Set up* button. Find the message here:

| The app says | What it means | What to do |
|---|---|---|
| Pick your LittleBigPlanet 2 disc | The first box is empty. | Browse to your .iso file. |
| Can't find ... | The path in the box doesn't exist (a typo, or the file moved). | Use *Browse...* instead of typing. |
| That's a compressed archive | You picked a .zip, .7z or .rar. | Extract it first, then pick the .iso inside. |
| That's an update package | You picked a .pkg in the first box. | Install the .pkg in RPCS3, then pick the .iso. |
| That's a single .rap/.edat/... file | You picked a key or game file, not the disc. | Pick the .iso (it's about 10 GB or more). |
| That's RPCS3's installed game/update folder | The update folder is in the disc box. | Move it to the second box; the first box needs the .iso. |
| There's no PS3_GAME folder in ... | That folder isn't a game disc. | Pick the .iso, or the folder that has `PS3_GAME` inside. |
| That file isn't an ISO 9660 disc image | The file isn't a PS3 disc image (or it's damaged or renamed). | Pick the .iso RPCS3 boots. |
| That ISO is BCES... (or another code), not the USA LittleBigPlanet 2 | It's another region or another game. | Only the USA disc, BCUS98245, works for now. |
| ... game archives can't be read: their contents are scrambled, or ... has no readable PARAM.SFO | The ISO is an **encrypted disc dump**; the files inside can't be read. | Use the ISO (or game folder) that RPCS3 actually plays. |
| The disc has no game archives | The disc copy is incomplete or it's another game. | Use a complete LittleBigPlanet 2 USA disc image. |
| Pick the update folder / No installed game at ... | The second box is empty or points at the wrong folder. | Pick `dev_hdd0\game\BCUS98245` in RPCS3's folder. Install the updates in RPCS3 first. |
| The second box needs a folder, not a file | A file is in the update box. | Pick the `BCUS98245` folder, not a file in it. |
| That update is for ... | That folder is another game or region. | Pick the `BCUS98245` folder. |
| The installed update is 01.xx | The game isn't fully updated. | Install every LittleBigPlanet 2 update in RPCS3 until it says 01.33. |
| Write ... / Can't create ... | The app couldn't save its copy (usually a full drive). | Free up about 2 GB on your C: drive and press *Set up* again. |
| Your game files changed | The ISO or update changed since the last *Set up*. | Press *Set up* again. |
| A window saying **The game files didn't load** | A mode started but couldn't read the files. | Close it, press *Set up* again, and retry. If it keeps happening, start over (below). |
| ... stopped: exited with ... | The mode closed itself. | Update your graphics driver, then try again. |

**Starting over:** close the app, delete the folder `%LOCALAPPDATA%\craftworld-rs`
(paste that into the File Explorer address bar to open it), and run *Set up* again. It
only holds the app's own copy, index and settings; your game and RPCS3 aren't touched.

If none of this helps, [open an issue](https://github.com/chilladept/craftworld-rs/issues)
with the exact red message. Please don't attach game files or screenshots of them.

## Game data rules

No game assets are included or ever committed: no textures, meshes, audio, levels,
archives, executables, keys, saves or emulator captures. `.gitignore` is default-deny.
The app needs only your own USA LittleBigPlanet 2 (BCUS98245) ISO plus the 01.33 update
as RPCS3 installs it (`dev_hdd0/game/BCUS98245`); it never needs a decrypted copy of
anything, and protected or encrypted resources are not decrypted by this project.

## Build from source (developers)

Requires a recent stable Rust toolchain (CI pins the version in
`.github/workflows/ci.yml`). Windows is the primary development platform.

```sh
cargo build --release -p lbp-cli -p lbp-bevy-viewer

# 1. Scan your game directory into an inventory (output must be a new file
#    outside the game directory; add the update directory after a ';' to overlay it)
cargo run --release -p lbp-cli -- --game-dir path/to/disc-root --json inventory.json

# 2. Explore it
cargo run --release -p lbp-bevy-viewer -- --snapshot inventory.json --game-dir path/to/disc-root
```

Useful viewer options: `--select-guid HEX` to open a resource, `--level-guid HEX`
to load a level, `--play-sackboy` to play in it, `--help` for the full list.
Controls: left-drag orbit, right/middle-drag pan, wheel zoom, `R` reset, `F` frame.
More detail: [viewer usage](docs/viewer.md), [Bevy preview](docs/bevy-preview.md),
[update overlay](docs/update-overlay.md).

Checks used by CI: `cargo fmt --all --check`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`. To contribute, see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Layout

| Path | Contents |
|---|---|
| `crates/lbp-core`, `lbp-io`, `lbp-archive`, `lbp-resource` | readers: containers, archives, resource headers |
| `crates/lbp-texture`, `lbp-mesh`, `lbp-preview`, `lbp-level` | texture, mesh, material, animation and level parsing and preparation |
| `crates/lbp-script-vm`, `lbp-runtime`, `lbp-sim`, `lbp-ui-script`, `lbp-gooey` | script VM, gameplay runtime, simulation, UI scripts |
| `crates/lbp-cli`, `lbp-catalog`, `lbp-viewer`, `lbp-bevy-viewer` | scanner, catalog, and the two viewers |
| `docs/` | format evidence and per-milestone notes ([architecture](docs/architecture.md), [formats](docs/formats.md), [roadmap](docs/roadmap.md)) |

Parsing crates are independent of any UI or renderer. Earlier research milestones
are logged in [docs/readme-history.md](docs/readme-history.md).

## Legal

- The authored source in this repository is licensed under either of
  [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. That license
  does not cover any game data.
- The implementation is written from independent analysis and published community
  format documentation. Third-party tools are used only as external references
  (see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) and
  [community projects](docs/community-projects.md)); no recovered or third-party
  game code is copied into the product.
- LittleBigPlanet is a trademark of Sony Interactive Entertainment. This project
  is not endorsed by or affiliated with Sony or Media Molecule.
- Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md);
  never submit game assets or decompiled code.
