//! D81: per-mesh coverage and corpus tally for the read-only mesh census tools.
//! Asset unit: distinct catalog GUID. A mesh hash's outcome applies to every GUID
//! whose MAP row names that hash (aliases preserved, as in compatibility_census).
//! "Viewable" = the mesh loads and has at least one nonzero-area triangle; this
//! confirms geometry availability, not pose or visual fidelity. "All-diffuse" =
//! viewable and every nonempty primitive group has a loaded diffuse binding; this is
//! binding coverage, not visual completeness (Complete stays a manual check).
//! D85: preview flags (`--animation-base`, `--instance-rgba R,G,B,A`) are parsed
//! here so corpus_mesh_census and compatibility_census accept the same values.
//! D188: standalone defaults (white instance colour, animation base values) apply
//! unless `--no-instance-default` / `--no-animation-base` opt out; material-less
//! groups are tallied separately and leave the all-diffuse denominator
//! (docs/d188-absent-material-primitives.md).
use crate::{AppliedDefaults, DiffuseTint, PreviewOptions, StandaloneDefaults};
use std::collections::{BTreeMap, BTreeSet};

/// Longest reason key kept in tallies; longer messages are cut at a char boundary.
pub const MAX_REASON_CHARS: usize = 160;

/// Parses `R,G,B,A` as a supplied linear instance color (finite, each in 0..=1).
pub fn parse_instance_rgba(text: &str) -> Result<DiffuseTint, String> {
    let values = text
        .split(',')
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "invalid RGBA")?;
    let values: [f32; 4] = values.try_into().map_err(|_| "four RGBA values required")?;
    let color = DiffuseTint {
        bits: values.map(f32::to_bits),
    };
    color.validate()?;
    Ok(color)
}

/// Census preview flags: explicit inputs plus the D188 default opt-outs.
#[derive(Debug, Clone, Copy, Default)]
pub struct PreviewFlags {
    pub explicit: PreviewOptions,
    pub no_instance_default: bool,
    pub no_animation_base: bool,
}
impl PreviewFlags {
    pub fn defaults(&self) -> StandaloneDefaults {
        StandaloneDefaults {
            instance_white: !self.no_instance_default,
            animation_base: !self.no_animation_base,
        }
    }
    /// Effective options and which defaults filled them.
    pub fn resolve(&self) -> (PreviewOptions, AppliedDefaults) {
        self.defaults().apply(self.explicit)
    }
}

