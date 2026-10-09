# P2u: mipmaps and anisotropic filtering for game textures

2026-10-09 (America/Chicago). Plan: `lbp2/research/shading-lighting.md` item 4.

Before: game textures were uploaded as a single level and sampled bilinear, so tiled cloth
(the Pod globe's 512 px texture, about 12x) and distant level surfaces shimmered and aliased.

After: `adapter::image_axes` / `image` / `linear_image` build a full box-filtered mip chain on
the CPU and use a trilinear sampler with 16x anisotropy.

- Colour textures average in linear light (sRGB decode, mean, encode); alpha is averaged as
  stored. Data textures (bump, `linear_image`) average stored values.
- Odd sides clamp the last row/column; a side of 1 stops shrinking while the other side continues.
- Textures whose asset asks for point sampling (`set_nearest`) get nearest min/mag/mip and
  anisotropy 1 (wgpu rejects anisotropy with non-linear filters).
- Cost: about +33% GPU texture memory for textures larger than 1x1 (lbp-preview's retained-bytes
  budget counts base levels only).

Not verified against the game: the real minification filter, LOD bias and anisotropy are
INFERRED (16x is a viewer choice). A sampler read in the RenderDoc capture requested from the PC
settles it (min/mag/mip filter, max aniso, whether the disc textures carry their own chains).
Alpha-tested textures lose coverage in low mips with a plain average; not compensated yet.
