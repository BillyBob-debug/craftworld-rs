//! D93 measured corpus counts for in-app display.
//!
//! Reads the versioned summary JSON written by the project's own
//! `corpus_mesh_census` (schema 1 default options, schema 2 opt-in options) and
//! `corpus_texture_census` (schema 1) examples in `lbp-preview`. These are
//! project-authored diagnostic formats, not game formats: no game data is read
//! here and no counts are built in. Files are opened read-only, size-bounded
//! before parsing, and validated strictly; an unknown tool, schema, scope, unit
//! or outcome is rejected with an explicit message rather than guessed.
//! Text lines are produced here (UI-free) so both viewers render the same
//! wording; rendering code only maps line kinds to widgets.
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

/// Largest census file accepted (the 01.33 texture census is ~12 MB).
pub const MAX_CENSUS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MESH_TOOL: &str = "corpus_mesh_census";
pub const TEXTURE_TOOL: &str = "corpus_texture_census";
const MESH_CORPUS_SCOPE: &str = "corpus_all_mesh_map_rows";
const MESH_COHORT_PREFIX: &str = "cohort_manifest:";
const TEXTURE_CORPUS_SCOPE: &str = "corpus_all_texture_map_rows";
const GUID_UNIT: &str = "distinct_catalog_guid";
const HASH_UNIT: &str = "distinct_texture_hash";
/// Outcome keys the texture census schema 1 can emit (its `definitions`).
const TEXTURE_OUTCOMES: [&str; 8] = [
    "decoded",
    "unsupported",
    "malformed",
    "over_budget",
    "missing_file",
    "ambiguous",
    "blocked_protected",
    "read_failed",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeshCensusMode {
    Default,
    AnimationBase,
    InstanceColor,
    Both,
}

impl MeshCensusMode {
    pub const ALL: [MeshCensusMode; 4] = [
        MeshCensusMode::Default,
        MeshCensusMode::AnimationBase,
        MeshCensusMode::InstanceColor,
        MeshCensusMode::Both,
    ];
    pub fn label(self) -> &'static str {
        match self {
            MeshCensusMode::Default => "default options",
            MeshCensusMode::AnimationBase => "opt-in animation base",
            MeshCensusMode::InstanceColor => "opt-in supplied instance color",
            MeshCensusMode::Both => "opt-in animation base + instance color",
        }
    }
    fn from_suffix(suffix: &str) -> Option<Self> {
        Some(match suffix {
            "" => MeshCensusMode::Default,
            "+animation_base" => MeshCensusMode::AnimationBase,
            "+instance_rgba" => MeshCensusMode::InstanceColor,
            "+animation_base+instance_rgba" => MeshCensusMode::Both,
            _ => return None,
        })
    }
    fn animation_base(self) -> bool {
        matches!(self, MeshCensusMode::AnimationBase | MeshCensusMode::Both)
    }
    fn instance_color(self) -> bool {
        matches!(self, MeshCensusMode::InstanceColor | MeshCensusMode::Both)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshCensusSummary {
    pub file_name: String,
    pub schema_version: u64,
    pub scope: String,
    pub scope_note: String,
    /// `Some(manifest)` for a `cohort_manifest:` census; `None` for the corpus.
    pub cohort: Option<String>,
    pub mode: MeshCensusMode,
    pub supplied_instance_rgba: Option<[f32; 4]>,
    pub definitions: BTreeMap<String, String>,
    pub catalog_guids: u64,
    pub viewable_guids: u64,
    pub all_diffuse_guids: u64,
    pub complete_verified_guids: u64,
    pub mesh_hashes: u64,
    pub viewable_hashes: u64,
    pub all_diffuse_hashes: u64,
    pub failed_hashes: u64,
    pub excluded_unmapped_mesh_hashes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutcomeCount {
    pub hashes: u64,
    pub guids: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureCensusSummary {
    pub file_name: String,
    pub schema_version: u64,
    pub milestone: Option<String>,
    pub scope: String,
    pub scope_note: String,
    pub definitions: BTreeMap<String, String>,
    pub texture_hashes: u64,
    pub catalog_guids: u64,
    pub guids_with_mixed_outcomes: u64,
    pub excluded_unmapped_texture_hashes: u64,
    /// Outcome key (validated against the schema's outcome set) to counts.
    pub by_outcome: BTreeMap<String, OutcomeCount>,
}

impl TextureCensusSummary {
    pub fn outcome(&self, key: &str) -> OutcomeCount {
        self.by_outcome.get(key).copied().unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CensusFile {
    Mesh(MeshCensusSummary),
    Texture(TextureCensusSummary),
}

#[derive(Deserialize)]
struct Header {
    tool: Option<String>,
    schema_version: Option<u64>,
}

#[derive(Deserialize)]
struct RawMesh {
    scope: String,
    #[serde(default)]
    scope_note: String,
    asset_unit: String,
    #[serde(default)]
    definitions: BTreeMap<String, String>,
    excluded_unmapped_mesh_hashes: u64,
    totals: RawMeshTotals,
    preview_options: Option<RawPreviewOptions>,
}

#[derive(Deserialize)]
struct RawMeshTotals {
    catalog_guids: u64,
    viewable_guids: u64,
    all_diffuse_guids: u64,
    complete_verified_guids: u64,
    mesh_hashes: u64,
    viewable_hashes: u64,
    all_diffuse_hashes: u64,
    failed_hashes: u64,
}

#[derive(Deserialize)]
struct RawPreviewOptions {
    animation_base_enabled: bool,
    supplied_instance_rgba: Option<[f32; 4]>,
    verified_game_instance: bool,
}

#[derive(Deserialize)]
struct RawTexture {
    milestone: Option<String>,
    scope: String,
    #[serde(default)]
    scope_note: String,
    units: Vec<String>,
    #[serde(default)]
    definitions: BTreeMap<String, String>,
    excluded_unmapped_texture_hashes: u64,
    totals: RawTextureTotals,
    by_outcome: BTreeMap<String, RawOutcome>,
}

#[derive(Deserialize)]
struct RawTextureTotals {
    texture_hashes: u64,
    catalog_guids: u64,
    guids_with_mixed_outcomes: u64,
}

#[derive(Deserialize)]
struct RawOutcome {
    hashes: u64,
    guids: u64,
}

/// Open `path` read-only and parse it as a mesh or texture census summary.
pub fn load_census_file(path: &Path) -> Result<CensusFile, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let file = File::open(path).map_err(|e| format!("{name}: cannot open census file: {e}"))?;
    let length = file
        .metadata()
        .map_err(|e| format!("{name}: cannot read census file size: {e}"))?
        .len();
    if length > MAX_CENSUS_FILE_BYTES {
        return Err(format!(
            "{name}: census file is {length} bytes; limit is {MAX_CENSUS_FILE_BYTES}"
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CENSUS_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{name}: cannot read census file: {e}"))?;
    if bytes.len() as u64 > MAX_CENSUS_FILE_BYTES {
        return Err(format!("{name}: census file grew past the size limit"));
    }
    parse_census(&name, &bytes)
}

/// Parse census JSON bytes. `name` is only used in messages and the display.
pub fn parse_census(name: &str, bytes: &[u8]) -> Result<CensusFile, String> {
    if bytes.len() as u64 > MAX_CENSUS_FILE_BYTES {
        return Err(format!("{name}: census input exceeds the size limit"));
    }
    let header: Header =
        serde_json::from_slice(bytes).map_err(|e| format!("{name}: not census JSON: {e}"))?;
    let tool = header
        .tool
        .ok_or_else(|| format!("{name}: missing \"tool\"; not a census summary"))?;
    let schema = header
        .schema_version
        .ok_or_else(|| format!("{name}: missing \"schema_version\""))?;
    match tool.as_str() {
        MESH_TOOL => parse_mesh(name, schema, bytes).map(CensusFile::Mesh),
        TEXTURE_TOOL => parse_texture(name, schema, bytes).map(CensusFile::Texture),
        other => Err(format!(
            "{name}: unknown census tool {other:?} (expected {MESH_TOOL} or {TEXTURE_TOOL})"
        )),
    }
}

fn parse_mesh(name: &str, schema: u64, bytes: &[u8]) -> Result<MeshCensusSummary, String> {
    if !matches!(schema, 1 | 2) {
        return Err(format!(
            "{name}: unsupported {MESH_TOOL} schema_version {schema} (this viewer reads 1 and 2)"
        ));
    }
    let raw: RawMesh = serde_json::from_slice(bytes)
        .map_err(|e| format!("{name}: invalid {MESH_TOOL} schema {schema}: {e}"))?;
    if raw.asset_unit != GUID_UNIT {
        return Err(format!(
            "{name}: unknown asset_unit {:?} (expected {GUID_UNIT})",
            raw.asset_unit
        ));
    }
    let (cohort, rest) = if let Some(rest) = raw.scope.strip_prefix(MESH_COHORT_PREFIX) {
        let split = [
            "+animation_base+instance_rgba",
            "+animation_base",
            "+instance_rgba",
        ]
        .iter()
        .find_map(|s| rest.strip_suffix(s).map(|head| (head, &rest[head.len()..])))
        .unwrap_or((rest, ""));
        if split.0.is_empty() {
            return Err(format!("{name}: cohort scope without a manifest name"));
        }
        (Some(split.0.to_string()), split.1)
    } else if let Some(rest) = raw.scope.strip_prefix(MESH_CORPUS_SCOPE) {
        (None, rest)
    } else {
        return Err(format!("{name}: unknown mesh census scope {:?}", raw.scope));
    };
    let mode = MeshCensusMode::from_suffix(rest)
        .ok_or_else(|| format!("{name}: unknown mesh census scope {:?}", raw.scope))?;
    let mut rgba = None;
    match (schema, mode, &raw.preview_options) {
        (1, MeshCensusMode::Default, None) => {}
        (1, _, _) => {
            return Err(format!(
                "{name}: schema 1 describes default options only, but scope/options say otherwise"
            ))
        }
        (_, MeshCensusMode::Default, _) | (_, _, None) => {
            return Err(format!(
                "{name}: schema 2 needs an opt-in scope suffix and preview_options"
            ))
        }
        (_, _, Some(options)) => {
            if options.animation_base_enabled != mode.animation_base()
                || options.supplied_instance_rgba.is_some() != mode.instance_color()
            {
                return Err(format!(
                    "{name}: preview_options disagree with scope {:?}",
                    raw.scope
                ));
            }
            if options.verified_game_instance {
                return Err(format!(
                    "{name}: verified_game_instance=true is not a known census state"
                ));
            }
            if let Some(values) = options.supplied_instance_rgba {
                if values.iter().any(|v| !v.is_finite()) {
                    return Err(format!("{name}: non-finite supplied instance color"));
                }
                rgba = Some(values);
            }
        }
    }
    let t = &raw.totals;
    if t.complete_verified_guids > t.all_diffuse_guids
        || t.all_diffuse_guids > t.viewable_guids
        || t.viewable_guids > t.catalog_guids
        || t.all_diffuse_hashes > t.viewable_hashes
        || t.viewable_hashes > t.mesh_hashes
        || t.failed_hashes > t.mesh_hashes
        || t.mesh_hashes > t.catalog_guids
    {
        return Err(format!(
            "{name}: inconsistent totals (need complete <= all-diffuse <= viewable <= catalog GUIDs, and hash counts within mesh_hashes)"
        ));
    }
    Ok(MeshCensusSummary {
        file_name: name.to_string(),
        schema_version: schema,
        scope: raw.scope.clone(),
        scope_note: raw.scope_note,
        cohort,
        mode,
        supplied_instance_rgba: rgba,
        definitions: raw.definitions,
        catalog_guids: t.catalog_guids,
        viewable_guids: t.viewable_guids,
        all_diffuse_guids: t.all_diffuse_guids,
        complete_verified_guids: t.complete_verified_guids,
        mesh_hashes: t.mesh_hashes,
        viewable_hashes: t.viewable_hashes,
        all_diffuse_hashes: t.all_diffuse_hashes,
        failed_hashes: t.failed_hashes,
        excluded_unmapped_mesh_hashes: raw.excluded_unmapped_mesh_hashes,
    })
}

fn parse_texture(name: &str, schema: u64, bytes: &[u8]) -> Result<TextureCensusSummary, String> {
    if schema != 1 {
        return Err(format!(
            "{name}: unsupported {TEXTURE_TOOL} schema_version {schema} (this viewer reads 1)"
        ));
    }
    let raw: RawTexture = serde_json::from_slice(bytes)
        .map_err(|e| format!("{name}: invalid {TEXTURE_TOOL} schema 1: {e}"))?;
    if raw.scope != TEXTURE_CORPUS_SCOPE {
        return Err(format!(
            "{name}: unknown texture census scope {:?}",
            raw.scope
        ));
    }
    if raw.units.len() != 2
        || !raw.units.iter().any(|u| u == GUID_UNIT)
        || !raw.units.iter().any(|u| u == HASH_UNIT)
    {
        return Err(format!(
            "{name}: unknown units {:?} (expected {GUID_UNIT} and {HASH_UNIT})",
            raw.units
        ));
    }
    let t = &raw.totals;
    let mut hash_sum: u64 = 0;
    let mut by_outcome = BTreeMap::new();
    for (key, count) in raw.by_outcome {
        if !TEXTURE_OUTCOMES.contains(&key.as_str()) {
            return Err(format!("{name}: unknown texture outcome {key:?}"));
        }
        if count.hashes > count.guids || count.guids > t.catalog_guids {
            return Err(format!("{name}: inconsistent counts for outcome {key:?}"));
        }
        hash_sum = hash_sum
            .checked_add(count.hashes)
            .ok_or_else(|| format!("{name}: outcome hash counts overflow"))?;
        by_outcome.insert(
            key,
            OutcomeCount {
                hashes: count.hashes,
                guids: count.guids,
            },
        );
    }
    if hash_sum != t.texture_hashes
        || t.texture_hashes > t.catalog_guids
        || t.guids_with_mixed_outcomes > t.catalog_guids
    {
        return Err(format!(
            "{name}: outcome hashes sum to {hash_sum}, totals say {} of {} GUIDs",
            t.texture_hashes, t.catalog_guids
        ));
    }
    Ok(TextureCensusSummary {
        file_name: name.to_string(),
        schema_version: schema,
        milestone: raw.milestone,
        scope: raw.scope,
        scope_note: raw.scope_note,
        definitions: raw.definitions,
        texture_hashes: t.texture_hashes,
        catalog_guids: t.catalog_guids,
        guids_with_mixed_outcomes: t.guids_with_mixed_outcomes,
        excluded_unmapped_texture_hashes: raw.excluded_unmapped_texture_hashes,
        by_outcome,
    })
}

/// How a viewer should present a line (heading / count / explanatory note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Heading,
    Value,
    Note,
}

/// Census files supplied on the command line, at most one per mesh mode and one
/// texture census, each with an optional earlier baseline file.
#[derive(Clone, Debug, Default)]
pub struct CorpusCounts {
    pub mesh: BTreeMap<MeshCensusMode, MeshCensusSummary>,
    pub mesh_baseline: BTreeMap<MeshCensusMode, MeshCensusSummary>,
    pub texture: Option<TextureCensusSummary>,
    pub texture_baseline: Option<TextureCensusSummary>,
}

/// Which slot a command-line census file is meant for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CensusRole {
    Mesh,
    Texture,
    Baseline,
}

impl CorpusCounts {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty() && self.texture.is_none()
    }

    /// Load `path` and store it in the slot for `role`.
    pub fn load(&mut self, role: CensusRole, path: &Path) -> Result<(), String> {
        let file = load_census_file(path)?;
        self.insert(role, file)
    }

    pub fn insert(&mut self, role: CensusRole, file: CensusFile) -> Result<(), String> {
        match (role, file) {
            (CensusRole::Mesh, CensusFile::Mesh(m)) => {
                insert_mode(&mut self.mesh, m, "--mesh-census")
            }
            (CensusRole::Baseline, CensusFile::Mesh(m)) => {
                insert_mode(&mut self.mesh_baseline, m, "--census-baseline")
            }
            (CensusRole::Texture, CensusFile::Texture(t)) => {
                set_once(&mut self.texture, t, "--texture-census")
            }
            (CensusRole::Baseline, CensusFile::Texture(t)) => {
                set_once(&mut self.texture_baseline, t, "--census-baseline")
            }
            (CensusRole::Mesh, CensusFile::Texture(t)) => Err(format!(
                "{}: --mesh-census needs a {MESH_TOOL} file, got {TEXTURE_TOOL}",
                t.file_name
            )),
            (CensusRole::Texture, CensusFile::Mesh(m)) => Err(format!(
                "{}: --texture-census needs a {TEXTURE_TOOL} file, got {MESH_TOOL}",
                m.file_name
            )),
        }
    }

    /// Display lines for the corpus-count panel and headless printing.
    pub fn lines(&self) -> Vec<(LineKind, String)> {
        let mut out = Vec::new();
        let mut push = |kind, text: String| out.push((kind, text));
        push(
            LineKind::Note,
            "Measured offline by the census tools and read from the files given on the command line; not recomputed by this session.".into(),
        );
        if self.mesh.is_empty() {
            push(LineKind::Heading, "Mesh census".into());
            push(
                LineKind::Value,
                "Not measured: corpus mesh totals unknown (pass --mesh-census FILE).".into(),
            );
        }
        for mode in MeshCensusMode::ALL {
            let Some(m) = self.mesh.get(&mode) else {
                if !self.mesh.is_empty() {
                    push(LineKind::Heading, format!("Mesh census, {}", mode.label()));
                    push(
                        LineKind::Note,
                        "Not measured for this mode (no census file); totals unknown.".into(),
                    );
                }
                continue;
            };
            let scope_kind = match &m.cohort {
                Some(c) => format!("cohort {c}"),
                None => "whole corpus".into(),
            };
            push(
                LineKind::Heading,
                format!("Mesh census, {} ({scope_kind})", mode.label()),
            );
            let total = group(m.catalog_guids);
            push(
                LineKind::Value,
                format!(
                    "Viewable: {} of {total} mesh GUIDs",
                    group(m.viewable_guids)
                ),
            );
            push(
                LineKind::Value,
                format!(
                    "All-diffuse: {} of {total} mesh GUIDs",
                    group(m.all_diffuse_guids)
                ),
            );
            push(
                LineKind::Value,
                format!(
                    "Complete (visually verified): {} of {total} mesh GUIDs",
                    group(m.complete_verified_guids)
                ),
            );
            if let Some(rgba) = m.supplied_instance_rgba {
                push(
                    LineKind::Note,
                    format!(
                        "Supplied instance color (linear RGBA) {}, {}, {}, {}; not a verified game instance.",
                        rgba[0], rgba[1], rgba[2], rgba[3]
                    ),
                );
            }
            push(
                LineKind::Note,
                format!(
                    "Unit: distinct catalog GUID ({} content hashes; {} failed loads; {} unmapped mesh hashes excluded).",
                    group(m.mesh_hashes),
                    group(m.failed_hashes),
                    group(m.excluded_unmapped_mesh_hashes)
                ),
            );
            push(
                LineKind::Note,
                format!("Scope: {}. {}", m.scope, m.scope_note),
            );
            push(
                LineKind::Note,
                format!(
                    "File: {} ({MESH_TOOL} schema {}).",
                    m.file_name, m.schema_version
                ),
            );
            push(LineKind::Note, self.mesh_baseline_line(m));
        }
        if let Some(m) = self.mesh.values().next() {
            for key in ["viewable", "all_diffuse", "complete"] {
                if let Some(text) = m.definitions.get(key) {
                    push(LineKind::Note, format!("{key}: {text}"));
                }
            }
        }
        push(LineKind::Heading, "Texture census".into());
        match &self.texture {
            None => push(
                LineKind::Value,
                "Not measured: corpus texture totals unknown (pass --texture-census FILE).".into(),
            ),
            Some(t) => {
                let decoded = t.outcome("decoded");
                push(
                    LineKind::Value,
                    format!(
                        "Decoded: {} of {} texture hashes ({} of {} GUIDs)",
                        group(decoded.hashes),
                        group(t.texture_hashes),
                        group(decoded.guids),
                        group(t.catalog_guids)
                    ),
                );
                for key in TEXTURE_OUTCOMES.iter().skip(1) {
                    let count = t.outcome(key);
                    if count.hashes > 0 || *key == "unsupported" {
                        push(
                            LineKind::Value,
                            format!(
                                "{}: {} hashes ({} GUIDs)",
                                outcome_label(key),
                                group(count.hashes),
                                group(count.guids)
                            ),
                        );
                    }
                }
                push(
                    LineKind::Note,
                    format!(
                        "Units: distinct texture hash and distinct catalog GUID ({} GUIDs with mixed outcomes; {} unmapped texture hashes excluded).",
                        group(t.guids_with_mixed_outcomes),
                        group(t.excluded_unmapped_texture_hashes)
                    ),
                );
                if let Some(text) = t.definitions.get("decoded") {
                    push(LineKind::Note, format!("decoded: {text}"));
                }
                push(
                    LineKind::Note,
                    format!("Scope: {}. {}", t.scope, t.scope_note),
                );
                push(
                    LineKind::Note,
                    format!(
                        "File: {} ({TEXTURE_TOOL} schema {}{}).",
                        t.file_name,
                        t.schema_version,
                        t.milestone
                            .as_ref()
                            .map(|m| format!(", {m}"))
                            .unwrap_or_default()
                    ),
                );
                push(LineKind::Note, self.texture_baseline_line(t));
            }
        }
        out
    }

    fn mesh_baseline_line(&self, m: &MeshCensusSummary) -> String {
        if let Some(b) = self.mesh_baseline.get(&m.mode) {
            if b.scope != m.scope || b.catalog_guids != m.catalog_guids {
                return format!(
                    "Baseline {}: scope or GUID total differs ({}, {} GUIDs); no delta computed.",
                    b.file_name,
                    b.scope,
                    group(b.catalog_guids)
                );
            }
            return format!(
                "Baseline {}: {} viewable / {} all-diffuse / {} Complete; change {} / {} / {}.",
                b.file_name,
                group(b.viewable_guids),
                group(b.all_diffuse_guids),
                group(b.complete_verified_guids),
                delta(m.viewable_guids, b.viewable_guids),
                delta(m.all_diffuse_guids, b.all_diffuse_guids),
                delta(m.complete_verified_guids, b.complete_verified_guids)
            );
        }
        if m.mode != MeshCensusMode::Default {
            if let Some(d) = self.mesh.get(&MeshCensusMode::Default) {
                if d.cohort == m.cohort && d.catalog_guids == m.catalog_guids {
                    return format!(
                        "Baseline: default-options census {}; change {} viewable / {} all-diffuse / {} Complete.",
                        d.file_name,
                        delta(m.viewable_guids, d.viewable_guids),
                        delta(m.all_diffuse_guids, d.all_diffuse_guids),
                        delta(m.complete_verified_guids, d.complete_verified_guids)
                    );
                }
            }
        }
        "Baseline: none supplied (pass an earlier census with --census-baseline FILE).".into()
    }

    fn texture_baseline_line(&self, t: &TextureCensusSummary) -> String {
        match &self.texture_baseline {
            None => "Baseline: none supplied (pass an earlier census with --census-baseline FILE)."
                .into(),
            Some(b) if b.scope != t.scope || b.catalog_guids != t.catalog_guids => format!(
                "Baseline {}: scope or GUID total differs ({}, {} GUIDs); no delta computed.",
                b.file_name,
                b.scope,
                group(b.catalog_guids)
            ),
            Some(b) => {
                let (now, then) = (t.outcome("decoded"), b.outcome("decoded"));
                format!(
                    "Baseline {}: {} decoded hashes ({} GUIDs); change {} hashes / {} GUIDs.",
                    b.file_name,
                    group(then.hashes),
                    group(then.guids),
                    delta(now.hashes, then.hashes),
                    delta(now.guids, then.guids)
                )
            }
        }
    }
}

fn insert_mode(
    slots: &mut BTreeMap<MeshCensusMode, MeshCensusSummary>,
    m: MeshCensusSummary,
    flag: &str,
) -> Result<(), String> {
    if let Some(old) = slots.get(&m.mode) {
        return Err(format!(
            "{flag}: {} and {} both describe the {} mode",
            old.file_name,
            m.file_name,
            m.mode.label()
        ));
    }
    slots.insert(m.mode, m);
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{flag}: only one texture census file is accepted"));
    }
    *slot = Some(value);
    Ok(())
}

fn outcome_label(key: &str) -> &str {
    match key {
        "unsupported" => "Unsupported",
        "malformed" => "Malformed",
        "over_budget" => "Over budget",
        "missing_file" => "Missing file",
        "ambiguous" => "Ambiguous",
        "blocked_protected" => "Blocked (protected)",
        "read_failed" => "Read failed",
        other => other,
    }
}

/// Signed difference without overflow (`+12`, `-3`, `+0`).
fn delta(now: u64, then: u64) -> String {
    if now >= then {
        format!("+{}", group(now - then))
    } else {
        format!("-{}", group(then - now))
    }
}

/// Thousands separators for display (`4261` -> `4,261`).
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Map a viewer flag (`--mesh-census`, `--texture-census`, `--census-baseline`)
/// to its slot; viewers then call [`CorpusCounts::load`] with the flag's path.
pub fn census_flag(flag: &str) -> Option<CensusRole> {
    match flag {
        "--mesh-census" => Some(CensusRole::Mesh),
        "--texture-census" => Some(CensusRole::Texture),
        "--census-baseline" => Some(CensusRole::Baseline),
        _ => None,
    }
}

/// Help text shared by both viewers.
pub const CENSUS_FLAGS_HELP: &str = "--mesh-census FILE (repeatable, one per mode), --texture-census FILE, --census-baseline FILE: show measured corpus counts from corpus_mesh_census / corpus_texture_census JSON";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn mesh(scope: &str, schema: u64, options: Option<Value>, viewable: u64, all: u64) -> Value {
        let mut v = json!({
            "schema_version": schema,
            "tool": MESH_TOOL,
            "scope": scope,
            "scope_note": "synthetic note",
            "asset_unit": GUID_UNIT,
            "definitions": {"viewable": "v-def", "all_diffuse": "a-def", "complete": "c-def"},
            "excluded_unmapped_mesh_hashes": 3,
            "totals": {"catalog_guids": 10, "viewable_guids": viewable, "all_diffuse_guids": all,
                "complete_verified_guids": 0, "mesh_hashes": 9, "viewable_hashes": 8,
                "all_diffuse_hashes": 6, "failed_hashes": 1, "no_geometry_hashes": 0},
            "meshes": [{"sha1": "00", "guids": ["1"], "status": "all_diffuse"}]
        });
        if let Some(o) = options {
            v["preview_options"] = o;
        }
        v
    }
    fn opts(anim: bool, rgba: Option<[f32; 4]>) -> Option<Value> {
        Some(
            json!({"animation_base_enabled": anim, "animation_time": null,
            "supplied_instance_color_bits": null, "supplied_instance_rgba": rgba,
            "verified_game_instance": false}),
        )
    }
    fn texture() -> Value {
        json!({
            "schema_version": 1, "tool": TEXTURE_TOOL, "milestone": "D90",
            "scope": TEXTURE_CORPUS_SCOPE, "scope_note": "tex note",
            "units": [GUID_UNIT, HASH_UNIT],
            "definitions": {"decoded": "top-mip decode"},
            "excluded_unmapped_texture_hashes": 2,
            "totals": {"texture_hashes": 7, "catalog_guids": 9, "guids_with_mixed_outcomes": 0},
            "by_outcome": {"decoded": {"hashes": 5, "guids": 6}, "unsupported": {"hashes": 2, "guids": 3}},
            "textures": []
        })
    }
    fn parse(v: &Value) -> Result<CensusFile, String> {
        parse_census("synthetic.json", &serde_json::to_vec(v).unwrap())
    }
    fn parse_mesh_ok(v: &Value) -> MeshCensusSummary {
        match parse(v).unwrap() {
            CensusFile::Mesh(m) => m,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mesh_modes_follow_schema_and_scope() {
        let d = parse_mesh_ok(&mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6));
        assert_eq!(
            (d.mode, d.cohort.clone(), d.viewable_guids),
            (MeshCensusMode::Default, None, 8)
        );
        let a = parse_mesh_ok(&mesh(
            "corpus_all_mesh_map_rows+animation_base",
            2,
            opts(true, None),
            8,
            7,
        ));
        assert_eq!(a.mode, MeshCensusMode::AnimationBase);
        let rgba = [0.25, 0.5, 0.75, 0.5];
        let i = parse_mesh_ok(&mesh(
            "corpus_all_mesh_map_rows+instance_rgba",
            2,
            opts(false, Some(rgba)),
            8,
            7,
        ));
        assert_eq!(
            (i.mode, i.supplied_instance_rgba),
            (MeshCensusMode::InstanceColor, Some(rgba))
        );
        let b = parse_mesh_ok(&mesh(
            "cohort_manifest:d61.json+animation_base+instance_rgba",
            2,
            opts(true, Some(rgba)),
            8,
            7,
        ));
        assert_eq!(
            (b.mode, b.cohort.as_deref()),
            (MeshCensusMode::Both, Some("d61.json"))
        );
    }

    #[test]
    fn rejects_unknown_schema_tool_scope_and_inconsistent_mesh_totals() {
        let e = parse(&mesh(MESH_CORPUS_SCOPE, 3, None, 8, 6)).unwrap_err();
        assert!(
            e.contains("unsupported corpus_mesh_census schema_version 3"),
            "{e}"
        );
        let mut v = mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6);
        v["tool"] = json!("other_tool");
        assert!(parse(&v).unwrap_err().contains("unknown census tool"));
        v.as_object_mut().unwrap().remove("tool");
        assert!(parse(&v).unwrap_err().contains("missing \"tool\""));
        assert!(parse(&mesh("something_else", 1, None, 8, 6))
            .unwrap_err()
            .contains("unknown mesh census scope"));
        assert!(parse(&mesh(
            "corpus_all_mesh_map_rows+weird",
            2,
            opts(true, None),
            8,
            6
        ))
        .is_err());
        // Schema 1 must not carry opt-in options; schema 2 must.
        assert!(parse(&mesh(
            "corpus_all_mesh_map_rows+animation_base",
            1,
            None,
            8,
            6
        ))
        .is_err());
        assert!(parse(&mesh(MESH_CORPUS_SCOPE, 2, opts(false, None), 8, 6)).is_err());
        assert!(parse(&mesh(
            "corpus_all_mesh_map_rows+animation_base",
            2,
            None,
            8,
            6
        ))
        .is_err());
        // Options must agree with the scope suffix.
        assert!(parse(&mesh(
            "corpus_all_mesh_map_rows+animation_base",
            2,
            opts(false, Some([1.; 4])),
            8,
            6
        ))
        .is_err());
        // all-diffuse above viewable, viewable above catalog.
        assert!(parse(&mesh(MESH_CORPUS_SCOPE, 1, None, 8, 9))
            .unwrap_err()
            .contains("inconsistent"));
        assert!(parse(&mesh(MESH_CORPUS_SCOPE, 1, None, 11, 6)).is_err());
        let mut unit = mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6);
        unit["asset_unit"] = json!("hash");
        assert!(parse(&unit).unwrap_err().contains("asset_unit"));
        let mut missing = mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6);
        missing["totals"]
            .as_object_mut()
            .unwrap()
            .remove("viewable_guids");
        assert!(parse(&missing).is_err());
        assert!(parse_census("x", b"[1,2]").is_err());
        assert!(parse_census("x", b"{not json").is_err());
    }

    #[test]
    fn texture_summary_is_validated() {
        let t = match parse(&texture()).unwrap() {
            CensusFile::Texture(t) => t,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            t.outcome("decoded"),
            OutcomeCount {
                hashes: 5,
                guids: 6
            }
        );
        assert_eq!(t.outcome("malformed"), OutcomeCount::default());
        let mut v = texture();
        v["schema_version"] = json!(2);
        assert!(parse(&v).unwrap_err().contains("schema_version 2"));
        let mut v = texture();
        v["by_outcome"]["mystery"] = json!({"hashes": 0, "guids": 0});
        assert!(parse(&v).unwrap_err().contains("unknown texture outcome"));
        let mut v = texture();
        v["totals"]["texture_hashes"] = json!(8);
        assert!(parse(&v).is_err());
        let mut v = texture();
        v["units"] = json!([GUID_UNIT]);
        assert!(parse(&v).unwrap_err().contains("units"));
        let mut v = texture();
        v["scope"] = json!("cohort");
        assert!(parse(&v).unwrap_err().contains("scope"));
    }

    #[test]
    fn size_limit_is_enforced_before_parsing() {
        let big = vec![b' '; (MAX_CENSUS_FILE_BYTES + 1) as usize];
        assert!(parse_census("big", &big)
            .unwrap_err()
            .contains("size limit"));
    }

    #[test]
    fn lines_report_unknown_when_no_file_given() {
        let lines = CorpusCounts::default().lines();
        let text: Vec<&str> = lines.iter().map(|(_, s)| s.as_str()).collect();
        assert!(text
            .iter()
            .any(|s| s.contains("corpus mesh totals unknown")));
        assert!(text
            .iter()
            .any(|s| s.contains("corpus texture totals unknown")));
        assert!(!text.iter().any(|s| s.contains("Viewable:")));
    }

    #[test]
    fn lines_show_counts_units_scope_and_baselines() {
        let mut c = CorpusCounts::default();
        let d = parse(&mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6)).unwrap();
        c.insert(CensusRole::Mesh, d.clone()).unwrap();
        assert!(c
            .insert(CensusRole::Mesh, d)
            .unwrap_err()
            .contains("both describe"));
        let a = parse(&mesh(
            "corpus_all_mesh_map_rows+animation_base",
            2,
            opts(true, None),
            8,
            7,
        ))
        .unwrap();
        c.insert(CensusRole::Mesh, a).unwrap();
        let older = parse(&mesh(MESH_CORPUS_SCOPE, 1, None, 7, 4)).unwrap();
        c.insert(CensusRole::Baseline, older).unwrap();
        assert!(c
            .insert(
                CensusRole::Texture,
                parse(&mesh(MESH_CORPUS_SCOPE, 1, None, 8, 6)).unwrap()
            )
            .is_err());
        c.insert(CensusRole::Texture, parse(&texture()).unwrap())
            .unwrap();
        let text: Vec<String> = c.lines().into_iter().map(|(_, s)| s).collect();
        let has = |needle: &str| text.iter().any(|s| s.contains(needle));
        assert!(has("Viewable: 8 of 10 mesh GUIDs"));
        assert!(has("All-diffuse: 6 of 10 mesh GUIDs"));
        assert!(has("Complete (visually verified): 0 of 10 mesh GUIDs"));
        assert!(has(
            "Unit: distinct catalog GUID (9 content hashes; 1 failed loads; 3 unmapped"
        ));
        assert!(has("Scope: corpus_all_mesh_map_rows. synthetic note"));
        assert!(has("File: synthetic.json (corpus_mesh_census schema 1)"));
        assert!(has("change +1 / +2 / +0"));
        assert!(has(
            "default-options census synthetic.json; change +0 viewable / +1 all-diffuse"
        ));
        assert!(has("Not measured for this mode"));
        assert!(has("Decoded: 5 of 7 texture hashes (6 of 9 GUIDs)"));
        assert!(has("Unsupported: 2 hashes (3 GUIDs)"));
        assert!(has("corpus_texture_census schema 1, D90"));
        assert!(has("viewable: v-def"));
    }

    #[test]
    fn grouping_and_delta() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(4261), "4,261");
        assert_eq!(group(1_234_567), "1,234,567");
        assert_eq!(delta(3, 5), "-2");
        assert_eq!(delta(u64::MAX, 0), format!("+{}", group(u64::MAX)));
    }

    /// Headless check against real local census files when they exist (never
    /// committed; skipped elsewhere). LBP_CENSUS_FILES lists them, `;`-separated.
    #[test]
    fn local_census_files_parse_when_present() {
        let Ok(list) = std::env::var("LBP_CENSUS_FILES") else {
            return;
        };
        let mut c = CorpusCounts::default();
        for path in list.split(';').filter(|p| !p.is_empty()) {
            let file = load_census_file(Path::new(path)).unwrap();
            let role = match file {
                CensusFile::Mesh(_) => CensusRole::Mesh,
                CensusFile::Texture(_) => CensusRole::Texture,
            };
            c.insert(role, file).unwrap();
        }
        for (_, line) in c.lines() {
            println!("{line}");
        }
    }
}
