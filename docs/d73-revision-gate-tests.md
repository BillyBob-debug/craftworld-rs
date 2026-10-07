# D73: synthetic tests for the 01.33 revision gates (952..=0x3f8)

Scope: prove the revision gates widened in D69 with synthetic bytes only. These tests
prove the gate (accept 952, 1000, 0x3f8 on the head-951 layout; reject 0x3f9+ as
`Unsupported`, not corrupt; lower bounds unchanged). They do not prove payload
compatibility on real 01.33 files. No game data is used and no gate or parser
behaviour was changed.

Evidence label: `CONFIRMED_FROM_COMMUNITY_CODE` for the gate values (Toolkit pin
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, MIT, `Revisions.LBP2_MAX = 0x3f8`, no
branch-0 condition above 951 for these layouts; see `docs/update-overlay.md`).
Applicability: ordinary clear binary resources, branch 0 / branch revision 0, flags 0/7.
Conflicts: none found. The tests themselves are synthetic recipes (no label upgrade).

| Gate | Source | Test (crates/lbp-resource/tests) | Lower bound kept |
|---|---|---|---|
| metadata `LBP2_MAX_REVISION` | `src/metadata.rs:287` (const at :16) | `metadata.rs::d73_gate_heads_952_1000_0x3f8_accepted_0x3f9_up_unsupported_not_corrupt` | head 0x108 has no dependency table |
| material prefix | `src/material.rs:134` | `material.rs::d73_widened_gate_...` | 919 rejected |
| material graph | via `inspect` prefix gate | `material_graph.rs::d73_widened_gate_...` | 919 rejected |
| material tail | via `inspect` prefix gate | `material_tail.rs::d73_widened_gate_...` | 919 rejected |
| plan prefix | `src/plan.rs:59` | `plan.rs::d73_widened_gate_...` | 892 rejected |
| plan inventory | no own gate; parent plan prefix | same plan test (`inspect_inventory` accepts and rejects) | 892 rejected |
| animation | `src/animation.rs:71` | `animation.rs::d73_widened_gate_...` | 887 rejected |

Revisions exercised: accept 951 (control), 952, 1000, 0x3f8; reject 0x3f9, 0x3fa,
0x10000 (metadata also u32::MAX). Six tests added in total (plan inventory shares the plan test).

Missed-gate search: `grep LBP2_MAX_REVISION|951` over `crates/lbp-resource/src` found
only the four gates above plus doc comments that still say "heads ..951" (plan.rs,
animation.rs, material.rs, material/graph.rs, material/tail.rs, plan/inventory.rs
module headers). Those comments are stale relative to D69 but out of scope here.

Bugs found: none.

Not tested: real 01.33 payloads, other branch ids, `lbp-archive` / `lbp-io` gates,
material preview routes, two-texture x COLOR materials.
