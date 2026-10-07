//! Shared independent bounded ordinary-binary payload handling.
//! Format evidence: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! SerializedResource::process, Compressor::decompressData; docs/mesh-prefix.md
//! and docs/material-prefix.md. Known clear headers only; no protected decoding.
use std::{fmt, io::Cursor};
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Blocked,
    Unsupported(&'static str),
    Invalid(&'static str),
    Budget,
    Cancelled,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

pub fn check_cancel(cancel: &impl Fn() -> bool) -> Result<(), Error> {
    if cancel() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
// Shared unsigned serialized integer: fixed BE32 or bounded ULEB32 when flag1 is
// set. Pinned MemoryInputStream::i32; existing material provenance/edge tests.
pub(crate) fn integer(data: &[u8], offset: &mut usize, compact: bool) -> Result<u32, Error> {
    if !compact {
        let end = offset.checked_add(4).ok_or(Error::Budget)?;
        let bytes = data
            .get(*offset..end)
            .ok_or(Error::Invalid("binary integer bounds"))?;
        *offset = end;
        return Ok(u32::from_be_bytes(bytes.try_into().unwrap()));
    }
    let mut result = 0;
    for i in 0..5 {
        let byte = *data
            .get(*offset)
            .ok_or(Error::Invalid("binary integer bounds"))?;
        *offset = offset.checked_add(1).ok_or(Error::Budget)?;
        if i == 4 && byte & 0xf0 != 0 {
            return Err(Error::Invalid("uleb32 overflow"));
        }
        result |= u32::from(byte & 127) << (i * 7);
        if byte & 128 == 0 {
            return Ok(result);
        }
    }
    Err(Error::Invalid("unterminated uleb32"))
}
pub fn expand(b: &[u8], max_expanded: usize, cancel: &impl Fn() -> bool) -> Result<Vec<u8>, Error> {
    if b.len() < 4 {
        return Err(Error::Invalid("chunk header"));
    }
    let short = |p| u16::from_be_bytes([b[p], b[p + 1]]) as usize;
    if short(0) > 1 {
        return Err(Error::Unsupported("compression flag"));
    }
    let count = short(2);
    if count > 32767 {
        return Err(Error::Budget);
    }
    let start = 4 + count * 4;
    if start > b.len() {
        return Err(Error::Invalid("chunk table"));
    }
    if count == 0 {
        if b.len() - 4 > max_expanded {
            return Err(Error::Budget);
        }
        check_cancel(cancel)?;
        return Ok(b[4..].to_vec());
    }
    let mut stored = 0usize;
    let mut expanded = 0usize;
    for r in b[4..start].as_chunks::<4>().0 {
        stored = stored
            .checked_add(u16::from_be_bytes([r[0], r[1]]) as usize)
            .ok_or(Error::Budget)?;
        expanded = expanded
            .checked_add(u16::from_be_bytes([r[2], r[3]]) as usize)
            .ok_or(Error::Budget)?;
        if expanded > max_expanded {
            return Err(Error::Budget);
        }
    }
    if stored != b.len() - start {
        return Err(Error::Invalid("chunk ranges/trailing bytes"));
    }
    let mut out = Vec::with_capacity(expanded);
    let mut p = start;
    for r in b[4..start].as_chunks::<4>().0 {
        check_cancel(cancel)?;
        let s = u16::from_be_bytes([r[0], r[1]]) as usize;
        let e = u16::from_be_bytes([r[2], r[3]]) as usize;
        let data = &b[p..p + s];
        p += s;
        if s == e {
            out.extend_from_slice(data);
            continue;
        }
        let mut z = flate2::Decompress::new(true);
        let mut dest = vec![0; e + 1];
        let status = z
            .decompress(data, &mut dest, flate2::FlushDecompress::Finish)
            .map_err(|_| Error::Invalid("zlib"))?;
        if status != flate2::Status::StreamEnd
            || z.total_in() != s as u64
            || z.total_out() != e as u64
        {
            return Err(Error::Invalid("zlib length/end/trailing"));
        }
        out.extend_from_slice(&dest[..e]);
    }
    Ok(out)
}

/// Immutable source bytes, expected magic and caller-specific allocation limits.
pub fn payload(
    b: &[u8],
    magic: &[u8; 4],
    max_input: usize,
    max_expanded: usize,
    cancel: &impl Fn() -> bool,
) -> Result<(crate::Header, Vec<u8>), Error> {
    check_cancel(cancel)?;
    if b.len() < 4 {
        return Err(Error::Invalid("magic"));
    }
    if crate::is_protected_prefix(&b[..4]) {
        return Err(Error::Blocked);
    }
    if &b[..4] != magic {
        return Err(Error::Unsupported("ordinary binary magic"));
    }
    if b.len() > max_input {
        return Err(Error::Budget);
    }
    let metadata = crate::inspect(&mut Cursor::new(b), 0, b.len() as u64, Default::default());
    if metadata.status != "metadata_parsed" {
        return Err(match metadata.status {
            "malformed" | "io_error" => Error::Invalid("resource metadata"),
            "limit_exceeded" => Error::Budget,
            _ => Error::Unsupported("resource header variant"),
        });
    }
    let h = metadata.header.ok_or(Error::Invalid("header absent"))?;
    let flags = h.compression_flags.ok_or(Error::Invalid("flags absent"))?;
    if !matches!(flags, 0 | 7) {
        return Err(Error::Unsupported("serialization flags"));
    }
    let start = usize::try_from(h.payload_offset.ok_or(Error::Invalid("payload offset"))?)
        .map_err(|_| Error::Budget)?;
    let end = usize::try_from(
        h.dependency_offset
            .ok_or(Error::Unsupported("unbounded payload"))?,
    )
    .map_err(|_| Error::Budget)?;
    let raw = b.get(start..end).ok_or(Error::Invalid("payload range"))?;
    let data = if h.compressed == Some(true) {
        expand(raw, max_expanded, cancel)?
    } else {
        if raw.len() > max_expanded {
            return Err(Error::Budget);
        }
        raw.to_vec()
    };
    check_cancel(cancel)?;
    Ok((h, data))
}
