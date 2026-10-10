# P3a: AlphaTestLevel cut-out under the LBP-shaped lighting

2026-10-09 (America/Chicago). Plan: `lbp2/research/shading-lighting.md` item 5 (first step).
Evidence: PC fpdec read, `lbp2/oracles/results/2026-10-09-shading-fpdec.md`: 885 materials carry a named
AlphaTestLevel constant and write alpha 0 below it; there is no KIL (cut-out by the RSX alpha test is INFERRED).

Before: every non-decal draw was opaque, so a leaf, fence or wire texture drew as a solid quad with its alpha ignored.

After: with the LBP-shaped lighting on (the same switch as bump/gloss: `--bump-gloss` / on with `--lbp-lighting`),
a material whose stored AlphaTestLevel (GfxMaterial scalar, clamped to 0..1, NaN or 0 = none) is above 0 discards
fragments whose colour alpha is below it. The surface report line counts alpha-tested draws.

Limits: the scalar is assumed to equal the shader's AlphaTestLevel constant (the Pod's materials store 0.5 and
their diffuse alpha is expected to be opaque, so the Pod should not change). If a material uses its diffuse alpha for
something else, discarding would punch holes; the PC check must compare a level with cut-out textures and the Pod
against RPCS3. Alpha-tested mips lose coverage with the plain box filter (not compensated). Glow, reflection and the
spec map are the next steps.
