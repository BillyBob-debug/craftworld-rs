# D700: Sackboy's model (stitches, zip, eyes)

FOUND = read from the 01.33 data or its compiled programs; INFERRED = our reading.

## D700: stitch and zip overlays blend by their texture's alpha
- `sack_boy.mol` (`0000043f`) draws each cloth group twice: the knit (`000025e2`, UV0)
  and, on the same vertices, a UV1 overlay: stitches `00001d92` (64x256 BC3, U clamp)
  and the shoulder/hip zips `00001d94` (128x512 BC3). FOUND (mesh_group_probe).
- Both overlays wire one texture box into the colour port (0) and the alpha port (1),
  with no swizzle. The stitch program fetches that texture a second time for `.w` alone
  (TEX, write mask w), so the alpha is the texture's own alpha. FOUND for `00001d92`;
  `00001d94` has the same wiring (INFERRED same program shape).
- The textures' RGB is black outside the thread and teeth, with the alpha carrying the
  thread plus a soft shadow. Drawn opaque, the overlay painted those black texels over
  the knit wherever it won the coplanar depth test: the black bands at the wrists,
  shoulders, knees and across the top of the head.
- Rule: a single-texture material whose ports 0 and 1 are each one unswizzled wire from
  the same port of the same texture box, and whose texture has any alpha below 255, is
  drawn as an overlay (`overlay_alpha`, so alpha blended with the D338 depth bias).
  `alpha_from_colour_sampler` in `linked.rs`; test `d700_alpha_from_the_colour_sampler`.

## Probes
- `material_graph_dump` now prints the texture slots' GUIDs (for `tex_dump`).
- `morph_probe SNAPSHOT GAME-DIRS MESH ANIM`: morph names, vertices moved, frame-0
  weight. Sackboy has 29 morphs (brows, eyelids, mouth, zip); `asymmetry_morph` is 1.0
  in frame 0 of clip `0001a697`, the rest 0.

## Open
- Eyes (`0000053c`: black, specular colour 1.81 on port 2): compare against a close
  RPCS3 frame.
