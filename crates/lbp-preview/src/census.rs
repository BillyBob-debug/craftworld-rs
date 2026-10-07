//! D81: per-mesh coverage and corpus tally for the read-only mesh census tools.
//! Asset unit: distinct catalog GUID. A mesh hash's outcome applies to every GUID
//! whose MAP row names that hash (aliases preserved, as in compatibility_census).
//! "Viewable" = the mesh loads and has at least one nonzero-area triangle; this
//! confirms geometry availability, not pose or visual fidelity. "All-diffuse" =
//! viewable and every nonempty primitive group has a loaded diffuse binding; this is
//! binding coverage, not visual completeness (Complete stays a manual check).
use std::collections::{BTreeMap, BTreeSet};

/// Longest reason key kept in tallies; longer messages are cut at a char boundary.
pub const MAX_REASON_CHARS: usize = 160;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshCoverage {
    pub geometry_available: bool,
    pub nonempty_groups: usize,
    pub supported_groups: usize,
}
impl MeshCoverage {
    pub fn all_diffuse(&self) -> bool {
        self.geometry_available
            && self.nonempty_groups > 0
            && self.supported_groups == self.nonempty_groups
    }
}

/// True when at least one triangle has a nonzero cross product (f64 arithmetic).
/// Out-of-range indices count as unavailable rather than panicking.
pub fn geometry_available(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> bool {
    triangles.iter().any(|t| {
        let Some([a, b, c]) = t
            .iter()
            .map(|&i| positions.get(i as usize).map(|p| p.map(f64::from)))
            .collect::<Option<Vec<_>>>()
            .and_then(|v| <[[f64; 3]; 3]>::try_from(v).ok())
        else {
            return false;
        };
        let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
        let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
        .iter()
        .any(|n| *n != 0.)
    })
}

/// Coverage from group triangle ranges paired with binding outcomes (Ok = loaded).
pub fn coverage<'a>(
    geometry_available: bool,
    groups: impl IntoIterator<Item = (usize, Result<(), &'a str>)>,
) -> (MeshCoverage, BTreeSet<String>) {
    let mut c = MeshCoverage {
        geometry_available,
        ..Default::default()
    };
    let mut reasons = BTreeSet::new();
    for (triangles, binding) in groups {
        if triangles == 0 {
            continue;
        }
        c.nonempty_groups += 1;
        match binding {
            Ok(()) => c.supported_groups += 1,
            Err(e) => {
                reasons.insert(reason_key(e));
            }
        }
    }
    (c, reasons)
}

pub fn reason_key(reason: &str) -> String {
    reason.chars().take(MAX_REASON_CHARS).collect()
}

/// Outcome of one mesh hash.
pub enum MeshOutcome {
    /// Resolve or load failed (reason kept).
    Failed(String),
    /// Loaded; neutral group reasons are the distinct reasons on nonempty groups.
    Loaded {
        coverage: MeshCoverage,
        neutral_reasons: BTreeSet<String>,
    },
}

#[derive(Default)]
pub struct CorpusTally {
    pub mesh_hashes: usize,
    pub viewable_hashes: usize,
    pub all_diffuse_hashes: usize,
    pub catalog_guids: BTreeSet<String>,
    pub viewable_guids: BTreeSet<String>,
    pub all_diffuse_guids: BTreeSet<String>,
    /// Mesh hashes that failed to load, by reason.
    pub failed_by_reason: BTreeMap<String, usize>,
    /// Loaded meshes without a nonzero-area triangle.
    pub no_geometry_hashes: usize,
    /// Viewable meshes that are not all-diffuse, counted once per distinct reason.
    pub partial_by_neutral_reason: BTreeMap<String, usize>,
    /// Viewable meshes with no nonempty primitive group.
    pub viewable_without_groups: usize,
    pub nonempty_groups: usize,
    pub supported_groups: usize,
}
impl CorpusTally {
    pub fn record(
        &mut self,
        guids: &BTreeSet<String>,
        outcome: &MeshOutcome,
    ) -> Result<(), String> {
        let inc = |n: &mut usize| -> Result<(), String> {
            *n = n.checked_add(1).ok_or("census count overflow")?;
            Ok(())
        };
        inc(&mut self.mesh_hashes)?;
        self.catalog_guids.extend(guids.iter().cloned());
        match outcome {
            MeshOutcome::Failed(reason) => {
                inc(self.failed_by_reason.entry(reason_key(reason)).or_default())?;
            }
            MeshOutcome::Loaded {
                coverage,
                neutral_reasons,
            } => {
                self.nonempty_groups = self
                    .nonempty_groups
                    .checked_add(coverage.nonempty_groups)
                    .ok_or("census group overflow")?;
                self.supported_groups = self
                    .supported_groups
                    .checked_add(coverage.supported_groups)
                    .ok_or("census group overflow")?;
                if !coverage.geometry_available {
                    return inc(&mut self.no_geometry_hashes);
                }
                inc(&mut self.viewable_hashes)?;
                self.viewable_guids.extend(guids.iter().cloned());
                if coverage.all_diffuse() {
                    inc(&mut self.all_diffuse_hashes)?;
                    self.all_diffuse_guids.extend(guids.iter().cloned());
                } else if coverage.nonempty_groups == 0 {
                    inc(&mut self.viewable_without_groups)?;
                } else {
                    for r in neutral_reasons {
                        inc(self.partial_by_neutral_reason.entry(r.clone()).or_default())?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guids(v: &[&str]) -> BTreeSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn geometry_requires_nonzero_area_and_valid_indices() {
        let p = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [2., 0., 0.]];
        assert!(geometry_available(&p, &[[0, 1, 2]]));
        assert!(!geometry_available(&p, &[[0, 1, 3]])); // collinear
        assert!(!geometry_available(&p, &[[0, 1, 9]])); // out of range
        assert!(!geometry_available(&p, &[]));
    }
    #[test]
    fn coverage_ignores_empty_groups_and_collects_reasons() {
        let (c, r) = coverage(
            true,
            [(3, Ok(())), (0, Err("ignored")), (2, Err("Unsupported: x"))],
        );
        assert_eq!((c.nonempty_groups, c.supported_groups), (2, 1));
        assert!(!c.all_diffuse());
        assert_eq!(r, guids(&["Unsupported: x"]));
        let (c, _) = coverage(true, [(0, Ok(()))]);
        assert!(!c.all_diffuse()); // no nonempty group
        let (c, _) = coverage(false, [(1, Ok(()))]);
        assert!(!c.all_diffuse()); // geometry required
        assert_eq!(reason_key(&"a".repeat(500)).len(), MAX_REASON_CHARS);
    }
    #[test]
    fn tally_unions_alias_guids_and_counts_outcomes() {
        let mut t = CorpusTally::default();
        let full = MeshCoverage {
            geometry_available: true,
            nonempty_groups: 2,
            supported_groups: 2,
        };
        let loaded = |coverage, r: &[&str]| MeshOutcome::Loaded {
            coverage,
            neutral_reasons: guids(r),
        };
        t.record(&guids(&["a", "b"]), &loaded(full, &[])).unwrap();
        let partial = MeshCoverage {
            supported_groups: 1,
            ..full
        };
        t.record(&guids(&["c"]), &loaded(partial, &["r1", "r2"]))
            .unwrap();
        let flat = MeshCoverage {
            geometry_available: false,
            ..full
        };
        t.record(&guids(&["d"]), &loaded(flat, &[])).unwrap();
        t.record(&guids(&["e"]), &MeshOutcome::Failed("boom".into()))
            .unwrap();
        t.record(&guids(&["a"]), &MeshOutcome::Failed("boom".into()))
            .unwrap();
        assert_eq!(t.mesh_hashes, 5);
        assert_eq!(t.catalog_guids.len(), 5);
        assert_eq!(t.viewable_guids, guids(&["a", "b", "c"]));
        assert_eq!(t.all_diffuse_guids, guids(&["a", "b"]));
        assert_eq!((t.viewable_hashes, t.all_diffuse_hashes), (2, 1));
        assert_eq!(t.no_geometry_hashes, 1);
        assert_eq!(t.failed_by_reason["boom"], 2);
        assert_eq!(t.partial_by_neutral_reason.len(), 2);
        assert_eq!((t.nonempty_groups, t.supported_groups), (6, 5));
    }
}
