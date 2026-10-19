//! D271 (docs/level-scripts.md, part 22): the video and voice-over requests of the scripts
//! ([`MediaState`]) and the profile's word of LBP2 game progression flags.
//!
//! Nothing here plays a video or a sound. A request is kept as the script made it, so
//! that a later video or audio system can take it from here; the script is told what
//! the game tells it when nothing started. D397: a voice-over line whose length the
//! game's voice-over bank gives "plays" silently for that long, so the scripts that wait
//! on it wait as long as the game's. D1003: so does a video whose length its Bink file
//! gives ([`MediaState::video_lengths`]).
use crate::LevelWorld;
use lbp_script_vm::Value;
use std::collections::{BTreeMap, BTreeSet};

/// One `Video.StartVideo__Q5Thinggb` call. No video started for it: the script got
/// [`lbp_script_vm::natives::NO_VIDEO`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoRequest {
    /// The Thing argument (the pod computer's scripts pass null).
    pub thing: Option<u32>,
    /// The GUID of the video.
    pub guid: u32,
    /// The bool argument as passed (what the game does with it was not read).
    pub flag: bool,
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// One `Audio.RequestVoiceOver__Q7StringAbb` call. Unless its line started (D397,
/// [`MediaState::voice_over_lengths`]) the script got false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceOverRequest {
    /// The name of the line; `None` for a null string.
    pub name: Option<String>,
    /// The two bool arguments as passed. INFERRED: "interrupt the line that plays" and
    /// "play even when the profile has heard it".
    pub flags: [bool; 2],
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// The video and voice-over requests of a level world ([`LevelWorld::media`]).
#[derive(Debug, Clone, Default)]
pub struct MediaState {
    /// `Video.StartVideo` calls, oldest first, at most [`MAX_MEDIA_REQUESTS`]; the calls
    /// past that are counted in `videos_dropped`.
    pub videos: Vec<VideoRequest>,
    pub videos_dropped: u64,
    /// `Video.StopVideo__i` calls.
    pub video_stops: u64,
    /// `Audio.RequestVoiceOver` calls, oldest first, at most [`MAX_MEDIA_REQUESTS`]; the
    /// calls past that are counted in `voice_overs_dropped`.
    pub voice_overs: Vec<VoiceOverRequest>,
    pub voice_overs_dropped: u64,
    /// `Audio.StopVoiceOver__` calls.
    pub voice_over_stops: u64,
    /// `Audio.SetMixOverlay__Q7StringAf` and `RemoveMixOverlay__Q7StringAf` calls by the
    /// name of the mix (a null string counts under the empty name).
    pub mix_overlays_set: BTreeMap<String, u64>,
    pub mix_overlays_removed: BTreeMap<String, u64>,
    /// D397: each voice-over line's length in ticks at 30 Hz, by its name without
    /// `.wav` ([`voice_over_lengths`]); empty when the bank was not read, and then no line
    /// starts.
    pub voice_over_lengths: BTreeMap<String, u32>,
    /// D397: the line that plays and its ticks left.
    pub voice_over: Option<(String, u32)>,
    /// D397: the lines started so far. FOUND (0x2de6b8): a line the profile has heard
    /// does not start again unless the request's second flag is set. INFERRED: the
    /// session stands for the profile (a fresh profile has heard none).
    pub voice_overs_heard: BTreeSet<String>,
    /// D397: whether the last `RequestVoiceOver` started its line.
    pub last_started: bool,
    /// D1003: each video's length in ticks at 30 Hz by its GUID ([`bink_ticks`]); empty
    /// when no video file was read, and then no video starts.
    pub video_lengths: BTreeMap<u32, u32>,
    /// D1003: the video that plays: its handle, GUID and ticks left.
    pub video: Option<(i32, u32, u32)>,
    /// D1003: the handle the next started video gets.
    pub next_video_handle: i32,
    /// D1003: the last `StartVideo`'s answer (the video's handle, or -1).
    pub last_video_handle: i32,
    /// D1003: the last `UpdateVideo`'s answer.
    pub last_video_update: bool,
    /// D1004: the interactive music `Audio.SetInteractiveMusic` set; `None` before it
    /// and after `StopInteractiveMusic`. Nothing plays yet.
    pub music: Option<InteractiveMusic>,
    /// D1004: `Audio.SetInteractiveMusic` calls.
    pub music_starts: u64,
}

