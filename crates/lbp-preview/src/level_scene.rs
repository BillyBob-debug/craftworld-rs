//! D113 (level-research L2/L5 subset): renderer-independent level scene.
//!
//! Reads one level with `lbp-level` and turns every Thing that has a RENDER_MESH into a
//! scene item: (shared mesh payload, world matrix). Meshes load once per mesh reference
//! through the same bounded single-mesh path (`load_mesh_groups_with_options`) and are
//! reused by every item that names them. Things that cannot be drawn are skipped and
//! counted by reason. Placement is the Thing's own POS world matrix (static bind pose):
//! bones, animation, decorations, stickers and material overrides are not applied.
//!
//! D120: RENDER_MESH Things without POS that carry a JOINT (bolts, strings, pistons...)
//! are placed from the joint's two endpoint Things (`joint_world`, preview
//! approximation, see docs/d120-bone-props.md). Every other Thing without POS stays
//! skipped with its reason.
//!
//! SHAPE Things with a visible GENERATED_MESH are extruded by D114
//! (`lbp_level::shape_mesh`, world-space geometry, preview approximation) and added by
//! `add_shapes`: shapes sharing one gfx material are merged into world-space batches
//! (`MeshSource::Shapes`, identity item matrix) so each material loads once per batch
//! through the same diffuse route as model meshes, with the generated UV0 as input.
//! The renderer consumes only `meshes` + `items`.
//!
//! No GUI or GPU types. The level reader recurses natively: call `load_level_scene` on
//! a thread with at least `LEVEL_STACK_BYTES` of stack.
use crate::{load_mesh_groups_with_options, resolve, GroupedMesh, MaterialAsset, PreviewOptions};
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_level::shape_mesh::{self, ShapeMesh};
use lbp_level::thing::ThingRecord;
use std::collections::{BTreeMap, HashMap};

/// D114 shape label, re-exported so renderers need not depend on `lbp-level`.
pub const SHAPE_PREVIEW_LABEL: &str = shape_mesh::PREVIEW_LABEL;
/// Worker stack for `load_level_scene` (the D107 examples use the same size).
pub const LEVEL_STACK_BYTES: usize = 512 * 1024 * 1024;
/// Distinct mesh references loaded per level.
pub const MAX_LEVEL_MESHES: usize = 1024;
/// Drawn items per level.
pub const MAX_LEVEL_ITEMS: usize = 100_000;
/// Aggregate retained material bytes over all loaded meshes of one level. Each mesh keeps
/// its own D66 limits (64 material loads, 640 MiB); this caps the sum.
pub const MAX_LEVEL_RETAINED_BYTES: usize = 1536 * 1024 * 1024;
/// Shape Things extruded per level and output vertices over all shapes.
pub const MAX_LEVEL_SHAPES: usize = 50_000;
pub const MAX_LEVEL_SHAPE_VERTICES: usize = 8_000_000;
/// Per shape batch: the display adapter's per-mesh limits (positions / triangles).
pub const MAX_BATCH_VERTICES: usize = 100_000;
pub const MAX_BATCH_TRIANGLES: usize = 50_000;
const PAGE: usize = 4096;

/// Where a scene mesh comes from. Shapes (D114) will add a variant here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshSource {
    /// Model mesh named by GUID in a RENDER_MESH descriptor.
    Guid(u32),
    /// Model mesh named only by SHA-1 (lowercase hex).
    Sha1(String),
    /// Merged world-space shape extrusions sharing one gfx material GUID (None = no
    /// GUID); `batch` numbers the chunks of that material.
    Shapes { material: Option<u32>, batch: usize },
}

impl MeshSource {
    pub fn label(&self) -> String {
        match self {
            Self::Guid(g) => format!("{g:08x}"),
            Self::Sha1(h) => format!("sha1 {h}"),
            Self::Shapes { material, batch } => format!(
                "shapes gfx {} batch {batch}",
                material.map_or("none".into(), |g| format!("{g:08x}"))
            ),
        }
    }
}

pub struct SceneMesh<T> {
    pub source: MeshSource,
    pub payload: T,
    /// Local axis-aligned bounds of the payload geometry (None when empty/nonfinite).
    pub bounds: Option<[[f32; 3]; 2]>,
    /// True when at least one non-empty primitive group has no supported material
    /// (the renderer shows it in the neutral diagnostic colour, as the single-mesh view).
    pub neutral_groups: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    /// Index into `LevelScene::meshes`.
    pub mesh: usize,
    /// Column-major world matrix (translation in elements 12..14), as stored in POS.
    pub world: [f32; 16],
    /// Thing index in reader (tag) order and its UID.
    pub thing: usize,
    pub uid: Option<i32>,
    /// Shape Things merged into this item (0 for a model item).
    pub shapes: usize,
}

/// Merged world-space shape geometry for one gfx material (D114 extrusions).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShapeBatch {
    pub material: Option<u32>,
    pub positions: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Thing indices of the merged shapes.
    pub things: Vec<usize>,
}

/// Counting units: Things (tag-table entries), items (= drawn Things), distinct mesh
/// references. Skip reasons count Things.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneCounts {
    pub things_total: usize,
    pub with_render_mesh: usize,
    pub drawn: usize,
    pub skipped: BTreeMap<String, usize>,
    /// Drawn Things whose POS names another Thing as bone owner (placed by own POS).
    pub drawn_bone_attached: usize,
    /// D120: drawn JOINT Things without POS, placed at contact A (angular joint types).
    pub drawn_joint_pivot: usize,
    /// D120: drawn JOINT Things without POS, fitted between contacts A and B (linear).
    pub drawn_joint_span: usize,
    /// Drawn Things whose mesh has neutral (unsupported material) groups.
    pub drawn_with_neutral_groups: usize,
    pub meshes_loaded: usize,
    pub meshes_failed: usize,
    /// Shape Things (SHAPE part present) and their extrusion outcomes.
    pub shapes_total: usize,
    pub shapes_extruded: usize,
    pub shapes_placed: usize,
    /// Selection outcomes (no GENERATED_MESH, edit-only, no POS): not drawn by design.
    pub shapes_not_selected: BTreeMap<String, usize>,
    /// Geometry rejections from the extrusion (by stable reason key) and budgets.
    pub shapes_rejected: BTreeMap<String, usize>,
    /// Placed shapes whose gfx material did not select (flat neutral colour), by reason.
    pub shapes_material_fallback: BTreeMap<String, usize>,
    pub shape_batches: usize,
    /// DECORATIONS entries (not drawn).
    pub decorations_not_drawn: usize,
    /// Reader stop (None = whole level read).
    pub reader_stop: Option<String>,
}

