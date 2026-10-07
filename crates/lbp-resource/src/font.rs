//! D79 FNTb font face: header and glyph-table metadata only, no glyph decoding or rendering.
//!
//! Format rules: CONFIRMED_FROM_COMMUNITY_CODE.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9
//! lib/cwlib/src/main/java/cwlib/resources/RFontFace.java (constructor, getGlyphData),
//! structs/font/GlyphInfo.java::serialize, io/streams/MemoryInputStream.java
//! (shortarray/intarray: big-endian u32 count then elements; no compressed integers).
//! Independent layout implementation, MIT knowledge notice in THIRD-PARTY-NOTICES.
//! Layout (big-endian): "FNTb", u32 revision, u32 declared data offset, u32 unused word,
//! glyph index (u32 count, i16 each), glyph pages used (u32 count, u32 each), glyphs
//! (u32 count, 14 bytes each), one compression bool only when revision > 0x272, then glyph
//! bitmap data to the end. Toolkit itself calls the 0x272 gate approximate; locally one
//! 01.33 sample (head 0x3d9) is known, so the gate is untested below LBP2 heads.
//! Glyph data spans are bounds-checked, never inflated (docs/d79-fntb-font.md).
use lbp_core::{ByteReader, Error, ErrorKind, Result};
use serde::Serialize;

pub const FONT_MAGIC: &[u8; 4] = b"FNTb";
/// Heads above this carry the compression bool (Toolkit `RFontFace`, approximate gate).
pub const FONT_COMPRESSION_BOOL_AFTER: u32 = 0x272;
/// GlyphInfo on disk: character i16, advance i16, four i8 box fields, offset i32, next i16.
pub const GLYPH_RECORD_BYTES: usize = 14;

#[derive(Debug, Clone, Copy)]
pub struct FontLimits {
    pub max_index_entries: usize,
    pub max_page_words: usize,
    pub max_glyphs: usize,
}
impl Default for FontLimits {
    fn default() -> Self {
        // Toolkit defaults are 128 index shorts and 16 page words; counts are untrusted.
        Self {
            max_index_entries: 65536,
            max_page_words: 4096,
            max_glyphs: 65536,
        }
    }
}

/// One raw GlyphInfo record. Field meanings beyond Toolkit's names are not asserted.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Glyph {
    /// UTF-16 code unit (Toolkit compares it with a Java `char`).
    pub character: u16,
    pub advance: i16,
    pub box_left: i8,
    pub box_top: i8,
    pub box_width: u8,
    pub box_height: u8,
    /// Offset into the glyph data region (Toolkit i32; kept unsigned, so negative is out of range).
    pub data_offset: u32,
    pub next_glyph: i16,
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct GlyphDataCheck {
    /// Span lies inside the data region (compressed: 2-byte size prefix >= 2 included).
    pub in_bounds: usize,
    /// Span, or its compressed size prefix, falls outside the data region. Not fatal.
    pub out_of_bounds: usize,
    /// Glyphs with a zero-width or zero-height box (still counted in the two above).
    pub empty_boxes: usize,
    /// Non-negative glyph index entries >= glyph count (negative entries are not counted).
    pub index_out_of_range: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    pub revision: u32,
    /// Word at +8 (Toolkit: "data offset", read and ignored). Recorded raw, not trusted.
    pub declared_data_offset: u32,
    /// Word at +12 (Toolkit: "Padding presumably").
    pub reserved_word: u32,
    pub glyph_index: Vec<i16>,
    pub glyph_page_used: Vec<u32>,
    pub glyphs: Vec<Glyph>,
    /// `None` when the revision has no compression bool (<= 0x272).
    pub compressed: Option<bool>,
    /// Absolute start of the glyph data region within the resource.
    pub data_start: u64,
    pub data_length: u64,
    pub glyph_data: GlyphDataCheck,
}

/// Scanner-sized summary: counts and scalars, no per-glyph arrays.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FontSummary {
    pub revision: u32,
    pub declared_data_offset: u32,
    pub declared_data_offset_matches_data_start: bool,
    pub reserved_word: u32,
    pub glyph_index_count: usize,
    pub glyph_page_word_count: usize,
    pub glyph_count: usize,
    pub compressed: Option<bool>,
    pub data_start: u64,
    pub data_length: u64,
    pub glyph_data: GlyphDataCheck,
}

impl FontFace {
    pub fn summary(&self) -> FontSummary {
        FontSummary {
            revision: self.revision,
            declared_data_offset: self.declared_data_offset,
            declared_data_offset_matches_data_start: u64::from(self.declared_data_offset)
                == self.data_start,
            reserved_word: self.reserved_word,
            glyph_index_count: self.glyph_index.len(),
            glyph_page_word_count: self.glyph_page_used.len(),
            glyph_count: self.glyphs.len(),
            compressed: self.compressed,
            data_start: self.data_start,
            data_length: self.data_length,
            glyph_data: self.glyph_data,
        }
    }
}

