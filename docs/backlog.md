# Backlog (Integrator-owned; refreshed 2026-10-07 from the 01.33 census)

Source: `local-analysis/d68-diffuse-route-shapes-0133.json` (schema 2, local only),
the D64 `diffuse_route_census` tool run on 01.33 (disc + update layers). Scope is
diffuse route selection only: these are material locations with MAP rows, not asset
counts and not payload compatibility. Counts only; no asset bytes or GUIDs here.

Totals (8,094 materials): texture 6,828; solid 603; unwired black 10; animation-base
only 60; instance-color only 207; unsupported 386; no MAP row 65.

Node names follow Toolkit `BoxType` @ d6a2ef80 (TEX=1, THING_COLOR=2, COLOR=3,
CONST=4, CONST4=7, MUL=10, MIX=12, BLEND=16). `TEX*` = sampler with a named
animation. Families are exclusive, assigned in this order: any THING_COLOR, any
BLEND, 2+ textures with an animated sampler, exactly 2 TEX + COLOR under MUL only,
any MIX, other.

## Ranked unsupported families (386 materials, exact)

Ranked by count and tractability (THING_COLOR is larger but blocked on real
instance values).

| Rank | Family | Materials | Notes |
|---|---|---|---|
| 1 | <a id="two-tex-color"></a>Two textures × static COLOR (MUL only, any nesting) | 113 | Top signatures 40 / 35 / 23 / 6 / 5. The 40/35/6 shapes are rejected by the sampler rule (sampler not static or not UV0/UV1); 23/5 by the multiply rule (product of a product). |
| 2 | <a id="animated-sampler-product"></a>2+ texture product with an animated sampler | 65 | Signatures 18 / 15 / 13 / 12. Single-sampler animation base opt-in exists already. |
| 3 | <a id="thing-color"></a>Any THING_COLOR in the diffuse chain | 117 | Includes the BLEND+THING shapes (7 / 7 / 6 / 5). Needs supplied-color opt-in; real values unknown. |
| 4 | <a id="mix"></a>MIX shapes | 14 | Two-texture alpha MIX 8. |
| 5 | <a id="blend"></a>BLEND without THING_COLOR | 14 | BLEND semantics not yet researched. |
| 6 | <a id="small"></a>Other | 63 | TEX/COLOR/MUL non-two-texture 11; TEX/MUL 10; CONST4 alone 9; COLOR/MUL 8; TEX+CONST 7; TEX alone 4; ADD-containing 8; rest ≤3. |

## Opt-in classes (not unsupported)
- Instance-color only 207 (126 multiply rule, 81 sampler rule).
- Animation-base only 60.

## Other open items (no census count)
- Glow/reflection/non-diffuse channels not rendered.
- Branch 0x4431 resources (116) and FNTb fonts unsupported; patch.edat not decrypted.
- BORDER color, CLAMP5, mirror-once 6–8, filtering fidelity unknown.
