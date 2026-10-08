# Backlog (Integrator-owned; refreshed 2026-10-08 after D99 from the 01.33 census)

Source: `local-analysis/b99-diffuse-route-shapes-0133.json` (schema 2, local only),
the D64 `diffuse_route_census` tool run on 01.33 (disc + update layers). Scope is
diffuse route selection only: these are material locations with MAP rows, not asset
counts and not payload compatibility. Counts only; no asset bytes or GUIDs here.

Totals (8,094 materials): texture 6,953; solid 619; unwired black 10; animation-base
only 121; instance-color only 267; unsupported 102; no MAP row 65.
(After D75: animation-base 60, instance 225, unsupported 258. Before D75: texture 6,828,
instance 207, unsupported 386.)

Node names follow Toolkit `BoxType` @ d6a2ef80 (TEX=1, THING_COLOR=2, COLOR=3,
CONST=4, CONST4=7, MUL=10, MIX=12, BLEND=16). `TEX*` = sampler with a named
animation. Families are exclusive, assigned in this order: any THING_COLOR, any
BLEND, 2+ textures with an animated sampler, exactly 2 TEX + COLOR under MUL only,
any MIX, other.

## Ranked unsupported families (102 materials after D99)

Ranked by count and tractability (THING_COLOR is larger but blocked on real
instance values).

| Rank | Family | Materials | Notes |
|---|---|---|---|
| done | <a id="two-tex-color"></a>Two textures × static COLOR (MUL only, any nesting) | 0–3 (was 113) | D75 merged: 110 now texture route, Toolkit parity 128/0 mismatches. Any leftovers are edge shapes now counted under Other. |
| 2 | <a id="animated-sampler-product"></a>2+ texture product with an animated sampler | few (was 65) | D78 one animated leaf, D92 2+ animated leaves, both under the animation-base opt-in. Nested-tint ThingColor forms still refused. |
| 3 | <a id="thing-color"></a>Any THING_COLOR in the diffuse chain | about 80 (D86 and D88 moved 16 to instance-only; left: BLEND 38, ThingColor in sampler ports 14, 2+ animated samplers 12, ADD 13) | Includes the BLEND+THING shapes (7 / 7 / 6 / 5). Needs supplied-color opt-in; real values unknown. |
| 4 | <a id="mix"></a>MIX shapes | few (was 14) | D83 grid alpha MIX, D99 off-grid alpha MIX via the Bevy dual path. |
| 5 | <a id="blend"></a>BLEND without THING_COLOR | 14 | D94 threshold rule; D97 masks (red sRGB, alpha linear) and two-sample BLEND on the grid. Left: products under a BLEND input (37 of all BLEND), three samples (3), off-grid (1). |
| 6 | <a id="small"></a>Other | about 37 (D89 took 4 ADD shapes; UV channel raw 2 (4) stays unsupported) | D87 took CONST4 alone, COLOR x COLOR, TEX with CONST/CONST2 UV scale (25). Left: ADD-containing 8, UV channel raw 2 (4), COLOR outside 0..1 (3), animated samplers, deeper products. |

## Opt-in classes (not unsupported)
- Instance-color only 267 (+8 D86, +8 D88, +3 D92, +6 D96, +9 D97, +5 D99).
- Animation-base only 106 (60 single-sampler + 46 from D78 multi-texture products).

## Other open items (no census count)
- Glow/reflection/non-diffuse channels not rendered.
- No unsupported resource headers (FNTb read since D79); patch.edat not decrypted. Branch 0x4431 (Vita
  DOUBLE11) headers now parse (D77, 116/116); their payloads are refused by design.
- BORDER color, CLAMP5, mirror-once 6–8, filtering fidelity unknown.

## Texture decode (D90)
- After D98, all 23,247 texture hashes on 01.33 decode (top mip). The volume layout
  (RPCS3 source only) and B8 remap rules are not Toolkit-oracle confirmed.
