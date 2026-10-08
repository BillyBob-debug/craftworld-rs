# U10: RScript (`.ff`) reader and disassembly listing

Date 2026-10-08 (America/Chicago). Cloud implementer from the UI research plan
([ui-research.md](ui-research.md), item U10). **Not verified against disc data**; the
Integrator assigns the D-number, runs the PC checks below and updates STATUS/MODLOG.

## Question
LBP2's menus (Gooey), the Popit and many gadgets are compiled Fish scripts (SCRIPT, type
11, `FSHb`). On 2026-10-08 the user chose to build an interpreter for them, not to
rewrite their behaviour by hand. The first step: can a bounded reader load every script
on 01.33 and print a deterministic listing of its classes, functions and bytecode?

Ownership (agreed through the coordinator): this item owns the reader and the
disassembler. The "Sackboy, physics and logic research" thread owns the VM and the
native-function bindings built on `lbp_level::script::Script`.

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| Field order: `upToDateScript` (head <= 0x360), class name `str`, superclass resource, modifiers, 6 row tables, shared arguments, bytecode, line numbers, locals, ASCII and wide string tables, s64 constants (>= 0x30c), f32 constants, depending GUIDs (>= 0x1ec); head <= 0x1eb refused | `resources/RScript.java` `serialize` | CONFIRMED_FROM_COMMUNITY_CODE |
| Modifiers: fixed i16 from 0x3d9, else i32; bit i = ModifierType i | `RScript`, `*Row.java`, `enums/ModifierType.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Row layouts (type ref 4 bytes + script resource + i32 name; field/function ref; field/property/function definitions; TypeOffset; local variable) | `structs/script/*Row.java`, `TypeOffset.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Function ranges: from 0x3d9 fixed u16 begin + u16 length, else i32 begin + i32 end | `FunctionDefinitionRow.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| Bytecode: i32 count + u64 per instruction (`MemoryInputStream.u64`, ULEB when flag 1); opcode = low byte | `RScript`, `structs/script/instructions/*` | CONFIRMED_FROM_COMMUNITY_CODE |
| 203 opcodes 0x00..0xca, 18 classes, operand bit fields per class | `enums/InstructionType.java`, `InstructionClass.java`, `instructions/*.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| String tables: intvector of starts, i32 size (`bytearray`, not zigzag), string i ends at the next start | `RScript` | CONFIRMED_FROM_COMMUNITY_CODE |
| `longvector` byte-plane layout under flag 2 | `io/serializer/Serializer.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| `upToDateScript` gate: code 0x360 vs `Revisions` 0x33a; s64 gate: code 0x30c vs `Revisions` 0x30b | `RScript` vs `enums/Revisions.java` | CONFLICT; code gates used, PC check 2 |
| Name and type-name indices point into the ASCII table; `LCsa`/`LCsw`/`LCs64` index the ASCII, wide and s64 tables; member `field` and `CALL` `call` index field/function references | not stated by Toolkit (it keeps raw indices) | STRONG_INFERENCE; the census counts out-of-range indices |
| A type-reference name index of 0xffffffff means unnamed, not out of range (2026-10-08 fix) | PC census d133: all 1,966 name misses were type rows with 0xffffffff, all builtin or array types (void, bool, s32, f32, v4, m44, s64, object_ref/safe_ptr arrays); named class types never had it; 0 misses in other rows | STRONG_INFERENCE (measured); counted as `unnamed_type_refs`, other misses broken down by row kind in the census |
| Native semantics | executable only | UNKNOWN |

## Implementation (`crates/lbp-level/src/script.rs`)
- `script::read(bytes, limits, cancel)` → `ScriptReport { revision, result:
  Result<Script, Stop>, trailing_bytes }`; `read_payload` for an expanded payload.
- `Script` keeps every table raw (indices unresolved). `Script::listing()` returns a
  deterministic text listing plus `ListingChecks` counts (unknown opcodes, name, range,
  constant and member indices out of range).
- `class(op)`, `MNEMONICS`, `operands(bits)`, `machine_type_name`, `modifier_names` for
  the VM and other tools.
- Bounds: counts through `Reader::count`, negative sizes Invalid, string-table starts
  past the end or decreasing Invalid, longvector widths above 8 Unsupported, 64 MiB input
  and 256 MiB expanded budgets, cancel checks every 4096 instructions. Branch != 0, LBP3
  and heads above `LBP2_MAX` are Unsupported.
- Not executed, not wired into the viewer.

## Tests (synthetic recipes only, `crates/lbp-level/tests/script.rs`)
`u10_reads_both_function_layouts` (heads 0x3f8/0x3d8/0x360, flags 0 and 7),
`u10_every_opcode_has_a_class_and_mnemonic`, `u10_untrusted_input_stops_explicitly`
(every truncation of a recipe is Invalid; listing counters).

Cloud (Linux, Rust 1.99.0): `cargo fmt --all --check` ok; `cargo +1.99.0 clippy -q -p
lbp-level --all-targets -- -D warnings` clean; `cargo test -q -p lbp-level` all pass
(3 new).

## PC verification (Integrator)
1. `cargo build -q --release -p lbp-level --example script_census`, then
   `target/release/examples/script_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/u10-script-census-0133.json local-analysis/u10-script-listings-0133`
   Report: scripts, status counts, stop counts, heads, layout split (< / >= 0x3d9),
   functions / native functions / instructions, `listing_check_totals` (all 0 supports
   the inferred index rules), unknown opcodes, UI-named classes count.
2. Gate conflicts: any script with head in 0x33b..0x360 (does it parse with the
   `upToDateScript` field?) or head 0x30b.
3. Toolkit spot-check: open 3 scripts (one UI-named class, one small gadget, the largest)
   in Toolkit; class name, superclass, function count and instruction count equal.
4. Listings stay under `local-analysis/` (derived from game files; never commit).

## Expected effect / not tested
- Viewable/Complete: +0 (scripts are not assets). New count for the script track:
  scripts read complete, unit distinct content hash, scope 01.33 overlay.
- Not tested: real scripts, the inferred index rules, Vita/LBP3 scripts (refused).
- Next: U11 (native API census from the census JSON) and the VM in the Sackboy thread.
