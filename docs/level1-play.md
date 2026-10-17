# Rookie Test played with its scripts (lane level1)

Goal: the first Story level, Rookie Test (`main_01_rookie_test`, GUID 0001e786, Da Vinci's
Hideout), played in the viewer with its Things, its scripts and the code-driven Sackboy:

```
lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json
  --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"
  --level-guid 0001e786 --level-scripts --play-sackboy [--window-16-9 --preview-lighting]
```

## Part 1: Da Vinci's first line (D500, 2026-10-17)

Before: with both `--level-scripts` and `--play-sackboy` on, Sackboy spawns at
(-13335, 6763, 0) and lands at y 5975.4 (the game: 5974, RPCS3 memory read, d-run
2026-10-09), the scripts run (create 189 done / 23 yielded), but no message ever opens.

Cause: Da Vinci (Thing 1029) is an NPC with a microchip, switch type 36. The runtime gave
every type-36 owner a board port offset of 1, so his board (1048) mirrored his unwired
input port 1 and stayed off. FOUND (01.33, read from the disassembly: the board offset
function 0x3c784 by object kind, the kind test 0x3ed2c): the NPC part is tested before
the switch (kind 0x88; YellowHead 0x89) and the offset for both kinds is 0.
`board_port_offset` now returns 0 for an owner with the NPC or YELLOWHEAD part.

The chain in the level (all on board 1048): player sensor 1019 (radius 840, around the
spawn) -> 1029 port 0 -> board output 0 -> timer 1059 (59 ticks) -> counter 1060 ->
speech bubble 1064 (`SwitchPerformReaction__bf` on: `TriggerStartMessage`). The bubbles'
own trigger children (1058, 1067 and the other 19 Trigger_Speech_Bubble triggers) carry
matrices far off the level and radii such as -11: they are not how Story dialogue starts.
Thing 1047 (an empty sequencer chip with a PTrigger) is not in this chain.

After: standing at the spawn, the first bubble opens on tick 63 at Da Vinci (1029) with
its translated line. Test: `switches::tests::an_npc_chips_board_mirrors_from_port_0`.

Not yet: the game's timing of that line (an RPCS3 capture from the level start); the line
never closes here (the bubble's dialogue length and audio clock), so the second line
(1054, via 1064's output -> counter 1068 -> AND 1052 -> timer 1053, 299 ticks) never
follows; the wall left of the spawn at x -14348 stops Sackboy before Da Vinci.

## Part 2: why the first line stays open (2026-10-17, findings only)

Bubble 1064 (Text `DV_MAIN01_10`, Behaviour 0, DialogueLengthMs 4363, Sound
`gibberish/lbp2/davinci_jolly`, TriggerThing 1067) opens as message type 2
(`Messaging_NewTriggerMessageNoPrompt`, prevent-close false). Its only input is counter 1060
(target 1), which fills once from timer 1059 (59 ticks) and stays full; nothing else is wired
to 1060 or 1064.

FOUND (Trigger_Speech_Bubble script): the bubble's output (`SwitchGetNewActivation`) is on only
while `OutputEndSignalFrames` > 0, set to 2 by `TriggerStopMessage`. `TriggerStopMessage` runs
when the input activation drops to 0 (`SwitchPerformReaction`), from the scoreboard, or (for a
divergent bubble only; 1064 is not, it has output wires) after 60 frames off screen. The
dialogue clock (`UpdateAudio__`, `AudioTime` against DialogueLengthMs * 30 / 1000) only stops
the mouth (`StopAnimatingMouth__`); it neither closes the message nor fires the output.

FOUND (01.33 EBOOT, messaging manager): type 2 has no timeout (only types 0 and 4 do). Circle
(pad mask 0x20) maps to action 0x37 at 0x3e7898, which only sends net message 0x49 to party
peers; offline it does nothing (no `OnMagicMouthClosed`, no removal). Type 2 closes only by
event 3 for its Thing (`Messaging_NewGameEvent` 0x735434, the component destructor 0x6a03cc,
or a per-frame part update 0x6a3264 when its active byte at +0x24 goes 1 -> 0), which calls
`OnMagicMouthClosed` (no output signal either). The engine never calls `TriggerStopMessage__b`.

So with our counter and timer rules the second line (1054: 1064's output -> counter 1068 ->
AND 1052 with board output 0 -> timer 1053, 299 ticks) can never start. OPEN: what turns
1060 or 1064's input off in the game (a counter reset rule, chip deactivation, or the part
update at 0x6a3264); needs a game observation of the level start.

The wall at x -14348 is Thing 1028 (UID 246497): a wooden post (base_wood, collidable, z
-290..+90, x -14490..-14384, y 5905..8162) hung from dark matter 1027 through piston 1026.
It is genuinely solid in the front lane; Da Vinci (1029) stands behind it at x -15601. Our
collision reads it correctly, so it is not changed here (slopes/walls stay paused).
