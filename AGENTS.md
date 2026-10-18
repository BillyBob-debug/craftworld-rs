# Project rules

craftworld-rs is an independent Rust/Bevy reimplementation of LittleBigPlanet 2 that runs from the user's own game data. It started as an asset explorer; since 2026-10-08 the user has approved gameplay runtime work (physics, logic, scripts, a playable Sackboy) in separate crates that the parsing crates never depend on; see `docs/runtime-research.md`.

## Hard lines

These protect what the public repository and the shipped app contain. They are not negotiable without the owner's explicit, typed decision.

1. **No Sony material ships.** The repository, releases and the exe contain no original LBP code, assets, game files, decrypted files, keys, firmware, saves, screenshots, emulator frames, captures or decompiler/disassembler output. Runtime assets come from the user's own game data at run time.
2. **Recovered code is evidence, never source.** Decompiled, disassembled, lifted or recompiled game code may be read for research, but is never copied, translated, adapted or ported (to Rust, C or anything else) into the product. Facts recovered from it (formats, constants, formulas, the order things happen in, state machines described in our own words) are findings we implement independently. Do not claim a strict clean-room process solely because the delivered code contains no copied source.
3. **The app needs only the game as RPCS3 installs it.** The only runtime requirement is the user's fully updated USA BCUS98245 ISO plus the update in `dev_hdd0/game/BCUS98245` (other regions later). No feature may require a decrypted copy of anything. Data that exists only in encrypted files (e.g. the scripts in patch.edat) gets a built-in fallback, so every mode works without it. This rules out shipping a static recompilation of the game's executable.

## Research methods

Every research method is allowed on the user's own copy, used in whatever order answers the question fastest. Results stay local; only independently written findings reach the repository.

- Documented specifications and open-source readers (respecting their licences: MIT-derived source keeps its notices; GPL/AGPL needs a deliberate licensing decision; public visibility is not reuse permission).
- Local decryption and protected-data inspection, executable disassembly and decompilation (Ghidra and similar), memory reads, and RPCS3 (frame-stepping, background input, memory observation; rules below).
- Interpreting the user's own compiled game scripts (RScript) at runtime (approved 2026-10-08). The interpreter and its native-function API are written independently from recorded findings; no script bytecode or disassembly is committed. The VM also serves as the dev oracle for native Rust ports, which are checked event for event against its traces.
- A local static-recompilation build of the user's own executable (e.g. ps3recomp-based) as an oracle, like RPCS3: built and run outside the repository or under ignored `local-analysis/`, never shipped and never a source for product code.
- Any other local experiment. Record the question, provenance, findings and limitations for each investigation.

Raw research material (decompiler output, lifted code, dumps, decrypted files, diagnostic JSON, byte samples, captures) lives only under ignored `local-analysis/`; game trees under ignored `game-data/`; community reference repositories under ignored `references/` (do not blindly copy their bundled assets or templates). Original ISO, disc images, extracted game directories, saves and executables are read-only: open them in read mode and never mount with write access or patch them. Online service modification is out of scope.

## Repository hygiene

- Keep the default-deny `.gitignore`. Check prospective tracked files before publication; extensions alone do not prove provenance. Never force-add game data. Commit synthetic recipes and expected metadata only, never original asset bytes.
- The project's own authored source is licensed MIT OR Apache-2.0. The physics dependency is avian2d (MIT OR Apache-2.0, approved 2026-10-08).
- Work goes through PRs to main. Threads push branches and open PRs without asking. Verified PRs (required checks green, checked against game data or oracles, one-line verdict comment) are merged without asking the owner (2026-10-09, 2026-10-16). GitHub releases and publishing outside the private pages need the owner's go.
- Ask before anything irreversible: deleting user data or saves, bulk deletions, outward GitHub settings changes, pushing to someone else's repo.

## Implementation

1. Start by reading `STATUS-CURRENT.md` (or the lane file in `docs/lanes/`), then only the relevant sections of `STATUS.md`, `RESEARCH.md` and the format/architecture documents; find sections by heading rather than reading these large files whole.
2. For each format rule, record source URL, pinned commit, class/method or section, evidence category, build/revision applicability, conflicts and test coverage near the implementation. Use the five labels in `docs/formats.md`.
3. Parsing libraries must not depend on UI/rendering. Keep disk layouts, intermediate representations and GPU/UI representations separate. Scanner and parsing crates stay free of GUI, physics and gameplay; runtime work goes in its own crates.
4. Implement small, reviewable milestones. No multiplayer or speculative format parser.
5. All lengths/counts/offsets/revisions are untrusted. Use checked arithmetic, strict bounds, decompression budgets, cycle limits and explicit unsupported/encrypted outcomes. Preserve duplicate MAP aliases and conflicting candidates rather than silently choosing one.
6. Resource type inferred from extension must remain distinct from type confirmed by magic. Unresolved is not corrupt; not parsed is not empty.
7. Use deterministic ordering and versioned JSON. Keep timing, absolute paths and generation timestamps outside the deterministic inventory body.
8. Compare with pinned community readers where feasible. A successful header probe does not prove payload compatibility or visual correctness.

## Verification and handoff

- Every asset-compatibility milestone reports newly viewable and newly complete assets, with baseline, counting unit and tested scope. Report unknown totals explicitly; material-route eligibility alone is not an asset count. Complete means all textures are showing and geometry is correct, independently of whether the renderer is finished. Display these distinctions and measured counts in the app.
- Run the narrow relevant checks, record exact commands and outcomes, and state what was not tested. For Rust changes, use `cargo fmt --check`, relevant tests and Clippy when a toolchain is available. Do not claim compilation if it is unavailable.
- Record failed experiments and blockers in `STATUS.md`; do not repeat an unchanged approach without new evidence. Update `STATUS.md`, `MODLOG.md` and the lane file after meaningful work; update architecture/formats when knowledge changes.
- Prefer game data, memory reads and oracles to pixels; view images only to answer a concrete visual question. Prefer headless checks.
- Research dates use the user's America/Chicago calendar. Repository commit timestamps may remain explicitly UTC.

## The owner's PC

- Never take window focus. Test programs and viewers open on the second (non-primary) monitor without activation (or `Start-Process -WindowStyle Hidden` with stdout/stderr redirected); verify placement first.
- RPCS3 (standing approval 2026-10-08, 2026-10-18: launches need no asking): one instance at a time, launched only through `local-analysis/pc-helpers/rpcs3_launch.py` and driven with background input and capture helpers, never by focusing its window. All three LBP2 game patches stay off. Gameplay and Sackboy measurements come from the blank Moon level with the default camera, never from story-level captures. Never save the level or profile, never inject into the process, no debugger breakpoints without asking. Close everything you launch.
- GPU captures use RPCS3's own RSX capture replayed offline.
- Disk: point `CARGO_TARGET_DIR` at an existing `target-pr*` dir, keep at most two live PR target dirs, clean targets of merged PRs, and delete nothing outside target dirs.

The local AI modding guides inform this workflow; their example AGENTS templates are adapted to this user's explicit scope, not copied as unrelated universal rules.
