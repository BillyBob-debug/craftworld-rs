# Script VM (runtime plan item Rq)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Phase RS of
`docs/runtime-research.md`: the user chose (2026-10-08) to interpret their own compiled
scripts. This item is the VM core in the new runtime crate `lbp-script-vm`, built on the
U10 reader (`lbp_level::script`, D126). It runs synthetic bytecode only: the 427 scripts
on 01.33 sit in the encrypted `patch.edat` and are not readable yet.

## What it does

- `Vm::load(script, guid)` registers a class; `Vm::call(host, class, name, args, budget)`
  runs a function. `args` fill the function's argument rows in order.
- All 203 opcodes have a defined action or an explicit fault. `MOVs`, `EQs` and `NEs` are
  Nop-class in Toolkit's table and do nothing.
- Script-to-script calls (`CALL`, virtual `CALLVo` dispatching on the receiver's class and
  its superclasses), script objects and arrays on a VM heap, v4/m44 math, casts.
- Everything engine-side goes to a `Host`: native functions by (class name, function
  name), Thing script members (`GET/SET_SP_MEMBER`), `ref_ptr` members, native members
  (`GET_SP_NATIVE_MEMBER`), `CASTsp` and `WRITE`.
- Limits, all explicit `Fault`s: instruction budget per call (per tick in the runtime),
  call depth 256, 2^20 heap objects, 2^20 array elements, registers past `stack_size`,
  branches leaving their function, unknown opcodes, type mismatches, divide by zero,
  null objects, failed `ASSERT`.
- `check::check(script)` counts opcode use, functions without a terminal instruction, and
  branch targets under both branch rules (below), for the PC census.

## Evidence

CONFIRMED_FROM_COMMUNITY_CODE (Toolkit d6a2ef80, via the U10 reader): instruction classes,
operand bit fields, function/argument/field/function-reference tables, machine types,
modifier bits (`native` = bit 1).

INFERRED (no executable or Toolkit statement; each is a place a real script can disagree):

| Rule | Choice | How it gets settled |
|---|---|---|
| Register model | One slot per 16-bit address in a frame of `stack_size` slots; values never span slots | Real scripts: no register read before write at another width (VM faults on type mismatch) |
| Arguments | `ARG` stages at the argument address; `CALL` copies staged values to the same addresses in the callee frame; host calls fill `shared_arguments` offsets in order | Argument rows' offsets vs `ARG` addresses in listings |
| Receiver | Member functions take the receiver as an argument; virtual dispatch uses the lowest staged address | Listings: argument row typed as the own class |
| Branch target | `pc + offset` (`Rules::branch_from_current`), alternative `pc + 1 + offset` | Settled by D133: all 15,866 branches in the 426 scripts land in range under `pc + offset` (15,399 under the alternative) |
| `LCv4` | Four consecutive f32 constants from `index` | Constant-table sizes vs `LCv4` indices |
| m44 layout | Row-major; `GET_M44_XY` = row X, column Y; `rX` row, `cX` column | RPCS3 comparison of a scripted transform |
| Comparisons and casts | C semantics: wrapping int arithmetic, truncating division, `f as i32` | Script behaviour vs RPCS3 |
| `ARRAY_FIND` | Index of the first equal element or -1 | Script behaviour |
| Function lookup | By name through the superclass chain; first definition wins and repeats are counted (`ambiguous_calls`) | Census of duplicate names per class |
| Strings | `LCsa`/`LCsw` produce string values (MachineType has no string type) | Listings |
| No register | Operand `0xffff` = no register: reads give Void, writes are dropped, a typed `CALL` with that dest discards its result | D138 listings (`B` source, discarded `CALL` results); Register 65535 faults in the test script and the Gooey trace |
| Locals | Declared locals (the function's local-variable rows) start at their type's default; other registers start Void | D138 test script: `CheckEqual` got Void in TestLocals and the vector tests |

UNKNOWN: native function semantics (they live in the executable). Each one gets a host
binding from recorded findings, in call-count order (items Rr/Rs).

`Vm::last_site()` names the class, function and relative pc of the last executed
instruction, which after a fault is the faulting one. `script_tests` prints it with each fault.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0): `cargo fmt --all --check` clean;
`cargo clippy -q -p lbp-script-vm -p lbp-level --all-targets -- -D warnings` clean;
`cargo test -q -p lbp-script-vm` 5 tests:
- a loop under both branch rules, plus budget exhaustion; the branch check tells the
  two encodings apart;
- script call, native call through the host, `WRITE`, wrong argument count;
- object fields, arrays (append, length, find), v4 cross product and component read,
  Thing member get and set through the host;
- virtual dispatch to a subclass override through an inherited caller;
- faults: unknown opcode, register range, divide by zero, falling off the end, branch out
  of range, type mismatch, bad field reference, failed assert, call depth.

CI now also runs `lbp-level` (the reader this crate depends on) and `lbp-script-vm`.

Not tested: any real script (none readable yet), any native function.

## PC checks (once `patch.edat` is decrypted)

1. Run `check()` over every script (extend the U10 `script_census` example): branch
   totals per rule, unknown opcodes, functions without a terminal. The branch rule with
   every target in range becomes the default.
2. From listings: do member functions have a first argument row typed as their own class?
   Do `ARG` addresses equal the callee's argument offsets?
3. Native-call census (G10) ranks the host bindings to write first.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

Rs started in `docs/script-natives.md` (first bindings, `System.Yield` coroutines);
Rt: PScript instance data as Thing members and the create/update entry points.
