# Level scripts, part 111: the viewer switches to the chosen level (2026-10-18, D364)

The pod handed the chosen story level to the game (part 110), but the viewer stayed in
the pod. It now loads that level in place of the pod, in both modes, as the game's level
load replaces the pod with the level.

### What changed

- `lbp-bevy-viewer` (`level_play.rs`): reading a level world (catalog, game files,
  scripts, story slots, translations, profile) is shared code (`WorldSource`), used at
  start and again for the launched level. On a hand-off with a root GUID a worker thread
  reads that level; the pod pauses until it is read, and plays on if the read fails.
  When it is ready, `switch_level` replaces the scripted world (VM, host, scripts and
  their per-level state), drops the shell level, the Story state, the pod's QA presses
  and the trace, removes the pod camera rig, the pod computer's state and the Story
  lights, and loads the level for display as the browser's Load button does.
  `--level-scripts-hold-at` keeps counting ticks across the switch.
- `main.rs`: `switch_to` (split out of `retarget`); `clear()` despawns with
  `try_despawn`, as the pod's own clean-up may have despawned the same entities.

MEASURED (the part 109 runs, scripted and native, hold at 900): both switch after tick
761 to the root level 0001f333 and load it, with no warnings or panics. With
`--preview-lighting` the capture is the fog colour only, as it is when the same level is
loaded on its own: the viewer's overview camera sits beyond the level's fog.

Left out: the game's loading screen and its timers, the level's start camera and
entrance (the camera frames the whole level), the pod's Sackboy after the switch, and
reusing the catalog (the worker reads it again).

### Next

Start the launched level as the game does: its camera at the entrance and the player
spawned there; then the warnings (0x402b24), the select sound and the other searches.
