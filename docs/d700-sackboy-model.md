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

## D701: the zip pull's gloss is its own diffuse
- `zip_pull_steel` (`00000439`) wires port 0 and port 2 each from a multiply of the same two
  samplers (brushed `684` on UV0 x AO `e93` on UV1). Its compiled program adds
  `2 x light x pow(R.L, CosinePower) x P` with P that same product, and no constant on the
  spec (the knit `000025e2` damps its spec by its colour box `ta4`). FOUND (gmat graph and
  the program's listing via `local-analysis/shading/scripts/cgb.py blobs zip_pull_steel`).
- Rule: a gloss port wired from a static multiply equal to the diffuse's (same inputs,
  ports and swizzles) tints the gloss by the diffuse colour (`specular_diffuse`, viewer mode
  `graph_info.y` 3). Test `d701_a_gloss_multiply_equal_to_the_diffuse_is_the_diffuse`.
- Effect in the matched Pod view is small (CosinePower 22 keeps the highlight tight): the
  pull stays about 0.7x the game's brightness (ours median 64/70/64, game 86/97/86) while
  the knit matches. Not explained yet; ruled out: the pull's normals (they agree with the
  belly's), the zip morphs (`zip04_morph` opens the teeth, so 0 is right), the light `.w`
  weight on the gloss (no change when dropped), the textures' sRGB flags (all four alike).

## Probes
- `material_graph_dump` now prints the texture slots' GUIDs (for `tex_dump`) and (D701)
  the stored scalars (CosinePower, BumpLevel, AlphaTestLevel, alpha mode).
- `morph_probe SNAPSHOT GAME-DIRS MESH ANIM`: morph names, vertices moved, frame-0
  weight. Sackboy has 29 morphs (brows, eyelids, mouth, zip); `asymmetry_morph` is 1.0
  in frame 0 of clip `0001a697`, the rest 0.

- `eye_probe SNAPSHOT GAME-DIRS MESH ANIM EYE-MATERIAL` (D702): per primitive its material, costume region and bounds (bind and frame 0), each eye's bounds, the morphs that reach the eye vertices and the bones the eyes follow with their local, world and bind scale.

## D703: the eyes are narrowed to the game's width
- MEASURED: RPCS3 at 300% resolution scale (a `--config` copy of the config, nothing
  persistent changed), F12 frame of the default Pod view, against the viewer with the new
  `--window-scale 3` (3840x2160, scale factor 1) on the D700 Pod play command. Dark-core eye
  blobs (luma < 12 / < 20): game left eye 23-26 px wide, 31-32 px tall; ours 29 x 32 px at
  a 1.05x larger eye spacing (91 px against 87). So ours were about 1.1-1.15x too wide at the
  right height, and round where the game's are upright ovals.
- The viewer narrows the eye vertices (material `0000053c`) across the face (model X) about
  each eye's centre by `EYE_WIDTH` 0.88 (`narrow_eyes`, main.rs). The eyes follow only their
  bones and no morph reaches them (D702), so the bind positions carry it to every pose.
- The game's eyes also sit in a soft dark ring (the face texture's AO around the socket);
  ours have a crisp edge. Not changed.

## Open
- Eyes: where the game narrows them (D703 is a measured width, not the game's rule). The
  EBOOT pass on the eye bones was not written; the eye bones, the eye material and a draw
  call scale are the places to look.
- Zip pull brightness (D701 above).
- Program decoding: the local `cgb.py` now prints the dst scale (`_x2`, `_d2`, ...; SRC1 bits
  28-30). Knit and pull end alike: `out = light^2 x P + 2 x spec x P'` (MUL_x2), then tint and a
  per-channel sqrt (DIVSQ x, x), so the pull's 0.7x gap is not in the combine. Earlier note: `cgb.py` dropped the
  instruction dst scale (bits 28-30 of word 3), so the N.V DP3 carries a hidden x2 (a true
  reflection) and the N.y ADD is /2; the decoded spec combine never reads a light's `.w`,
  while `gloss_term` multiplies by `w`. Re-check the shading doc before changing either.
