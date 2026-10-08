//! L3 serializer core: bounded primitive reads, compressed integers/vectors/matrices,
//! resource descriptors and the shared reference tag table with depth/tag budgets.
//!
//! Independent specification notes (no code copied/translated), written from
//! ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9:
//! - `io/streams/MemoryInputStream.java` (i32/s32/u32/u64/s64/uleb128, f32, v2/v3/v4,
//!   m44, sha1, guid, bool, i16) and `io/serializer/Serializer.java` (intvector,
//!   resource, reference, array/arraylist). CONFIRMED_FROM_COMMUNITY_CODE.
//! - Compression flag bits (`enums/CompressionFlags.java`): 1 compressed integers,
//!   2 compressed vectors, 4 compressed matrices. CONFIRMED_FROM_COMMUNITY_CODE; local
//!   resources carry 0 or 7 only (D27, CONFIRMED_FROM_LOCAL_DATA via header census).
//!
//! Rules as implemented (big-endian everywhere):
//! - i32/u32: flag 1 clear -> fixed BE32; set -> ULEB128 truncated to 32 bits. This
//!   reader rejects ULEB encodings longer than 5 bytes or with bits above 32 (Toolkit
//!   loops unbounded and masks): a deliberate strictness conflict, reported as Invalid.
//! - s32: flag 1 clear -> BE32; set -> 32-bit zigzag of the ULEB value.
//! - u64: flag 1 clear -> BE64; set -> ULEB128 (at most 10 bytes, strict).
//! - f32 is always BE32 IEEE; v2/v3/v4 are 2/3/4 plain f32 (flag 2 does not change them).
//! - m44: flag 4 set -> u16 bit mask then one f32 per set bit (bit i = element i),
//!   identity for the rest; flag 4 clear -> 16 f32. Element order is the serialized order;
//!   Toolkit hands it to JOML `Matrix4f.set(float[])` (column-major), so translation is
//!   elements 12..14 (STRONG_INFERENCE from the community reader's use, not observed).
//! - intvector: flag 2 clear -> i32 count + i32 per element; set -> i32 count, if count is
//!   nonzero a u8 byte width then `width` planes of `count` bytes (plane i = byte i,
//!   little-endian within the element). Widths above 4 are refused here (Toolkit would
//!   shift a 32-bit int by >= 32).
//! - guid: u32 (compressed per flag 1); 0 means none.
//! - resource descriptor: for head > 0x22e and not a "descriptor" call, an i32 flags word;
//!   then an i8 kind byte: bit 2 GUID (u32), bit 1 SHA1 (20 bytes), 0 none. Below head
//!   0x191 the bits swap for class pointers (not reachable in the LBP2 range read here).
//! - reference: i32 tag; 0 null; a tag seen before is a back-reference to that object
//!   (one table across all classes per serializer); a new tag is followed by the object
//!   inline, read recursively. Tags are registered before the inline body, so cycles
//!   resolve to back-references.
//!
//! Every count is checked against the remaining bytes times the minimum element size
//! before any allocation; depth, tag and Thing totals have explicit budgets.
use serde::Serialize;
use std::collections::HashMap;

pub const FLAG_INTEGERS: u8 = 1;
pub const FLAG_VECTORS: u8 = 2;
pub const FLAG_MATRICES: u8 = 4;

/// Explicit stop outcome. `offset` is the payload byte offset where the stop happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Stop {
    /// Format variant or part this reader does not implement.
    Unsupported {
        what: String,
        offset: usize,
    },
    /// A present Thing part with no reader here. `parts_revision` is the Thing's value.
    UnsupportedPart {
        part: &'static str,
        head: u32,
        parts_revision: i32,
        offset: usize,
    },
    /// Bytes contradict the implemented rules (truncation, bad marker, overflow).
    Invalid {
        what: String,
        offset: usize,
    },
    /// A depth, tag, Thing or allocation budget was exceeded.
    Budget {
        what: &'static str,
        offset: usize,
    },
    Cancelled,
}

