# P3d: gloss colour maps (output port 2 wired from one sampler)

Census (PR #108, 8,094 materials): port 2 is a sampler in 2,132 graphs, a static colour in
1,752 (drawn since P2q), and a type10 computed subgraph in 1,853 (not drawn).

Change: a port 2 wired straight from one unswizzled sampler loads like the bump map and
multiplies the gloss term's colour (`spec = map.rgb * pow(R.L, CosinePower)`, INFERRED from
the static-colour form in P2q; alpha unused). The map rides graph slot 5 (free unless a D189
per-fragment graph is bound), with `graph_info.y` = UV mode: 1 when its coordinates equal the
diffuse sampling UVs, 2 on UV1 when UV1 is free or already holds the same coordinates as the
bump map. Otherwise the draw names "gloss map not drawn" in its details. Only under
`--lbp-lighting` like the other P2q surface parts.

Not done: type10 subgraph gloss (needs the fragment evaluator), glow (port 4), reflection
(port 6, 81% type12-computed), per-material spec shaping beyond CosinePower.
