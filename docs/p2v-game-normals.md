# P2v: the game's decoded normals for model meshes

2026-10-09 (America/Chicago). Plan: `lbp2/research/shading-lighting.md` item 2.

Before: every model mesh was shaded with normals we generated (face-averaged from our
triangles). The decoded per-vertex normals (`lbp_mesh::decode_skin`, docs/mesh-skin.md) were
read for joints and weights but never used for light.

After: `lbp_preview::load_mesh_groups_with_options` keeps the decoded normal stream
(`GroupedMesh::normals`, one per position, mesh space, only when its length matches). The
viewer replaces the generated normal of every referenced vertex with it. A stored normal
that is non-finite or shorter than 0.5 (for example a clamped packed one) keeps the generated
normal. When every referenced vertex took a game normal the draw counts as trusted and is lit
smoothly; otherwise the earlier rule (flat face lighting for untrusted draws, P2s) still applies.

Per mesh the viewer detail lines now include
`Normals: game (decoded) on U/T referenced vertices, mean |dot| with generated X`.
X near 1 means the stored normals agree with our geometry; a low value on a mesh means a
different frame or winding there and needs a look before trusting the lighting.

Scope and limits (INFERRED, to be settled by the PC check):
- Mesh space is assumed to be the position frame (same vertex record). Shading flips normals
  into the viewer-facing hemisphere of the face, so a sign error would not show; a frame
  rotation would.
- Per-bone skinned copies (`--per-bone-placement`) carry no normals (positions move, normals
  would need the weighted bone rotation): they keep generated normals. A follow-up.
- Generated shapes and bevels, backdrops and decals are unchanged (no stored normals).
- Tangents (bump mapping) still come from UV derivatives; the stored tangents are unused.
- The `smooth_normals` stream is not used; which stream the game's light reads is unconfirmed.
