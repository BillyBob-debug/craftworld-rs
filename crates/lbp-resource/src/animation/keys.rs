//! Independent ANMb keyframe and locator decoder (runtime plan item Rc); CPU only.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! RAnimation::serialize (modern read branch, version >= 0x378), getLoopedFrame,
//! getAnimDataForFrame/unpack, structs/animation/Locator::serialize,
//! MemoryInputStream::i16/f16/str/v3.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RAnimation.java
//! Applicability, conflicts (f16, morph frame 0, w clamp) and coverage: docs/animation-keys.md.
use super::*;

const MAX_LOCATORS: u32 = 4096;
const MAX_LOCATOR_NAME: usize = 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnimBoneEntry {
    pub anim_hash: u32,
    /// Stored bytes read as i8 (0xFF = -1, no link).
    pub parent: i8,
    pub first_child: i8,
    pub next_sibling: i8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Locator {
    pub position: [f32; 3],
    pub name: String,
    pub looping: i8,
    pub kind: i8,
}

/// Decoded keyframe tables. Frame 0 holds every bone (and every morph); frame f >= 1
/// holds only the animated tracks, at `count + (f - 1) * animated.len() + k`.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub inspection: AnimationInspection,
    pub bones: Vec<AnimBoneEntry>,
    /// Raw 4-byte locator-key records (structure unknown; Toolkit skips them).
    pub locator_keys: Vec<[u8; 4]>,
    pub rot_animated: Vec<u8>,
    pub pos_animated: Vec<u8>,
    pub scale_animated: Vec<u8>,
    pub morph_animated: Vec<u8>,
    /// Unit quaternions (x, y, z, w).
    pub rotations: Vec<[f32; 4]>,
    pub positions: Vec<[f32; 3]>,
    pub scales: Vec<[f32; 3]>,
    pub morphs: Vec<f32>,
    /// Rotations whose 1 - x² - y² - z² was negative (w clamped to 0).
    pub clamped_rotations: usize,
    /// animData bytes left after the morph keys (the writer pads to 16).
    pub trailing_bytes: usize,
    pub locators: Vec<Locator>,
    /// Payload bytes after the locator array.
    pub unparsed_after_locators: usize,
}

/// One frame's full pose: per bone TRS and per morph weight.
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    pub rotations: Vec<[f32; 4]>,
    pub positions: Vec<[f32; 3]>,
    pub scales: Vec<[f32; 3]>,
    pub morphs: Vec<f32>,
}

/// IEEE 754 binary16 to f32 (exact; subnormals, infinities and NaN preserved).
pub(crate) fn half(h: u16) -> f32 {
    let sign = u32::from(h >> 15) << 31;
    let exp = u32::from((h >> 10) & 0x1f);
    let mant = u32::from(h & 0x3ff);
    let bits = match (exp, mant) {
        (0, 0) => sign,
        (0, m) => {
            // Subnormal: value = m * 2^-24.
            return sign_factor(h) * (m as f32) * f32::powi(2.0, -24);
        }
        (0x1f, m) => sign | 0x7f80_0000 | (m << 13),
        (e, m) => sign | ((e + 112) << 23) | (m << 13),
    };
    f32::from_bits(bits)
}

fn sign_factor(h: u16) -> f32 {
    if h & 0x8000 != 0 {
        -1.0
    } else {
        1.0
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.p.checked_add(n).ok_or(Error::Budget)?;
        let v = self
            .b
            .get(self.p..end)
            .ok_or(Error::Invalid("animation data bounds"))?;
        self.p = end;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_bits(u32::from_be_bytes(
            self.take(4)?.try_into().unwrap(),
        )))
    }
}

/// Track table length `base + animated * (frames - 1)`; zero frames allow no tracks.
fn table_len(base: usize, animated: usize, frames: u16) -> Result<usize, Error> {
    if frames == 0 {
        return if animated == 0 {
            Ok(base)
        } else {
            Err(Error::Invalid("animated tracks with zero frames"))
        };
    }
    animated
        .checked_mul(usize::from(frames) - 1)
        .and_then(|n| n.checked_add(base))
        .ok_or(Error::Budget)
}

