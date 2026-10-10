# P3g: dynamic sun shadows

Question: level meshes draw with fragment variant 2 (dynamic shadow + AO volume + sprite
lights, no decals) and the Moon capture shows a real 768x768 sun-shadow depth pass
(`lbp2/oracles/results/2026-10-09-shading-moon.md`, `-shading-fpdec.md`). The viewer lit the
sun term without occlusion.

Change (only with `--lbp-lighting`; `--no-lbp-shadows` turns it off for A/B shots):
- one Bevy `DirectionalLight` (illuminance 1, so StandardMaterial objects stay black) with
  `shadow_maps_enabled`, three cascades (far bounds 3,500 and 40,000 level units x the display
  scale, last = max distance), pointing along `-toward_light`; it is respawned when the display
  scale changes and re-aimed when the planets turn the light.
- the viewer shader multiplies the sun term of `preview_light`, and the sun share of the gloss
  weight and the specular lobe, by Bevy's `fetch_directional_shadow` (light 0). Ambient and rim
  are unshadowed.
- the Pod gets no shadows at all, shell and planets layers (it draws with variant 8, CONFIRMED;
  the PC saw the room walls darken the planets 15-50% under the outside sun, so they are
  excluded whenever a shell is present).
- the sun query is disjoint from the light-root queries (`SunFilter`); a unit test initialises
  `apply_lighting` so a Bevy B0001 conflict fails CI instead of panicking at start-up.

Evidence labels: variant 2 for level meshes INFERRED (EBOOT variant rule not looked up);
768 map size CONFIRMED from the capture, 1024 here INFERRED-substitute (Bevy wants a power of
two); three cascades match "3 shadow maps" in the engine target names (INFERRED, names from
community notes); cascade cut and biases are preview choices; the sun being a point light at
SunPosition x scale is approximated by the existing single direction from the level centre.
Not modelled: the game's VSM-like filtering, the AO volumes, casters list (castShadows),
shadowed ambient.

Not run in the cloud: any render (no GPU). The shader still validates headlessly (naga test).
PC verification wanted: fmt/test/clippy on 1.99, then Bravery Test and tutorial_23 with and
without `--no-lbp-shadows` at gameplay distance: look for acne (speckle on lit faces), peter-
panning (floating shadows), shadow-map swimming while panning, and missing shadows at the
cascade seams; check the Pod is unchanged.