/// D1004: the stereo pairs `SetInteractiveMusicStereoPairVolume` sets (the Pod's script
/// sets pairs 0 to 5).
pub const MUSIC_PAIRS: usize = 6;

/// D1004: the arguments of the last `Audio.SetInteractiveMusic__ggffiQ5Thing` and the
/// stereo pairs' volumes since.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InteractiveMusic {
    /// The music bank's GUID (the Pod's is 42103, `I_POD.FSB` on the disc).
    pub guid: u32,
    /// The settings GUID (the Pod's is 42108).
    pub settings: u32,
    pub volume: f32,
    /// The delay before it starts, in seconds.
    pub delay: f32,
    /// Each pair's volume, 0 until set. INFERRED: a call for another bank is ignored.
    pub pair_volumes: [f32; MUSIC_PAIRS],
}

/// Most requests [`MediaState::videos`] and [`MediaState::voice_overs`] keep each.
pub const MAX_MEDIA_REQUESTS: usize = 256;

impl MediaState {
    /// All `Video.StartVideo` calls, kept or not.
    pub fn video_requests(&self) -> u64 {
        self.videos.len() as u64 + self.videos_dropped
    }

    /// All `Audio.RequestVoiceOver` calls, kept or not.
    pub fn voice_over_requests(&self) -> u64 {
        self.voice_overs.len() as u64 + self.voice_overs_dropped
    }

    /// D397: `Audio.IsVoiceOverPlaying__`, whether a started line has ticks left.
    pub fn voice_over_playing(&self) -> bool {
        self.voice_over.as_ref().is_some_and(|(_, left)| *left > 0)
    }

    /// D1006 (FOUND 0x6793ec, `Video.IsVideoPlaying__`): the video manager's one slot
    /// is in use (bit 0 of the word at +0x400), from `StartVideo` until the video is
    /// finished or stopped, whatever its handle. INFERRED: the finish (0x67a25c ->
    /// 0x679978) frees the slot, as `UpdateVideo` does here once the video has ended.
    pub fn video_in_use(&self) -> bool {
        self.video.is_some()
    }

    /// D1003: whether the video with this handle has ticks left.
    pub fn video_playing(&self, handle: i32) -> bool {
        self.video
            .is_some_and(|(h, _, left)| h == handle && handle >= 0 && left > 0)
    }

    /// D1003 (FOUND 0x67c620): the video starts when its file was read, and gets a
    /// handle; else the answer is -1. INFERRED: one video plays at a time, as the pod
    /// computer keeps one handle, so a new one replaces it.
    fn start_video(&mut self, guid: u32) -> i32 {
        let Some(&ticks) = self.video_lengths.get(&guid) else {
            return -1;
        };
        let handle = self.next_video_handle;
        self.next_video_handle = self.next_video_handle.wrapping_add(1).max(0);
        self.video = Some((handle, guid, ticks));
        handle
    }

    /// D1003 (FOUND 0x67c22c): no video with that handle, false; else true while it
    /// plays, and once it has ended it is finished (0x67a25c) and the answer is false.
    fn update_video(&mut self, handle: i32) -> bool {
        let playing = self.video_playing(handle);
        if !playing && self.video.is_some_and(|(h, ..)| h == handle) {
            self.video = None;
        }
        playing
    }

    /// D397: one tick of the line that plays; it ends when none is left. D1003: and of
    /// the video that plays (it stays until `UpdateVideo` finishes it).
    pub fn tick(&mut self) {
        if let Some((_, _, left)) = &mut self.video {
            *left = left.saturating_sub(1);
        }
        if let Some((_, left)) = &mut self.voice_over {
            *left = left.saturating_sub(1);
            if *left == 0 {
                self.voice_over = None;
            }
        }
    }

