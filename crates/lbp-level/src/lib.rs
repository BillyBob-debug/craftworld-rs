//! D107: bounded, UI-independent RLevel/Thing reader for the LBP2 band
//! (0x341 <= head <= 0x3f8, branch 0 or LEERDAMMER, subVersion 0).
//!
//! Layers (docs/level-research.md items): `serial` = L3 serializer core, `thing` = L4
//! Thing header and part flags, `parts` = L5 subset, `level` = RLevel entry.
//! Rules, evidence labels and limits: docs/d107-level-reader.md. Independent
//! implementation from specification notes; no community reader code copied.
pub mod level;
pub mod parts;
pub mod serial;
pub mod thing;

pub use level::{read, read_payload, LevelReport};
pub use serial::{Limits, Revision, Stop};

/// Last LBP2 retail head (Toolkit `Revisions.LBP2_MAX`), shared with lbp-resource.
pub const LBP2_MAX: u32 = lbp_resource::LBP2_MAX_REVISION;