/// Applies one preview flag. Returns `Ok(false)` when `flag` is not a preview flag
/// (the caller handles it); repeated or contradictory preview flags are rejected.
pub fn apply_preview_flag<'a>(
    flags: &mut PreviewFlags,
    flag: &str,
    next: impl FnOnce() -> Option<&'a str>,
) -> Result<bool, String> {
    match flag {
        "--animation-base" => {
            if flags.explicit.animation_base {
                return Err("repeated --animation-base".into());
            }
            if flags.no_animation_base {
                return Err("--animation-base contradicts --no-animation-base".into());
            }
            flags.explicit.animation_base = true;
        }
        "--no-animation-base" => {
            if flags.no_animation_base {
                return Err("repeated --no-animation-base".into());
            }
            if flags.explicit.animation_base {
                return Err("--no-animation-base contradicts --animation-base".into());
            }
            flags.no_animation_base = true;
        }
        "--instance-rgba" => {
            if flags.explicit.instance_color.is_some() {
                return Err("repeated --instance-rgba".into());
            }
            if flags.no_instance_default {
                return Err("--instance-rgba contradicts --no-instance-default".into());
            }
            flags.explicit.instance_color =
                Some(parse_instance_rgba(next().ok_or("RGBA required")?)?);
        }
        "--no-instance-default" => {
            if flags.no_instance_default {
                return Err("repeated --no-instance-default".into());
            }
            if flags.explicit.instance_color.is_some() {
                return Err("--no-instance-default contradicts --instance-rgba".into());
            }
            flags.no_instance_default = true;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Scope suffix naming the active inputs in a fixed order: `+animation_base` (flag)
/// or `+animation_base_default`, then `+instance_rgba` (flag) or
/// `+instance_white_default`. Empty when both inputs are off.
pub fn preview_scope_suffix(options: &PreviewOptions, applied: AppliedDefaults) -> String {
    let mut s = String::new();
    if options.animation_base {
        s.push_str(if applied.animation_base {
            "+animation_base_default"
        } else {
            "+animation_base"
        });
    }
    if options.instance_color.is_some() {
        s.push_str(if applied.instance_white {
            "+instance_white_default"
        } else {
            "+instance_rgba"
        });
    }
    s
}

/// Evidence labels recorded with every D188 census (labels from docs/formats.md).
pub const DEFAULTS_EVIDENCE: [(&str, &str); 4] = [
    (
        "instance_white_value",
        "CONFIRMED_FROM_LOCAL_DATA: 96% of placed RENDER_MESH Things carry editorColor ffffffff (instance_color_census, docs/instance-color-census.md); CONFIRMED_FROM_COMMUNITY_CODE: Toolkit PRenderMesh editorColor default -1 = opaque white (docs/d80-thing-color.md E7)",
    ),
    (
        "editor_color_to_thing_color",
        "INFERRED: editorColor feeding the THING_COLOR shader input is not shown by a capture",
    ),
    (
        "animation_base",
        "preview policy: stored base values (t = base), no game time (D61 snapshot); game animation time UNKNOWN",
    ),
    (
        "materialless_group",
        "descriptor shape CONFIRMED_FROM_LOCAL_DATA (all-zero material and alternatives descriptors); game skips the colour draw (D192, Ghidra CONFIRMED; depth/shadow use INFERRED); Toolkit MeshExporter emits a default white material CONFIRMED_FROM_COMMUNITY_CODE (export only); excluded from the all-diffuse denominator, not drawn",
    ),
];

/// One primitive group's binding outcome for coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupBinding<'a> {
    Loaded,
    /// D188 material-less group: neither supported nor blocking.
    Materialless,
    Blocked(&'a str),
}
impl<'a> GroupBinding<'a> {
    /// From a binding result plus the D188 material-less classification.
    pub fn of<T>(binding: &'a Result<T, String>, materialless: bool) -> Self {
        match binding {
            Ok(_) => GroupBinding::Loaded,
            Err(_) if materialless => GroupBinding::Materialless,
            Err(e) => GroupBinding::Blocked(e),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshCoverage {
    pub geometry_available: bool,
    /// Nonempty groups that need a diffuse binding (material-less groups excluded).
    pub nonempty_groups: usize,
    pub supported_groups: usize,
    /// D188: nonempty material-less groups (not counted in `nonempty_groups`).
    pub materialless_groups: usize,
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

/// Coverage from group triangle ranges paired with binding outcomes.
pub fn coverage<'a>(
    geometry_available: bool,
    groups: impl IntoIterator<Item = (usize, GroupBinding<'a>)>,
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
        match binding {
            GroupBinding::Materialless => c.materialless_groups += 1,
            GroupBinding::Loaded => {
                c.nonempty_groups += 1;
                c.supported_groups += 1;
            }
            GroupBinding::Blocked(e) => {
                c.nonempty_groups += 1;
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
    /// D188: viewable meshes whose nonempty groups are all material-less.
    pub viewable_only_materialless: usize,
    pub nonempty_groups: usize,
    pub supported_groups: usize,
    /// D188: nonempty material-less groups over loaded meshes.
    pub materialless_groups: usize,
    /// D188: loaded mesh hashes with at least one material-less group.
    pub materialless_hashes: usize,
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
                self.materialless_groups = self
                    .materialless_groups
                    .checked_add(coverage.materialless_groups)
                    .ok_or("census group overflow")?;
                if coverage.materialless_groups > 0 {
                    inc(&mut self.materialless_hashes)?;
                }
                if !coverage.geometry_available {
                    return inc(&mut self.no_geometry_hashes);
                }
                inc(&mut self.viewable_hashes)?;
                self.viewable_guids.extend(guids.iter().cloned());
                if coverage.all_diffuse() {
                    inc(&mut self.all_diffuse_hashes)?;
                    self.all_diffuse_guids.extend(guids.iter().cloned());
                } else if coverage.nonempty_groups == 0 && coverage.materialless_groups > 0 {
                    inc(&mut self.viewable_only_materialless)?;
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
        use GroupBinding::{Blocked, Loaded};
        let (c, r) = coverage(
            true,
            [
                (3, Loaded),
                (0, Blocked("ignored")),
                (2, Blocked("Unsupported: x")),
            ],
        );
        assert_eq!((c.nonempty_groups, c.supported_groups), (2, 1));
        assert!(!c.all_diffuse());
        assert_eq!(r, guids(&["Unsupported: x"]));
        let (c, _) = coverage(true, [(0, Loaded)]);
        assert!(!c.all_diffuse()); // no nonempty group
        let (c, _) = coverage(false, [(1, Loaded)]);
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
            materialless_groups: 0,
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
    #[test]
    fn instance_rgba_parses_four_bounded_values() {
        let c = parse_instance_rgba("0.25,0.5,0.75,0.5").unwrap();
        assert_eq!(c.values(), [0.25, 0.5, 0.75, 0.5]);
        assert_eq!(
            parse_instance_rgba("0,0,0,1").unwrap().values(),
            [0., 0., 0., 1.]
        );
        for bad in [
            "",
            "0.1,0.2,0.3",
            "0.1,0.2,0.3,0.4,0.5",
            "a,0,0,0",
            "0, 0,0,0",
        ] {
            assert!(parse_instance_rgba(bad).is_err(), "{bad}");
        }
        for out in ["1.5,0,0,0", "-0.1,0,0,0", "NaN,0,0,0", "inf,0,0,0"] {
            assert!(parse_instance_rgba(out).is_err(), "{out}");
        }
    }
    #[test]
    fn preview_flags_default_override_and_scope_suffix() {
        // D188: no flags = white instance + animation base defaults, recorded.
        let flags = PreviewFlags::default();
        let (o, applied) = flags.resolve();
        assert!(o.animation_base);
        assert_eq!(o.instance_color, Some(DiffuseTint::default()));
        assert_eq!(o.instance_color.unwrap().values(), [1.; 4]);
        assert!(applied.instance_white && applied.animation_base);
        assert_eq!(
            preview_scope_suffix(&o, applied),
            "+animation_base_default+instance_white_default"
        );
        // Both opt-outs reproduce the pre-D188 source-only options.
        let mut f = PreviewFlags::default();
        assert!(apply_preview_flag(&mut f, "--no-instance-default", || None).unwrap());
        assert!(apply_preview_flag(&mut f, "--no-animation-base", || None).unwrap());
        let (o, applied) = f.resolve();
        assert!(!o.animation_base && o.instance_color.is_none() && !applied.any());
        assert_eq!(preview_scope_suffix(&o, applied), "");
        // Explicit values win and are not labelled as defaults.
        let mut f = PreviewFlags::default();
        assert!(!apply_preview_flag(&mut f, "--hashes", || None).unwrap());
        assert!(
            apply_preview_flag(&mut f, "--instance-rgba", || Some("0.25,0.5,0.75,0.5")).unwrap()
        );
        let (o, applied) = f.resolve();
        assert_eq!(
            o.instance_color.unwrap().bits,
            [0.25f32, 0.5, 0.75, 0.5].map(f32::to_bits)
        );
        assert!(!applied.instance_white && applied.animation_base);
        assert_eq!(
            preview_scope_suffix(&o, applied),
            "+animation_base_default+instance_rgba"
        );
        assert!(apply_preview_flag(&mut f, "--animation-base", || None).unwrap());
        let (o, applied) = f.resolve();
        assert_eq!(
            preview_scope_suffix(&o, applied),
            "+animation_base+instance_rgba"
        );
        // Repeats, contradictions and a missing value are errors, not overrides.
        assert!(apply_preview_flag(&mut f, "--animation-base", || None).is_err());
        assert!(apply_preview_flag(&mut f, "--instance-rgba", || Some("0,0,0,0")).is_err());
        assert!(apply_preview_flag(&mut f, "--no-animation-base", || None).is_err());
        assert!(apply_preview_flag(&mut f, "--no-instance-default", || None).is_err());
        let mut g = PreviewFlags::default();
        assert!(apply_preview_flag(&mut g, "--no-instance-default", || None).unwrap());
        assert!(apply_preview_flag(&mut g, "--no-instance-default", || None).is_err());
        assert!(apply_preview_flag(&mut g, "--instance-rgba", || Some("1,1,1,1")).is_err());
        assert!(apply_preview_flag(&mut g, "--no-animation-base", || None).unwrap());
        assert!(apply_preview_flag(&mut g, "--animation-base", || None).is_err());
        let mut fresh = PreviewFlags::default();
        assert!(apply_preview_flag(&mut fresh, "--instance-rgba", || None).is_err());
        assert!(apply_preview_flag(&mut fresh, "--instance-rgba", || Some("2,0,0,0")).is_err());
        assert!(fresh.explicit.instance_color.is_none());
    }
    #[test]
    fn materialless_groups_leave_the_denominator_and_are_tallied() {
        use GroupBinding::{Blocked, Loaded, Materialless};
        let absent: Result<(), String> = Err("Material-less".into());
        let unresolved: Result<(), String> = Err("Unresolved: GUID".into());
        assert_eq!(GroupBinding::of(&absent, true), Materialless);
        assert_eq!(
            GroupBinding::of(&unresolved, false),
            Blocked("Unresolved: GUID")
        );
        assert_eq!(GroupBinding::of(&Ok::<(), String>(()), true), Loaded);
        // One material-less group beside loaded groups: all-diffuse.
        let (c, r) = coverage(true, [(4, Materialless), (6, Loaded), (0, Materialless)]);
        assert_eq!(
            (c.nonempty_groups, c.supported_groups, c.materialless_groups),
            (1, 1, 1)
        );
        assert!(c.all_diffuse() && r.is_empty());
        // Any other unresolved group still blocks.
        let (c, r) = coverage(true, [(4, Materialless), (6, Blocked("Unresolved: GUID"))]);
        assert!(!c.all_diffuse());
        assert_eq!(r, guids(&["Unresolved: GUID"]));
        // Only material-less groups: not all-diffuse, tallied apart from no-group meshes.
        let (only, _) = coverage(true, [(4, Materialless)]);
        assert!(!only.all_diffuse());
        let mut t = CorpusTally::default();
        let loaded = |coverage| MeshOutcome::Loaded {
            coverage,
            neutral_reasons: BTreeSet::new(),
        };
        t.record(&guids(&["a"]), &loaded(only)).unwrap();
        let mixed = coverage(true, [(4, Materialless), (6, Loaded)]).0;
        t.record(&guids(&["b"]), &loaded(mixed)).unwrap();
        assert_eq!(t.viewable_only_materialless, 1);
        assert_eq!(t.viewable_without_groups, 0);
        assert_eq!((t.materialless_groups, t.materialless_hashes), (2, 2));
        assert_eq!(t.all_diffuse_guids, guids(&["b"]));
    }
}