    /// D397 (FOUND 0x2de6b8, the core of `RequestVoiceOver`): the line starts unless
    /// voice-over is off (here: no bank was read, or the bank has no such line), the
    /// profile has heard it and `replay` (the second flag) is clear, or a line plays and
    /// `interrupt` (the first flag) is clear.
    fn request_voice_over(&mut self, name: Option<&str>, interrupt: bool, replay: bool) -> bool {
        let Some((name, &ticks)) = name.and_then(|n| self.voice_over_lengths.get_key_value(n))
        else {
            return false;
        };
        if (!replay && self.voice_overs_heard.contains(name))
            || (self.voice_over_playing() && !interrupt)
        {
            return false;
        }
        self.voice_overs_heard.insert(name.clone());
        self.voice_over = Some((name.clone(), ticks));
        true
    }
}

/// D1003 (FOUND, the USA disc): the videos whose length is read, by GUID and path under
/// `USRDIR`; loose and not encrypted on the disc. 139230 is the Pod's introduction.
pub const VIDEO_FILES: [(u32, &str); 1] = [(139230, "gamedata/videos/lbp2/lbp2_podintro.bik")];
/// D1003: the bytes [`bink_ticks`] reads.
pub const BINK_HEADER: usize = 0x24;

/// D1003: a Bink video's length in ticks at 30 Hz (rounded up) from its header (`BIK`
/// and a version letter, then little-endian u32s: the file size less 8, the frame count,
/// the largest frame, the frame count again, the width, the height, and the frame rate's
/// numerator and denominator at 0x1c and 0x20). FOUND, the Pod intro's on the USA disc:
/// `BIKi`, 378 frames, 640x360, 2997/100 fps, so 379 ticks.
pub fn bink_ticks(head: &[u8]) -> Result<u32, String> {
    if head.len() < BINK_HEADER || &head[..3] != b"BIK" {
        return Err("Bink: not a Bink header".into());
    }
    let u = |o: usize| u32::from_le_bytes(head[o..o + 4].try_into().unwrap()) as u64;
    let (frames, num, den) = (u(8), u(0x1c), u(0x20));
    if num == 0 || den == 0 {
        return Err("Bink: no frame rate".into());
    }
    Ok((frames * 30 * den).div_ceil(num) as u32)
}

/// D397: the game's voice-over bank, `gamedata/audio/voiceover.fsb` on the disc (an FMOD
/// FSB4 bank, little-endian, not encrypted), the English one at that path.
pub const VOICE_OVER_BANK: &str = "gamedata/audio/voiceover.fsb";
/// D397: the bytes [`voice_over_lengths`] needs at most: the 48-byte header and the
/// sample headers (3,600 bytes for the 45 lines of 01.00).
pub const VOICE_OVER_HEADER_MAX: usize = 1 << 16;

/// D397 (FOUND, the 01.00 disc's bank: `FSB4`, 45 samples, version 0x40000, every one at
/// 44100 Hz): each line's length in ticks at 30 Hz (rounded up) by its name without
/// `.wav`, from the bank's sample headers (u16 size, 30-byte name, u32 length in samples,
/// then the compressed size, the loop start and end, the mode and the default frequency).
pub fn voice_over_lengths(bank: &[u8]) -> Result<BTreeMap<String, u32>, String> {
    let u32_at = |o: usize| -> Result<u32, String> {
        let b = bank.get(o..o + 4).ok_or("FSB4: truncated header")?;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    };
    if bank.get(..4) != Some(b"FSB4") {
        return Err("FSB4: not an FSB4 bank".into());
    }
    let (count, headers) = (u32_at(4)? as usize, u32_at(8)? as usize);
    if 48 + headers > bank.len() {
        return Err("FSB4: truncated sample headers".into());
    }
    let mut out = BTreeMap::new();
    let mut o = 48;
    for _ in 0..count {
        let size = u16::from_le_bytes([bank[o], bank[o + 1]]) as usize;
        if size < 56 || o + size > 48 + headers {
            return Err("FSB4: bad sample header".into());
        }
        let raw = &bank[o + 2..o + 32];
        let name = String::from_utf8_lossy(&raw[..raw.iter().position(|&b| b == 0).unwrap_or(30)]);
        let (samples, rate) = (u32_at(o + 32)? as u64, u32_at(o + 52)? as u64);
        if rate > 0 {
            let name = name.strip_suffix(".wav").unwrap_or(&name).to_string();
            out.insert(name, (samples * 30).div_ceil(rate) as u32);
        }
        o += size;
    }
    Ok(out)
}

