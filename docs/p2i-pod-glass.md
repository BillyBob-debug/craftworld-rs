# P2i: Pod side panels as glass (`--glass`)

Question: the Pod side panels (pod.mol 00002f6c, primitive 2) draw solid dark grey; RPCS3
shows swirly art there. P2e/P2g runtime textures in slot 0 (000027fb, 00004277) drew black
or flat lavender, so neither is the binding.

## Findings (PC read-only research, 2026-10-08 America/Chicago; notes in the project's
pod-assets-0133 research file, "PlanetsWorld base trace (D178)")

- Primitive 2 uses window_glassy 0001d14c. Slot 0 is tissue_paper_bump 00002649 and feeds
  the bump port only. Diffuse constant 0.118, specular 0.6, alpha_mode 4.
  CONFIRMED_FROM_LOCAL_DATA (material graph).
- Runtime slot 5 is the theme transition mask set by SetPodTransition (default 00020493);
  window_glassy does not wire it. CONFIRMED_FROM_LOCAL_DATA (script + graph).
- 000027fb and 00004277 are screen props, not panel art. CONFIRMED_FROM_LOCAL_DATA.
- So the panels are dark, bumped, specular glass, and the swirls are most likely the room
  behind showing through or reflecting. STRONG_INFERENCE; alpha_mode 4's meaning and the
  glass opacity are UNKNOWN.

## Change

`--glass MESH:SLOT:ALPHA` (repeatable, at most 16, same MESH:SLOT once): with a level (and
`--pod-shell`), buckets of MESH whose material lists SLOT as unrendered draw alpha blended
at ALPHA, so the planets room behind the shell shows through. Untextured surfaces only
change colour alpha; bump and specular are not drawn (the viewer is unlit). A count line
per scene ("Shell: glass (inferred) 00002f6c:0:0.35 N buckets") goes on the info panel.
The selection is the same as `--runtime-texture` (mesh GUID + unrendered slot), so it
needs no material GUID.

Test: `p2i_glass_rule_parses_and_refuses_bad_values` (lbp-bevy-viewer).

## PC check

`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f --play-animation 0001a697 --pod-hold-animation 000042b2 --animation-frame 0 --glass 00002f6c:0:0.35 --qa-hide-ui --qa-capture local-analysis/NEW.png`

Compare the side panels with the RPCS3 closed frame at 0.35, then try 0.2 and 0.5. Report
the bucket count line and which alpha is closest. If the room behind the panels isn't the
graffiti wall in our layering, the swirls need the reflection (not drawn).

Not tested here: any capture (no game data in the cloud session).