impl Stop {
    pub fn label(&self) -> String {
        match self {
            Stop::Unsupported { what, .. } => format!("unsupported: {what}"),
            Stop::UnsupportedPart {
                part,
                head,
                parts_revision,
                ..
            } => format!(
                "unsupported part {part} (head 0x{head:x}, partsRevision 0x{parts_revision:x})"
            ),
            Stop::Invalid { what, .. } => format!("invalid: {what}"),
            Stop::Budget { what, .. } => format!("budget: {what}"),
            Stop::Cancelled => "cancelled".into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Stop>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum nesting of inline reference bodies (Thing -> part -> Thing ...). The reader
    /// recurses natively, so the calling thread's stack must fit this bound: the default
    /// 256 needs a large stack (the D107 examples run on a 512 MiB worker thread; the
    /// deepest local level reaches 162).
    pub max_depth: usize,
    pub max_tags: usize,
    pub max_things: usize,
    /// Maximum element count of any single array/vector.
    pub max_array: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 256,
            max_tags: 2_000_000,
            max_things: 500_000,
            max_array: 4_000_000,
        }
    }
}

/// Resource revision context (head, branch) and compression flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Revision {
    pub head: u32,
    pub branch_id: u16,
    pub branch_revision: u16,
    pub compression: u8,
}