fn name_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Str(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn float_of(v: Option<&Value>) -> f32 {
    match v {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f32,
        _ => 0.,
    }
}

fn guid_of(v: Option<&Value>) -> u32 {
    match v {
        Some(Value::Int(i)) => *i as u32,
        Some(Value::Raw(u) | Value::Ref(u)) => *u,
        _ => 0,
    }
}

impl LevelWorld {
    /// D1004: the interactive-music calls of
    /// [`lbp_script_vm::natives::World::audio_command`], kept in [`MediaState::music`].
    /// INFERRED: `GetCurrMusicGuid` answers the bank this set (the Pod's script calls
    /// `SetInteractiveMusic` again only while the answer is not its bank).
    pub(crate) fn music_call(&mut self, name: &str, args: &[Value]) {
        let media = &mut self.media;
        match name {
            "SetInteractiveMusic" => {
                media.music_starts += 1;
                media.music = Some(InteractiveMusic {
                    guid: guid_of(args.first()),
                    settings: guid_of(args.get(1)),
                    volume: float_of(args.get(2)),
                    delay: float_of(args.get(3)),
                    pair_volumes: [0.; MUSIC_PAIRS],
                });
            }
            "SetInteractiveMusicStereoPairVolume" => {
                let pair = match args.get(1) {
                    Some(Value::Int(i)) => *i as usize,
                    _ => return,
                };
                if let Some(m) = &mut media.music {
                    if m.guid == guid_of(args.first()) && pair < MUSIC_PAIRS {
                        m.pair_volumes[pair] = float_of(args.get(2));
                    }
                }
            }
            "StopInteractiveMusic" => media.music = None,
            _ => {}
        }
    }

    /// The video and audio calls of [`lbp_script_vm::natives::World::engine_call`]: kept in
    /// [`LevelWorld::media`]. Other names are not for this list.
    pub(crate) fn media_call(&mut self, name: &str, thing: Option<u32>, args: &[Value]) {
        let frame = self.pworld.frame;
        let flag = |i: usize| matches!(args.get(i), Some(Value::Bool(true)));
        let media = &mut self.media;
        match name {
            "StartVideo" => {
                let guid = guid_of(args.get(1));
                media.last_video_handle = media.start_video(guid);
                if media.videos.len() < MAX_MEDIA_REQUESTS {
                    let flag = flag(2);
                    media.videos.push(VideoRequest {
                        thing,
                        guid,
                        flag,
                        frame,
                    });
                } else {
                    media.videos_dropped += 1;
                }
            }
            "UpdateVideo" => {
                let handle = match args.first() {
                    Some(Value::Int(h)) => *h,
                    _ => -1,
                };
                media.last_video_update = media.update_video(handle);
            }
            "StopVideo" => {
                media.video_stops += 1;
                if let (Some(Value::Int(h)), Some((v, ..))) = (args.first(), media.video) {
                    if *h == v {
                        media.video = None;
                    }
                }
            }
            "RequestVoiceOver" => {
                let name = name_of(args.first());
                media.last_started = media.request_voice_over(name.as_deref(), flag(1), flag(2));
                if media.voice_overs.len() < MAX_MEDIA_REQUESTS {
                    let flags = [flag(1), flag(2)];
                    media
                        .voice_overs
                        .push(VoiceOverRequest { name, flags, frame });
                } else {
                    media.voice_overs_dropped += 1;
                }
            }
            "StopVoiceOver" => {
                media.voice_over_stops += 1;
                media.voice_over = None;
            }
            "SetMixOverlay" | "RemoveMixOverlay" => {
                let by_name = if name == "SetMixOverlay" {
                    &mut media.mix_overlays_set
                } else {
                    &mut media.mix_overlays_removed
                };
                *by_name
                    .entry(name_of(args.first()).unwrap_or_default())
                    .or_default() += 1;
            }
            _ => {}
        }
    }
}

/// The value of `--flags` of the `level_scripts` example and of the viewer's
/// `--level-scripts-flags`: bit masks for [`LevelWorld::progression_flags`], separated
/// by commas and ORed together. A mask is decimal, or hexadecimal after `0x`.
pub fn parse_progression_flags(text: &str) -> Result<u32, String> {
    let mut word = 0;
    for mask in text.split(',').map(str::trim) {
        let parsed = match mask.strip_prefix("0x").or_else(|| mask.strip_prefix("0X")) {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => mask.parse::<u32>(),
        };
        word |= parsed.map_err(|e| format!("progression flag mask {mask:?}: {e}"))?;
    }
    Ok(word)
}

/// D341 (part 88): the viewer's `--profile-records` file, the local profile's goodies
/// counts by `SlotID` for [`LevelWorld::profile_goodies`]. One record per line, `TYPE ID
/// COLLECTED TOTAL` (decimal, separated by spaces); blank lines and text from `#` on are
/// skipped. `scripts/rpcs3_profile_records.py` writes it from a running game: the save
/// itself is encrypted, so the records are read where the game has loaded them.
///
/// D342 (part 89): a line `colours C0 C1 C2 C3` (hex 0xAARRGGBB words) gives the
/// profile's four colours ([`ProfileRecords::colours`]).
///
/// D346 (part 93): a line `overlays TYPE ID MASK` (MASK hex) gives the overlay slots a
/// slot's badge shows ([`ProfileRecords::overlays`]).
pub fn parse_profile_records(text: &str) -> Result<ProfileRecords, String> {
    let mut profile = ProfileRecords::default();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let bad = |e: &dyn std::fmt::Display| format!("profile record line {}: {e}", n + 1);
        let words: Vec<&str> = line.split_whitespace().collect();
        if let ["colours", rest @ ..] = &words[..] {
            let hex = |w: &&str| {
                let digits = w.strip_prefix("0x").or_else(|| w.strip_prefix("0X"));
                u32::from_str_radix(digits.unwrap_or(w), 16).map_err(|e| bad(&e))
            };
            let colours: Vec<u32> = rest.iter().map(hex).collect::<Result<_, _>>()?;
            let colours = colours
                .try_into()
                .map_err(|_| bad(&"expected colours C0 C1 C2 C3"))?;
            profile.colours = Some(colours);
            continue;
        }
        if let ["overlays", ty, id, mask] = words[..] {
            let int = |w: &str| w.parse::<i32>().map_err(|e| bad(&e));
            let digits = mask.strip_prefix("0x").unwrap_or(mask);
            let mask = u16::from_str_radix(digits, 16).map_err(|e| bad(&e))?;
            profile.overlays.insert((int(ty)?, int(id)?), mask);
            continue;
        }
        let [ty, id, collected, total] = words[..] else {
            return Err(bad(&"expected TYPE ID COLLECTED TOTAL"));
        };
        let int = |w: &str| w.parse::<i32>().map_err(|e| bad(&e));
        let count = |w: &str| w.parse::<u32>().map_err(|e| bad(&e));
        let counts = [count(collected)?, count(total)?];
        if counts[0] > counts[1] {
            return Err(bad(&"more collected than the total"));
        }
        profile.goodies.insert((int(ty)?, int(id)?), counts);
    }
    Ok(profile)
}

