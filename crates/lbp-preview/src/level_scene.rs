//! D113 (level-research L2/L5 subset): renderer-independent level scene.
//!
//! Reads one level with `lbp-level` and turns every Thing that has a RENDER_MESH into a
//! scene item: (shared mesh payload, world matrix). Meshes load once per mesh reference
//! through the same bounded single-mesh path (`load_mesh_groups_with_options`) and are
//! reused by every item that names them. Things that cannot be drawn are skipped and
//! counted by reason. Placement is the Thing's own POS world matrix (static bind pose):
//! bones, animation, decorations, stickers and material overrides are not applied.
//!
//! SHAPE / GENERATED_MESH Things are counted but not drawn yet. A later shape extrusion
//! module adds its meshes as further `SceneMesh` entries with `MeshSource::Shape` and
//! pushes items the same way (`LevelScene::push_item`); the renderer consumes only
//! `meshes` + `items` and never needs to know where a mesh came from.
//!
//! No GUI or GPU types. The level reader recurses natively: call `load_level_scene` on
//! a thread with at least `LEVEL_STACK_BYTES` of stack.
use crate::{load_mesh_groups_with_options, resolve, GroupedMesh, PreviewOptions};
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_level::thing::ThingRecord;
use std::collections::{BTreeMap, HashMap};

/// Worker stack for `load_level_scene` (the D107 examples use the same size).
pub const LEVEL_STACK_BYTES: usize = 512 * 1024 * 1024;
/// Distinct mesh references loaded per level.
pub const MAX_LEVEL_MESHES: usize = 1024;
/// Drawn items per level.
pub const MAX_LEVEL_ITEMS: usize = 100_000;
/// Aggregate retained material bytes over all loaded meshes of one level. Each mesh keeps
/// its own D66 limits (64 material loads, 640 MiB); this caps the sum.
pub const MAX_LEVEL_RETAINED_BYTES: usize = 1536 * 1024 * 1024;
const PAGE: usize = 4096;

/// Where a scene mesh comes from. Shapes (D114) will add a variant here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshSource {
    /// Model mesh named by GUID in a RENDER_MESH descriptor.
    Guid(u32),
    /// Model mesh named only by SHA-1 (lowercase hex).
    Sha1(String),
}

impl MeshSource {
    pub fn label(&self) -> String {
        match self {
            Self::Guid(g) => format!("{g:08x}"),
            Self::Sha1(h) => format!("sha1 {h}"),
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
}

/// Counting units: Things (tag-table entries), items (= drawn Things), distinct mesh
/// references. Skip reasons count Things.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneCounts {
    pub things_total: usize,
    pub with_render_mesh: usize,
    pub drawn: usize,
    pub skipped: BTreeMap<String, usize>,
    /// Drawn Things whose POS names a bone owner (placed by own POS; approximation).
    pub drawn_bone_attached: usize,
    /// Drawn Things whose mesh has neutral (unsupported material) groups.
    pub drawn_with_neutral_groups: usize,
    pub meshes_loaded: usize,
    pub meshes_failed: usize,
    /// Things with SHAPE / GENERATED_MESH (not drawn in D113) and DECORATIONS entries.
    pub shapes_not_drawn: usize,
    pub generated_meshes_not_drawn: usize,
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
            format!(
                "Skipped: {} Things with RENDER_MESH",
                self.skipped_total()
            ),
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
            "Not drawn yet: {} SHAPE Things, {} GENERATED_MESH Things, {} decorations",
            self.shapes_not_drawn, self.generated_meshes_not_drawn, self.decorations_not_drawn
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
    if !(det.abs() > 1e-12) {
        return Err("non-invertible world matrix".into());
    }
    Ok(())
}

/// Builds the scene from reader Things. `load` is called once per distinct mesh
/// reference; failures are cached and count every Thing that names that mesh.
pub fn build_scene<T>(
    things: &[ThingRecord],
    mut load: impl FnMut(&MeshSource) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
) -> LevelScene<T> {
    let mut scene = LevelScene::<T>::default();
    let mut by_source: HashMap<MeshSource, Result<usize, String>> = HashMap::new();
    let c = &mut scene.counts;
    c.things_total = things.len();
    let skip = |c: &mut SceneCounts, reason: String| *c.skipped.entry(reason).or_default() += 1;
    for t in things {
        let c = &mut scene.counts;
        c.shapes_not_drawn += usize::from(t.data.shape.is_some());
        c.generated_meshes_not_drawn += usize::from(t.data.generated_mesh.is_some());
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
        let Some(pos) = &t.data.pos else {
            skip(c, "no POS (joint/bone placement not implemented)".into());
            continue;
        };
        if let Err(e) = world_matrix_check(&pos.world) {
            skip(c, e);
            continue;
        }
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
        c.drawn += 1;
        c.drawn_bone_attached += usize::from(pos.bone_of.is_some());
        c.drawn_with_neutral_groups += usize::from(scene.meshes[mesh].neutral_groups);
        scene.items.push(SceneItem {
            mesh,
            world: pos.world,
            thing: t.index,
            uid: t.uid,
        });
    }
    scene
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

fn read_all(
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
        let page =
            source.read_hex_checked(key, bytes.len() as u64, (size - bytes.len()).min(PAGE), control)?;
        if page.bytes.is_empty() {
            return Err("Invalid: truncated level read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

fn grouped_bounds(g: &GroupedMesh) -> Option<[[f32; 3]; 2]> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &g.geometry.positions {
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
) -> Result<LevelScene<GroupedMesh>, String> {
    let bytes = read_all(source, level, control)?;
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
    let mut scene = build_scene(
        &report.things,
        |src| {
            control.check()?;
            let key = match src {
                MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
            };
            let mesh = load_mesh_groups_with_options(source, catalog, &key, control, options)?;
            retained_total = retained_total
                .checked_add(retained(&mesh))
                .filter(|&t| t <= MAX_LEVEL_RETAINED_BYTES)
                .ok_or("Unsupported: level material memory budget (1536 MiB retained)")?;
            Ok(mesh)
        },
        grouped_bounds,
        grouped_neutral,
    );
    control.check()?;
    source.check_stable()?;
    scene.counts.reader_stop = report.stop.as_ref().map(|s| format!("{s:?}"));
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_level::parts::{PartData, Pos, RenderMesh};
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
        let mut plain = ThingRecord::default();
        plain.index = 7;
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
        assert_eq!(c.skipped["no POS (joint/bone placement not implemented)"], 1);
        assert_eq!(c.skipped["non-finite world matrix"], 1);
        assert_eq!(c.skipped["non-invertible world matrix"], 1);
        assert_eq!(scene.meshes.len(), 2);
        assert_eq!(
            scene.items.iter().map(|i| i.mesh).collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
        assert_eq!(
            scene.bounds(),
            Some([[-6., -1., -1.], [11., 1., 1.]])
        );
        assert!(c.lines().iter().any(|l| l == "Placed (scene items): 3 Things"));
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
                uid: None
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
            })
            .unwrap();
        assert_eq!(scene.bounds(), None);
        // Mirror (negative determinant) is invertible and accepted.
        let mut mirror = identity_at(0.);
        mirror[0] = -1.;
        assert!(world_matrix_check(&mirror).is_ok());
        assert_eq!(transform_point(&identity_at(2.), [1., 2., 3.]), [3., 2., 3.]);
    }
}
