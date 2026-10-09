//! B176: per-instance color census over read Things (renderer-independent, read-only).
//!
//! For every RENDER_MESH part: mesh descriptor and `editorColor` (PRenderMesh, packed ARGB
//! i32 in the LBP2 band; default -1 = opaque white). For every SHAPE part: `PShape.color`
//! keyed by the GENERATED_MESH gfx material (the material the D114 extrusion renders with)
//! and, separately, by the PShape physics `material` descriptor. Field layout:
//! CONFIRMED_FROM_COMMUNITY_CODE (docs/d80-thing-color.md E7; Toolkit `PRenderMesh.java`,
//! `PShape.java`). The link from these fields to the material THING_COLOR input is NOT
//! shown here; this module only counts stored values. docs/instance-color-census.md.
use crate::parts::ShapeColor;
use crate::serial::{hex, Descriptor};
use crate::thing::ThingRecord;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const SCOPE: &str = "placement colors from level Things; the link from editorColor to the material THING_COLOR input is CONFIRMED_FROM_COMMUNITY_CODE for the field only, not yet shown";

/// Packed opaque white (`-1` as i32), the PRenderMesh/PShape default.
pub const DEFAULT_WHITE: u32 = 0xFFFF_FFFF;

/// Stable key for a resource descriptor: `guid:XXXXXXXX` when a GUID is stored, else
/// `sha1:<hex>`; None when the descriptor is absent or empty.
pub fn descriptor_key(d: Option<&Descriptor>) -> Option<String> {
    let d = d?;
    match (d.guid, d.sha1) {
        (Some(g), _) => Some(format!("guid:{g:08x}")),
        (None, Some(h)) => Some(format!("sha1:{}", hex(&h))),
        _ => None,
    }
}

/// Packed ARGB as 8 lower-case hex digits.
pub fn packed_key(c: u32) -> String {
    format!("{c:08x}")
}

/// PShape color key: packed hex, or `v4:r,g,b,a` for the pre-0x38a float layout.
pub fn shape_color_key(c: &ShapeColor) -> String {
    match c {
        ShapeColor::Packed(p) => packed_key(*p),
        ShapeColor::Vector(v) => format!("v4:{},{},{},{}", v[0], v[1], v[2], v[3]),
    }
}

#[derive(Debug, Default)]
struct ColorTally {
    things: usize,
    files: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct KeyTally {
    things: usize,
    files: BTreeSet<String>,
    colors: BTreeMap<String, ColorTally>,
}

/// Histogram of color keys per resource key; counting unit: one Thing per containing file.
#[derive(Debug, Default)]
pub struct Histogram {
    keys: BTreeMap<String, KeyTally>,
    /// Parts with no usable descriptor (counted, not keyed).
    pub without_key: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ColorSummary {
    pub things: usize,
    pub files: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct KeySummary {
    pub things: usize,
    pub files: usize,
    pub distinct_colors: usize,
    pub colors: BTreeMap<String, ColorSummary>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Totals {
    pub keys: usize,
    pub things: usize,
    pub things_without_key: usize,
    pub things_default_white: usize,
    pub distinct_colors: usize,
    pub keys_single_color: usize,
    pub keys_only_default_white: usize,
    pub keys_multi_color: usize,
}

impl Histogram {
    pub fn add(&mut self, file: &str, key: Option<String>, color: String) {
        let Some(key) = key else {
            self.without_key += 1;
            return;
        };
        let k = self.keys.entry(key).or_default();
        k.things += 1;
        k.files.insert(file.to_string());
        let c = k.colors.entry(color).or_default();
        c.things += 1;
        c.files.insert(file.to_string());
    }

    pub fn keys(&self) -> BTreeMap<String, KeySummary> {
        self.keys
            .iter()
            .map(|(k, t)| {
                let colors = t
                    .colors
                    .iter()
                    .map(|(c, ct)| {
                        (
                            c.clone(),
                            ColorSummary {
                                things: ct.things,
                                files: ct.files.len(),
                            },
                        )
                    })
                    .collect();
                (
                    k.clone(),
                    KeySummary {
                        things: t.things,
                        files: t.files.len(),
                        distinct_colors: t.colors.len(),
                        colors,
                    },
                )
            })
            .collect()
    }

    pub fn totals(&self) -> Totals {
        let white = packed_key(DEFAULT_WHITE);
        let distinct: BTreeSet<&String> =
            self.keys.values().flat_map(|k| k.colors.keys()).collect();
        Totals {
            keys: self.keys.len(),
            things: self.keys.values().map(|k| k.things).sum::<usize>() + self.without_key,
            things_without_key: self.without_key,
            things_default_white: self
                .keys
                .values()
                .filter_map(|k| k.colors.get(&white))
                .map(|c| c.things)
                .sum(),
            distinct_colors: distinct.len(),
            keys_single_color: self.keys.values().filter(|k| k.colors.len() == 1).count(),
            keys_only_default_white: self
                .keys
                .values()
                .filter(|k| k.colors.len() == 1 && k.colors.contains_key(&white))
                .count(),
            keys_multi_color: self.keys.values().filter(|k| k.colors.len() > 1).count(),
        }
    }
}

/// The three histograms of one census.
#[derive(Debug, Default)]
pub struct InstanceColors {
    /// RENDER_MESH: mesh descriptor -> editorColor.
    pub meshes: Histogram,
    /// SHAPE: GENERATED_MESH gfx material -> PShape.color.
    pub shapes_by_gfx_material: Histogram,
    /// SHAPE: PShape physics material -> PShape.color.
    pub shapes_by_physics_material: Histogram,
}

impl InstanceColors {
    /// Adds every read RENDER_MESH and SHAPE part of `things`, attributed to `file`.
    pub fn add_things(&mut self, file: &str, things: &[ThingRecord]) {
        for t in things {
            if let Some(rm) = &t.data.render_mesh {
                self.meshes.add(
                    file,
                    descriptor_key(rm.mesh.as_ref()),
                    packed_key(rm.editor_color),
                );
            }
            if let Some(s) = &t.data.shape {
                let color = shape_color_key(&s.color);
                let gfx = t
                    .data
                    .generated_mesh
                    .as_ref()
                    .and_then(|g| g.gfx_material.as_ref());
                self.shapes_by_gfx_material
                    .add(file, descriptor_key(gfx), color.clone());
                self.shapes_by_physics_material.add(
                    file,
                    descriptor_key(s.material.as_ref()),
                    color,
                );
            }
        }
    }
}
