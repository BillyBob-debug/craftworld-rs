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

## Part 3: Circle closes the line, the next one follows (D502, 2026-10-17)

OBSERVED (RPCS3, Rookie Test from the Story menu, all game patches off): the first line
opens in a letterboxed cutscene with a "Close" prompt (Circle) and stays open (watched for
over a minute). On Circle the cutscene ends and the second line, "Don't wait around here!",
opens about 10 s later with its own Close prompt. After it the level goes on to the right
(an arrow sign), so the post left of the spawn is not on the route.

This corrects Part 2: Circle is not a no-op offline. FOUND (01.33): action 0x37 at
0x3e7898 sends net message 0x49 (0x3e7dc8 -> 0x3e498c), whose handler (0xa04d4c ->
0x3e482c, game mode 4 or 5) finds the message's Thing and calls the script's
`OnMagicMouthRemoteStop__b(true)` (wrapper 0x6551e4). INFERRED: offline the message comes
back to the local machine (the observation needs it; not read statically).
`OnMagicMouthRemoteStop__b` clears `SpeechBubbleOpen` and calls `TriggerStopMessage__b(true)`,
which closes the message (event 3) and, for a non-divergent active bubble, sets
`OutputEndSignalFrames` to 2; `Update__` counts it down and `SwitchGetNewActivation__i`
answers 1.0 while it is above 0.

Before: nothing called the bubble's stop, and no code read a scripted Thing's output, so
the pulse to counter 1068 could not exist. After: C (or the pad's Circle) closes the first
open bubble that may be closed (`LevelScripts::player_close_message`), and before the
switch pass every scripted Thing whose class has `SwitchGetNewActivation__i` gives each
output's signal (the stored signal the pass keeps for types it does not compute). With
`--level-scripts-close-at 120` the first line closes on tick 120 and the second opens on
tick 419 (299 ticks of timer 1053 after the pulse), at Da Vinci. Test:
`switches::tests::a_scripted_pulse_is_on_only_when_full`.

Not yet: the second line's close and what follows it; the cutscene camera while line 1 is
open; the "Close" prompt on the no-prompt bubble type (the game draws it here).

## Part 4: line 2 closes, the walk right reaches line 3 (D503, 2026-10-17)

`--level-scripts-close-at` takes a comma list of ticks now (`120,600`), one press per tick
in order. Test: `level_play::tests::d503_the_close_option_is_a_list_of_ticks_pressed_in_order`.

OBSERVED (viewer, `--level-scripts --play-sackboy --level-scripts-close-at 120,600,1150
--play-sackboy-input "-:620,r:160,rj:20,r:330,f:15,-:5,r:400,rj:20,r:400"`): line 1 opens
on tick 63 and closes on 120, line 2 opens on 419 and closes on 600, at Da Vinci (Thing
1029). Nothing else opens while Sackboy stands at the spawn (watched to tick 1500): line 2's
bubble 1054 has no SWITCH part, so its end pulse feeds nothing, which fits the game going
on to the right after it. Walking right:
- x -8431 (tick 792): a book on the floor stops him; a jump clears it.
- tick 882 (about x -5700): line 3 opens at bubble 1485 (-262, 7377, 120), "You can use
  up and down on [lstick] to move in and out of the screen". The Close press on 1150
  closes it.
- about x -2400 he drops about 1650 units to y 3555, then stops at x -938 (y 3512). A
  move back (`b`) takes him to z -200 and he is still stopped there; a jump from z 0
  clears it and he lands at x -385 (y 3406), stopped again.
- with no input after tick 1965 he falls from x -385 (y 2072 on tick 1999): INFERRED, he
  was held on an edge while pushing right.

While he is pushed into a wall the code-driven Sackboy turns to the camera and plays
`stand` (vx 0), not a push.

Not yet: the route after x -385 (no game reference for this part; the RPCS3 frames of
Part 3 stop at line 2); whether line 3 needs the Close press in the game; the camera at
x -938 (a viewer frame at tick 1600 looks through a front wall).

## Part 5: the speech bubble in native Rust, no decrypted files (D504, 2026-10-18)

Before: every game script is only in the encrypted patch.edat, so without the decrypted
overlay Rookie Test loaded no scripts (`scripts loaded 0`) and no line ever opened.

After: `Trigger_Speech_Bubble` (g000047f4) is a port in the D417 native script registry
(`crates/lbp-runtime/src/native_bubble.rs`), run when its script is not loaded, or always
with `--native-scripts`. The switch pass now also asks a native port for
`SwitchGetNewActivation__i`, so the bubble's output pulse reaches the next line. The port
is written from the script's 01.33 listing: `OnCreate__` and its forever loop (`Update__` while `HasUpdateWorkToDo`, one yield a
tick), the switch reaction, Enter/Leave, Circle's remote stop, the output pulse, the mouth
and audio exports. Left out: the create-mode tweak menus and editor drawing.

