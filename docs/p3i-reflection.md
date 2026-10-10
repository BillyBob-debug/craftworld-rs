# P3i: reflection (material output port 6)

Question: port 6 is the reflection term (CONFIRMED by d104-uv-channel E5). The #108 census
shows it is a graph in 1,317 of ~1,630 wired cases (type12 mix), a bare sampler in 286, a
colour in 20, type10 in 10. The PC port-6 census (1,619 graphs, 8,043 unique gmats): 59 use
cube maps only, 1,539 use 2D only, 1,596 use a sampler slot that port 0 cannot reach.

Evidence (fpdec, `oracles/results/2026-10-10-node-types-10-12.md`, `-reflection-uv.md`; all
CONFIRMED from listings): reflection = `mix(colour, TXB sample, w)` x luma(lit light) added to
the diffuse; the sample coordinate is the plain reflection vector R = V - 2N(N.V) with V =
normalize(TC4) and N the bump-perturbed normal; a 2D fetch uses R.xy raw in [-1,1], a cube fetch
R.xyz; the LOD bias is ReflectionBlur. No sphere-map or lat-long remap.

Change (only with the LBP surface extras and the LBP-shaped lighting):
- `surface::reflection_program` compiles the port-6 graph with the existing diffuse graph
  compiler (type 10 multiply, 12 lerp, colours, samplers) into per-fragment steps whose
  samplers ignore UV channel and scale/offset; `Surface.reflection` holds up to 4 decoded 2D
  textures (cube maps are refused by the texture decoder: "reflection not drawn (...)") and
  ReflectionBlur.
- the viewer binds them to graph texture slots 0..=3 (steps rows 0..=30, count in
  `graph_info.z >> 8`, blur in `graph_param[31].w`): no new bindings. A material whose diffuse
  uses a per-fragment graph or a second/third/fourth sample shows no reflection.
- the shader evaluates the graph at R (explicit gradients scaled by 2^blur, the TXB bias),
  multiplies by luma(light factor) and adds before the sqrt output.

R is in world orientation (PC verification 2026-10-10: fpdec builds R from TC4, and the vertex
program sets TC4 = pos - camera, so no view rotation; first draft used view space and was
wrong). World axes assumed equal to ours (INFERRED); luma weights 0.299/0.587/
0.114 and the light factor being the squared factor; output scale 1 (the NV40 scale field is
open); wrap mode is the material sampler's own (Repeat/Mirror/Clamp) for [-1,1] coordinates.
Not done: cube-map reflections (59 graphs, 3.6%), the glass refraction grab (TEX u15), type 12
port-2 weights (the graph compiler already takes port 2 when wired), animated values.

No GPU in the cloud: not rendered. PC verification wanted: fmt/test/clippy on 1.99; census
unchanged; surface-report counts of "reflection" draws on Bravery Test and tutorial_23;
before/after at gameplay distance on metal and glass (proc_da_vinci_metal, proc_base_glass
users); compare with an RPCS3 capture of the same object if one exists (decides view vs world
space); check nothing else changes.