impl SceneCounts {
    pub fn skipped_total(&self) -> usize {
        self.skipped.values().sum()
    }
    /// Display / QA-log lines with explicit units.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            format!("Things total: {} (tag-table Things)", self.things_total),
            format!("Things with RENDER_MESH: {}", self.with_render_mesh),
            format!("Placed (scene items): {} Things", self.drawn),
            format!("Skipped: {} Things with RENDER_MESH", self.skipped_total()),
        ];
        for (reason, n) in &self.skipped {
            out.push(format!("  skipped {n}: {reason}"));
        }
        out.push(format!(
            "Distinct meshes: {} loaded, {} failed (mesh references)",
            self.meshes_loaded, self.meshes_failed
        ));
        out.push(format!(
            "Drawn with neutral (unsupported material) groups: {} Things",
            self.drawn_with_neutral_groups
        ));
        out.push(format!(
            "Drawn bone-attached at own POS (approximation): {} Things",
            self.drawn_bone_attached
        ));
        out.push(format!(
            "Placed from JOINT endpoints (D120 preview approximation): {} Things ({} pivot at contact A, {} span fitted A to B; no skinning, angle/renderScale not applied)",
            self.drawn_joint_pivot + self.drawn_joint_span,
            self.drawn_joint_pivot,
            self.drawn_joint_span
        ));
        out.push(format!(
            "Shapes: {} SHAPE Things; {} extruded; {} placed in {} material batches",
            self.shapes_total, self.shapes_extruded, self.shapes_placed, self.shape_batches
        ));
        for (reason, n) in &self.shapes_not_selected {
            out.push(format!("  shape not selected {n}: {reason}"));
        }
        for (reason, n) in &self.shapes_rejected {
            out.push(format!("  shape rejected {n}: {reason}"));
        }
        let fallback: usize = self.shapes_material_fallback.values().sum();
        out.push(format!(
            "Shapes with flat fallback colour (gfx material not selected): {fallback} Things"
        ));
        for (reason, n) in &self.shapes_material_fallback {
            out.push(format!("  fallback {n}: {reason}"));
        }
        if self.shapes_placed > 0 {
            out.push(format!("Shapes are a {}", shape_mesh::PREVIEW_LABEL));
        }
        out.push(format!(
            "Not drawn: {} decorations (DECORATIONS entries)",
            self.decorations_not_drawn
        ));
        out.push(match &self.reader_stop {
            None => "Level reader: complete".into(),
            Some(s) => format!("Level reader stopped: {s}"),
        });
        out
    }
}

pub struct LevelScene<T> {
    pub meshes: Vec<SceneMesh<T>>,
    pub items: Vec<SceneItem>,
    pub counts: SceneCounts,
}

impl<T> Default for LevelScene<T> {
    fn default() -> Self {
        Self {
            meshes: Vec::new(),
            items: Vec::new(),
            counts: SceneCounts::default(),
        }
    }
}

impl<T> LevelScene<T> {
    /// Adds a drawable item; extension point for later mesh sources (shapes).
    pub fn push_item(&mut self, item: SceneItem) -> Result<(), String> {
        if item.mesh >= self.meshes.len() {
            return Err("Invalid: scene item mesh index".into());
        }
        if self.items.len() >= MAX_LEVEL_ITEMS {
            return Err("Unsupported: level item budget".into());
        }
        world_matrix_check(&item.world)?;
        self.items.push(item);
        Ok(())
    }

    /// World-space bounds of all items (mesh local bounds through each world matrix).
    pub fn bounds(&self) -> Option<[[f32; 3]; 2]> {
        self.bounds_where(|_| true)
    }

    /// World-space bounds of the items `keep` accepts (e.g. those a renderer displays).
    pub fn bounds_where(&self, keep: impl Fn(&SceneItem) -> bool) -> Option<[[f32; 3]; 2]> {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for item in self.items.iter().filter(|i| keep(i)) {
            let Some([a, b]) = self.meshes[item.mesh].bounds else {
                continue;
            };
            for corner in 0..8 {
                let p = [
                    if corner & 1 == 0 { a[0] } else { b[0] },
                    if corner & 2 == 0 { a[1] } else { b[1] },
                    if corner & 4 == 0 { a[2] } else { b[2] },
                ];
                let w = transform_point(&item.world, p);
                for i in 0..3 {
                    lo[i] = lo[i].min(w[i]);
                    hi[i] = hi[i].max(w[i]);
                }
            }
        }
        (lo.iter().chain(&hi).all(|v| v.is_finite()))
            .then(|| [lo.map(|v| v as f32), hi.map(|v| v as f32)])
    }
}

/// Column-major point transform in f64.
pub fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f64; 3] {
    let m = m.map(f64::from);
    let [x, y, z] = p.map(f64::from);
    std::array::from_fn(|r| m[r] * x + m[4 + r] * y + m[8 + r] * z + m[12 + r])
}

/// Finite, affine (last row 0,0,0,1) and invertible upper 3x3.
pub fn world_matrix_check(m: &[f32; 16]) -> Result<(), String> {
    if m.iter().any(|v| !v.is_finite()) {
        return Err("non-finite world matrix".into());
    }
    let tol = 1e-4;
    if m[3].abs() > tol || m[7].abs() > tol || m[11].abs() > tol || (m[15] - 1.).abs() > tol {
        return Err("projective world matrix (last row not 0,0,0,1)".into());
    }
    let a = m.map(f64::from);
    let det = a[0] * (a[5] * a[10] - a[9] * a[6]) - a[4] * (a[1] * a[10] - a[9] * a[2])
        + a[8] * (a[1] * a[6] - a[5] * a[2]);
    if det.is_nan() || det.abs() <= 1e-12 {
        return Err("non-invertible world matrix".into());
    }
    Ok(())
}