/// Parse a whole FNTb resource held in memory.
pub fn parse_font_face(bytes: &[u8], limits: FontLimits) -> Result<FontFace> {
    let mut fetch = |offset: u64, size: usize| -> Result<Vec<u8>> {
        let start =
            usize::try_from(offset).map_err(|_| Error::malformed("font offset overflow"))?;
        let end = start
            .checked_add(size)
            .ok_or_else(|| Error::malformed("font range overflow"))?;
        bytes
            .get(start..end)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| Error::malformed("font range outside resource"))
    };
    read_font_face(&mut fetch, bytes.len() as u64, limits)
}

/// Staged reads through `fetch(offset, size)` so a caller can enforce its own byte budget.
pub(crate) fn read_font_face(
    fetch: &mut dyn FnMut(u64, usize) -> Result<Vec<u8>>,
    size: u64,
    limits: FontLimits,
) -> Result<FontFace> {
    if size < 16 {
        return Err(Error::malformed("font face shorter than 16-byte header"));
    }
    let head = fetch(0, 16)?;
    if &head[..4] != FONT_MAGIC {
        return Err(Error::new(ErrorKind::Unsupported, "not an FNTb font face"));
    }
    let mut words = ByteReader::new(&head[4..]);
    let revision = words.be_u32()?;
    let declared_data_offset = words.be_u32()?;
    let reserved_word = words.be_u32()?;
    let mut position = 16_u64;

    let raw = read_array(
        fetch,
        size,
        &mut position,
        2,
        limits.max_index_entries,
        "glyph index",
    )?;
    let glyph_index = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_be_bytes(*c))
        .collect::<Vec<_>>();
    let raw = read_array(
        fetch,
        size,
        &mut position,
        4,
        limits.max_page_words,
        "glyph page",
    )?;
    let glyph_page_used = raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_be_bytes(*c))
        .collect::<Vec<_>>();
    let raw = read_array(
        fetch,
        size,
        &mut position,
        GLYPH_RECORD_BYTES,
        limits.max_glyphs,
        "glyph",
    )?;
    let glyphs = raw
        .as_chunks::<GLYPH_RECORD_BYTES>()
        .0
        .iter()
        .map(|c| Glyph {
            character: u16::from_be_bytes([c[0], c[1]]),
            advance: i16::from_be_bytes([c[2], c[3]]),
            box_left: c[4] as i8,
            box_top: c[5] as i8,
            box_width: c[6],
            box_height: c[7],
            data_offset: u32::from_be_bytes(c[8..12].try_into().unwrap()),
            next_glyph: i16::from_be_bytes([c[12], c[13]]),
        })
        .collect::<Vec<_>>();

    let compressed = if revision > FONT_COMPRESSION_BOOL_AFTER {
        if position >= size {
            return Err(Error::malformed(
                "font face truncated before compression flag",
            ));
        }
        let flag = fetch(position, 1)?[0];
        position += 1;
        if flag > 1 {
            // Toolkit accepts any non-zero byte; a strict 0/1 is required here.
            return Err(Error::new(
                ErrorKind::Unsupported,
                "font compression flag is not 0 or 1",
            ));
        }
        Some(flag == 1)
    } else {
        None
    };
    let data_start = position;
    let data_length = size - data_start;

    let mut check = GlyphDataCheck::default();
    for glyph in &glyphs {
        let pixels = u64::from(glyph.box_width) * u64::from(glyph.box_height);
        if pixels == 0 {
            check.empty_boxes += 1;
        }
        let start = u64::from(glyph.data_offset);
        let span = if compressed == Some(true) {
            // Big-endian u16 size that includes its own two bytes; zlib stream follows.
            if start.checked_add(2).is_some_and(|end| end <= data_length) {
                let prefix = fetch(data_start + start, 2)?;
                let length = u64::from(u16::from_be_bytes([prefix[0], prefix[1]]));
                (length >= 2).then_some(length)
            } else {
                None
            }
        } else {
            // Two bytes per pixel when stored uncompressed.
            Some(pixels * 2)
        };
        match span.and_then(|length| start.checked_add(length)) {
            Some(end) if end <= data_length => check.in_bounds += 1,
            _ => check.out_of_bounds += 1,
        }
    }
    check.index_out_of_range = glyph_index
        .iter()
        .filter(|&&entry| entry >= 0 && entry as usize >= glyphs.len())
        .count();

    Ok(FontFace {
        revision,
        declared_data_offset,
        reserved_word,
        glyph_index,
        glyph_page_used,
        glyphs,
        compressed,
        data_start,
        data_length,
        glyph_data: check,
    })
}

fn read_array(
    fetch: &mut dyn FnMut(u64, usize) -> Result<Vec<u8>>,
    size: u64,
    position: &mut u64,
    element: usize,
    max: usize,
    what: &str,
) -> Result<Vec<u8>> {
    if size.saturating_sub(*position) < 4 {
        return Err(Error::malformed(format!("font {what} count truncated")));
    }
    let count = ByteReader::new(&fetch(*position, 4)?).be_u32()? as usize;
    *position += 4;
    if count > max {
        return Err(Error::limit(format!("font {what} count exceeds limit")));
    }
    let length = count
        .checked_mul(element)
        .ok_or_else(|| Error::malformed(format!("font {what} length overflow")))?;
    if length as u64 > size - *position {
        return Err(Error::malformed(format!(
            "font {what} count exceeds remaining bytes"
        )));
    }
    let bytes = fetch(*position, length)?;
    *position += length as u64;
    Ok(bytes)
}