Rookie Test's 221 scripted Things use 9 classes: PostItNote 165, Trigger_Speech_Bubble 21,
Launcher 19, TriggerMusic 9, TutorialMonitor 3, PostItNoteUnlockStickers 1,
PostItNoteUnlockSuicide 1, Score_Board 1, WorldMooCamera 1. Two script functions the bubble
calls are in other classes and are inlined: `Thing.GetDescendentWithPTrigger__Q5Thing` (the
Thing itself or its first direct child with a PTrigger) and `World.GetCurrentChallenge__`
(the world's `CurrentChallenge` member).

The oracle: a per-tick record of every native call (arguments and result, strings as
length and hash) and member write of the 21 bubbles, made on the Part 4 route with the VM
and with the port (a throwaway recorder on this branch before it moved onto the D417
registry). The VM's trace (5,951 events) and the port's agree call for call, apart from 21
writes of `HasUpdateWorkToDo` in `OnCreate__`: the native class starts that field at its
`.init__` value (true), while the VM starts a Thing's fields at their type defaults (it does
not run `.init__` for Thing scripts), so the VM skips the first loop turn's `Update__`. The
other difference is the part handles' numbers. Without the overlay (`d68` snapshot, disc and
update only, `scripts loaded 0`) the three lines open on ticks 63, 419 and 882 and close on
120, 600 and 1150, as with the decrypted scripts. Test:
`native_bubble::d504_the_native_bubble_opens_closes_and_pulses_without_the_script`.

Not yet: the other eight classes (the post-it notes, launchers, music and score board have
no native class, so they do nothing without the overlay); the route past x -385.

## Part 6: the launch pads in native Rust (D505, 2026-10-19)

Before: without the decrypted overlay Rookie Test's 19 launch pads ran no script, so their
lights kept the level's stored sprite light.

After: `Launcher` (g00012d39, base `Tweakable`) is a port in the D417 registry
(`crates/lbp-runtime/src/native_launcher.rs`). Its 01.33 listing is small: `OnCreate__`
sets the light Thing's sprite light, `FarDist` to `Distance * 2.5` and `Color` to
(0.7, 0.7, 1, 1), the class's only vector constant; `SetDistance__f` stores the distance
and sets `FarDist` again; `SetTeamFilter__i` stores 0 to 4 and ignores other values; the
getters return the fields and `SwitchPerformReaction__b` is empty. The launch itself is
the engine's. Left out: the create-mode tweak menu (`UpdateUI__`). A field the level does
not store reads as its `.init__` value (`Distance` 1, `TeamFilter` 0).

Test: `native_launcher::d505_the_native_launcher_sets_its_light` (synthetic Things, no
script loaded).

## Part 7: the music triggers in native Rust (D506, 2026-10-19)

Before: without the decrypted overlay Rookie Test's 9 music triggers ran no script, so a
player entering one started no interactive music.

After: `TriggerMusic` (g00004750, base `Tweakable`) is a port in the D417 registry
(`crates/lbp-runtime/src/native_music.rs`). From its 01.33 listing: `Play__fi(fade,
player)` calls `Audio.SetInteractiveMusic(FSB, SettingsFile, 1, StartPoint, player,
this)` and sets the six stereo pairs to `Pair_<n>_Volume` (fade -1 when `fade` is
negative, else 0); `Enter__Q5Thing` plays (-1, 0) when music is safe to trigger and input
0 is unwired; `SwitchPerformReaction__b` plays the same when input 0's binary activation
or the argument is on; `Mute__` sets every pair to 0. The GUID, start point and mixer
calls are ported too. Left out: the create-mode tweak menu and preview, the editor's
radius drawing and the dead sync-point branch of `SetMixerChannel__if`. A field the level
does not store reads as its `.init__` value (`StartPoint` 0, each pair 1).

Test: `native_music::d506_the_native_trigger_music_plays_and_mutes` (synthetic Things,
no script loaded).