/// How a drawn RENDER_MESH Thing is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointPlacement {
    /// Angular joint (BOLT, SPRING_ANGULAR, MOTOR, QUANTIZED): bone 0 at contact A.
    Pivot,
    /// Linear joint (ELASTIC, SPRING, CHAIN, PISTON, STRING, ROD): fitted A to B.
    Span,
}

/// Joint type classes (PJoint.type, Toolkit EJointType comment; INFERRED for the
/// angular/linear split, which follows the comment's "start of angular" marker).
pub fn joint_placement_kind(joint_type: i32) -> Option<JointPlacement> {
    match joint_type {
        1..=6 => Some(JointPlacement::Span),
        7..=10 => Some(JointPlacement::Pivot),
        _ => None,
    }
}

type M3 = [[f64; 3]; 3];

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn scaled(a: [f64; 3], k: f64) -> [f64; 3] {
    a.map(|v| v * k)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot(a, a).sqrt();
    (n.is_finite() && n > 1e-6).then(|| scaled(a, 1. / n))
}
/// Column vectors -> matrix applying them (m[col][row]).
fn mul_mv(m: &M3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|r| m[0][r] * v[0] + m[1][r] * v[1] + m[2][r] * v[2])
}
fn mul_mm(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|c| mul_mv(a, b[c]))
}
fn transpose(m: &M3) -> M3 {
    std::array::from_fn(|c| std::array::from_fn(|r| m[r][c]))
}
/// Orthonormal basis (columns) whose first axis is `x` and whose second axis is world
/// +Z (the layer depth axis) made orthogonal to `x` (+Y when `x` is nearly along Z).
fn frame(x: [f64; 3]) -> Option<M3> {
    let reference = if x[2].abs() > 0.99 {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let y = unit(sub(reference, scaled(x, dot(reference, x))))?;
    Some([x, y, cross(x, y)])
}

/// D120 joint placement (INFERRED_FROM_LOCAL_DATA, preview approximation; rules and
/// evidence in docs/d120-bone-props.md). Contact points: pA = world(a) x aContact,
/// pB = world(b) x bContact (contacts are in each endpoint's local frame). The joint
/// mesh's bone-0 bind rotation is undone (bone axes aligned with world axes, bone local
/// Z = layer depth), and:
/// - Pivot: bone 0's bind position lands on pA.
/// - Span: the mesh's extent along its bind bone0->bone1 axis is fitted onto pA..pB,
///   rotated so that axis follows pA->pB (cross-section rigid, local Z kept nearest
///   world Z). Static: no skinning, so end pieces stretch with the span.
///
/// `bind` = bone skin-pose matrices of the joint mesh (column-major, bone order);
/// `bounds` = model-space bounds of the mesh.
pub fn joint_world(
    things: &[ThingRecord],
    joint: &lbp_level::parts::Joint,
    bind: &[[f32; 16]],
    bounds: Option<[[f32; 3]; 2]>,
) -> Result<([f32; 16], JointPlacement), String> {
    let kind = joint_placement_kind(joint.joint_type).ok_or_else(|| {
        format!(
            "JOINT type {} without POS (placement not implemented)",
            joint.joint_type
        )
    })?;
    let endpoint = |e: Option<usize>, contact: [f32; 3]| -> Result<[f64; 3], String> {
        let pos = e
            .and_then(|i| things.get(i))
            .and_then(|t| t.data.pos.as_ref())
            .ok_or("JOINT endpoint without POS")?;
        world_matrix_check(&pos.world).map_err(|e| format!("JOINT endpoint {e}"))?;
        let p = transform_point(&pos.world, contact);
        p.iter()
            .all(|v| v.is_finite())
            .then_some(p)
            .ok_or_else(|| "JOINT contact not finite".to_string())
    };
    let pa = endpoint(joint.a, joint.a_contact)?;
    let pb = endpoint(joint.b, joint.b_contact)?;
    let skin0 = bind
        .first()
        .ok_or("JOINT mesh without bones")?
        .map(f64::from);
    let t0 = [skin0[12], skin0[13], skin0[14]];
    // Bone-0 bind rotation: skin-pose columns normalized; must be orthonormal.
    let mut p0: M3 = [[0.; 3]; 3];
    for (c, col) in p0.iter_mut().enumerate() {
        *col = unit([skin0[4 * c], skin0[4 * c + 1], skin0[4 * c + 2]])
            .ok_or("JOINT mesh bone 0 bind pose degenerate")?;
    }
    if (0..3).any(|i| (i + 1..3).any(|j| dot(p0[i], p0[j]).abs() > 1e-3)) {
        return Err("JOINT mesh bone 0 bind pose not a scaled rotation".into());
    }
    let p0_inv = transpose(&p0);
    let (linear, translation) = match kind {
        JointPlacement::Pivot => (p0_inv, sub(pa, mul_mv(&p0_inv, t0))),
        JointPlacement::Span => {
            let skin1 = bind
                .get(1)
                .ok_or("span JOINT mesh with fewer than 2 bones")?
                .map(f64::from);
            let t1 = [skin1[12], skin1[13], skin1[14]];
            let axis = unit(mul_mv(&p0_inv, sub(t1, t0)))
                .ok_or("JOINT mesh bones coincide in bind pose")?;
            let d = sub(pb, pa);
            let length = dot(d, d).sqrt();
            let dir = unit(d).ok_or("span JOINT contacts coincide")?;
            let [lo_b, hi_b] = bounds.ok_or("JOINT mesh without bounds")?;
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for corner in 0..8 {
                let v = [
                    if corner & 1 == 0 { lo_b[0] } else { hi_b[0] },
                    if corner & 2 == 0 { lo_b[1] } else { hi_b[1] },
                    if corner & 4 == 0 { lo_b[2] } else { hi_b[2] },
                ]
                .map(f64::from);
                let s = dot(mul_mv(&p0_inv, sub(v, t0)), axis);
                lo = lo.min(s);
                hi = hi.max(s);
            }
            if (hi - lo).is_nan() || hi - lo <= 1e-6 {
                return Err("JOINT mesh has no extent along its bone axis".into());
            }
            let k = length / (hi - lo);
            let (Some(f), Some(w)) = (frame(axis), frame(dir)) else {
                return Err("JOINT frame degenerate".into());
            };
            let rot = mul_mm(&w, &transpose(&f));
            // Stretch along `axis` by k: I + (k - 1) axis axis^T.
            let stretch: M3 = std::array::from_fn(|c| {
                std::array::from_fn(|r| f64::from(u8::from(r == c)) + (k - 1.) * axis[r] * axis[c])
            });
            let linear = mul_mm(&rot, &mul_mm(&stretch, &p0_inv));
            let translation = sub(sub(pa, mul_mv(&linear, t0)), scaled(dir, k * lo));
            (linear, translation)
        }
    };
    let mut m = [0f32; 16];
    for c in 0..3 {
        for r in 0..3 {
            m[4 * c + r] = linear[c][r] as f32;
        }
    }
    for r in 0..3 {
        m[12 + r] = translation[r] as f32;
    }
    m[15] = 1.;
    world_matrix_check(&m).map_err(|e| format!("JOINT placement {e}"))?;
    Ok((m, kind))
}

/// Builds the scene from reader Things. `load` is called once per distinct mesh
/// reference; failures are cached and count every Thing that names that mesh.
/// `bind_poses` returns a loaded mesh's bone skin-pose matrices (D120 joint placement).
pub fn build_scene<T>(
    things: &[ThingRecord],
    mut load: impl FnMut(&MeshSource) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
    bind_poses: impl Fn(&T) -> Vec<[f32; 16]>,
) -> LevelScene<T> {
    let mut scene = LevelScene::<T>::default();
    let mut by_source: HashMap<MeshSource, Result<usize, String>> = HashMap::new();
    let c = &mut scene.counts;
    c.things_total = things.len();
    let skip = |c: &mut SceneCounts, reason: String| *c.skipped.entry(reason).or_default() += 1;
    for t in things {
        let c = &mut scene.counts;
        c.decorations_not_drawn += t.data.decorations.len();
        let Some(rm) = &t.data.render_mesh else {
            continue;
        };
        c.with_render_mesh += 1;
        let source = match &rm.mesh {
            Some(d) => match (d.guid, d.sha1) {
                (Some(g), _) if g != 0 => MeshSource::Guid(g),
                (_, Some(h)) if h.iter().any(|&b| b != 0) => {
                    MeshSource::Sha1(h.iter().map(|b| format!("{b:02x}")).collect())
                }
                _ => {
                    skip(c, "RENDER_MESH with null mesh reference".into());
                    continue;
                }
            },
            None => {
                skip(c, "RENDER_MESH without mesh reference".into());
                continue;
            }
        };
        let pos = match (&t.data.pos, &t.data.joint) {
            (Some(pos), _) => {
                if let Err(e) = world_matrix_check(&pos.world) {
                    skip(c, e);
                    continue;
                }
                Some(pos)
            }
            (None, Some(_)) => None,
            (None, None) => {
                skip(c, "no POS and no JOINT (no placement source)".into());
                continue;
            }
        };
        let index = match by_source.get(&source) {
            Some(r) => r.clone(),
            None => {
                let r = if scene.meshes.len() >= MAX_LEVEL_MESHES {
                    Err("Unsupported: level distinct-mesh budget".to_string())
                } else {
                    load(&source).map(|payload| {
                        scene.meshes.push(SceneMesh {
                            source: source.clone(),
                            bounds: local_bounds(&payload),
                            neutral_groups: neutral_groups(&payload),
                            payload,
                        });
                        scene.meshes.len() - 1
                    })
                };
                let c = &mut scene.counts;
                if r.is_ok() {
                    c.meshes_loaded += 1;
                } else {
                    c.meshes_failed += 1;
                }
                by_source.insert(source.clone(), r.clone());
                r
            }
        };
        let c = &mut scene.counts;
        let mesh = match index {
            Ok(i) => i,
            Err(e) => {
                skip(c, format!("mesh unavailable: {e}"));
                continue;
            }
        };
        if scene.items.len() >= MAX_LEVEL_ITEMS {
            skip(c, "Unsupported: level item budget".into());
            continue;
        }
        let world = match (pos, &t.data.joint) {
            (Some(pos), _) => {
                c.drawn_bone_attached += usize::from(pos.bone_of.is_some_and(|b| b != t.index));
                pos.world
            }
            (None, Some(joint)) => {
                let m = &scene.meshes[mesh];
                match joint_world(things, joint, &bind_poses(&m.payload), m.bounds) {
                    Ok((world, kind)) => {
                        match kind {
                            JointPlacement::Pivot => c.drawn_joint_pivot += 1,
                            JointPlacement::Span => c.drawn_joint_span += 1,
                        }
                        world
                    }
                    Err(e) => {
                        skip(c, e);
                        continue;
                    }
                }
            }
            (None, None) => unreachable!("skipped above"),
        };
        c.drawn += 1;
        c.drawn_with_neutral_groups += usize::from(scene.meshes[mesh].neutral_groups);
        scene.items.push(SceneItem {
            mesh,
            world,
            thing: t.index,
            uid: t.uid,
            shapes: 0,
        });
    }
    scene
}

/// Extrudes every SHAPE Thing (D114 selection rules) and adds the results as merged
/// world-space batches per gfx material, in material-GUID order then Thing order.
/// `wrap` turns a batch into the payload and reports whether its material selected.
pub fn add_shapes<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    mut wrap: impl FnMut(ShapeBatch) -> (T, Result<(), String>),
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
) {
    let mut groups: BTreeMap<Option<u32>, Vec<(usize, ShapeMesh)>> = BTreeMap::new();
    let mut vertices = 0usize;
    for t in things.iter().filter(|t| t.data.shape.is_some()) {
        let c = &mut scene.counts;
        c.shapes_total += 1;
        if c.shapes_extruded >= MAX_LEVEL_SHAPES {
            *c.shapes_rejected
                .entry("level_shape_budget".into())
                .or_default() += 1;
            continue;
        }
        match shape_mesh::extrude_thing(t) {
            Ok(m) => {
                if vertices + m.positions.len() > MAX_LEVEL_SHAPE_VERTICES
                    || m.positions.len() > MAX_BATCH_VERTICES
                    || m.triangle_count() > MAX_BATCH_TRIANGLES
                {
                    *c.shapes_rejected
                        .entry("level_shape_vertex_budget".into())
                        .or_default() += 1;
                    continue;
                }
                vertices += m.positions.len();
                c.shapes_extruded += 1;
                groups
                    .entry(m.material_guid())
                    .or_default()
                    .push((t.index, m));
            }
            Err(r) if r.is_selection() => {
                *c.shapes_not_selected.entry(r.reason().into()).or_default() += 1;
            }
            Err(r) => *c.shapes_rejected.entry(r.reason().into()).or_default() += 1,
        }
    }
    for (material, shapes) in groups {
        let mut batches = vec![ShapeBatch {
            material,
            ..Default::default()
        }];
        for (thing, m) in shapes {
            let current = batches.last_mut().expect("one batch");
            if current.positions.len() + m.positions.len() > MAX_BATCH_VERTICES
                || current.triangles.len() + m.triangle_count() > MAX_BATCH_TRIANGLES
            {
                batches.push(ShapeBatch {
                    material,
                    ..Default::default()
                });
            }
            let b = batches.last_mut().expect("one batch");
            let base = b.positions.len() as u32;
            b.positions.extend_from_slice(&m.positions);
            b.uv0.extend_from_slice(&m.uv0);
            b.triangles.extend(
                m.indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| t.map(|i| i + base)),
            );
            b.things.push(thing);
        }
        for (batch_index, batch) in batches.into_iter().enumerate() {
            let n = batch.things.len();
            let first = batch.things.first().copied().unwrap_or_default();
            let (payload, selected) = wrap(batch);
            let c = &mut scene.counts;
            if let Err(e) = &selected {
                *c.shapes_material_fallback.entry(e.clone()).or_default() += n;
            }
            if scene.items.len() >= MAX_LEVEL_ITEMS {
                *c.shapes_rejected
                    .entry("level_item_budget".into())
                    .or_default() += n;
                continue;
            }
            c.shapes_placed += n;
            c.shape_batches += 1;
            scene.meshes.push(SceneMesh {
                source: MeshSource::Shapes {
                    material,
                    batch: batch_index,
                },
                bounds: local_bounds(&payload),
                neutral_groups: selected.is_err(),
                payload,
            });
            scene.items.push(SceneItem {
                mesh: scene.meshes.len() - 1,
                world: shape_mesh::IDENTITY,
                thing: first,
                uid: None,
                shapes: n,
            });
        }
    }
}

