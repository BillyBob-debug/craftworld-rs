# D403: The Popit in the viewer's Pod (Popit part 4)

Date 2026-10-17. Before: the viewer's Pod (`--play-sackboy --level-scripts --pod-camera N
--pod-shell HEX`) had no Popit (D281), because Square there was always the pod computer's
engage button, so the Pod card from D402 was only seen through `ui_trace`. Now the Pod loads
the Popit too, and Square opens it unless Sackboy stands where Square is the pod computer's.

## Rule
- Square is the pod computer's engage button when Sackboy is in the pod computer's pad zone
  (`LevelWorld::in_pad_zone`, as the tick before left the triggers' lists) or the pod
  computer is in use; elsewhere it opens and closes the Popit. While the Popit is open,
  Square closes it.
- INFERRED: the game's own gate between the two was not read. The pod's script asks for
  the engage action with its prompt only in the pad zone (FOUND, part 24), and RPCS3 shows
  the Popit opening with Square away from the computer (MEASURED below). A Sackboy in the
  pad zone before the planets intro has set the progression flags (mask 2) gets no Popit
  here; what the game does then was not checked.

## Built
- `sackboy_play`: the Popit assets load in the Pod as everywhere else;
  `PodSackboy::at_computer` carries the rule, written by `level_play::tick` before each
  script tick (it logs `Square is the pod computer's engage button` / `the Popit's`).
- `popit_play::popit_input`: Square does not toggle a closed Popit while `at_computer`.
  The key still reaches the pod's script as before (`main.rs` is unchanged); out of the pad
  zone the script ignores it.

## Result
- Viewer, Pod with `--popit-open --popit-mode play`: the card is the script's
  `[[6, 10], [4, 0], [7]]` (log line `popit card: script Poppet.DoMainMenu`).
- MEASURED against an RPCS3 frame of the Popit in our own Pod (2026-10-11, Sackboy left of
  the computer): the same five buttons in the same places (Customize Character and Pod
  Decoration, then Personal and Stickers, then Text Chat), focus on Customize Character.
- At the start of the Pod session Sackboy stands in the pad zone (tick 26), so Square
  engages the computer there, as before.

## Open
- The scripted Pod's early frames look from inside the shell (camera before the scripts
  place it); the card is drawn over that view. In the game frame the card sits beside
  Sackboy at the left of the shell.
- Page switching inside a collection; highlight sizing 264 / style 4 (D400, D402).
