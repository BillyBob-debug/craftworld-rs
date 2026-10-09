//! D107: bounded, UI-independent RLevel/Thing reader for the LBP2 band
//! (0x341 <= head <= 0x3f8, branch 0, subVersion 0).
//!
//! Layers (docs/level-research.md items): `serial` = L3 serializer core, `thing` = L4
//! Thing header and part flags, `parts` = L5/L6 part readers and the PWorld tail, `level` =
//! RLevel entry (Lw), `item_details` = U5 InventoryItemDetails + RPlan details tail, `slots` = U4 slot/pin/palette/pack readers, `script` = U10 RScript reader + listing, `shape_mesh` = SHAPE
//! extrusion v1 (Lx, D114, renderer-independent), `bevel` = RBevel reader (Lv, D122) used
//! by the opt-in bevelled extrusion (Ly, D122).
//! Rules, evidence labels and limits: docs/d107-level-reader.md. Independent
//! implementation from specification notes; no community reader code copied.
pub mod bevel;
pub mod decal_mesh;
pub mod item_details;
pub mod level;
pub mod parts;
pub mod plan;
pub mod script;
pub mod serial;
pub mod shape_mesh;
pub mod slots;
pub mod thing;

pub use level::{read, read_payload, LevelReport};
pub use plan::{read_plan, read_plan_payload, PlanReport};
pub use serial::{Limits, Revision, Stop};
pub use shape_mesh::{
    extrude, extrude_bevelled, extrude_thing, extrude_thing_bevelled, ShapeMesh, ShapeReject,
};

/// Last LBP2 retail head (Toolkit `Revisions.LBP2_MAX`), shared with lbp-resource.
pub const LBP2_MAX: u32 = lbp_resource::LBP2_MAX_REVISION;
