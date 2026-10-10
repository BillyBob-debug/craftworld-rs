# P3c: what feeds each material output port

Question: the shading plan's material channels (sampled specular, glow, reflection) need to
know which output-node ports real materials wire, and from what (static colour, one sampler,
or a computed subgraph). Today only port 2 as one static COLOR (gloss) and port 3 as one
sampler (bump) are drawn (P2q).

Tool: `cargo run --release -p lbp-preview --example output_port_census -- SNAPSHOT GAME-DIR
MATERIAL-CENSUS local-analysis/NEW.json` (structure only, no textures decoded). Output:
`port_feed_counts` (port -> feed kind -> graphs) and `port_combination_counts`.
Feed kinds: `color`, `sampler`, `typeN_unwired`, `typeN_computed`, with `_swz` when the wire
swizzles or leaves a non-zero source port.

Evidence label for any port meaning inferred from this: INFERRED until matched against the
decoded shaders (port 6 = reflection is CONFIRMED by d104-uv-channel E5).
