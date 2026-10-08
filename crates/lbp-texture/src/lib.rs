//! Independent CPU-only GTF and TEX/DDS top-mip reader. No GUI/filesystem. Uncompressed GTF is unswizzled (D91).
//! Format knowledge: CONFIRMED_FROM_COMMUNITY_CODE, Toolkit
//! d6a2ef80b5a09999f08f23a803b516096055e5c9 SerializedResource::process,
//! CellGcmTexture constructor, Compressor::decompressData and RTexture::parseGTF.
//! Source: https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib
//! BC1/BC3: CONFIRMED_FROM_DOCUMENTATION, Microsoft block-compression specification:
//! https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression
//! Applicability: BCUS98245 01.00 clear ordinary GTF, raw 0x86/0x88, default remap,
//! 2D depth 1, zero pitch/offset; top mip only. Tail/mip offsets UNKNOWN.
//! Independent block implementation, no translated community pixel/swizzle code.
//! Synthetic coverage and pinned local oracle results: docs/texture-preview.md.
//! DDS: CONFIRMED_FROM_DOCUMENTATION, Microsoft DDS_HEADER/DDS_PIXELFORMAT:
//! https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header
//! https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-pixelformat
//! TEX envelope/footer reference: pinned Toolkit RTexture::process (BUMP suffix).
//! Applicability/conflicts/oracle coverage: docs/tex-preview.md; higher mips UNKNOWN.
//! BGRA8: CONFIRMED_FROM_DOCUMENTATION, DDS_PIXELFORMAT masks/flags and DDS_HEADER
//! pitch, plus DDS programming-guide uncompressed scanline calculation (URLs above;
//! https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dx-graphics-dds-pguide).
//! BCUS98245 clear TEX standard masks only. Legacy LINEARSIZE convention corroborated
//! by pinned RTexture::process / DDSReader::read external oracle on one local sample.
//! Independent mask-derived byte order; no copied pixel/swizzle implementation.
//! Field rules, conflict, synthetic tests and exact oracle coverage: docs/bgra-preview.md.
//! D91 uncompressed GTF 0x85 A8R8G8B8, 0x81 B8 and their 0x20 (LN) linear variants:
//! CONFIRMED_FROM_COMMUNITY_CODE, pinned Toolkit CellGcmEnumForGtf (values, LN/UN mask,
//! bytes per texel), DDS.convertSwizzleGtf (swizzle order, big-endian texels),
//! DDS.getDDSHeader/DDSReader.readL8 (B8 as opaque gray). Independent implementation of
//! written findings; rules, conflicts and oracle coverage: docs/d91-gtf-uncompressed.md.
//! D95: B8 pixel budget 16 Mi (same byte budgets); BC1/BC3 GTF cube maps through the
//! separate `decode_layered*` entry points only (face order: Toolkit DDS.getDDSHeader cube
//! caps, CONFIRMED_FROM_COMMUNITY_CODE; 128-byte face stride: CONFIRMED_FROM_LOCAL_DATA,
//! conflicts with Toolkit's unpadded pass-through). Volumes and remap 0x02ff stay refused.
//! Rules and evidence: docs/d95-texture-rest.md.
use serde::Serialize;
use std::fmt;
pub const MAX_INPUT: usize = 16 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 32 * 1024 * 1024;
/// Pixel budget for every format except B8, and the cap the preview composition and GPU
/// upload paths keep using.
pub const MAX_PIXELS: usize = 4 * 1024 * 1024;
/// D95: pixel budget for one-byte-per-texel B8 (4096 x 4096). Its top mip is 16 MiB, under
/// `MAX_EXPANDED`; the decoded RGBA stays under `MAX_RGBA`.
pub const MAX_PIXELS_ONE_BYTE: usize = 16 * 1024 * 1024;
/// D95: decoded RGBA output budget per texture, all cube faces together.
pub const MAX_RGBA: usize = 64 * 1024 * 1024;
/// Faces of a GTF cube map; order +X, -X, +Y, -Y, +Z, -Z (DDS cube caps order).
pub const CUBE_FACES: usize = 6;
/// D95: locally measured alignment of each cube face's mip chain.
const CUBE_FACE_ALIGN: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Blocked,
    Unsupported(&'static str),
    Invalid(&'static str),
    Budget,
    Cancelled,
    Input(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocked => write!(f, "Blocked: protected resource"),
            Self::Unsupported(s) => write!(f, "Unsupported: {s}"),
            Self::Invalid(s) => write!(f, "Invalid: {s}"),
            Self::Budget => write!(f, "Texture budget exceeded"),
            Self::Cancelled => write!(f, "Texture decode cancelled"),
            Self::Input(s) => write!(f, "Source read failed: {s}"),
        }
    }
}
impl std::error::Error for Error {}
/// CPU pixel formats with separately retained original wrapper metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PixelFormat {
    BC1,
    BC3,
    BGRA8,
    /// GTF 0x85: big-endian A,R,G,B bytes per texel.
    A8R8G8B8,
    /// GTF 0x81: one byte per texel, decoded as opaque gray (Toolkit L8 convention).
    B8,
}
impl PixelFormat {
    /// Bytes per 4x4 block (BC) or per texel (uncompressed).
    fn bytes(self) -> usize {
        match self {
            Self::BC1 => 8,
            Self::BC3 => 16,
            Self::BGRA8 | Self::A8R8G8B8 => 4,
            Self::B8 => 1,
        }
    }
    fn is_block(self) -> bool {
        matches!(self, Self::BC1 | Self::BC3)
    }
}
impl fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BC1 => "BC1",
            Self::BC3 => "BC3",
            Self::BGRA8 => "BGRA8",
            Self::A8R8G8B8 => "A8R8G8B8",
            Self::B8 => "B8",
        })
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Header {
    pub pixel_format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub wrapper: &'static str,
    pub flags: Option<u8>,
    pub remap: Option<u32>,
    pub format_raw: Option<u8>,
    pub no_srgb: bool,
    pub dds: Option<DdsHeader>,
}
#[derive(Debug, Clone, Serialize)]
pub struct DdsHeader {
    pub flags: u32,
    pub pitch_or_linear_size: u32,
    pub depth: u32,
    pub pixel_flags: u32,
    pub fourcc: String,
    pub rgb_bits: u32,
    pub masks: [u32; 4],
    pub caps: [u32; 4],
}
/// D95 layered result: `texture` is the 2D top mip (face 0, +X, for a cube map) and
/// `extra_faces` holds the top mips of faces 1..6 (-X, +Y, -Y, +Z, -Z) as RGBA, same size.
/// The preview renders only `texture`; the extra faces are decoded data, not rendered.
#[derive(Debug, Serialize)]
pub struct LayeredTexture {
    pub texture: Texture,
    /// "2d" or "cube".
    pub layout: &'static str,
    pub faces_decoded: u32,
    /// Byte distance between consecutive faces in the expanded payload (cube only).
    pub face_stride: Option<usize>,
    #[serde(skip)]
    pub extra_faces: Vec<Vec<u8>>,
}
#[derive(Debug, Serialize)]
pub struct Texture {
    pub header: Header,
    pub expanded_bytes: usize,
    pub top_mip_block_bytes: usize,
    pub unparsed_after_top_mip: usize,
    #[serde(skip)]
    pub rgba: Vec<u8>,
}
fn be16(b: &[u8], p: usize) -> usize {
    u16::from_be_bytes([b[p], b[p + 1]]) as usize
}
fn be32(b: &[u8], p: usize) -> u32 {
    u32::from_be_bytes(b[p..p + 4].try_into().unwrap())
}
fn checked(cancel: &impl Fn() -> bool) -> Result<(), Error> {
    if cancel() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
/// Header preflight permits rejection before callers read a full resource.
/// 2D only: cube maps are refused here and accepted by `inspect_layered`.
pub fn inspect(b: &[u8]) -> Result<Header, Error> {
    inspect_gtf(b, false).map(|(h, _)| h)
}
/// D95: like `inspect`, but also accepts BC1/BC3 cube maps; the flag reports a cube.
pub fn inspect_layered(b: &[u8]) -> Result<(Header, bool), Error> {
    inspect_gtf(b, true)
}
fn inspect_gtf(b: &[u8], allow_cube: bool) -> Result<(Header, bool), Error> {
    if b.len() < 4 {
        return Err(Error::Invalid("truncated magic"));
    }
    if matches!(&b[..4], b"GTFe" | b"TEXe" | b"NPD\0" | b"SCE\0") {
        return Err(Error::Blocked);
    }
    if &b[..4] != b"GTF " {
        return Err(Error::Unsupported("only ordinary GTF BC1/BC3 is supported"));
    }
    if b.len() < 32 {
        return Err(Error::Invalid("truncated GTF header"));
    }
    // Raw format byte = base format | 0x20 (LN, linear rows) | 0x40 (UN). BC stays
    // LN-free as before; LN is accepted only for the uncompressed formats.
    let linear = b[4] & 0x20 != 0;
    let pixel_format = match b[4] {
        0x86 => PixelFormat::BC1,
        0x88 => PixelFormat::BC3,
        0x85 | 0xa5 => PixelFormat::A8R8G8B8,
        0x81 | 0xa1 => PixelFormat::B8,
        _ => return Err(Error::Unsupported("pixel format or layout flags")),
    };
    // Toolkit CellGcmTexture: dimension > 2 is a volume, cubemap byte 1 is a cube map.
    let (dimension, cube_byte, depth) = (b[6], b[7], be16(b, 16));
    if dimension == 3 && cube_byte == 0 && depth > 1 {
        return Err(Error::Unsupported("volume texture: slice layout UNKNOWN"));
    }
    if dimension != 2 || cube_byte > 1 || depth != 1 {
        return Err(Error::Unsupported("cube/volume/dimension"));
    }
    let cube = cube_byte == 1;
    if cube && !allow_cube {
        return Err(Error::Unsupported("cube map (2D decode)"));
    }
    if cube && !pixel_format.is_block() {
        return Err(Error::Unsupported("uncompressed cube map"));
    }
    // 0xa9ff (Toolkit REMAP_BBBB) is the B8 gray remap; other remaps are refused.
    let remap = be32(b, 8);
    let remap_ok = remap == 0xaae4 || (pixel_format == PixelFormat::B8 && remap == 0xa9ff);
    if !remap_ok || be32(b, 24) != 0 || b[18] != 0 || b[19] & !1 != 0 {
        return Err(Error::Unsupported("remap/pitch/offset/location/flags"));
    }
    let (width, height) = (be16(b, 12), be16(b, 14));
    if width == 0 || height == 0 || b[5] == 0 {
        return Err(Error::Invalid("zero dimensions or mip count"));
    }
    let max_pixels = if pixel_format == PixelFormat::B8 {
        MAX_PIXELS_ONE_BYTE
    } else {
        MAX_PIXELS
    };
    if width > 4096
        || height > 4096
        || width.checked_mul(height).ok_or(Error::Budget)? > max_pixels
        || b[5] > 13
    {
        return Err(Error::Budget);
    }
    if cube && width != height {
        return Err(Error::Unsupported("non-square cube map"));
    }
    let pitch = be32(b, 20) as usize;
    if linear {
        let row = width
            .checked_mul(pixel_format.bytes())
            .ok_or(Error::Budget)?;
        if pitch < row {
            return Err(Error::Invalid("GTF linear pitch shorter than row"));
        }
    } else if pitch != 0 {
        return Err(Error::Unsupported("remap/pitch/offset/location/flags"));
    } else if !pixel_format.is_block() && !(width.is_power_of_two() && height.is_power_of_two()) {
        return Err(Error::Unsupported("non-power-of-two swizzled GTF"));
    }
    let header = Header {
        pixel_format,
        width: width as u32,
        height: height as u32,
        mips: b[5] as u32,
        wrapper: "GTF",
        flags: Some(b[19]),
        remap: Some(be32(b, 8)),
        format_raw: Some(b[4]),
        no_srgb: b[19] & 1 != 0,
        dds: None,
    };
    Ok((header, cube))
}
fn envelope(b: &[u8], allow_cube: bool) -> Result<usize, Error> {
    if b.len() < 4 {
        return Err(Error::Invalid("truncated magic"));
    }
    if &b[..4] != b"TEX " {
        inspect_gtf(b, allow_cube)?;
        return Ok(28);
    }
    if b.len() < 8 {
        return Err(Error::Invalid("truncated TEX envelope"));
    }
    Ok(4)
}
fn expand(b: &[u8], start: usize, cancel: &impl Fn() -> bool) -> Result<Vec<u8>, Error> {
    let count = be16(b, start + 2);
    if be16(b, start) > 1 {
        return Err(Error::Unsupported("compression flag"));
    }
    if count > 32767 {
        return Err(Error::Budget);
    }
    let payload = start + 4 + count * 4;
    if payload > b.len() {
        return Err(Error::Invalid("chunk table bounds"));
    }
    if count == 0 {
        return Ok(b[start + 4..].to_vec());
    }
    let table = &b[start + 4..payload];
    let (mut stored, mut expanded) = (0usize, 0usize);
    for r in table.as_chunks::<4>().0 {
        stored = stored.checked_add(be16(r, 0)).ok_or(Error::Budget)?;
        expanded = expanded.checked_add(be16(r, 2)).ok_or(Error::Budget)?;
        if expanded > MAX_EXPANDED {
            return Err(Error::Budget);
        }
    }
    if stored != b.len() - payload {
        return Err(Error::Invalid("chunk ranges or trailing archived bytes"));
    }
    let mut out = Vec::with_capacity(expanded);
    let mut offset = payload;
    for r in table.as_chunks::<4>().0 {
        checked(cancel)?;
        let (s, e) = (be16(r, 0), be16(r, 2));
        let chunk = &b[offset..offset + s];
        offset += s;
        if s == e {
            out.extend_from_slice(chunk);
            continue;
        }
        let mut z = flate2::Decompress::new(true);
        let mut decoded = vec![0; e + 1];
        let status = z
            .decompress(chunk, &mut decoded, flate2::FlushDecompress::Finish)
            .map_err(|_| Error::Invalid("zlib stream"))?;
        if status != flate2::Status::StreamEnd
            || z.total_in() != s as u64
            || z.total_out() != e as u64
        {
            return Err(Error::Invalid("zlib length/end/trailing input"));
        }
        out.extend_from_slice(&decoded[..e]);
    }
    Ok(out)
}
fn inspect_dds(b: &[u8]) -> Result<Header, Error> {
    if b.len() < 128 || &b[..4] != b"DDS " {
        return Err(Error::Invalid("missing/truncated DDS header"));
    }
    let le = |p| u32::from_le_bytes(b[p..p + 4].try_into().unwrap());
    if le(4) != 124 || le(76) != 32 {
        return Err(Error::Invalid("DDS structure sizes"));
    }
    let dds = DdsHeader {
        flags: le(8),
        pitch_or_linear_size: le(20),
        depth: le(24),
        pixel_flags: le(80),
        fourcc: String::from_utf8_lossy(&b[84..88]).into_owned(),
        rgb_bits: le(88),
        masks: [le(92), le(96), le(100), le(104)],
        caps: [le(108), le(112), le(116), le(120)],
    };
    if dds.fourcc == "DX10" {
        return Err(Error::Unsupported("DDS DX10 header"));
    }
    let pixel_format = if dds.fourcc == "\0\0\0\0"
        && dds.pixel_flags == 0x41
        && dds.rgb_bits == 32
        && dds.masks == [0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000]
    {
        PixelFormat::BGRA8
    } else if matches!(dds.fourcc.as_str(), "DXT1" | "DXT5")
        && dds.pixel_flags & 4 != 0
        && dds.pixel_flags & !5 == 0
    {
        if dds.fourcc == "DXT1" {
            PixelFormat::BC1
        } else {
            PixelFormat::BC3
        }
    } else {
        return Err(Error::Unsupported("DDS pixel format/flags/masks"));
    };
    if dds.depth > 1 || dds.flags & 0x800000 != 0 || dds.caps[1..].iter().any(|v| *v != 0) {
        return Err(Error::Unsupported("DDS cube/volume/extended caps"));
    }
    let (width, height, mips) = (le(16), le(12), le(28));
    if width == 0 || height == 0 {
        return Err(Error::Invalid("zero DDS dimensions"));
    }
    if width > 4096
        || height > 4096
        || u64::from(width) * u64::from(height) > MAX_PIXELS as u64
        || mips > 13
    {
        return Err(Error::Budget);
    }
    // DDS mip count zero is retained verbatim, not replaced by an invented count.
    Ok(Header {
        pixel_format,
        width,
        height,
        mips,
        wrapper: "TEX/DDS",
        flags: None,
        remap: None,
        format_raw: None,
        no_srgb: false,
        dds: Some(dds),
    })
}
// Documented scanline pitch; strict legacy LINEARSIZE compatibility for our
// oracle-confirmed clear TEX sample. Unset pitch uses width*4, not the raw field.
fn bgra_pitch(header: &Header) -> Result<usize, Error> {
    let dds = header
        .dds
        .as_ref()
        .ok_or(Error::Unsupported("BGRA8 wrapper"))?;
    let tight = (header.width as usize)
        .checked_mul(4)
        .ok_or(Error::Budget)?;
    let pitch = if dds.flags & 8 != 0 {
        if dds.flags & 0x80000 != 0 {
            return Err(Error::Unsupported(
                "DDS conflicting pitch/linear size flags",
            ));
        }
        let pitch = usize::try_from(dds.pitch_or_linear_size).map_err(|_| Error::Budget)?;
        if pitch < tight {
            return Err(Error::Invalid("DDS pitch shorter than row"));
        }
        pitch
    } else {
        if dds.flags & 0x80000 != 0
            && u64::from(dds.pitch_or_linear_size)
                != u64::from(header.width) * u64::from(header.height) * 4
        {
            return Err(Error::Unsupported(
                "DDS legacy linear size differs from tight top mip",
            ));
        }
        tight
    };
    Ok(pitch)
}
#[derive(Clone, Copy)]
struct PlainLayout {
    /// Top-mip byte count read from the expanded payload.
    size: usize,
    /// Row pitch for LN linear layout; None means swizzled.
    pitch: Option<usize>,
}
/// Top-mip extent of an uncompressed GTF. Header fields were bounded by `inspect`.
fn gtf_plain_layout(b: &[u8], header: &Header) -> Result<PlainLayout, Error> {
    let (w, h) = (header.width as usize, header.height as usize);
    let bpp = header.pixel_format.bytes();
    let row = w.checked_mul(bpp).ok_or(Error::Budget)?;
    if header.format_raw.unwrap_or(0) & 0x20 != 0 {
        let pitch = usize::try_from(be32(b, 20)).map_err(|_| Error::Budget)?;
        // The last row needs only its texels, not a full pitch.
        let size = pitch
            .checked_mul(h - 1)
            .and_then(|n| n.checked_add(row))
            .ok_or(Error::Budget)?;
        Ok(PlainLayout {
            size,
            pitch: Some(pitch),
        })
    } else {
        let size = row.checked_mul(h).ok_or(Error::Budget)?;
        Ok(PlainLayout { size, pitch: None })
    }
}
/// Texel index of (x, y) in a power-of-two swizzled surface: the low d = log2(min(w, h))
/// bits of x and y interleave (x on even, y on odd bit positions); the remaining high
/// bits of the longer axis follow above bit 2d. Callers guarantee x < w, y < h, both POT.
fn swizzle_index(x: usize, y: usize, w: usize, h: usize) -> usize {
    let d = w.min(h).trailing_zeros();
    let mut m = 0;
    for i in 0..d {
        m |= (((x >> i) & 1) << (2 * i)) | (((y >> i) & 1) << (2 * i + 1));
    }
    let high = if w > h { x >> d } else { y >> d };
    m | (high << (2 * d))
}
fn rgb565(c: u16) -> [u8; 4] {
    let (r, g, b) = ((c >> 11) as u8, ((c >> 5) & 63) as u8, (c & 31) as u8);
    // CPU oracle convention: nearest normalized 8-bit endpoints, floor interpolation.
    // C2 synthetic 0x18e3 and 16 local samples corroborate Toolkit DDSReader BIT5/BIT6.
    // This arithmetic is independently derived, not a copied lookup table/GPU claim.
    [
        ((r as u16 * 255 + 15) / 31) as u8,
        ((g as u16 * 255 + 31) / 63) as u8,
        ((b as u16 * 255 + 15) / 31) as u8,
        255,
    ]
}
// CONFIRMED_FROM_DOCUMENTATION: Microsoft Block Compression, BC3 section.
// Alpha endpoints and packed 48-bit little-endian indices precede RGB565 colors.
// BC3 always uses four color entries even when color0 <= color1. Floor interpolation
// is our CPU oracle convention; synthetic alpha modes and pinned comparisons: docs/bc3-preview.md.
fn block_pixels(block: &[u8], format: PixelFormat) -> [[u8; 4]; 16] {
    let bc3 = format == PixelFormat::BC3;
    let colors_offset = if bc3 { 8 } else { 0 };
    let color = &block[colors_offset..];
    let a = u16::from_le_bytes(color[..2].try_into().unwrap());
    let b = u16::from_le_bytes(color[2..4].try_into().unwrap());
    let (first, second) = (rgb565(a), rgb565(b));
    let mix = |wa: u16, wb: u16, den: u16| {
        std::array::from_fn(|i| ((wa * first[i] as u16 + wb * second[i] as u16) / den) as u8)
    };
    let four = bc3 || a > b;
    let colors = [
        first,
        second,
        if four { mix(2, 1, 3) } else { mix(1, 1, 2) },
        if four { mix(1, 2, 3) } else { [0; 4] },
    ];
    let indices = u32::from_le_bytes(color[4..8].try_into().unwrap());
    let mut alpha = [0u8; 8];
    let mut alpha_indices = 0u64;
    if bc3 {
        let (a, b) = (u16::from(block[0]), u16::from(block[1]));
        alpha[0] = a as u8;
        alpha[1] = b as u8;
        let (den, last) = if a > b { (7u16, 7) } else { (5u16, 5) };
        for (i, value) in alpha.iter_mut().enumerate().take(last + 1).skip(2) {
            let wb = (i - 1) as u16;
            *value = (((den - wb) * a + wb * b) / den) as u8;
        }
        if a <= b {
            alpha[6] = 0;
            alpha[7] = 255;
        }
        for (i, byte) in block[2..8].iter().enumerate() {
            alpha_indices |= u64::from(*byte) << (i * 8);
        }
    }
    std::array::from_fn(|i| {
        let mut rgba = colors[((indices >> (2 * i)) & 3) as usize];
        if bc3 {
            rgba[3] = alpha[((alpha_indices >> (3 * i)) & 7) as usize];
        }
        rgba
    })
}
pub fn decode(b: &[u8]) -> Result<Texture, Error> {
    decode_checked(b, || false)
}
/// The owner provides approved read-only range access; this crate owns no paths.
/// Reject unsupported headers before full reads; each callback is at most 4096 bytes.
/// 2D only; cube maps are refused (see `decode_reader_layered`).
pub fn decode_reader(
    size: u64,
    read: impl FnMut(u64, usize) -> Result<Vec<u8>, Error>,
    cancel: impl Fn() -> bool,
) -> Result<Texture, Error> {
    reader(size, read, &cancel, false).map(|l| l.texture)
}
/// D95: `decode_reader` that also decodes the six top-mip faces of a BC1/BC3 cube map.
pub fn decode_reader_layered(
    size: u64,
    read: impl FnMut(u64, usize) -> Result<Vec<u8>, Error>,
    cancel: impl Fn() -> bool,
) -> Result<LayeredTexture, Error> {
    reader(size, read, &cancel, true)
}
fn reader(
    size: u64,
    mut read: impl FnMut(u64, usize) -> Result<Vec<u8>, Error>,
    cancel: &impl Fn() -> bool,
    allow_cube: bool,
) -> Result<LayeredTexture, Error> {
    checked(cancel)?;
    let size = usize::try_from(size).map_err(|_| Error::Budget)?;
    if size > MAX_INPUT {
        return Err(Error::Budget);
    }
    let mut bytes = read(0, size.min(32))?;
    if bytes.len() != size.min(32) {
        return Err(Error::Invalid("short source header read"));
    }
    envelope(&bytes, allow_cube)?;
    bytes.reserve(size - bytes.len());
    while bytes.len() < size {
        checked(cancel)?;
        let count = (size - bytes.len()).min(4096);
        let part = read(bytes.len() as u64, count)?;
        if part.len() != count {
            return Err(Error::Invalid("short source page"));
        }
        bytes.extend(part);
    }
    decode_impl(&bytes, cancel, allow_cube)
}
/// 2D top mip; cube maps are refused (see `decode_layered_checked`).
pub fn decode_checked(b: &[u8], cancel: impl Fn() -> bool) -> Result<Texture, Error> {
    decode_impl(b, &cancel, false).map(|l| l.texture)
}
/// D95: also decodes the six top-mip faces of a BC1/BC3 GTF cube map. Face 0 (+X) is
/// `texture`; a 2D texture returns the same image as `decode_checked`.
pub fn decode_layered_checked(
    b: &[u8],
    cancel: impl Fn() -> bool,
) -> Result<LayeredTexture, Error> {
    decode_impl(b, &cancel, true)
}
/// Byte size of a full BC mip chain; mips <= 13 and sides <= 4096 were checked by `inspect`.
fn block_chain_bytes(header: &Header) -> Result<usize, Error> {
    let (mut w, mut h) = (header.width as usize, header.height as usize);
    let mut total = 0usize;
    for _ in 0..header.mips {
        let blocks = w
            .div_ceil(4)
            .checked_mul(h.div_ceil(4))
            .and_then(|n| n.checked_mul(header.pixel_format.bytes()))
            .ok_or(Error::Budget)?;
        total = total.checked_add(blocks).ok_or(Error::Budget)?;
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    Ok(total)
}
/// Top-mip BC blocks at `offset`; callers checked `offset + top-mip size <= data.len()`.
fn decode_blocks(
    data: &[u8],
    offset: usize,
    header: &Header,
    cancel: &impl Fn() -> bool,
) -> Result<Vec<u8>, Error> {
    let (w, h) = (header.width as usize, header.height as usize);
    let (bx, by) = (w.div_ceil(4), h.div_ceil(4));
    let mut rgba = vec![0; w * h * 4];
    for yb in 0..by {
        checked(cancel)?;
        for xb in 0..bx {
            let p = offset + (yb * bx + xb) * header.pixel_format.bytes();
            let block = &data[p..p + header.pixel_format.bytes()];
            let pixels = block_pixels(block, header.pixel_format);
            for y in 0..4 {
                for x in 0..4 {
                    let (px, py) = (xb * 4 + x, yb * 4 + y);
                    if px >= w || py >= h {
                        continue;
                    }
                    let color = pixels[y * 4 + x];
                    rgba[(py * w + px) * 4..(py * w + px + 1) * 4].copy_from_slice(&color);
                }
            }
        }
    }
    Ok(rgba)
}
fn decode_impl(
    b: &[u8],
    cancel: &impl Fn() -> bool,
    allow_cube: bool,
) -> Result<LayeredTexture, Error> {
    checked(cancel)?;
    if b.len() > MAX_INPUT {
        return Err(Error::Budget);
    }
    let start = envelope(b, allow_cube)?;
    let data = expand(b, start, cancel)?;
    let (mut header, offset, cube): (Header, usize, bool) = if start == 4 {
        (inspect_dds(&data)?, 128, false)
    } else {
        let (header, cube) = inspect_gtf(b, allow_cube)?;
        (header, 0, cube)
    };
    let (w, h) = (header.width as usize, header.height as usize);
    let (bx, by) = (w.div_ceil(4), h.div_ceil(4));
    let pitch = if header.pixel_format == PixelFormat::BGRA8 {
        Some(bgra_pitch(&header)?)
    } else {
        None
    };
    // Uncompressed GTF: Some(row pitch) for LN linear rows, None for swizzled.
    let gtf_plain = if start != 4 && !header.pixel_format.is_block() {
        Some(gtf_plain_layout(b, &header)?)
    } else {
        None
    };
    let size = if let Some(layout) = gtf_plain {
        Some(layout.size)
    } else if let Some(pitch) = pitch {
        pitch.checked_mul(h)
    } else {
        bx.checked_mul(by)
            .and_then(|n| n.checked_mul(header.pixel_format.bytes()))
    }
    .ok_or(Error::Budget)?;
    if size > MAX_EXPANDED {
        return Err(Error::Budget);
    }
    let end = offset.checked_add(size).ok_or(Error::Budget)?;
    if data.len() < end {
        return Err(Error::Invalid("truncated top mip data"));
    }
    if start == 4 && data.len() >= end + 4 && data.ends_with(b"BUMP") {
        header.no_srgb = true;
    }
    let rgba_size = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::Budget)?;
    let faces = if cube { CUBE_FACES } else { 1 };
    if rgba_size.checked_mul(faces).ok_or(Error::Budget)? > MAX_RGBA {
        return Err(Error::Budget);
    }
    // Cube: each face holds a full mip chain padded to 128 bytes. The payload must be
    // exactly six such faces, otherwise the layout does not hold for this file.
    let (face_stride, end) = if cube {
        let stride = block_chain_bytes(&header)?
            .checked_next_multiple_of(CUBE_FACE_ALIGN)
            .ok_or(Error::Budget)?;
        if stride.checked_mul(CUBE_FACES) != Some(data.len()) {
            return Err(Error::Unsupported(
                "cube payload is not six 128-byte-aligned mip chains",
            ));
        }
        (Some(stride), stride * (CUBE_FACES - 1) + end)
    } else {
        (None, end)
    };
    let mut extra_faces = Vec::new();
    let rgba = if let Some(stride) = face_stride {
        for face in 1..CUBE_FACES {
            extra_faces.push(decode_blocks(&data, face * stride, &header, cancel)?);
        }
        decode_blocks(&data, 0, &header, cancel)?
    } else if let Some(layout) = gtf_plain {
        let mut rgba = vec![0; rgba_size];
        let bpp = header.pixel_format.bytes();
        let top = &data[offset..end];
        for y in 0..h {
            checked(cancel)?;
            for x in 0..w {
                let p = match layout.pitch {
                    Some(pitch) => y * pitch + x * bpp,
                    None => swizzle_index(x, y, w, h) * bpp,
                };
                let t = &top[p..p + bpp];
                let color = if bpp == 1 {
                    [t[0], t[0], t[0], 255]
                } else {
                    [t[1], t[2], t[3], t[0]]
                };
                rgba[(y * w + x) * 4..(y * w + x + 1) * 4].copy_from_slice(&color);
            }
        }
        rgba
    } else if let Some(pitch) = pitch {
        let mut rgba = vec![0; rgba_size];
        for y in 0..h {
            checked(cancel)?;
            let row = &data[offset + y * pitch..offset + y * pitch + w * 4];
            for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
                rgba[(y * w + x) * 4..(y * w + x + 1) * 4]
                    .copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
        }
        rgba
    } else {
        decode_blocks(&data, offset, &header, cancel)?
    };
    checked(cancel)?;
    let texture = Texture {
        header,
        expanded_bytes: data.len(),
        top_mip_block_bytes: size,
        // Cube: measured after the last face's top mip.
        unparsed_after_top_mip: data.len() - end,
        rgba,
    };
    Ok(LayeredTexture {
        texture,
        layout: if cube { "cube" } else { "2d" },
        faces_decoded: faces as u32,
        face_stride,
        extra_faces,
    })
}
