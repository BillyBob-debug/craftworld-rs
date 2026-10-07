# Backlog (Integrator-owned; refreshed 2026-10-07 after D75 from the 01.33 census)

Source: `local-analysis/d75-diffuse-route-shapes-0133.json` (schema 2, local only),
the D64 `diffuse_route_census` tool run on 01.33 (disc + update layers). Scope is
diffuse route selection only: these are material locations with MAP rows, not asset
counts and not payload compatibility. Counts only; no asset bytes or GUIDs here.

Totals (8,094 materials): texture 6,938; solid 603; unwired black 10; animation-base
only 60; instance-color only 225; unsupported 258; no MAP row 65.
(Before D75: texture 6,828; instance-color only 207; unsupported 386.)

Node names follow Toolkit `BoxType` @ d6a2ef80 (TEX=1, THING_COLOR=2, COLOR=3,
CONST=4, CONST4=7, MUL=10, MIX=12, BLEND=16). `TEX*` = sampler with a named
animation. Families are exclusive, assigned in this order: any THING_COLOR, any
BLEND, 2+ textures with an animated sampler, exactly 2 TEX + COLOR under MUL only,
any MIX, other.

## Ranked unsupported families (258 materials after D75)

Ranked by count and tractability (THING_COLOR is larger but blocked on real
instance values).

| Rank | Family | Materials | Notes |
|---|---|---|---|
| done | <a id="two-tex-color"></a>Two textures × static COLOR (MUL only, any nesting) | 0–3 (was 113) | D75 merged: 110 now texture route, Toolkit parity 128/0 mismatches. Any leftovers are edge shapes now counted under Other. |
| 2 | <a id="animated-sampler-product"></a>2+ texture product with an animated sampler | 65 | Signatures 18 / 15 / 13 / 12. Single-sampler animation base opt-in exists already. |
| 3 | <a id="thing-color"></a>Any THING_COLOR in the diffuse chain | 99 (was 117; 18 moved to instance-color only by D75) | Includes the BLEND+THING shapes (7 / 7 / 6 / 5). Needs supplied-color opt-in; real values unknown. |
| 4 | <a id="mix"></a>MIX shapes | 14 | Two-texture alpha MIX 8. |
| 5 | <a id="blend"></a>BLEND without THING_COLOR | 14 | D76 researched it: proposed rule and test plan in docs/d76-blend.md; not implemented. |
| 6 | <a id="small"></a>Other | about 66 | TEX/COLOR/MUL non-two-texture 11; TEX/MUL 10; CONST4 alone 9; COLOR/MUL 8; TEX+CONST 7; TEX alone 4; ADD-containing 8; rest ≤3. |

## Opt-in classes (not unsupported)
- Instance-color only 225 (144 multiply rule, 81 sampler rule).
- Animation-base only 60.

## Other open items (no census count)
- Glow/reflection/non-diffuse channels not rendered.
- FNTb font (1 resource) unsupported; patch.edat not decrypted. Branch 0x4431 (Vita
  DOUBLE11) headers now parse (D77, 116/116); their payloads are refused by design.
- BORDER color, CLAMP5, mirror-once 6–8, filtering fidelity unknown.