## Part 8: the post-it notes in native Rust (D507, 2026-10-19)

Before: without the decrypted overlay Rookie Test's post-it notes ran no script, so
another script asking a note for its text, or a switch wired to an unlock note, got
nothing back.

After: `PostItNote` (g000174fc, base `Tweakable`) and its unlock subclasses
`PostItNoteUnlockStickers` (g0001e9cf), `PostItNoteUnlockSuicide` (g0001e9d0) and
`PostItNoteUnlockCostume` (g0001e9b7) are ports in the D417 registry
(`crates/lbp-runtime/src/native_postit.rs`). From the 01.33 listing: the note translates
its `Text` as a LAMS key unless a player typed it, keeps a typed text's filtered and
original forms (each cut to 256 units), upper-cases a new LAMS key, keeps a typed text
in `OldUserEnteredText` when Webternate replaces it, and asks
`PostItNote.IsVisibleInPlayMode__` whether it renders in play mode. An unlock note's
`SwitchPerformReaction__b(true)` sets its progression flag (4864, 2048 or 128) every time
it is on. Left out: the bubble drawing (`Render__`, `UpdateBubble__`, `RenderBubble__`;
nothing calls a script's `Render__` yet) and the create-mode text entry. INFERRED:
`SetLAMSKey__`'s inherited `SetNeedsUpdate__b(true)` sets `NeedUpdate` to 1.

Test: `native_postit::d507_the_native_post_it_note_keeps_its_text_and_unlocks`
(synthetic Things, no script loaded).

## Part 9: the tutorial monitors in native Rust (D508, 2026-10-19)

Before: without the decrypted overlay Rookie Test's 3 tutorial monitors ran no script, so
a switch turning one on asked for no tutorial video.

After: `TutorialMonitor` (g00006b3c, base `Thing`) is a port in the D417 registry
(`crates/lbp-runtime/src/native_tutorial.rs`). From its 01.33 listing: `OnCreate__` sets
`VideoHandle` to -1; `PlayVideo__` calls `Game.PlayTutorialMonitorVideo(VideoFile,
this)`; `SwitchPerformReaction__b` makes the same call when input 0's binary activation
is on and no video has started (the argument is not read); the video GUID, handle and
render size (640 x 360) are ported too. The game call is not bound yet, so it is only
counted and no video plays. A field the level does not store reads as its `.init__`
value (`VideoHandle` -1, `CurrentVolume` 1, `FadeOutTime` 60, `FadeInTime` 15).

Test: `native_tutorial::d508_the_native_tutorial_monitor_plays_its_video` (synthetic
Things, no script loaded).

## Part 10: the world script in native Rust (D509, 2026-10-19)

Before: without the decrypted overlay Rookie Test's world Thing ran no script, so no
level time limit counted down, collected fluff fed no score multiplier and a score or
time limit never went to the scoreboard.

After: `WorldMooCamera` (g00004d20, the world script of 220 story levels) is a port in
the D417 registry (`crates/lbp-runtime/src/native_world.rs`). It has only stubs of its
own, so the port is its base `World` (g00002d28). From the 01.33 listing: `OnCreate__`
resets the multiplier timers and counter, then runs `Update__` once a frame until
`OnDestroy__`. In play mode `Update__` calls `Game.GotoScoreboard(true)` once the score
reaches `ScoreLimit` or `Timer` reaches `TimeLimit` (30 frames a second, paused during a
cut-scene movie), with the countdown sound each second of the last 10 and the game-over
sound at 0. Each frame the multiplier timers count down and a timer reaching 0 gives the
bonuses (bonus bank x counter / 5 added to the score); a button press keeps the pie chart
up. `OnCollectFluff` feeds the co-op multiplier (counter + 1, 30 frames, cap 495) and
answers 0 during a scoreboard challenge. The getters, setters, `Kick`,
`SetCurrentChallenge` and `ShouldShowPieChart` (mode 0 and a challenge or a score in the
last 600 frames) are ported too. Left out: the `Bonus` popups and the `VCRDisplay`
(script objects with no port), `BaseWorld`'s `OnCreate`/`Update` (as in the Pod port), a
`Race_Start` challenge's own fluff handling, `NotifyDeath`, the direct-control prompt
list and reverb names (script arrays), and the create-mode UI and rendering.

Test: `native_world::d509_the_native_world_counts_down_and_keeps_the_multiplier`
(synthetic Things, no script loaded).

Not yet: Score_Board.