/// What [`parse_profile_records`] reads: the goodies counts by `SlotID` (`[collected,
/// total]`) and the profile's colours, when the file gives them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileRecords {
    pub goodies: BTreeMap<(i32, i32), [u32; 2]>,
    /// D342 FOUND (01.33, the Pod's per-frame update 0x4381d8, block 0x438278..0x438318):
    /// the Story view's Gooey takes its colours (0x48e9f8: fill, then the three others)
    /// from the local profile's colour record `*(profile+0x514)` (0x18ae48), words +0x4c to
    /// +0x58 (0x15894c), each frame before the bubble is built. MEASURED (the owner's
    /// profile in RPCS3): that record is the same object the player record points at
    /// (+0x16c), so its first colour is the player's.
    pub colours: Option<[u32; 4]>,
    /// D346: by `SlotID`, bit `n` set for each badge overlay slot `n` (1..12: padlock,
    /// finish, played, collect, ace, heart, playlist, rosette, versus, cutscene, lbp1,
    /// move) the game shows, read from its badge results in a running game.
    pub overlays: BTreeMap<(i32, i32), u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank of one line, laid out as the disc's (a 48-byte header, one 80-byte sample
    /// header).
    fn bank(name: &str, samples: u32) -> Vec<u8> {
        let mut b = b"FSB4".to_vec();
        for w in [1u32, 80, 0, 0x40000, 0x20, 0, 0, 0, 0, 0, 0] {
            b.extend(w.to_le_bytes());
        }
        let mut h = 80u16.to_le_bytes().to_vec();
        let mut n = name.as_bytes().to_vec();
        n.resize(30, 0);
        h.extend(n);
        for w in [samples, 0, 0, samples - 1, 0x80220, 44100] {
            h.extend(w.to_le_bytes());
        }
        h.resize(80, 0);
        b.extend(h);
        b
    }

    #[test]
    fn d397_a_line_plays_for_its_length_once() {
        let lengths = voice_over_lengths(&bank("pod_1st_time.wav", 1_411_200)).unwrap();
        assert_eq!(lengths["pod_1st_time"], 960);
        let mut m = MediaState {
            voice_over_lengths: lengths,
            ..Default::default()
        };
        assert!(!m.request_voice_over(Some("pod_select_story"), true, true));
        assert!(m.request_voice_over(Some("pod_1st_time"), false, false));
        assert!(m.voice_over_playing());
        // A line plays and the request may not interrupt it.
        assert!(!m.request_voice_over(Some("pod_1st_time"), false, true));
        for _ in 0..959 {
            m.tick();
        }
        assert!(m.voice_over_playing());
        m.tick();
        assert!(!m.voice_over_playing());
        // Heard, and the second flag is clear.
        assert!(!m.request_voice_over(Some("pod_1st_time"), false, false));
        assert!(m.request_voice_over(Some("pod_1st_time"), false, true));
        assert!(voice_over_lengths(b"FSB5").is_err());
    }

    #[test]
    fn d341_profile_records_are_read_by_slot_id() {
        let text = "# type id collected total\n0 20354 36 40\n\n4 7 0 0  # a group\n";
        let records = parse_profile_records(text).unwrap();
        assert_eq!(records.goodies.get(&(0, 20354)), Some(&[36, 40]));
        assert_eq!(records.goodies.get(&(4, 7)), Some(&[0, 0]));
        assert_eq!(records.goodies.len(), 2);
        assert_eq!(records.colours, None);
        assert!(parse_profile_records("0 1 2").is_err());
        assert!(records.overlays.is_empty());
        assert!(parse_profile_records("0 1 5 4").is_err());
        assert!(parse_profile_records("0 x 1 1").is_err());
    }

    /// D342: the colours line gives the profile's four colours.
    #[test]
    fn d342_profile_colours_line() {
        let text = "colours 0xffceb424 0xffc331a0 0xff3bcfc1 ffe60f64
0 1 0 0
";
        let records = parse_profile_records(text).unwrap();
        assert_eq!(
            records.colours,
            Some([0xffce_b424, 0xffc3_31a0, 0xff3b_cfc1, 0xffe6_0f64])
        );
        assert_eq!(records.goodies.len(), 1);
        assert!(parse_profile_records("colours 0xff 0xff 0xff").is_err());
        assert!(parse_profile_records("colours 0xff 0xff 0xff zz").is_err());
    }

    #[test]
    fn d346_profile_overlays_line() {
        let records = parse_profile_records(
            "overlays 4 7 0x34
0 1 0 0
",
        )
        .unwrap();
        assert_eq!(records.overlays.get(&(4, 7)), Some(&0x34));
        assert_eq!(records.goodies.len(), 1);
        assert!(parse_profile_records("overlays 4 7 zz").is_err());
    }

    #[test]
    fn progression_flag_masks_are_ored() {
        assert_eq!(parse_progression_flags("2"), Ok(2));
        assert_eq!(parse_progression_flags("1, 2,0x10"), Ok(0x13));
        assert_eq!(parse_progression_flags("0X1ffc"), Ok(0x1ffc));
        assert!(parse_progression_flags("").is_err());
        assert!(parse_progression_flags("-1").is_err());
        assert!(parse_progression_flags("two").is_err());
    }
}