impl Revision {
    /// Toolkit `Revision::getVersion` is the low 16 bits of head; the LBP2 band has no
    /// subVersion, so the reader refuses heads above 0xffff before use.
    pub fn version(&self) -> u32 {
        self.head & 0xffff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Descriptor {
    pub flags: Option<u32>,
    pub kind_byte: u8,
    pub guid: Option<u32>,
    #[serde(serialize_with = "hex_opt")]
    pub sha1: Option<[u8; 20]>,
}

fn hex_opt<S: serde::Serializer>(
    v: &Option<[u8; 20]>,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    match v {
        Some(h) => s.serialize_str(&hex(h)),
        None => s.serialize_none(),
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// What a reference tag resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obj {
    Thing(usize),
    /// A part body (or other struct) by class name and a per-class sequence number.
    Other(&'static str, usize),
}

/// Outcome of reading a reference tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Null,
    /// Back-reference to an already registered object.
    Seen(Obj),
    /// New tag; the caller must read the inline body now (already registered).
    New(u32),
}

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    pub rev: Revision,
    pub limits: Limits,
    tags: HashMap<u32, Obj>,
    depth: usize,
    pub max_depth_seen: usize,
    other_counts: HashMap<&'static str, usize>,
    cancel: &'a dyn Fn() -> bool,
}

impl<'a> Reader<'a> {
    pub fn new(
        data: &'a [u8],
        rev: Revision,
        limits: Limits,
        cancel: &'a dyn Fn() -> bool,
    ) -> Self {
        Self {
            data,
            pos: 0,
            rev,
            limits,
            tags: HashMap::new(),
            depth: 0,
            max_depth_seen: 0,
            other_counts: HashMap::new(),
            cancel,
        }
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }
    pub fn check_cancel(&self) -> Result<()> {
        if (self.cancel)() {
            Err(Stop::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn invalid<T>(&self, what: impl Into<String>) -> Result<T> {
        Err(Stop::Invalid {
            what: what.into(),
            offset: self.pos,
        })
    }
    pub fn unsupported<T>(&self, what: impl Into<String>) -> Result<T> {
        Err(Stop::Unsupported {
            what: what.into(),
            offset: self.pos,
        })
    }
    pub fn budget<T>(&self, what: &'static str) -> Result<T> {
        Err(Stop::Budget {
            what,
            offset: self.pos,
        })
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = match self.pos.checked_add(n) {
            Some(e) if e <= self.data.len() => e,
            _ => return self.invalid("truncated"),
        };
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }
    /// Toolkit `bool`: one byte, nonzero is true.
    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }
    fn be32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn uleb(&mut self, max_bits: u32) -> Result<u64> {
        let mut result = 0u64;
        let mut shift = 0u32;
        loop {
            let b = self.u8()?;
            let chunk = u64::from(b & 0x7f);
            if shift >= max_bits
                || (shift > 0 && max_bits - shift < 7 && chunk >> (max_bits - shift) != 0)
            {
                return self.invalid("uleb overflow");
            }
            result |= chunk << shift;
            if b & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
    }
    fn compact(&self) -> bool {
        self.rev.compression & FLAG_INTEGERS != 0
    }
    /// Toolkit `i32()`/`u32()`: raw 32 bits.
    pub fn u32(&mut self) -> Result<u32> {
        if self.compact() {
            Ok(self.uleb(32)? as u32)
        } else {
            self.be32()
        }
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    /// Toolkit `s32()`: zigzag when integers are compressed.
    pub fn s32(&mut self) -> Result<i32> {
        if self.compact() {
            let v = self.uleb(32)? as u32;
            Ok(((v >> 1) as i32) ^ -((v & 1) as i32))
        } else {
            Ok(self.be32()? as i32)
        }
    }
    pub fn u64(&mut self) -> Result<u64> {
        if self.compact() {
            self.uleb(64)
        } else {
            let b = self.bytes(8)?;
            Ok(u64::from_be_bytes(b.try_into().unwrap()))
        }
    }
    /// Toolkit `i32(value, force32 = true)`: always fixed BE32.
    pub fn fixed_u32(&mut self) -> Result<u32> {
        self.be32()
    }
    /// Toolkit `s64(value, force64 = true)`: always fixed BE64.
    pub fn fixed_u64(&mut self) -> Result<u64> {
        let b = self.bytes(8)?;
        Ok(u64::from_be_bytes(b.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.be32()?))
    }
    pub fn v2(&mut self) -> Result<[f32; 2]> {
        Ok([self.f32()?, self.f32()?])
    }
    pub fn v3(&mut self) -> Result<[f32; 3]> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    pub fn v4(&mut self) -> Result<[f32; 4]> {
        Ok([self.f32()?, self.f32()?, self.f32()?, self.f32()?])
    }
    pub fn m44(&mut self) -> Result<[f32; 16]> {
        let mut m = [0f32; 16];
        for i in [0, 5, 10, 15] {
            m[i] = 1.0;
        }
        let mask = if self.rev.compression & FLAG_MATRICES != 0 {
            self.u16()?
        } else {
            0xffff
        };
        for (i, v) in m.iter_mut().enumerate() {
            if mask & (1 << i) != 0 {
                *v = self.f32()?;
            }
        }
        Ok(m)
    }
    pub fn sha1(&mut self) -> Result<[u8; 20]> {
        Ok(self.bytes(20)?.try_into().unwrap())
    }
    pub fn guid(&mut self) -> Result<Option<u32>> {
        let g = self.u32()?;
        Ok((g != 0).then_some(g))
    }
    /// Validated element count: within `max_array` and `count * min_size <= remaining`.
    pub fn count(&mut self, min_size: usize) -> Result<usize> {
        let raw = self.u32()?;
        let n = raw as usize;
        if n > self.limits.max_array {
            return self.budget("array count");
        }
        match n.checked_mul(min_size.max(1)) {
            Some(b) if b <= self.remaining() => Ok(n),
            _ => self.invalid("array count exceeds remaining bytes"),
        }
    }
    /// Skips `n` big-endian f32 values.
    pub fn skip_f32(&mut self, n: usize) -> Result<()> {
        self.bytes(n * 4).map(|_| ())
    }
    /// Toolkit `str()`: s32 byte length then bytes. Returns the length.
    pub fn str8(&mut self) -> Result<usize> {
        let n = self.s32()?;
        let n = usize::try_from(n).or_else(|_| self.invalid("negative string length"))?;
        self.bytes(n)?;
        Ok(n)
    }
    /// Toolkit `str()` kept as text (lossy UTF-8, NULs removed).
    pub fn string8(&mut self) -> Result<String> {
        let n = self.s32()?;
        let n = usize::try_from(n).or_else(|_| self.invalid("negative string length"))?;
        Ok(String::from_utf8_lossy(self.bytes(n)?).replace('\0', ""))
    }
    /// Toolkit `wstr()` kept as text (UTF-16BE, lossy, NULs removed).
    pub fn wstring(&mut self) -> Result<String> {
        let n = self.s32()?;
        let n = usize::try_from(n).or_else(|_| self.invalid("negative string length"))?;
        let b = n.checked_mul(2).ok_or(Stop::Budget {
            what: "string length",
            offset: self.pos,
        })?;
        let units: Vec<u16> = self
            .bytes(b)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        Ok(String::from_utf16_lossy(&units).replace('\0', ""))
    }
    /// Toolkit `wstr()`: s32 UTF-16 unit count then 2 bytes per unit. Returns the count.
    pub fn wstr(&mut self) -> Result<usize> {
        let n = self.s32()?;
        let n = usize::try_from(n).or_else(|_| self.invalid("negative string length"))?;
        let b = n.checked_mul(2).ok_or(Stop::Budget {
            what: "string length",
            offset: self.pos,
        })?;
        self.bytes(b)?;
        Ok(n)
    }
    /// Toolkit `intarray` (unsigned): i32 count then one i32 per element.
    pub fn int_array(&mut self) -> Result<Vec<u32>> {
        let n = self.count(1)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.u32()?);
        }
        Ok(v)
    }
    /// Toolkit `intvector` (signed = false).
    pub fn int_vector(&mut self) -> Result<Vec<u32>> {
        if self.rev.compression & FLAG_VECTORS == 0 {
            return self.int_array();
        }
        let n = self.count(0)?;
        if n == 0 {
            return Ok(Vec::new());
        }
        let width = self.u8()? as usize;
        if width > 4 {
            return self.unsupported("intvector byte width above 4");
        }
        let total = match n.checked_mul(width) {
            Some(t) if t <= self.remaining() => t,
            _ => return self.invalid("intvector planes exceed remaining bytes"),
        };
        let planes = self.bytes(total)?;
        let mut v = vec![0u32; n];
        for (i, plane) in planes.chunks_exact(n.max(1)).enumerate() {
            for (dst, b) in v.iter_mut().zip(plane) {
                *dst |= u32::from(*b) << (8 * i);
            }
        }
        Ok(v)
    }
    /// Toolkit `resource(...)` without type field. `descriptor` skips the flags word.
    pub fn resource(&mut self, descriptor: bool) -> Result<Option<Descriptor>> {
        if self.rev.version() < 0x191 {
            return self.unsupported("resource descriptor below head 0x191");
        }
        let flags = if self.rev.version() > 0x22e && !descriptor {
            Some(self.u32()?)
        } else {
            None
        };
        let kind_byte = self.u8()?;
        if kind_byte == 0 {
            return Ok(None);
        }
        let guid = if kind_byte & 2 != 0 {
            Some(self.u32()?)
        } else {
            None
        };
        let sha1 = if kind_byte & 1 != 0 {
            Some(self.sha1()?)
        } else {
            None
        };
        Ok(Some(Descriptor {
            flags,
            kind_byte,
            guid,
            sha1,
        }))
    }
    /// Reads a reference tag. A `New` tag is registered as `obj` before returning.
    pub fn tag(
        &mut self,
        class: &'static str,
        new_obj: impl FnOnce(&mut Self) -> Result<Obj>,
    ) -> Result<Tag> {
        let t = self.u32()?;
        if t == 0 {
            return Ok(Tag::Null);
        }
        if let Some(&o) = self.tags.get(&t) {
            let ok = match o {
                Obj::Thing(_) => class == "Thing",
                Obj::Other(c, _) => c == class,
            };
            if !ok {
                return self.invalid(format!("reference tag {t} reused across classes ({class})"));
            }
            return Ok(Tag::Seen(o));
        }
        if self.tags.len() >= self.limits.max_tags {
            return self.budget("reference tags");
        }
        let o = new_obj(self)?;
        self.tags.insert(t, o);
        Ok(Tag::New(t))
    }
    /// Allocates a per-class sequence number for a non-Thing object.
    pub fn next_other(&mut self, class: &'static str) -> Obj {
        let c = self.other_counts.entry(class).or_insert(0);
        let o = Obj::Other(class, *c);
        *c += 1;
        o
    }
    pub fn other_count(&self, class: &str) -> usize {
        self.other_counts.get(class).copied().unwrap_or(0)
    }
    /// Enters one level of inline body nesting.
    pub fn enter(&mut self) -> Result<()> {
        if self.depth >= self.limits.max_depth {
            return self.budget("reference depth");
        }
        self.depth += 1;
        self.max_depth_seen = self.max_depth_seen.max(self.depth);
        Ok(())
    }
    pub fn leave(&mut self) {
        self.depth -= 1;
    }
}
