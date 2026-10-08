//! D114: read-only census of SHAPE extrusion v1 (`lbp_level::shape_mesh`) over every Thing
//! with SHAPE data in every level resource of a snapshot (magic-confirmed `level` MAP rows,
//! grouped by content hash), plus the same totals for `tutorial_23.bin` alone and a layer
//! tiling check of the half-depth rule (docs/d108-shape-geometry.md D3).
//! Output: a NEW JSON file under an ignored `local-analysis/` directory; nothing else is
//! written. Usage: shape_census SNAPSHOT GAME-DIR[;GAME-DIR] NEW-JSON [--bevels]
//! Counting unit: one Thing with SHAPE data, per containing level content hash.
//! D122 `--bevels`: also loads every referenced RBevel (`lbp_level::bevel`) and runs the
//! bevelled extrusion (`extrude_thing_bevelled`, default options) beside the straight one;
//! the straight totals and the tiling check are unchanged, the bevel totals are separate.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::bevel::{self, Bevel};
use lbp_level::serial::Descriptor;
use lbp_level::shape_mesh::{
    extrude_thing, extrude_thing_bevelled, BevelOptions, BevelOutcome, ShapeMesh, BEVEL_LABEL,
    PREVIEW_LABEL, UV_SCALE,
};
use lbp_level::Limits;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;
const MAX_LEVELS: usize = 10_000;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const FOCUS: &str = "tutorial_23.bin";

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > lbp_level::level::MAX_INPUT {
        return Err("archived size exceeds level input budget".into());
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
            return Err("truncated source read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

#[derive(Default)]
struct Tally {
    levels: usize,
    shapes: usize,
    extruded: usize,
    selection: BTreeMap<&'static str, usize>,
    rejected: BTreeMap<&'static str, usize>,
    triangles: u64,
    vertices: u64,
    front_triangles: u64,
    mirrored: usize,
    with_holes: usize,
    rewound_loops: usize,
    same_winding_holes: usize,
    duplicate_vertices_removed: usize,
    no_material: usize,
    materials: BTreeSet<u32>,
    bevel_recorded_unused: usize,
}

impl Tally {
    fn add_ok(&mut self, m: &ShapeMesh) {
        self.extruded += 1;
        self.triangles += m.triangle_count() as u64;
        self.vertices += m.positions.len() as u64;
        self.front_triangles += (m.front.len() / 3) as u64;
        self.mirrored += usize::from(m.mirrored);
        self.with_holes += usize::from(m.loops > 1);
        self.rewound_loops += m.rewound_loops;
        self.same_winding_holes += m.same_winding_holes;
        self.duplicate_vertices_removed += m.duplicate_vertices_removed;
        match m.material_guid() {
            Some(g) => {
                self.materials.insert(g);
            }
            None => self.no_material += 1,
        }
        self.bevel_recorded_unused += usize::from(m.bevel_unused.is_some());
    }
    fn merge(&mut self, o: &Tally) {
        self.levels += o.levels;
        self.shapes += o.shapes;
        self.extruded += o.extruded;
        for (k, v) in &o.selection {
            *self.selection.entry(k).or_default() += v;
        }
        for (k, v) in &o.rejected {
            *self.rejected.entry(k).or_default() += v;
        }
        self.triangles += o.triangles;
        self.vertices += o.vertices;
        self.front_triangles += o.front_triangles;
        self.mirrored += o.mirrored;
        self.with_holes += o.with_holes;
        self.rewound_loops += o.rewound_loops;
        self.same_winding_holes += o.same_winding_holes;
        self.duplicate_vertices_removed += o.duplicate_vertices_removed;
        self.no_material += o.no_material;
        self.materials.extend(&o.materials);
        self.bevel_recorded_unused += o.bevel_recorded_unused;
    }
    fn json(&self) -> Value {
        json!({
            "levels": self.levels,
            "shapes": self.shapes,
            "skipped_by_selection": self.selection,
            "selected": self.shapes - self.selection.values().sum::<usize>(),
            "extruded_ok": self.extruded,
            "rejected_by_reason": self.rejected,
            "rejected": self.rejected.values().sum::<usize>(),
            "triangles": self.triangles,
            "front_cap_triangles": self.front_triangles,
            "vertices": self.vertices,
            "mirrored": self.mirrored,
            "with_holes": self.with_holes,
            "rewound_loops": self.rewound_loops,
            "same_winding_holes": self.same_winding_holes,
            "duplicate_vertices_removed": self.duplicate_vertices_removed,
            "extruded_without_gfx_material": self.no_material,
            "distinct_gfx_materials": self.materials.len(),
            "extruded_with_bevel_recorded_unused": self.bevel_recorded_unused,
        })
    }
}

/// D122 bevelled-extrusion totals (unit: Thing with SHAPE data, as `Tally`).
#[derive(Default)]
struct BevelTally {
    outcomes: BTreeMap<String, usize>,
    rejected: BTreeMap<&'static str, usize>,
    shrink_steps: BTreeMap<u32, usize>,
    triangles: u64,
    vertices: u64,
    multi_material: usize,
    slot_material_fallbacks: usize,
    hidden_strips: usize,
}

impl BevelTally {
    fn add(&mut self, r: &Result<ShapeMesh, lbp_level::ShapeReject>) {
        match r {
            Ok(m) => {
                let key = match m.bevel {
                    BevelOutcome::Fallback(reason) => format!("bevel_fallback:{reason}"),
                    o => o.key().to_string(),
                };
                *self.outcomes.entry(key).or_default() += 1;
                if let BevelOutcome::Applied { shrink_steps, .. } = m.bevel {
                    *self.shrink_steps.entry(shrink_steps).or_default() += 1;
                }
                self.triangles += m.triangle_count() as u64;
                self.vertices += m.positions.len() as u64;
                self.multi_material += usize::from(m.material_parts().len() > 1);
                self.slot_material_fallbacks += m.slot_material_fallbacks;
                self.hidden_strips += m.hidden_strips;
            }
            Err(e) if e.is_selection() => {}
            Err(e) => *self.rejected.entry(e.reason()).or_default() += 1,
        }
    }
    fn merge(&mut self, o: &BevelTally) {
        for (k, v) in &o.outcomes {
            *self.outcomes.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &o.rejected {
            *self.rejected.entry(k).or_default() += v;
        }
        for (k, v) in &o.shrink_steps {
            *self.shrink_steps.entry(*k).or_default() += v;
        }
        self.triangles += o.triangles;
        self.vertices += o.vertices;
        self.multi_material += o.multi_material;
        self.slot_material_fallbacks += o.slot_material_fallbacks;
        self.hidden_strips += o.hidden_strips;
    }
    fn json(&self) -> Value {
        let applied: usize = self
            .outcomes
            .iter()
            .filter(|(k, _)| k.starts_with("bevelled"))
            .map(|(_, v)| v)
            .sum();
        json!({
            "extruded_ok": self.outcomes.values().sum::<usize>(),
            "bevelled": applied,
            "outcomes": self.outcomes,
            "rejected_by_reason": self.rejected,
            "rejected": self.rejected.values().sum::<usize>(),
            "bevelled_by_shrink_steps": self.shrink_steps,
            "triangles": self.triangles,
            "vertices": self.vertices,
            "with_more_than_one_material": self.multi_material,
            "slot_material_fallbacks": self.slot_material_fallbacks,
            "hidden_mode_strips_drawn": self.hidden_strips,
        })
    }
}

type BevelKey = (Option<u32>, Option<[u8; 20]>);

/// Loads (once) the bevel a descriptor names; None when unresolved or unreadable.
fn load_bevel<'a>(
    cache: &'a mut BTreeMap<BevelKey, Result<Bevel, String>>,
    d: &Descriptor,
    catalog: &lbp_catalog::Catalog,
    source: &AttachedSource,
    control: &LoadControl,
) -> Option<&'a Bevel> {
    let key = (d.guid, d.sha1);
    cache
        .entry(key)
        .or_insert_with(|| {
            let hash = d.sha1.map(|h| lbp_level::serial::hex(&h));
            let loc = match (d.guid, &hash) {
                (Some(g), _) => lbp_preview::resolve(catalog, Some(g), None),
                (None, Some(h)) => lbp_preview::resolve(catalog, None, Some(h)),
                _ => Err("no identifier".into()),
            }?;
            if loc.size > bevel::MAX_INPUT as u64 {
                return Err("bevel input budget".into());
            }
            let bytes = read_all(source, &loc, control)?;
            bevel::read(&bytes, &|| control.check().is_err()).map_err(|e| e.to_string())
        })
        .as_ref()
        .ok()
}

/// Layer tiling of distinct root-shape slab z-ranges under one depth hypothesis.
#[derive(Default)]
struct Tiling {
    intervals: usize,
    partial_overlaps: usize,
    gaps: usize,
    /// Intervals whose top or bottom meets another interval's bottom or top (within 0.01).
    touching: usize,
}

fn tiling(intervals: &BTreeSet<(i64, i64)>) -> Tiling {
    let v: Vec<_> = intervals.iter().copied().collect();
    let mut t = Tiling {
        intervals: v.len(),
        ..Tiling::default()
    };
    for i in 0..v.len() {
        for j in i + 1..v.len() {
            // Sorted by lo: j overlaps i with positive length and differs from it.
            if v[j].0 < v[i].1 && v[j] != v[i] {
                t.partial_overlaps += 1;
            }
        }
    }
    t.touching = v
        .iter()
        .enumerate()
        .filter(|(i, a)| {
            v.iter()
                .enumerate()
                .any(|(j, b)| *i != j && ((a.1 - b.0).abs() <= 1 || (a.0 - b.1).abs() <= 1))
        })
        .count();
    let mut reach = v.first().map(|x| x.1).unwrap_or(0);
    for x in v.iter().skip(1) {
        if x.0 > reach {
            t.gaps += 1;
        }
        reach = reach.max(x.1);
    }
    t
}

/// World-z interval in hundredths of a unit; `full` = the rejected full-depth hypothesis
/// (slab spans thickness, not 2 * thickness).
fn interval(m: &ShapeMesh, full: bool) -> (i64, i64) {
    let (lo, hi) = (f64::from(m.world_z.0), f64::from(m.world_z.1));
    let (c, h) = ((lo + hi) / 2.0, (hi - lo) / 2.0);
    let h = if full { h / 2.0 } else { h };
    (
        ((c - h) * 100.0).round() as i64,
        ((c + h) * 100.0).round() as i64,
    )
}

/// The slab is axis-aligned in z (z column and z row of the 3x3 are (0, 0, s)).
fn z_aligned(w: &[f32; 16]) -> bool {
    w[8].abs() < 1e-4 && w[9].abs() < 1e-4 && w[2].abs() < 1e-4 && w[6].abs() < 1e-4
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let bevels = args.len() == 5 && args[4] == "--bevels";
    if args.len() != 4 && !bevels {
        return Err("usage: shape_census SNAPSHOT GAME-DIR NEW-JSON [--bevels]".into());
    }
    let output = Path::new(&args[3]);
    let parent_ok = output
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "local-analysis");
    if output.exists() || !parent_ok {
        return Err("new output under an ignored local-analysis directory required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "level" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.path.clone());
        }
    }
    if by_hash.is_empty() || by_hash.len() > MAX_LEVELS {
        return Err("level hash budget".into());
    }
    let limits = Limits {
        max_depth: 4096,
        ..Limits::default()
    };
    let mut all = Tally::default();
    let mut focus = Tally::default();
    let (mut bevel_all, mut bevel_focus) = (BevelTally::default(), BevelTally::default());
    let mut bevel_cache: BTreeMap<BevelKey, Result<Bevel, String>> = BTreeMap::new();
    let bevel_options = BevelOptions::default();
    let mut focus_rows = Vec::new();
    let mut statuses = BTreeMap::<&str, usize>::new();
    let mut examples = BTreeMap::<&str, Vec<Value>>::new();
    let mut rows = Vec::new();
    let mut total_bytes = 0u64;
    let (mut tiled_levels, mut tiled_levels_full, mut tiling_levels) = (0, 0, 0);
    let (mut overlaps_half, mut gaps_half, mut overlaps_full, mut gaps_full) = (0, 0, 0, 0);
    let (mut intervals_total, mut touch_half, mut touch_full) = (0, 0, 0);
    let mut focus_intervals = BTreeSet::new();
    let mut thickness_hist = BTreeMap::<String, usize>::new();
    for (sha, paths) in &by_hash {
        control.check()?;
        let is_focus = paths.iter().any(|p| p.ends_with(FOCUS));
        let mut row = json!({"sha1": sha, "paths": paths});
        let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Ok(k) => k,
            Err(e) => {
                *statuses.entry("unresolved").or_default() += 1;
                row["status"] = json!("unresolved");
                row["reason"] = json!(e);
                rows.push(row);
                continue;
            }
        };
        total_bytes += key.size;
        if total_bytes > MAX_TOTAL_BYTES {
            return Err("total byte budget".into());
        }
        let bytes = match read_all(&source, &key, &control) {
            Ok(b) => b,
            Err(e) => {
                *statuses.entry("read_failed").or_default() += 1;
                row["status"] = json!("read_failed");
                row["reason"] = json!(e);
                rows.push(row);
                continue;
            }
        };
        let report = match lbp_level::read(&bytes, limits, &|| control.check().is_err()) {
            Ok(r) => r,
            Err(e) => {
                *statuses.entry("wrapper_error").or_default() += 1;
                row["status"] = json!("wrapper_error");
                row["reason"] = json!(e.to_string());
                rows.push(row);
                continue;
            }
        };
        let st = match (&report.stop, report.trailing_bytes) {
            (None, _) => "complete",
            (Some(_), _) if report.world_things_walked.is_some() => "world_things_walked",
            (Some(_), _) if report.things.is_empty() => "out_of_band",
            (Some(_), _) => "stopped_in_things",
        };
        *statuses.entry(st).or_default() += 1;
        row["status"] = json!(st);
        if st == "out_of_band" {
            rows.push(row);
            continue;
        }
        let mut t = Tally {
            levels: 1,
            ..Tally::default()
        };
        let mut bt = BevelTally::default();
        let (mut half, mut full) = (BTreeSet::new(), BTreeSet::new());
        for thing in &report.things {
            if thing.data.shape.is_none() {
                continue;
            }
            t.shapes += 1;
            if bevels {
                let r = extrude_thing_bevelled(
                    thing,
                    |d| load_bevel(&mut bevel_cache, d, &catalog, &source, &control),
                    &bevel_options,
                );
                bt.add(&r);
            }
            match extrude_thing(thing) {
                Ok(m) => {
                    t.add_ok(&m);
                    let w = thing.data.pos.as_ref().map(|p| p.world);
                    if thing.parent.is_none() && w.is_some_and(|w| z_aligned(&w)) {
                        half.insert(interval(&m, false));
                        full.insert(interval(&m, true));
                        *thickness_hist
                            .entry(format!("{}", m.thickness))
                            .or_default() += 1;
                    }
                    if is_focus {
                        focus_rows.push(json!({
                            "thing": thing.index, "uid": thing.uid,
                            "triangles": m.triangle_count(), "loops": m.loops,
                            "material_guid": m.material_guid(),
                            "mirrored": m.mirrored, "world_z": [m.world_z.0, m.world_z.1],
                        }));
                    }
                }
                Err(e) => {
                    let key = e.reason();
                    if e.is_selection() {
                        *t.selection.entry(key).or_default() += 1;
                    } else {
                        *t.rejected.entry(key).or_default() += 1;
                        let ex = examples.entry(key).or_default();
                        if ex.len() < 5 {
                            ex.push(json!({"sha1": sha, "thing": thing.index, "detail": format!("{e:?}")}));
                        }
                    }
                    if is_focus {
                        focus_rows
                            .push(json!({"thing": thing.index, "uid": thing.uid, "reason": key}));
                    }
                }
            }
        }
        if !half.is_empty() {
            tiling_levels += 1;
            let h = tiling(&half);
            let f = tiling(&full);
            tiled_levels += usize::from(h.partial_overlaps == 0 && h.gaps == 0);
            tiled_levels_full += usize::from(f.partial_overlaps == 0 && f.gaps == 0);
            overlaps_half += h.partial_overlaps;
            gaps_half += h.gaps;
            overlaps_full += f.partial_overlaps;
            gaps_full += f.gaps;
            intervals_total += h.intervals;
            touch_half += h.touching;
            touch_full += f.touching;
            row["tiling_half_depth"] = json!({"intervals": h.intervals, "partial_overlaps": h.partial_overlaps, "gaps": h.gaps, "touching": h.touching});
            row["tiling_full_depth"] = json!({"intervals": f.intervals, "partial_overlaps": f.partial_overlaps, "gaps": f.gaps, "touching": f.touching});
            if is_focus {
                focus_intervals = half.clone();
            }
        }
        row["shapes"] = t.json();
        if bevels {
            row["bevelled"] = bt.json();
        }
        if is_focus {
            focus.merge(&t);
            bevel_focus.merge(&bt);
        }
        all.merge(&t);
        bevel_all.merge(&bt);
        rows.push(row);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "D114",
        "label": PREVIEW_LABEL,
        "uv_scale": UV_SCALE,
        "scope": "every Thing with SHAPE data in every magic-confirmed level MAP row (by content hash) whose RLevel read is in band; partially read levels contribute the Things read before the stop",
        "unit": "Thing with SHAPE data, per containing level content hash",
        "level_hashes": by_hash.len(),
        "level_status_counts": statuses,
        "all": all.json(),
        "focus": {"name": FOCUS, "totals": focus.json(), "intervals_half_depth_centi": focus_intervals, "things": focus_rows},
        "tiling": {
            "scope": "distinct world z-ranges of extruded root shapes (no parent) with a z-aligned transform, per level",
            "levels_checked": tiling_levels,
            "distinct_intervals": intervals_total,
            "half_depth": {"levels_tiled": tiled_levels, "partial_overlaps": overlaps_half, "gaps": gaps_half, "intervals_touching_a_neighbour": touch_half},
            "full_depth_alternative": {"levels_tiled": tiled_levels_full, "partial_overlaps": overlaps_full, "gaps": gaps_full, "intervals_touching_a_neighbour": touch_full},
            "root_thickness_histogram": thickness_hist,
        },
        "reject_examples": examples,
        "bevels": bevels.then(|| {
            let mut status = BTreeMap::<String, usize>::new();
            for r in bevel_cache.values() {
                *status
                    .entry(match r {
                        Ok(_) => "parsed".into(),
                        Err(e) => format!("failed: {e}"),
                    })
                    .or_default() += 1;
            }
            json!({
                "label": BEVEL_LABEL,
                "options": format!("{bevel_options:?}"),
                "distinct_bevel_descriptors": bevel_cache.len(),
                "bevel_load_status": status,
                "all": bevel_all.json(),
                "focus": bevel_focus.json(),
            })
        }),
        "archived_bytes_read": total_bytes,
        "rows": rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("level hashes {}; status {:?}", by_hash.len(), statuses);
    println!("all: {}", all.json());
    println!("{FOCUS}: {}", focus.json());
    if bevels {
        println!(
            "bevels: {} descriptors; all: {}",
            bevel_cache.len(),
            bevel_all.json()
        );
        println!("bevels {FOCUS}: {}", bevel_focus.json());
    }
    println!(
        "tiling: {tiling_levels} levels; {intervals_total} intervals; half-depth tiled {tiled_levels} (overlaps {overlaps_half}, gaps {gaps_half}, touching {touch_half}); full-depth tiled {tiled_levels_full} (overlaps {overlaps_full}, gaps {gaps_full}, touching {touch_full})"
    );
    Ok(())
}

fn main() {
    // Inline references nest; the reader bounds depth, the stack must fit that bound.
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("spawn census worker");
    if let Err(e) = worker.join().expect("census worker panicked") {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
