# D80 THING_COLOR (box 2) instance-color research — 2026-10-07

Research only. No route, parser or renderer code changes. Backlog rank 3
([backlog.md#thing-color](backlog.md#thing-color)): 99 materials on the 01.33 census
contain a THING_COLOR node in the diffuse chain and are still unsupported after D75
(225 more already select as "instance-color only" through the D61 opt-in). Counts are
material locations with MAP rows, not asset counts. Cloud session: no game data was
read; nothing here is verified against disc data.

## Question

Where does the value a THING_COLOR box outputs come from, what can a viewer that
reads only the user's resources legitimately supply for it, and what is the smallest
extension of the D61 supplied-color route ([instance-preview.md](instance-preview.md))
that could select more of the 99 without claiming game fidelity?

## Evidence

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (MIT). Paths under
`lib/cwlib/src/main/java/cwlib/` unless noted. Findings are restated independently;
no Toolkit code is copied, adapted or translated.

| # | Finding | Label | Source |
|---|---|---|---|
| E1 | Box type 2 is THING_COLOR. The enum comment guesses the box carries one u32 parameter, value 8 ("u32 (8)?"); the enum is "incomplete list". | `CONFIRMED_FROM_COMMUNITY_CODE` | [enums/BoxType.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java) line 10 |
| E2 | The shader generator emits no code for the box: it resolves to the shader input `iColor`, width 4. No parameter, port or swizzle of the box is read. | `CONFIRMED_FROM_COMMUNITY_CODE` | [util/gfx/GfxAssembler.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java) `resolve`, lines 268–269 |
| E3 | In the PS3 Cg template `iColor` is the fragment input bound to the `COLOR0` interpolant, i.e. a per-vertex/per-draw value produced by the game's vertex stage, not a material constant. The same `iColor` is also passed to the specular, glow, alpha, reflection, ramp, fuzz and ST7 helpers, so one value feeds every material channel. | `CONFIRMED_FROM_COMMUNITY_CODE` | `resources/shaders/templates/cg/brdf.cg` lines 499, 710 (`float4 iColor: COLOR0`), 282–347 helper signatures |
| E4 | For GLSL output the generator renames `iColor` to `thing_color`. The GLSL viewer template declares it as a `uniform vec4`; no Java code in the pin sets that uniform. The GL material baker templates instead define it as the constant `vec4(1,1,1,1)`. | `CONFIRMED_FROM_COMMUNITY_CODE` | GfxAssembler lines 971, 1131; `resources/shaders/templates/glsl/brdf.fs` line 64; `lib/cwlib-gl/src/main/resources/shaders/templates/glsl/bake.fs` line 20, `alphabake.fs` line 21 |
| E5 | Toolkit's glTF scene exporter bakes non-trivial diffuse graphs through that baker, so its exports evaluate THING_COLOR as opaque white; no Thing part is consulted for it. | `CONFIRMED_FROM_COMMUNITY_CODE` | `lib/cwlib-gl/.../gl/exporter/SceneExporter.java` 288–294 (`MaterialBaker`), E4 |
| E6 | Material XML export names the box `Color type="Thing"`, writing no value. | `CONFIRMED_FROM_COMMUNITY_CODE` | [structs/gmat/MaterialBox.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java) lines 294–297 |
| E7 | Candidate per-instance color sources stored in Things: `PShape.color` (RGBA, default `0xFFFFFFFF`; v4 floats up to revision 0x389, packed ARGB i32 after), `PShape.brightness` (from 0x301), `PShape.colorOff`/`brightnessOff` (from 0x303; v4 before 0x38a), `PShape.playerNumberColor` (from 0x2a3), and `PRenderMesh.editorColor` (default −1 = opaque white; v4 floats up to 0x31a, ARGB i32 after). | `CONFIRMED_FROM_COMMUNITY_CODE` (fields exist; their link to THING_COLOR is not shown) | `structs/things/parts/PShape.java` lines 55–85, 179–226, 278–282; `structs/things/parts/PRenderMesh.java` lines 32, 99–108 |
| E8 | Toolkit's v4→ARGB conversion forces alpha to 1.0 and quantises to 8 bits per channel, so Toolkit's own JSON round trip of pre-0x31a/0x389 colors does not preserve alpha or float precision. | `CONFIRMED_FROM_COMMUNITY_CODE` | `util/Colors.java` `RGBA32.getARGB` / `fromARGB` |
| E9 | Local data: THING_COLOR boxes seen so far are subtype 0, parameter word 0 = 8, others 0, unwired (D61 cohort, 01.00, branch 0). | `CONFIRMED_FROM_LOCAL_DATA` (D61 only; the 99 are not yet profiled) | [instance-preview.md](instance-preview.md) |
| E10 | Which Thing field (if any) the game writes into `COLOR0`, how shape color, brightness, on/off state, player colour and editor tint combine, the color space (sRGB-encoded vs linear), whether alpha is used, per-frame changes, and what a mesh without a Thing (inventory icon, catalog preview) receives. | `UNKNOWN` | — |

Applicability: E1–E6 come from Toolkit's LBP2 (PS3 Cg / GLSL) generator path; the Vita
template (`brdf_vita.cg`) also threads an `iColor` argument but D77 refuses Vita
payloads, so it is out of scope. E7 revisions are Toolkit serializer gates; the repo
has no Thing part decoder (`grep -rn PShape\|PRenderMesh crates` → 0 hits), so none of
E7 is reachable today.

Conflicts: E4 — Toolkit's viewer uniform (unset, i.e. GL default zero) and its baker
(white) disagree; neither claims to be the game value. E7 offers several plausible
sources and Toolkit links none of them to E2.

## Consequences for a preview

- A material resource alone cannot determine THING_COLOR. Any value is a policy
  input, so every route that evaluates it stays "supplied" and can never be marked
  Complete (D61 rule unchanged).
- White `(1,1,1,1)` is the only value with community-code precedent (E4/E5), and it
  is the identity for MULTIPLY, so it is the sensible default *preset* for a supplied
  color, labelled "Toolkit baker white", never as "game default".
- Because the same value feeds all channels (E3), a future glow/specular route must
  read the same supplied color, not a second one.
- A later "from level" source is possible only after a Thing part decoder exists
  and local evidence links one E7 field to rendered color (E10). That is its own
  milestone; D80 does not propose it as a route.

## Why the 99 fail today (hypotheses for the PC census)

`diffuse_route_census` counts a material as instance-color only when
`project_instance_color(graph, white)` succeeds *and* the derived graph selects
through an existing solid/texture route. A rank-3 material fails one of:

- H1 projection rejects the ThingColor itself (not variant 8, subtype ≠ 0, wired,
  animated) — reason "needs an unwired static ThingColor variant 8".
- H2 projection rejects sampler inputs other than `.a` on port 2 (ThingColor into
  UV ports 0/1, or a non-`a` swizzle) — reason "only supports scalar alpha
  subtraction on sampler port 2".
- H3 projection succeeds but the derived graph is a shape no route selects (BLEND —
  backlog lists BLEND+THING signatures 7/7/6/5 — MIX, ADD, a third multiply, an
  animated sampler).
- H4 ThingColor appears under a swizzle (e.g. `.a` into a MULTIPLY), which the
  derived COLOR keeps but downstream selectors reject.

Only H1/H2/H4 are THING_COLOR work; H3 is BLEND (D76), MIX (rank 4) or
animated-sampler (rank 2) work with an extra constant. The census split decides
which milestone gets the count.

## Proposed supplied-color route (not implemented)

Keep the D61 shape: derive a graph, keep source bytes and the original graph,
substitute a COLOR, then let existing selectors decide. Changes are additive:

1. **Census first.** Add a `thing_color_reason` bucket to
   `crates/lbp-preview/examples/diffuse_route_census.rs` for materials whose diffuse
   ancestry contains box 2: the `project_instance_color` error string verbatim (H1/H2),
   or `projected_unselected:<derived default-route reason>` (H3/H4). Also tally
   ThingColor `parameters_raw` words and subtype. Counts only.
2. **Swizzled ThingColor (H4).** Allow wires leaving a ThingColor with a single
   component swizzle `x/y/z/w` (`r/g/b/a`) followed by NULs into a MULTIPLY port, by
   substituting a COLOR whose four components all equal the selected component
   (independently derived from E2: the box is a plain 4-wide value, swizzle picks a
   component, width broadcast as for COLOR). Reject multi-component and unknown
   swizzle bytes.
3. **Sampler UV ports (H2).** Phase 1 still rejects ThingColor into sampler ports 0/1
   (UV add/multiply): a per-instance UV offset/scale is unbounded preview policy.
   Revisit only if the census shows a real count.
4. **Parameter guard (H1).** Keep variant 8 exact until the census shows other
   values; then add each value only with local evidence.
5. **Composition (H3).** No new evaluation here. When D76 BLEND or rank-4 MIX routes
   land, they consume the derived COLOR like any static COLOR; the projection must
   run before their selectors (it already runs before `select_diffuse`).
6. **Color input.** Unchanged API (`PreviewOptions::instance_color`, linear RGBA,
   finite, 0..1 validated by `DiffuseTint`). UI adds a "White (Toolkit baker)"
   preset next to the existing picker; the label stays "Supplied linear RGBA; actual
   game instance values are unverified". The census keeps using white.
7. **Projection record.** Extend `InstanceProjection` with
   `swizzled_color_nodes: Vec<(usize, u8)>` so the UI can show which component was
   broadcast. The default (no supplied color) path stays byte-identical.

Expected count change is unknown until step 1 runs on the PC. Steps 2/4 can only
move materials whose sole blocker is H4/H1; H3 materials move with their own route.

## Synthetic test plan (for the implementing milestone)

All fixtures are graphs built in test code from `GraphMetadata`/`BoxMetadata`
literals; no game bytes.

Projection (`instance.rs` tests):
- T1 regression: all existing D61 tests unchanged; no supplied color → graph returned
  untouched and no projection record.
- T2 accept: THING(.a) × TEX → derived COLOR with all four components = supplied `a`;
  `swizzled_color_nodes` = [(thing, b'a')]; selection succeeds via the tint route.
- T3 accept: THING(.x) and THING(.r) produce identical derived graphs.
- T4 reject: swizzle `xy`, `ax`, unknown byte `q`, non-NUL tail after one component.
- T5 reject: THING into sampler port 0 or 1 (any swizzle), with the H2 reason string.
- T6 reject: variant ≠ 8, subtype 1, wired ThingColor, named animation — H1 string.
- T7 bounds: >1024 boxes / >4096 wires / negative or out-of-range endpoints rejected
  before any substitution; ThingColor outside the diffuse ancestry left untouched.
- T8 color: NaN, ±inf, components outside 0..1 rejected by `DiffuseTint::validate`;
  white preset bits are exactly `0x3f800000` ×4.
- T9 determinism: two runs with the same input give equal derived graphs; wire order
  permutation gives the same projection record.

Census (`diffuse_route_census.rs`):
- C1 one synthetic graph per H1/H2/H4 reason and one H3 (THING × BLEND) asserting the
  `thing_color_reason` bucket string and that totals still sum to the material count.
- C2 a THING-free graph never gets a `thing_color_reason`.

## PC verify (Integrator)

Docs only, so no code check. To size the implementing milestone, run the D64 census
on 01.33 (disc + update) with a temporary local tally (or the step-1 bucket once
implemented) over the 99 rank-3 materials: ThingColor subtype/parameter words,
outgoing wire destinations (box type, port, swizzle), and the
`project_instance_color` error string or derived-route reason. Report counts per
bucket in the batch report; no GUIDs or bytes in the repo.

## Not tested

No Rust code changed; `cargo` checks unaffected. No game data, oracle run or viewer
QA in this session. The game-side source of `COLOR0`, its color space and its use of
alpha remain `UNKNOWN`; no Thing part is decoded by this repository.