/// Real-data payload: a model mesh or a merged shape batch with its material outcome.
pub enum ScenePayload {
    Model(Box<GroupedMesh>),
    Shapes {
        batch: ShapeBatch,
        material: Result<std::sync::Arc<MaterialAsset>, String>,
    },
}

/// Resolves a level by GUID or by MAP path (exact, or a `/`-suffix of the stored path,
/// case-insensitive). Returns the location and the matching MAP path(s).
pub fn resolve_level(
    catalog: &Catalog,
    guid: Option<u32>,
    path: Option<&str>,
) -> Result<(LocationKey, String), String> {
    let mut guids = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    let wanted = path.map(|p| p.trim().replace('\\', "/").to_ascii_lowercase());
    for r in &catalog.snapshot.resources {
        let row_path = r.map_row.path.replace('\\', "/").to_ascii_lowercase();
        let row_guid = u32::from_str_radix(r.map_row.guid.trim(), 16).ok();
        let hit = match (&wanted, guid) {
            (Some(w), _) => row_path == *w || row_path.ends_with(&format!("/{w}")),
            (None, Some(g)) => row_guid == Some(g),
            (None, None) => return Err("Level GUID or path required".into()),
        };
        if hit {
            if r.display_type() != "level" {
                return Err(format!(
                    "{} is a {} resource, not a magic-confirmed level",
                    r.map_row.path,
                    r.display_type()
                ));
            }
            paths.insert(r.map_row.path.clone());
            if let Some(g) = row_guid {
                guids.insert(g);
            }
        }
    }
    if paths.is_empty() {
        return Err("Unresolved: no level MAP row matches".into());
    }
    if guids.len() != 1 {
        return Err(format!(
            "Ambiguous level: {} GUIDs match ({})",
            guids.len(),
            paths.into_iter().collect::<Vec<_>>().join(" | ")
        ));
    }
    let key = resolve(catalog, guids.first().copied(), None)?;
    Ok((key, paths.into_iter().collect::<Vec<_>>().join(" | ")))
}

