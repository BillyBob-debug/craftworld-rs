# P3l: cube-map reflections (material output port 6)

Question: the P3i reflection graph skipped 59 graphs (3.6% of port-6 graphs) because their
sampler is a cube map and the 2D decoder refuses cubes. The fpdec listings (oracles
2026-10-10-node-types-10-12, -reflection-uv) say a cube fetch reads the plain reflection
vector R.xyz with the ReflectionBlur LOD bias (CONFIRMED from listings).

Change (only with the LBP surface extras and LBP lighting, like P3i):
- `surface::load_slot` decodes with `lbp_texture::decode_reader_layered`; `Reflection.textures`
  is now `ReflectionMap::Flat(texture, wrap)` or `ReflectionMap::Cube(layered)`. A cube's sample
  steps get UV-channel field `CUBE_CHANNEL` (2). One cube per graph; more is "reflection not
  drawn". Cubes must be square with six equal faces.
- viewer: `adapter::cube_image` builds a 6-layer Rgba8UnormSrgb cube texture (box-filtered chain
  per face, layer-major, clamped, trilinear, <= 1024 px) on binding 140/141; it leaves a
  placeholder (None) in the cube's 2D graph slot. `graph_color` takes the direction and its
  gradients (scaled by 2^blur like the 2D path) and samples the cube for channel 2.

INFERRED, unconfirmed: the cube lookup uses our world axes; the decoder's face order (+X, -X, +Y,
-Y, +Z, -Z, DDS caps order) and per-face orientation match wgpu's cube convention (no flips
applied); sRGB decoding of the cube like the other reflection maps; only the top mip is decoded
(mips are rebuilt on the CPU). Not rendered in the cloud (no GPU, no game data).

PC verification wanted: fmt/test/clippy on 1.99; surface-report count of "reflection" draws
rises by the cube graphs and no "cube" skips remain except multi-cube graphs; a gameplay-distance
capture of one cube-reflection material; if the reflection looks mirrored/rotated, flip or swap
faces here (one place: `adapter::cube_image`).
