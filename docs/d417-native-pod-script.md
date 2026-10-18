# D417: native script ports, first the Pod (Popit part 12)

Before: with only the disc and the installed update, the Pod's two scripts (`Pod`, the
world script of `pod.bin`, and `Trigger_Pod`, its pod computer trigger) had no class
(all 426 scripts are in the encrypted `patch.edat`, D133), so they did nothing: no
engage request, no change-over, and the pad-zone test never matched (no class names).
After: they run as native Rust ports. The same run with the disc and update only opens
the pod: engage press, `RequestHostUsePod(true)`, "in use", the change-over value 0 to 1
in 20 ticks and the pod mesh swap, as with the decrypted scripts.

## The rule (approved strategy, docs/project-boost.md "Decide first" 1)
- A registry of ports (`lbp_runtime::NATIVE_SCRIPTS`, by script GUID or class name).
  `Runner` runs the VM when the class is loaded, else the port; `--native-scripts`
  (viewer and `level_scripts`) forces the port for a check against the VM.
- A port makes the same host calls the script makes (natives, member reads and writes,
  class tests) with the same names and arguments, so every engine effect stays in the
  bindings the VM uses. A Thing with a port gets the port's class names
  (`ThingState::script_classes`), so casts and `in_pad_zone` see `Trigger_Pod`.
- The VM is the oracle: `Bindings::trace` records every host call (tick, Thing, script
  site, name, arguments, answer; `lbp_script_vm::trace`, hand-written JSON).

## Port
- `NativeScript`: `class`, `guid`, `superclasses`, `on_create`, `update` (per tick, or
  the loop after a yield), `call` (other entries by name). `crates/lbp-runtime/src/native_script.rs`.
- `Pod` (g00004905, FOUND in the 01.33 script): `OnCreate__` (checkpoint
  `pod_checkpoint` active, then "update, yield" until `Finish`), `Update__` (the engage
  chain of part 21), `SetPodTransition__b`, `EnterPuterTrigger__Q5Thing`, `OnDestroy__`.
- `Trigger_Pod` (g00002d07): `Enter__Q5Thing` (the world's `EnterPuterTrigger` when the
  world is a `Pod`), `Leave__Q5Thing`, `IsPodComputerTrigger__` (true).

## Check
- Golden trace: `level_scripts <d133 edat snapshot> <disc;update;d133-edat-overlay>
  gamedata/levels/pod/pod.bin <new json> --play=30 --flags=2 --enter-pad-zone=3
  --press=6:0x80 --player-colour=ff3366cc --trace=<file>`: 890 events of the ported
  code. The forced port (`--native-scripts`) and the disc + update run (no scripts)
  give the same 890 events. Fixture: ticks 0-1, 3-8, 25-27
  (`crates/lbp-runtime/tests/fixtures/d417_pod_vm_trace.rs`, JSON, names and numbers only),
  replayed by `tests/native_pod_trace.rs`.
- Viewer, disc + update (`d79-inventory-0133.json`): `--level-guid 00018fdb
  --pod-camera 1 --pod-shell 000027b9 --play-sackboy --level-scripts
  --level-scripts-press-at 40 --level-scripts-flags 2`: request on tick 40, in use 41,
  change-over 0.05 at 41 and 1.00 at 60. The D411 camera fallback line still shows.

## Open
- `BaseWorld.OnCreate__` / `Update__` (score and player-indicator state) and
  `Pod.Render__` are not ported; the planets level's scripts (camera dolly, planets)
  have no port yet, so the D411 resting camera stays.
