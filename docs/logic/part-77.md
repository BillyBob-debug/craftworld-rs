# Logic, Part 77

See docs/logic.md for Parts 1-49.

## Part 77: a one-shot piston or motor runs its pattern once (D1176)

FOUND (EBOOT 01.33), 0xc9790, the length rule for types 3, 4 and 9 (Part 76):
- f11 is the retimed clock: (frame − `*(+0x190)+0x34`) × `+0x3c` + `+0x34` + `+0x38`.
- With r5 set, or no input (`*(*(joint+8)+0x60)` 0), it runs free (0xc9834, Part 76).
  INFERRED: r5 is 0 in the usual calls.
- Wired in mode 1 or 4 (0xc993c): base + f11 × range / (time × 0.5) when time is 1
  (TOC 0xd95134) or more, else the base, as D1104 has it. It has no motor branch, so it is
  not centred.
- Wired in mode 2 (0xc97e0), with no phase:
  - pattern 0 (0xc998c): the wave at fsel(f11 − (time + pause), time + pause, f11), that
    is min(f11, time + pause), then the eased (or, for a motor, centred) rule. The wave is
    back at 0 at time + pause, so it goes out and back once and stays at the base;
  - pattern 1 (0xc99a4): f11 × speed `+0xd4` while |f11 × speed| is under 2π
    (TOC 0xd95130 = 0x40c90fdb), else 0, the base (D1172 has this for motors);
  - pattern 2 (0xc9804): range × (1 − f11 / time) while f11 is under time and time is 1
    or more, else 0;
  - else 0. The result is base `+0xf0` + that (0xc9890).
- Free-running pattern 2 (0xc98d0) takes fmod (0x88fe2c) of frame + phase by time +
  pause, as our D1102 rule has it.

What we do: `SceneJoint::one_shot_length` (crates/lbp-sim/src/scene.rs) is the mode-2
rule; `length` uses it for a wired piston in mode 2 (it looped before), and
`motor_angle` takes `length` for a motor wired in mode 1, 2 or 4. The D1105 clamp to
[base, base + range] still applies.

Test: `d1176_a_one_shot_piston_runs_its_pattern_once`.
