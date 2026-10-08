# Script natives, first bindings (runtime plan item Rs)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Builds on the VM
(D128, `docs/script-vm.md`) and the G10 native-call census (D133,
`docs/d133-patch-scripts.md`; ranked export in the project files,
`lbp2/research/g10-native-top200.md`). Bindings are written from the names, counts and
general vector-library conventions only. No script bytecode, listing or engine code was
used or is committed.

## What it adds (`lbp-script-vm`)

- `signature::parse`: splits a mangled native name (`Name__` + parameter codes).
- `natives`: a table of bindings keyed by (declaring class, mangled name); a `World`
  trait for the engine side, whose methods default to an explicit "not available" error;
  `Bindings<W>`, which implements the VM `Host` and counts calls to unbound natives.
- `System.Yield__` suspends the VM: `Vm::start` / `Vm::resume` return
  `Step::Done(value)` or `Step::Yielded(task)`. `Vm::call` faults with `YieldInCall`.
- `Vm::has_code(class, function)`: whether a call resolves to script code.
- Example `native_coverage SCRIPT-DIR NEW-JSON`: the share of native call sites that have
  a binding, over all scripts and over gameplay scripts.

## Bound natives (41 including Yield)

| Group | Natives | Needs from `World` |
|---|---|---|
| Math | `floor__f`, `clamp__fff`, `atan2__ff`, `rotateX/Y/Z__f`, `translate__r`, `scale__r`, `mulPerElem__rr`, `length3__r`, `normalize3__r`, `length__p`, `NonDeterministicRandomBetween__ff` | random only |
| String | `Concat__Q6StringQ6String`, `StringFromInt__i`, `InfinitySymbol__` | nothing |
| LAMS | `Translate__i` | text by key |
| Thing | `GetThingUID__`, `GetParent/GetFirstChild/GetNextSibling__Q5Thing`, `GetWorld__` (also System), `GetWorldPosition__`, `GetCOM__`, `SetLocalPosition__m`, `IsInactive__` | thing tree, transforms |
| Switch inputs | `IsSwitchTriggered__i`, `GetSwitchActivationBinary/Analogue/Player__i` | switch input per port |
| PartList | `HasPJoint/PSwitch/PTrigger/PShape/PSpriteLight__Q5Thing` | parts |
| Other | `Audio.PlaySound__Q5ThingQ7StringA`, `Resource.LoadTextureByFilename__g`, `Resource.LoadPlanByFilename__g`, `Game.GetNumPlayers__`, `System.Yield__` (in the VM) | audio, resources, players |

Coverage against the G10 export, computed from its tables (unit: call instruction):

- **Gameplay top 100:** 953 of 1,356 calls are bound, across 40 of the 100 natives.
- **All-scripts top 200:** 2,981 of 11,999 calls are bound. Gooey (menu layout)
  dominates that list and is UI work, not Rs.

The PC run of `native_coverage` gives the exact totals over all native calls.

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80 `cwlib/enums/BuiltinType.java` lists
V2, V3 and V4 as consecutive builtins, all four floats natively.

INFERRED (each is a place real scripts or RPCS3 can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Mangling | `b c i f` bool/char/s32/f32; `p q r` = V2/V3/V4 (following BuiltinType order); `m` m44; `g` GUID; `Q<len><name>` class or enum | `native_coverage` reports unknown codes and unparsed names; argument machine types in listings |
| Receiver | One staged value more than the signature's parameters = member call, receiver first | Arity faults on real scripts |
| `Get*__Q5Thing` | The Thing parameter is the subject, in both static and member form | Listings |
| Yield | Suspends the calling script until the next tick; returns nothing | 52 of 255 gameplay scripts call it; RPCS3 timing (X1) |
| Matrix layout | Column vectors in column-major memory order, translation at indices 12..14 as in level-file world matrices | RPCS3 comparison of a scripted transform |
| Angles | Radians | RPCS3 |
| `atan2(a, b)` | C order: `atan2(y, x)` | RPCS3 |
| `normalize3` | xyz scaled to length 1, w kept; zero vector faults | Script behaviour |
| Switch natives | Separate fields: triggered, binary, analogue, player | Logic runtime (G-checks) |
| GUID values | Carried as s32 or pointer-sized values | Listings (`LC` before the call) |

UNKNOWN: everything else the census lists, including Gooey (menu layout, UI thread),
TutorialBase, PodComputer, the online search classes, and `Native.GetA/GetB`.

## Verification

Cloud (Linux):
- `cargo fmt --all --check`: clean.
- `cargo clippy -q -p lbp-script-vm --all-targets -- -D warnings`: clean.
- `cargo test -q -p lbp-script-vm`: 4 unit tests and 7 VM tests. They cover:
  - signature shapes, including a malformed `Q` length;
  - every table entry parsing with no unknown codes, and no duplicate entries;
  - math results (translation at 12..14, rotateZ maps +x to +y);
  - receiver split, null and missing receivers, and arity errors;
  - a world that lacks a feature saying so;
  - Yield over three ticks through `start`/`resume`, and `YieldInCall` from `call`;
  - `Math.floor` run through `Bindings`, and an unbound Gooey call counted.

Not tested: any real script (they are on the PC only). No `World` implementation over a
real level exists yet.

## PC checks

1. `cargo run -q -p lbp-script-vm --example native_coverage -- <script dir> local-analysis/<new>.json`
   on the D133 script directory. Report the totals line, the unknown signature codes and
   the top unbound gameplay natives.
2. If unknown codes appear, list each code with an example mangled name (names only).

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

- Rt: a `World` over the level runtime. It needs the thing tree from the level reader,
  then PScript instance data as Thing members, and then the per-tick create/update calls
  with Yield tasks.
- Rs, continued: the next unbound gameplay natives from the PC coverage run.
