//! D271 (docs/level-scripts.md, part 22): the video and voice-over requests of the scripts
//! ([`MediaState`]) and the profile's word of LBP2 game progression flags.
//!
//! Nothing here plays a video or a sound. A request is kept as the script made it, so
//! that a later video or audio system can take it from here; the script is told what
//! the game tells it when nothing started.
use crate::LevelWorld;
use lbp_script_vm::Value;
use std::collections::BTreeMap;

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

/// One `Audio.RequestVoiceOver__Q7StringAbb` call. No line started for it: the script
/// got false.
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
}

fn name_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Str(s)) => Some(s.to_string()),
        _ => None,
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
    /// The video and audio calls of [`lbp_script_vm::natives::World::engine_call`]: kept in
    /// [`LevelWorld::media`]. Other names are not for this list.
    pub(crate) fn media_call(&mut self, name: &str, thing: Option<u32>, args: &[Value]) {
        let frame = self.pworld.frame;
        let flag = |i: usize| matches!(args.get(i), Some(Value::Bool(true)));
        let media = &mut self.media;
        match name {
            "StartVideo" if media.videos.len() < MAX_MEDIA_REQUESTS => {
                media.videos.push(VideoRequest {
                    thing,
                    guid: guid_of(args.get(1)),
                    flag: flag(2),
                    frame,
                });
            }
            "StartVideo" => media.videos_dropped += 1,
            "StopVideo" => media.video_stops += 1,
            "RequestVoiceOver" if media.voice_overs.len() < MAX_MEDIA_REQUESTS => {
                media.voice_overs.push(VoiceOverRequest {
                    name: name_of(args.first()),
                    flags: [flag(1), flag(2)],
                    frame,
                });
            }
            "RequestVoiceOver" => media.voice_overs_dropped += 1,
            "StopVoiceOver" => media.voice_over_stops += 1,
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
pub fn parse_profile_records(text: &str) -> Result<BTreeMap<(i32, i32), [u32; 2]>, String> {
    let mut records = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let bad = |e: &dyn std::fmt::Display| format!("profile record line {}: {e}", n + 1);
        let words: Vec<&str> = line.split_whitespace().collect();
        let [ty, id, collected, total] = words[..] else {
            return Err(bad(&"expected TYPE ID COLLECTED TOTAL"));
        };
        let int = |w: &str| w.parse::<i32>().map_err(|e| bad(&e));
        let count = |w: &str| w.parse::<u32>().map_err(|e| bad(&e));
        let counts = [count(collected)?, count(total)?];
        if counts[0] > counts[1] {
            return Err(bad(&"more collected than the total"));
        }
        records.insert((int(ty)?, int(id)?), counts);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d341_profile_records_are_read_by_slot_id() {
        let text = "# type id collected total\n0 20354 36 40\n\n4 7 0 0  # a group\n";
        let records = parse_profile_records(text).unwrap();
        assert_eq!(records.get(&(0, 20354)), Some(&[36, 40]));
        assert_eq!(records.get(&(4, 7)), Some(&[0, 0]));
        assert_eq!(records.len(), 2);
        assert!(parse_profile_records("0 1 2").is_err());
        assert!(parse_profile_records("0 1 5 4").is_err());
        assert!(parse_profile_records("0 x 1 1").is_err());
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