/// Decodes the whole modern ANMb payload. Track indices must address existing bones
/// (or morphs); a violation is Invalid because no pose could be built from it.
pub fn decode(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<Animation, Error> {
    let (inspection, data) = inspect_data(bytes, &cancel)?;
    let p = &inspection.prefix;
    let frames = p.frames_raw_u16;
    let [bone_count, rot_n, pos_n, scale_n, morph_n, locator_keys] = p.counts.map(usize::from);
    let morph_count = i8::try_from(p.morph_count_raw_u8)
        .map_err(|_| Error::Invalid("negative morph count"))? as usize;
    let blob = &data[p.animation_data_start..p.animation_data_start + p.animation_data_bytes];
    let mut c = Cursor { b: blob, p: 0 };
    let mut bones = Vec::with_capacity(bone_count);
    for _ in 0..bone_count {
        let r = c.take(8)?;
        bones.push(AnimBoneEntry {
            anim_hash: u32::from_be_bytes(r[..4].try_into().unwrap()),
            parent: r[4] as i8,
            first_child: r[5] as i8,
            next_sibling: r[6] as i8,
        });
    }
    let locator_keys = c
        .take(locator_keys.checked_mul(4).ok_or(Error::Budget)?)?
        .as_chunks::<4>()
        .0
        .to_vec();
    let rot_animated = c.take(rot_n)?.to_vec();
    let pos_animated = c.take(pos_n)?.to_vec();
    let scale_animated = c.take(scale_n)?.to_vec();
    let morph_animated = c.take(morph_n)?.to_vec();
    for (list, limit) in [
        (&rot_animated, bone_count),
        (&pos_animated, bone_count),
        (&scale_animated, bone_count),
        (&morph_animated, morph_count),
    ] {
        if list.iter().any(|&i| usize::from(i) >= limit) {
            return Err(Error::Invalid("animated track index"));
        }
    }
    if !c.p.is_multiple_of(2) {
        c.take(1)?;
    }
    let mut rotations = Vec::new();
    let mut clamped_rotations = 0;
    for i in 0..table_len(bone_count, rot_n, frames)? {
        if i % 1024 == 0 {
            check_cancel(&cancel)?;
        }
        let r = c.take(6)?;
        let raw = |k: usize| i16::from_be_bytes([r[k], r[k + 1]]);
        let x0 = raw(0);
        let negative_w = x0 & 1 != 0;
        let x = f32::from(x0 & !1) / 32767.0;
        let y = f32::from(raw(2)) / 32767.0;
        let z = f32::from(raw(4)) / 32767.0;
        let rad = 1.0 - (x * x + y * y + z * z);
        clamped_rotations += usize::from(rad < 0.0);
        let w = rad.max(0.0).sqrt();
        rotations.push([x, y, z, if negative_w { -w } else { w }]);
    }
    let mut vec3s = |len: usize| -> Result<Vec<[f32; 3]>, Error> {
        let mut out = Vec::new();
        for i in 0..len {
            if i % 1024 == 0 {
                check_cancel(&cancel)?;
            }
            out.push([half(c.u16()?), half(c.u16()?), half(c.u16()?)]);
        }
        Ok(out)
    };
    let positions = vec3s(table_len(bone_count, pos_n, frames)?)?;
    let scales = vec3s(table_len(bone_count, scale_n, frames)?)?;
    let mut morphs = Vec::new();
    for _ in 0..table_len(morph_count, morph_n, frames)? {
        morphs.push(half(c.u16()?));
    }
    let trailing_bytes = blob.len() - c.p;
    // Locators follow the byte array: array(Locator) with the payload's integer form.
    let compact = inspection.compression_flags & 1 != 0;
    let mut offset = p.animation_data_start + p.animation_data_bytes;
    let count = crate::binary::integer(&data, &mut offset, compact)?;
    if count > MAX_LOCATORS {
        return Err(Error::Budget);
    }
    let mut locators = Vec::new();
    for _ in 0..count {
        check_cancel(&cancel)?;
        let mut t = Cursor {
            b: &data,
            p: offset,
        };
        let position = [t.f32()?, t.f32()?, t.f32()?];
        offset = t.p;
        let raw = crate::binary::integer(&data, &mut offset, compact)?;
        // MemoryInputStream::s32: zigzag when integers are compressed.
        let len = if compact {
            (raw >> 1) as i32 ^ -((raw & 1) as i32)
        } else {
            raw as i32
        };
        let len = usize::try_from(len).map_err(|_| Error::Invalid("locator name length"))?;
        if len > MAX_LOCATOR_NAME {
            return Err(Error::Budget);
        }
        let mut t = Cursor {
            b: &data,
            p: offset,
        };
        let name = t.take(len)?;
        let flags = t.take(2)?;
        offset = t.p;
        locators.push(Locator {
            position,
            name: name
                .iter()
                .filter(|&&b| b != 0)
                .map(|&b| if b.is_ascii() { b as char } else { '\u{fffd}' })
                .collect(),
            looping: flags[0] as i8,
            kind: flags[1] as i8,
        });
    }
    check_cancel(&cancel)?;
    let unparsed_after_locators = data.len() - offset;
    Ok(Animation {
        inspection,
        bones,
        locator_keys,
        rot_animated,
        pos_animated,
        scale_animated,
        morph_animated,
        rotations,
        positions,
        scales,
        morphs,
        clamped_rotations,
        trailing_bytes,
        locators,
        unparsed_after_locators,
    })
}

impl Animation {
    pub fn frame_count(&self) -> u16 {
        self.inspection.prefix.frames_raw_u16
    }

    /// RAnimation::getLoopedFrame: looped frames wrap into [loopStart, frames);
    /// otherwise clamp to [0, frames - 1]. None if the loop range is empty.
    pub fn looped_frame(&self, frame: i64, looped: bool) -> Option<u16> {
        let frames = i64::from(self.frame_count());
        if looped {
            let start = i64::from(self.inspection.prefix.loop_start_raw_u16);
            let span = frames - start;
            (span > 0).then(|| (start + (frame - start).rem_euclid(span)) as u16)
        } else {
            (frames > 0).then(|| frame.clamp(0, frames - 1) as u16)
        }
    }

    /// Full pose at an integer frame (getAnimDataForFrame). Unlike Toolkit, morph
    /// tracks are only applied for frame >= 1, like the bone channels.
    pub fn pose(&self, frame: u16) -> Option<Pose> {
        if frame >= self.frame_count().max(1) {
            return None;
        }
        let n = self.bones.len();
        let mut pose = Pose {
            rotations: self.rotations[..n].to_vec(),
            positions: self.positions[..n].to_vec(),
            scales: self.scales[..n].to_vec(),
            morphs: self.morphs[..self.morph_base()].to_vec(),
        };
        if frame > 0 {
            let f = usize::from(frame) - 1;
            fn apply<T: Copy>(out: &mut [T], src: &[T], animated: &[u8], base: usize, f: usize) {
                let start = base + animated.len() * f;
                for (k, &target) in animated.iter().enumerate() {
                    out[usize::from(target)] = src[start + k];
                }
            }
            apply(
                &mut pose.rotations,
                &self.rotations,
                &self.rot_animated,
                n,
                f,
            );
            apply(
                &mut pose.positions,
                &self.positions,
                &self.pos_animated,
                n,
                f,
            );
            apply(&mut pose.scales, &self.scales, &self.scale_animated, n, f);
            let m = self.morph_base();
            apply(&mut pose.morphs, &self.morphs, &self.morph_animated, m, f);
        }
        Some(pose)
    }

    fn morph_base(&self) -> usize {
        usize::from(self.inspection.prefix.morph_count_raw_u8)
    }
}

#[cfg(test)]
mod tests {
    use super::half;

    #[test]
    fn half_floats_are_exact() {
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0xc000), -2.0);
        assert_eq!(half(0x3555), 0.333_251_95);
        assert_eq!(half(0x7bff), 65504.0);
        assert_eq!(half(0x0001), 2f32.powi(-24));
        assert_eq!(half(0x83ff), -(1023.0 * 2f32.powi(-24)));
        assert_eq!(half(0x8000).to_bits(), 0x8000_0000);
        assert!(half(0x7c00).is_infinite() && half(0x7e00).is_nan());
    }
}
