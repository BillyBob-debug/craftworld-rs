# D193 — sampler port folds under the nested instance tint (2026-10-09)

Follows D96 ([d96-thing-sampler-ports.md](d96-thing-sampler-ports.md)) and the nested
tint of D88/D92. Route code only (`lbp-preview`); no parser, shader or viewer change.

## Question

After D192, Bravery Test (GUID 0001e795) still shows 22 neutral Things, all mesh
00019e98, whose one remaining group fails with "Unsupported: folded sampler port inputs
under a nested instance tint" (D96 rule 5). The same reason is the only one-off left on
the 25ff1aa3 shape in the D96 census: `TEX a × (THING × animated TEX b)` with port 2 of
`a` = `MIX(CONST4, CONST4, THING.a)`.

## Rule

`select_instance_diffuse` no longer refuses the combination. The two rewrites act on
disjoint parts of the derived graph and are both exact:

1. D96 folds the static port inputs of a sampler into that derived sampler's
   scale/offset and drops its incoming wires (`fold_sampler_ports`).
2. The nested-tint rewrite replaces the inner `THING × TEX b` product with a copy of the
   derived `b` box and keeps the projected ThingColor as `nested_tint`. It only fires
   when `b` has no remaining inputs, so if `b` itself was port-folded the copy carries
   the folded transform.

The result is the existing two-sample product, tinted by `nested_tint` in `linked.rs`
("exactly two direct samples" guard unchanged). D96 rule 1 (folded samplers are static),
the D92 several-animated refusal and the animation-base requirement for the animated
leaf are unchanged.

## Evidence

- INFERRED: composition of two rewrites that each have oracle parity (D96: 0 mismatches;
  D88/D92 nested tint: 0 mismatches). No oracle row covers the combination itself; the
  Toolkit diffuse oracle has no instance-color input.
- That 00019e98's failing group is the 25ff1aa3 shape is INFERRED from the shared reason;
  the PC level report confirms the outcome.

## Tests

`d193_port_folded_sampler_composes_with_the_nested_tint` (instance.rs): the 25ff1aa3
shape projects with `folded_sampler = [inner, b]`, one port fold on `a` with offset
(-0.25, -0.75), refuses without `--animation-base`, selects two samples with it.

## Not tested (cloud)

No game data: Bravery Test level report, corpus census and the gameplay-distance capture
are the PC verification.