/// Reads a level's stored bytes (bounded by the level input budget).
pub fn read_level_bytes(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > lbp_level::level::MAX_INPUT {
        return Err("Unsupported: stored level exceeds the level input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(
            key,
            bytes.len() as u64,
            (size - bytes.len()).min(PAGE),
            control,
        )?;
        if page.bytes.is_empty() {
            return Err("Invalid: truncated level read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

fn grouped_bounds(g: &GroupedMesh) -> Option<[[f32; 3]; 2]> {
    points_bounds(&g.geometry.positions)
}

fn payload_bounds(p: &ScenePayload) -> Option<[[f32; 3]; 2]> {
    match p {
        ScenePayload::Model(g) => grouped_bounds(g),
        ScenePayload::Shapes { batch, .. } => points_bounds(&batch.positions),
    }
}

fn points_bounds(points: &[[f32; 3]]) -> Option<[[f32; 3]; 2]> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in points {
        if p.iter().any(|v| !v.is_finite()) {
            return None;
        }
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo[0] <= hi[0]).then_some([lo, hi])
}

fn grouped_neutral(g: &GroupedMesh) -> bool {
    g.geometry
        .primitive_triangles
        .iter()
        .zip(&g.bindings)
        .any(|(range, b)| !range.is_empty() && b.is_err())
}

fn retained(g: &GroupedMesh) -> usize {
    let mut seen = Vec::new();
    let mut total = 0usize;
    for b in g.bindings.iter().flatten() {
        if !seen.iter().any(|s| std::sync::Arc::ptr_eq(s, b)) {
            seen.push(b.clone());
            total = total.saturating_add(b.retained_bytes());
        }
    }
    total
}

/// Reads the level and loads every referenced mesh. Run on a `LEVEL_STACK_BYTES` stack.
pub fn load_level_scene(
    source: &AttachedSource,
    catalog: &Catalog,
    level: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
) -> Result<LevelScene<ScenePayload>, String> {
    let bytes = read_level_bytes(source, level, control)?;
    // Default depth bound (256) fits the 512 MiB stack; deepest local level is 162.
    let limits = lbp_level::Limits::default();
    let report = lbp_level::read(&bytes, limits, &|| control.check().is_err())
        .map_err(|e| format!("Level wrapper: {e}"))?;
    control.check()?;
    if report.things.is_empty() {
        return Err(format!(
            "Level not read: {}",
            report
                .stop
                .as_ref()
                .map_or("no Things".into(), |s| format!("{s:?}"))
        ));
    }
    let mut retained_total = 0usize;
    let mut charge = |bytes: usize| -> Result<(), String> {
        retained_total = retained_total
            .checked_add(bytes)
            .filter(|&t| t <= MAX_LEVEL_RETAINED_BYTES)
            .ok_or("Unsupported: level material memory budget (1536 MiB retained)")?;
        Ok(())
    };
    let mut scene = build_scene(
        &report.things,
        |src| {
            control.check()?;
            let key = match src {
                MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                MeshSource::Shapes { .. } => return Err("Invalid: shape source".into()),
            };
            let mesh = load_mesh_groups_with_options(source, catalog, &key, control, options)?;
            charge(retained(&mesh))?;
            Ok(ScenePayload::Model(Box::new(mesh)))
        },
        payload_bounds,
        |p| matches!(p, ScenePayload::Model(m) if grouped_neutral(m)),
        |p| match p {
            ScenePayload::Model(m) => m
                .geometry
                .skeleton
                .bones
                .iter()
                .map(|b| b.skin_pose)
                .collect(),
            ScenePayload::Shapes { .. } => Vec::new(),
        },
    );
    control.check()?;
    add_shapes(
        &mut scene,
        &report.things,
        |batch| {
            let material = (|| {
                control.check()?;
                let guid = batch
                    .material
                    .ok_or("Unsupported: GENERATED_MESH without gfx material GUID")?;
                let key = resolve(catalog, Some(guid), None)?;
                let asset = crate::linked::load_material(
                    source,
                    catalog,
                    key,
                    batch.positions.len(),
                    crate::linked::UvInput::Generated(&batch.uv0),
                    control,
                    options,
                )?;
                charge(asset.retained_bytes())?;
                Ok(std::sync::Arc::new(asset))
            })();
            let selected = material.as_ref().map(|_| ()).map_err(Clone::clone);
            (ScenePayload::Shapes { batch, material }, selected)
        },
        payload_bounds,
    );
    control.check()?;
    source.check_stable()?;
    scene.counts.reader_stop = report.stop.as_ref().map(|s| format!("{s:?}"));
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_level::parts::{GeneratedMesh, Joint, PartData, Pos, RenderMesh, Shape, ShapeColor};
    use lbp_level::serial::Descriptor;

    fn identity_at(x: f32) -> [f32; 16] {
        let mut m = [0.; 16];
        m[0] = 1.;
        m[5] = 1.;
        m[10] = 1.;
        m[15] = 1.;
        m[12] = x;
        m
    }

    fn thing(index: usize, guid: Option<u32>, world: Option<[f32; 16]>) -> ThingRecord {
        ThingRecord {
            index,
            uid: Some(index as i32 + 1),
            data: PartData {
                pos: world.map(|world| Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world,
                }),
                render_mesh: Some(RenderMesh {
                    mesh: guid.map(|g| Descriptor {
                        flags: None,
                        kind_byte: 2,
                        guid: Some(g),
                        sha1: None,
                    }),
                    bone_things: Vec::new(),
                    anim: None,
                    anim_pos: 0.,
                    anim_speed: 0.,
                    anim_loop: false,
                    editor_color: 0,
                    cast_shadows: 0,
                    visibility_flags: 0,
                    poppet_render_scale: 1.,
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn synthetic_scene_shares_meshes_and_counts_skips() {
        let mut nan = identity_at(0.);
        nan[13] = f32::NAN;
        let mut flat = identity_at(0.);
        flat[10] = 0.;
        let plain = ThingRecord {
            index: 7,
            ..Default::default()
        };
        let things = vec![
            thing(0, Some(0x10), Some(identity_at(0.))),
            thing(1, Some(0x10), Some(identity_at(10.))),
            thing(2, Some(0x20), Some(identity_at(-5.))),
            thing(3, Some(0x30), Some(identity_at(3.))), // load fails
            thing(4, Some(0x30), Some(identity_at(4.))), // cached failure
            thing(5, None, Some(identity_at(0.))),
            thing(6, Some(0x10), None),
            thing(8, Some(0x10), Some(nan)),
            thing(9, Some(0x10), Some(flat)),
            plain,
        ];
        let mut calls = Vec::new();
        let scene = build_scene(
            &things,
            |s| {
                calls.push(s.clone());
                match s {
                    MeshSource::Guid(0x30) => Err("Unresolved: GUID".into()),
                    MeshSource::Guid(g) => Ok(*g),
                    _ => unreachable!(),
                }
            },
            |_| Some([[-1., -1., -1.], [1., 1., 1.]]),
            |&g| g == 0x20,
            |_| Vec::new(),
        );
        assert_eq!(
            calls,
            vec![
                MeshSource::Guid(0x10),
                MeshSource::Guid(0x20),
                MeshSource::Guid(0x30)
            ]
        );
        let c = &scene.counts;
        assert_eq!((c.things_total, c.with_render_mesh, c.drawn), (10, 9, 3));
        assert_eq!((c.meshes_loaded, c.meshes_failed), (2, 1));
        assert_eq!(c.drawn_with_neutral_groups, 1);
        assert_eq!(c.skipped_total(), 6);
        assert_eq!(c.skipped["mesh unavailable: Unresolved: GUID"], 2);
        assert_eq!(c.skipped["RENDER_MESH without mesh reference"], 1);
        assert_eq!(c.skipped["no POS and no JOINT (no placement source)"], 1);
        assert_eq!((c.drawn_joint_pivot, c.drawn_joint_span), (0, 0));
        assert_eq!(c.skipped["non-finite world matrix"], 1);
        assert_eq!(c.skipped["non-invertible world matrix"], 1);
        assert_eq!(scene.meshes.len(), 2);
        assert_eq!(
            scene.items.iter().map(|i| i.mesh).collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
        assert_eq!(scene.bounds(), Some([[-6., -1., -1.], [11., 1., 1.]]));
        assert!(c
            .lines()
            .iter()
            .any(|l| l == "Placed (scene items): 3 Things"));
    }

    fn joint_thing(index: usize, guid: u32, joint_type: i32, a: usize, b: usize) -> ThingRecord {
        let mut t = thing(index, Some(guid), None);
        t.data.joint = Some(Joint {
            a: Some(a),
            b: Some(b),
            a_contact: [1., 2., 0.],
            b_contact: [0., 0., 0.],
            length: 0.,
            angle: 0.,
            settings: None,
            bone_idx: vec![-1, -1],
            bone_lengths: [1., 1., 1., 1.],
            joint_type,
            render_scale: 1.,
            hide_in_play_mode: false,
        });
        t
    }

    /// Bind pose like the local joint meshes: scale s, local Y -> model +Z, local Z ->
    /// model -Y (column-major), translation `t`.
    fn joint_bone(s: f32, t: [f32; 3]) -> [f32; 16] {
        [
            s, 0., 0., 0., 0., 0., s, 0., 0., -s, 0., 0., t[0], t[1], t[2], 1.,
        ]
    }

    fn apply(m: &[f32; 16], p: [f32; 3]) -> [f64; 3] {
        transform_point(m, p)
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    #[test]
    fn joint_things_place_from_endpoints() {
        // Endpoints: A at (100, 0, 0), B at (100, 0, -100) for the bolt; C at (0, 50, 0)
        // for the string from A's contact (101, 2, 0) to C.
        let things = vec![
            thing(0, Some(0x10), Some(identity_at(100.))),
            {
                let mut m = identity_at(100.);
                m[14] = -100.;
                let mut t = thing(1, Some(0x10), Some(m));
                t.data.pos.as_mut().unwrap().world[12] = 101.;
                t.data.pos.as_mut().unwrap().world[13] = 2.;
                t
            },
            {
                let mut m = identity_at(0.);
                m[13] = 50.;
                thing(2, Some(0x10), Some(m))
            },
            joint_thing(3, 0x40, 7, 0, 1), // bolt: pivot at A contact
            joint_thing(4, 0x50, 5, 0, 2), // string: span A -> C
            joint_thing(5, 0x40, 0, 0, 1), // legacy type: skipped
            {
                let mut t = joint_thing(6, 0x40, 7, 0, 1);
                t.data.joint.as_mut().unwrap().b = None; // endpoint missing
                t
            },
            joint_thing(7, 0x60, 5, 0, 2), // one-bone mesh: span impossible
        ];
        // POS Things keep their exact matrices.
        let bolt_bind = vec![
            joint_bone(0.1, [0., 0., 0.]),
            joint_bone(0.08, [0., 0., 0.]),
        ];
        let string_bind = vec![
            joint_bone(1.4, [0., 0., 26.]),
            joint_bone(1.7, [0., 0., -26.]),
        ];
        let scene = build_scene(
            &things,
            |s| match s {
                MeshSource::Guid(g) => Ok(*g),
                _ => unreachable!(),
            },
            |&g| match g {
                0x50 => Some([[-20., -10., -88.], [20., 10., 88.]]),
                _ => Some([[-40., -32., -40.], [40., 0., 40.]]),
            },
            |_| false,
            |&g| match g {
                0x40 => bolt_bind.clone(),
                0x50 => string_bind.clone(),
                0x60 => vec![joint_bone(1., [0.; 3])],
                _ => Vec::new(),
            },
        );
        let c = &scene.counts;
        assert_eq!(
            (c.drawn, c.drawn_joint_pivot, c.drawn_joint_span),
            (5, 1, 1)
        );
        assert_eq!(
            c.skipped["JOINT type 0 without POS (placement not implemented)"],
            1
        );
        assert_eq!(c.skipped["JOINT endpoint without POS"], 1);
        assert_eq!(c.skipped["span JOINT mesh with fewer than 2 bones"], 1);
        for (item, t) in scene.items.iter().zip(&things).take(3) {
            assert_eq!(item.world, t.data.pos.as_ref().unwrap().world);
        }
        // Bolt: model origin (bone 0) at contact A (101, 2, 0); model -Y -> world +Z,
        // model +Z -> world +Y, unit scale.
        let bolt = &scene.items[3];
        assert_eq!(bolt.thing, 3);
        assert!(close(apply(&bolt.world, [0., 0., 0.]), [101., 2., 0.]));
        assert!(close(apply(&bolt.world, [0., -32., 0.]), [101., 2., 32.]));
        assert!(close(apply(&bolt.world, [0., 0., 40.]), [101., 42., 0.]));
        assert!(close(apply(&bolt.world, [40., 0., 0.]), [141., 2., 0.]));
        // String: model extent along the bone0 -> bone1 axis (model z from 88 to -88)
        // fitted onto A (101, 2, 0) .. C (0, 50, 0); cross-section kept at unit scale.
        let string = &scene.items[4];
        assert_eq!(string.thing, 4);
        assert!(close(apply(&string.world, [0., 0., 88.]), [101., 2., 0.]));
        assert!(close(apply(&string.world, [0., 0., -88.]), [0., 50., 0.]));
        let mid = apply(&string.world, [0., 0., 0.]);
        let side = apply(&string.world, [0., -10., 0.]);
        assert!(close(sub(side, mid), [0., 0., 10.]));
        let across = sub(apply(&string.world, [20., 0., 0.]), mid);
        assert!((dot(across, across).sqrt() - 20.).abs() < 1e-3);
        assert!(dot(across, sub([0., 50., 0.], [101., 2., 0.])).abs() < 1e-2);
        assert!(c.lines().iter().any(|l| l.starts_with(
            "Placed from JOINT endpoints (D120 preview approximation): 2 Things (1 pivot"
        )));
    }

    #[test]
    fn matrix_checks_and_push_item_extension_point() {
        let mut m = identity_at(1.);
        assert!(world_matrix_check(&m).is_ok());
        m[3] = 0.5;
        assert!(world_matrix_check(&m)
            .unwrap_err()
            .starts_with("projective"));
        let mut scene = LevelScene::<u8>::default();
        assert!(scene
            .push_item(SceneItem {
                mesh: 0,
                world: identity_at(0.),
                thing: 0,
                uid: None,
                shapes: 0,
            })
            .is_err());
        scene.meshes.push(SceneMesh {
            source: MeshSource::Sha1("ab".into()),
            payload: 1,
            bounds: None,
            neutral_groups: false,
        });
        scene
            .push_item(SceneItem {
                mesh: 0,
                world: identity_at(2.),
                thing: 0,
                uid: None,
                shapes: 0,
            })
            .unwrap();
        assert_eq!(scene.bounds(), None);
        // Mirror (negative determinant) is invertible and accepted.
        let mut mirror = identity_at(0.);
        mirror[0] = -1.;
        assert!(world_matrix_check(&mirror).is_ok());
        assert_eq!(
            transform_point(&identity_at(2.), [1., 2., 3.]),
            [3., 2., 3.]
        );
    }

    fn shape_thing(index: usize, gfx: Option<u32>, visible: bool, x: f32) -> ThingRecord {
        let square = [[0., 0.], [100., 0.], [100., 100.], [0., 100.]];
        let d = |g| Descriptor {
            flags: Some(0),
            kind_byte: 2,
            guid: Some(g),
            sha1: None,
        };
        ThingRecord {
            index,
            data: PartData {
                pos: Some(Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world: identity_at(x),
                }),
                shape: Some(Shape {
                    vertex_count: 4,
                    requires_z: false,
                    vertices: square.iter().map(|p: &[f32; 2]| [p[0], p[1], 0.]).collect(),
                    loops: vec![4],
                    material: None,
                    old_material: None,
                    thickness: 10.,
                    mass_depth: 1.,
                    color: ShapeColor::Packed(0xffff_ffff),
                    brightness: 0.,
                    bevel_size: 0.,
                    com: None,
                    behavior: 0,
                    flags: 0,
                    contacts: 0,
                }),
                generated_mesh: Some(GeneratedMesh {
                    gfx_material: gfx.map(d),
                    bevel: None,
                    uv_offset: [0.; 4],
                    plan_guid: None,
                    play_mode_visible: visible,
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn shapes_merge_per_material_and_count_outcomes() {
        let mut collider = shape_thing(4, Some(7), true, 0.);
        collider.data.generated_mesh = None;
        let mut bad = shape_thing(5, Some(7), true, 0.);
        bad.data.shape.as_mut().unwrap().loops = vec![3];
        let things = vec![
            shape_thing(0, Some(7), true, 0.),
            shape_thing(1, Some(9), true, 500.),
            shape_thing(2, Some(7), true, 1000.),
            shape_thing(3, Some(7), false, 0.),
            collider,
            bad,
        ];
        let mut scene = LevelScene::<ShapeBatch>::default();
        add_shapes(
            &mut scene,
            &things,
            |b| {
                let ok = if b.material == Some(9) {
                    Err("Unresolved: GUID".to_string())
                } else {
                    Ok(())
                };
                (b, ok)
            },
            |b| points_bounds(&b.positions),
        );
        let c = &scene.counts;
        assert_eq!(
            (c.shapes_total, c.shapes_extruded, c.shapes_placed),
            (6, 3, 3)
        );
        assert_eq!(c.shape_batches, 2);
        assert_eq!(c.shapes_not_selected["edit_only"], 1);
        assert_eq!(c.shapes_not_selected["no_generated_mesh"], 1);
        assert_eq!(c.shapes_rejected["loop_sum_mismatch"], 1);
        assert_eq!(c.shapes_material_fallback["Unresolved: GUID"], 1);
        // Material 7 first (BTreeMap order): two shapes merged, indices rebased.
        let b = &scene.meshes[0].payload;
        assert_eq!(b.things, vec![0, 2]);
        let half = b.positions.len() / 2;
        assert!(b.triangles[b.triangles.len() / 2..]
            .iter()
            .flatten()
            .all(|&i| i as usize >= half));
        assert_eq!(scene.items[0].shapes, 2);
        assert!(scene.meshes[1].neutral_groups);
        let [lo, hi] = scene.bounds().unwrap();
        assert_eq!((lo[0], hi[0]), (0., 1100.));
        assert!(c
            .lines()
            .iter()
            .any(|l| l.contains(shape_mesh::PREVIEW_LABEL)));
    }
}
