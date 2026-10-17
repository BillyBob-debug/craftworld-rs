//! D400: animated texture (`ATXb`, resource type 45) payloads: the sprite sheet a texture
//! reference names, plus its frame count and cell size. Used by the Popit's icons
//! (`Resource.LoadAnimatedTextureByFilename` in the `Poppet` script).
//!
//! Layout (INFERRED from the ten Popit icon `.atx` files on BCUS98245 01.33, revision
//! 0x38a, flags 7, all consistent with the sheets' measured cells and frame counts in
//! docs/d235-popit.md): a resource reference (a compact flags integer, then a type byte:
//! 0 none, 1 SHA-1 (20 bytes), 2 GUID (compact integer)), then compact integers frame
//! count, cell width, cell height, and one more integer (1 on every file seen, meaning
//! UNKNOWN).
use crate::binary::{integer, Error};

const MAX_INPUT: usize = 4096;
const MAX_EXPANDED: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureRef {
    Guid(u32),
    Sha1([u8; 20]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimatedTexture {
    pub revision: u32,
    pub texture: Option<TextureRef>,
    pub frames: u32,
    pub cell: [u32; 2],
    /// The trailing integer (UNKNOWN meaning; 1 on every Popit icon).
    pub tail: u32,
}

pub fn parse_animated_texture(
    bytes: &[u8],
    cancel: &impl Fn() -> bool,
) -> Result<AnimatedTexture, Error> {
    let (header, data) = crate::binary::payload(bytes, b"ATXb", MAX_INPUT, MAX_EXPANDED, cancel)?;
    let compact = header.compression_flags.unwrap_or(0) & 1 != 0;
    let mut at = 0;
    let _ref_flags = integer(&data, &mut at, compact)?;
    let kind = *data.get(at).ok_or(Error::Invalid("texture reference"))?;
    at += 1;
    let texture = match kind {
        0 => None,
        1 => {
            let sha = data
                .get(at..at + 20)
                .ok_or(Error::Invalid("texture sha1 bounds"))?;
            at += 20;
            Some(TextureRef::Sha1(sha.try_into().unwrap()))
        }
        2 => Some(TextureRef::Guid(integer(&data, &mut at, compact)?)),
        _ => return Err(Error::Unsupported("texture reference type")),
    };
    let frames = integer(&data, &mut at, compact)?;
    let cell = [
        integer(&data, &mut at, compact)?,
        integer(&data, &mut at, compact)?,
    ];
    let tail = integer(&data, &mut at, compact)?;
    if at != data.len() {
        return Err(Error::Invalid("animated texture trailing bytes"));
    }
    Ok(AnimatedTexture {
        revision: header.revision,
        texture,
        frames,
        cell,
        tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic `ATXb` file with the observed header shape (revision 0x38a, branch 0,
    /// flags 7, uncompressed chunk table) and one dependency.
    fn synthetic(body: &[u8]) -> Vec<u8> {
        let mut b = b"ATXb".to_vec();
        b.extend(0x38a_u32.to_be_bytes());
        let dependency_offset = 12 + 4 + 2 + 4 + body.len();
        b.extend((dependency_offset as u32).to_be_bytes());
        b.extend([0, 0, 0, 0, 7, 1, 0, 1, 0, 0]);
        b.extend(body);
        b.extend([0, 0, 0, 0]);
        b
    }

    #[test]
    fn d400_reads_sheet_reference_frames_and_cell() {
        // GUID 0x12345 as ULEB (c5 c6 04), 25 frames, 64 x 64 cells, tail 1.
        let b = synthetic(&[0, 2, 0xc5, 0xc6, 0x04, 25, 64, 64, 1]);
        let a = parse_animated_texture(&b, &|| false).unwrap();
        assert_eq!(a.texture, Some(TextureRef::Guid(0x12345)));
        assert_eq!((a.frames, a.cell, a.tail), (25, [64, 64], 1));
        // 128 x 128 cells take two bytes each.
        let b = synthetic(&[0, 2, 0x05, 32, 0x80, 1, 0x80, 1, 1]);
        let a = parse_animated_texture(&b, &|| false).unwrap();
        assert_eq!((a.texture, a.cell), (Some(TextureRef::Guid(5)), [128, 128]));
    }

    #[test]
    fn d400_rejects_trailing_bytes_and_unknown_reference() {
        let b = synthetic(&[0, 2, 0x05, 32, 64, 64, 1, 9]);
        assert!(parse_animated_texture(&b, &|| false).is_err());
        let b = synthetic(&[0, 3, 0x05, 32, 64, 64, 1]);
        assert!(parse_animated_texture(&b, &|| false).is_err());
    }
}
