//! UI-independent identities, reader limits and errors. No game bytes are bundled.
use serde::{Serialize, Serializer};
use std::{fmt, io};

/// MAP GUIDs are unsigned 32-bit logical IDs, not archive offsets/content hashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guid(pub u32);

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}
impl Serialize for Guid {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Raw SHA-1 identifier from an archive/MAP table; no payload integrity implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 20]);

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    pub max_table_bytes: usize,
    pub max_entries: usize,
    pub max_path_bytes: usize,
}
impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_table_bytes: 64 * 1024 * 1024,
            max_entries: 1_000_000,
            max_path_bytes: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Io,
    Malformed,
    Unsupported,
    LimitExceeded,
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn malformed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Malformed, message)
    }
    pub fn limit(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::LimitExceeded, message)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::new(ErrorKind::Io, value.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// Checked bounded cursor shared by original-format readers.
pub struct ByteReader<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    pub fn position(&self) -> usize {
        self.position
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }
    pub fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| Error::malformed("length overflow"))?;
        let bytes = self.data.get(self.position..end).ok_or_else(|| {
            Error::malformed(format!(
                "truncated field at byte {} (need {length})",
                self.position
            ))
        })?;
        self.position = end;
        Ok(bytes)
    }
    pub fn be_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn be_i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn hash(&mut self) -> Result<ContentHash> {
        Ok(ContentHash(self.take(20)?.try_into().unwrap()))
    }
}
