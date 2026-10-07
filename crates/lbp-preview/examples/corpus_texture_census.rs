//! D90: read-only corpus-wide texture decode census; output stays under ignored
//! `local-analysis/`, no pixels are written anywhere.
//! Scope: every MAP row whose magic-confirmed type is `texture`, grouped by content
//! hash, resolved with `lbp_preview::resolve` and decoded once per hash with the
//! existing `lbp_texture::decode_checked` (top mip, CPU RGBA, discarded at once).
//! Outcomes apply to every GUID naming the hash (units: distinct catalog GUID and
//! distinct texture hash). Extension-hinted rows whose magic is not texture are
//! excluded and counted by confirmed type; texture inspections without a MAP row are
//! excluded and counted.
//! Format labels come from an independent wrapper probe (GTF raw format byte; TEX DDS
//! FourCC/RGB bit count from the first expanded 128 bytes). Wrapper layouts:
//! CONFIRMED_FROM_COMMUNITY_CODE, ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! SerializedResource::process, CellGcmTexture constructor, Compressor::decompressData;
//! DDS_HEADER: CONFIRMED_FROM_DOCUMENTATION (Microsoft). See docs/textures.md and
//! docs/d90-texture-census.md. The probe only labels; it never decides an outcome.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

const MAX_TEXTURE_HASHES: usize = 50_000;
const DEFAULT_MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TOTAL_BYTES_LIMIT: u64 = 16 * 1024 * 1024 * 1024;
const PAGE: usize = 4096;
/// One zlib chunk expands to at most u16::MAX bytes; the probe needs 128 DDS bytes.
const PROBE_DDS_BYTES: usize = 128;
const PROBE_MAX_CHUNKS: usize = 16;

/// Label-only wrapper metadata. Never used to decide an outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Probe {
    format: String,
    width: Option<u32>,
    height: Option<u32>,
    mips: Option<u32>,
    layout: Option<&'static str>,
}

fn be16(b: &[u8], p: usize) -> usize {
    u16::from_be_bytes([b[p], b[p + 1]]) as usize
}

fn unknown(format: String) -> Probe {
    Probe {
        format,
        width: None,
        height: None,
        mips: None,
        layout: None,
    }
}

/// First `PROBE_DDS_BYTES` of a TEX expanded payload (compression envelope at byte 4),
/// with checked chunk bounds and strict zlib ends. `None` when not determinable.
fn tex_prefix(b: &[u8]) -> Option<Vec<u8>> {
    let header = b.get(4..8)?;
    let (flag, count) = (be16(header, 0), be16(header, 2));
    if flag > 1 {
        return None;
    }
    let table_end = 8usize.checked_add(count.checked_mul(4)?)?;
    let table = b.get(8..table_end)?;
    if count == 0 {
        return b.get(8..8 + PROBE_DDS_BYTES).map(<[u8]>::to_vec);
    }
    let mut out = Vec::with_capacity(PROBE_DDS_BYTES);
    let mut offset = table_end;
    for row in table.as_chunks::<4>().0.iter().take(PROBE_MAX_CHUNKS) {
        let (s, e) = (be16(row, 0), be16(row, 2));
        let end = offset.checked_add(s)?;
        let chunk = b.get(offset..end)?;
        offset = end;
        if s == e {
            out.extend_from_slice(chunk);
        } else {
            let mut z = flate2::Decompress::new(true);
            let mut decoded = vec![0; e + 1];
            let status = z
                .decompress(chunk, &mut decoded, flate2::FlushDecompress::Finish)
                .ok()?;
            if status != flate2::Status::StreamEnd
                || z.total_in() != s as u64
                || z.total_out() != e as u64
            {
                return None;
            }
            out.extend_from_slice(&decoded[..e]);
        }
        if out.len() >= PROBE_DDS_BYTES {
            out.truncate(PROBE_DDS_BYTES);
            return Some(out);
        }
    }
    None
}

fn probe(b: &[u8]) -> Probe {
    let Some(magic) = b.get(..4) else {
        return unknown("truncated".into());
    };
    match magic {
        b"GTF " if b.len() >= 32 => {
            let (dimension, cube, depth) = (b[6], b[7], be16(b, 16));
            Probe {
                format: format!("GTF 0x{:02x}", b[4]),
                width: Some(be16(b, 12) as u32),
                height: Some(be16(b, 14) as u32),
                mips: Some(u32::from(b[5])),
                layout: Some(if cube != 0 {
                    "cube"
                } else if dimension > 2 || depth > 1 {
                    "volume"
                } else {
                    "2d"
                }),
            }
        }
        b"TEX " => match tex_prefix(b) {
            Some(d) if &d[..4] == b"DDS " => {
                let le = |p: usize| u32::from_le_bytes(d[p..p + 4].try_into().unwrap());
                let fourcc = &d[84..88];
                let format = if fourcc == [0; 4] {
                    format!("TEX/DDS RGB{} flags 0x{:x}", le(88), le(80))
                } else {
                    format!("TEX/DDS {}", String::from_utf8_lossy(fourcc))
                };
                Probe {
                    format,
                    width: Some(le(16)),
                    height: Some(le(12)),
                    mips: Some(le(28)),
                    layout: Some(if le(112) != 0 {
                        "cube"
                    } else if le(24) > 1 {
                        "volume"
                    } else {
                        "2d"
                    }),
                }
            }
            Some(_) => unknown("TEX non-DDS payload".into()),
            None => unknown("TEX prefix not determinable".into()),
        },
        b"GTF " => unknown("GTF truncated".into()),
        m => unknown(format!(
            "magic {:02x}{:02x}{:02x}{:02x}",
            m[0], m[1], m[2], m[3]
        )),
    }
}

/// Census outcome classes; reasons are the decoder's or resolver's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Decoded,
    Unsupported(String),
    Blocked,
    Malformed(String),
    OverBudget(String),
    MissingFile(String),
    Ambiguous(String),
    ReadFailed(String),
}

impl Outcome {
    fn key(&self) -> &'static str {
        match self {
            Self::Decoded => "decoded",
            Self::Unsupported(_) => "unsupported",
            Self::Blocked => "blocked_protected",
            Self::Malformed(_) => "malformed",
            Self::OverBudget(_) => "over_budget",
            Self::MissingFile(_) => "missing_file",
            Self::Ambiguous(_) => "ambiguous",
            Self::ReadFailed(_) => "read_failed",
        }
    }
    fn reason(&self) -> Option<&str> {
        match self {
            Self::Decoded => None,
            Self::Blocked => Some("protected resource"),
            Self::Unsupported(r)
            | Self::Malformed(r)
            | Self::OverBudget(r)
            | Self::MissingFile(r)
            | Self::Ambiguous(r)
            | Self::ReadFailed(r) => Some(r),
        }
    }
    fn from_decoder(e: &lbp_texture::Error) -> Self {
        use lbp_texture::Error as E;
        match e {
            E::Blocked => Self::Blocked,
            E::Unsupported(r) => Self::Unsupported((*r).into()),
            E::Invalid(r) => Self::Malformed((*r).into()),
            E::Budget => Self::OverBudget("decoder input/expanded/pixel budget".into()),
            E::Cancelled => Self::ReadFailed("cancelled".into()),
            E::Input(r) => Self::ReadFailed(r.clone()),
        }
    }
    fn from_resolve(e: &str) -> Self {
        if e.starts_with("Ambiguous") {
            Self::Ambiguous(e.into())
        } else {
            Self::MissingFile(e.into())
        }
    }
}

fn size_bucket(bytes: u64) -> &'static str {
    const KIB: u64 = 1024;
    match bytes {
        0..=4095 => "<4KiB",
        4096..=16383 => "4-16KiB",
        16384..=65535 => "16-64KiB",
        65536..=262_143 => "64-256KiB",
        262_144..=1_048_575 => "256KiB-1MiB",
        _ if bytes < 4 * KIB * KIB => "1-4MiB",
        _ if bytes < 16 * KIB * KIB => "4-16MiB",
        _ => ">=16MiB",
    }
}

fn dimension_bucket(width: Option<u32>, height: Option<u32>) -> &'static str {
    match width.zip(height).map(|(w, h)| w.max(h)) {
        None => "unknown",
        Some(0) => "zero",
        Some(1..=16) => "<=16",
        Some(17..=64) => "17-64",
        Some(65..=256) => "65-256",
        Some(257..=1024) => "257-1024",
        Some(1025..=4096) => "1025-4096",
        Some(_) => ">4096",
    }
}

#[derive(Default)]
struct Count {
    hashes: usize,
    guids: BTreeSet<String>,
}

impl Count {
    fn add(&mut self, guids: &BTreeSet<String>) {
        self.hashes += 1;
        self.guids.extend(guids.iter().cloned());
    }
    fn json(&self) -> Value {
        json!({"hashes": self.hashes, "guids": self.guids.len()})
    }
}

#[derive(Default)]
struct Tally {
    all: Count,
    by_outcome: BTreeMap<&'static str, Count>,
    by_reason: BTreeMap<(&'static str, String), Count>,
    by_format: BTreeMap<String, BTreeMap<&'static str, Count>>,
    by_size: BTreeMap<&'static str, BTreeMap<&'static str, Count>>,
    by_dimension: BTreeMap<&'static str, BTreeMap<&'static str, Count>>,
    /// GUIDs mapping to several hashes may land in several outcomes; counted once each.
    guid_outcomes: BTreeMap<String, BTreeSet<&'static str>>,
}

impl Tally {
    fn record(
        &mut self,
        guids: &BTreeSet<String>,
        outcome: &Outcome,
        format: &str,
        size: &'static str,
        dimension: &'static str,
    ) {
        let key = outcome.key();
        self.all.add(guids);
        self.by_outcome.entry(key).or_default().add(guids);
        if let Some(r) = outcome.reason() {
            self.by_reason
                .entry((key, r.to_owned()))
                .or_default()
                .add(guids);
        }
        for map in [
            self.by_format.entry(format.to_owned()).or_default(),
            self.by_size.entry(size).or_default(),
            self.by_dimension.entry(dimension).or_default(),
        ] {
            map.entry(key).or_default().add(guids);
        }
        for g in guids {
            self.guid_outcomes.entry(g.clone()).or_default().insert(key);
        }
    }
    fn mixed_outcome_guids(&self) -> usize {
        self.guid_outcomes.values().filter(|s| s.len() > 1).count()
    }
    fn nested(map: &BTreeMap<impl ToString + Ord, BTreeMap<&'static str, Count>>) -> Value {
        map.iter()
            .map(|(k, v)| {
                let inner: serde_json::Map<String, Value> =
                    v.iter().map(|(o, c)| (o.to_string(), c.json())).collect();
                (k.to_string(), Value::Object(inner))
            })
            .collect::<serde_json::Map<_, _>>()
            .into()
    }
}

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
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

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        return Err(
            "corpus_texture_census SNAPSHOT GAME-DIR NEW-JSON [--max-seconds N] [--max-total-bytes N]"
                .into(),
        );
    }
    let output = Path::new(&args[3]);
    let mut max_seconds = 7200u64;
    let mut max_total_bytes = DEFAULT_MAX_TOTAL_BYTES;
    let mut flags = args[4..].iter();
    while let Some(flag) = flags.next() {
        let value = flags.next().ok_or("option value required")?;
        match flag.as_str() {
            "--max-seconds" => {
                max_seconds = value.parse().map_err(|_| "invalid seconds")?;
                if max_seconds == 0 || max_seconds > 86_400 {
                    return Err("time budget must be 1..=86400 s".into());
                }
            }
            "--max-total-bytes" => {
                max_total_bytes = value.parse().map_err(|_| "invalid byte budget")?;
                if max_total_bytes == 0 || max_total_bytes > MAX_TOTAL_BYTES_LIMIT {
                    return Err("byte budget must be 1..=16 GiB".into());
                }
            }
            _ => return Err("unknown option".into()),
        }
    }
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new ignored output required".into());
    }
    let started = Instant::now();
    let deadline = Duration::from_secs(max_seconds);
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;

    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut map_rows = 0usize;
    let mut excluded_hint_only: BTreeMap<String, usize> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        let hinted = r.type_hint.as_ref().is_some_and(|h| h.name == "texture");
        if r.display_type() != "texture" {
            if hinted {
                *excluded_hint_only
                    .entry(r.display_type().to_owned())
                    .or_default() += 1;
            }
            continue;
        }
        map_rows += 1;
        by_hash
            .entry(r.map_row.sha1.to_ascii_lowercase())
            .or_default()
            .insert(r.map_row.guid.to_ascii_lowercase());
    }
    let unmapped: BTreeSet<String> = catalog
        .snapshot
        .resource_inspections
        .iter()
        .filter(|l| {
            l.inspection
                .confirmed_type
                .as_ref()
                .is_some_and(|k| k.name == "texture")
        })
        .filter_map(|l| l.sha1.as_deref().map(str::to_ascii_lowercase))
        .filter(|sha| !catalog.by_hash.contains_key(sha))
        .collect();
    if by_hash.is_empty() || by_hash.len() > MAX_TEXTURE_HASHES {
        return Err("texture hash budget".into());
    }

    let mut tally = Tally::default();
    let mut rows = Vec::with_capacity(by_hash.len());
    let mut bytes_read = 0u64;
    let mut decoded_expanded = 0u64;
    let mut decoded_with_tail = 0usize;
    let mut decoded_no_srgb = 0usize;
    let mut decoded_pixel_formats: BTreeMap<String, usize> = BTreeMap::new();
    for (n, (sha, guids)) in by_hash.iter().enumerate() {
        control.check()?;
        if started.elapsed() > deadline {
            return Err(format!("time budget exceeded after {n} texture hashes"));
        }
        let mut probed = unknown("not read".into());
        let mut archived = None;
        let mut decoded = None;
        let outcome = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Err(e) => Outcome::from_resolve(&e),
            Ok(key) => {
                archived = Some(key.size);
                if key.size > lbp_texture::MAX_INPUT as u64 {
                    Outcome::OverBudget("archived size exceeds per-texture input budget".into())
                } else {
                    bytes_read = bytes_read
                        .checked_add(key.size)
                        .ok_or("byte counter overflow")?;
                    if bytes_read > max_total_bytes {
                        return Err(format!(
                            "total byte budget exceeded after {n} texture hashes"
                        ));
                    }
                    match read_all(&source, &key, &control) {
                        Err(e) => Outcome::ReadFailed(e),
                        Ok(bytes) => {
                            probed = probe(&bytes);
                            match lbp_texture::decode_checked(&bytes, || control.check().is_err()) {
                                Err(e) => Outcome::from_decoder(&e),
                                Ok(t) => {
                                    decoded = Some((
                                        t.header.pixel_format.to_string(),
                                        t.header.width,
                                        t.header.height,
                                        t.expanded_bytes,
                                        t.unparsed_after_top_mip,
                                        t.header.no_srgb,
                                    ));
                                    Outcome::Decoded
                                }
                            }
                        }
                    }
                }
            }
        };
        if let Some((pf, _, _, expanded, tail, no_srgb)) = &decoded {
            *decoded_pixel_formats.entry(pf.clone()).or_default() += 1;
            decoded_expanded = decoded_expanded
                .checked_add(*expanded as u64)
                .ok_or("expanded counter overflow")?;
            decoded_with_tail += usize::from(*tail > 0);
            decoded_no_srgb += usize::from(*no_srgb);
        }
        let size = archived.map_or("unresolved", size_bucket);
        let dimension = dimension_bucket(probed.width, probed.height);
        tally.record(guids, &outcome, &probed.format, size, dimension);
        let mut row = json!({
            "sha1": sha, "guids": guids, "outcome": outcome.key(), "reason": outcome.reason(),
            "format": probed.format, "layout": probed.layout, "width": probed.width,
            "height": probed.height, "mips": probed.mips, "archived_bytes": archived
        });
        if let Some((pf, w, h, expanded, tail, no_srgb)) = decoded {
            row["decoded"] = json!({"pixel_format": pf, "width": w, "height": h,
                "expanded_bytes": expanded, "unparsed_after_top_mip": tail, "no_srgb": no_srgb});
        }
        rows.push(row);
    }
    source.check_stable()?;
    let reasons: Vec<Value> = tally
        .by_reason
        .iter()
        .map(|((o, r), c)| json!({"outcome": o, "reason": r, "hashes": c.hashes, "guids": c.guids.len()}))
        .collect();
    let outcomes: serde_json::Map<String, Value> = tally
        .by_outcome
        .iter()
        .map(|(k, c)| (k.to_string(), c.json()))
        .collect();
    let report = json!({
        "schema_version": 1,
        "tool": "corpus_texture_census",
        "milestone": "D90",
        "scope": "corpus_all_texture_map_rows",
        "scope_note": "MAP rows with magic-confirmed type texture (snapshot), one resolve and top-mip decode per content hash with lbp_texture::decode_checked; pixels discarded",
        "units": ["distinct_catalog_guid", "distinct_texture_hash"],
        "definitions": {
            "decoded": "decoder returned a top-mip RGBA image (decodability, not visual verification; higher mips not decoded)",
            "unsupported": "decoder refused a format/layout it does not implement",
            "malformed": "decoder reported invalid structure (chunk/zlib/DDS/size)",
            "over_budget": "per-texture input/expanded/pixel budget refused",
            "missing_file": "MAP row bytes unresolved or unavailable",
            "ambiguous": "conflicting candidates for the hash",
            "blocked_protected": "protected marker; not read further",
            "read_failed": "approved source read failed",
            "format": "label from an independent wrapper probe; never decides an outcome"
        },
        "budgets": {
            "max_texture_hashes": MAX_TEXTURE_HASHES,
            "max_total_archived_bytes": max_total_bytes,
            "per_texture_input_bytes": lbp_texture::MAX_INPUT,
            "per_texture_expanded_bytes": lbp_texture::MAX_EXPANDED,
            "per_texture_pixels": lbp_texture::MAX_PIXELS,
            "max_seconds": max_seconds
        },
        "snapshot_texture_map_rows": map_rows,
        "excluded_hint_only_rows_by_confirmed_type": excluded_hint_only,
        "excluded_unmapped_texture_hashes": unmapped.len(),
        "totals": {
            "texture_hashes": tally.all.hashes,
            "catalog_guids": tally.all.guids.len(),
            "guids_with_mixed_outcomes": tally.mixed_outcome_guids(),
            "archived_bytes_read": bytes_read,
            "decoded_expanded_bytes": decoded_expanded,
            "decoded_with_unparsed_tail_hashes": decoded_with_tail,
            "decoded_no_srgb_hashes": decoded_no_srgb
        },
        "by_outcome": outcomes,
        "by_outcome_reason": reasons,
        "decoded_pixel_formats_hashes": decoded_pixel_formats,
        "by_format": Tally::nested(&tally.by_format),
        "by_archived_size": Tally::nested(&tally.by_size),
        "by_max_dimension": Tally::nested(&tally.by_dimension),
        "textures": rows
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    let summary: Vec<String> = tally
        .by_outcome
        .iter()
        .map(|(k, c)| format!("{k} {}/{}", c.hashes, c.guids.len()))
        .collect();
    println!(
        "{} texture hashes / {} GUIDs (hashes/GUIDs): {}; {} bytes read; {:.0} s",
        tally.all.hashes,
        tally.all.guids.len(),
        summary.join(", "),
        bytes_read,
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gtf(format: u8, dimension: u8, cube: u8, depth: u16) -> Vec<u8> {
        let mut b = vec![0; 40];
        b[..4].copy_from_slice(b"GTF ");
        b[4] = format;
        b[5] = 3;
        b[6] = dimension;
        b[7] = cube;
        b[12..14].copy_from_slice(&64u16.to_be_bytes());
        b[14..16].copy_from_slice(&32u16.to_be_bytes());
        b[16..18].copy_from_slice(&depth.to_be_bytes());
        b
    }

    fn dds(fourcc: &[u8; 4], rgb_bits: u32) -> Vec<u8> {
        let mut d = vec![0; 128];
        d[..4].copy_from_slice(b"DDS ");
        d[4] = 124;
        d[76] = 32;
        d[12..16].copy_from_slice(&8u32.to_le_bytes());
        d[16..20].copy_from_slice(&16u32.to_le_bytes());
        d[28..32].copy_from_slice(&5u32.to_le_bytes());
        d[84..88].copy_from_slice(fourcc);
        d[88..92].copy_from_slice(&rgb_bits.to_le_bytes());
        d
    }

    #[test]
    fn probe_gtf_labels_and_layouts() {
        let p = probe(&gtf(0x85, 2, 0, 1));
        assert_eq!(p.format, "GTF 0x85");
        assert_eq!((p.width, p.height, p.mips), (Some(64), Some(32), Some(3)));
        assert_eq!(p.layout, Some("2d"));
        assert_eq!(probe(&gtf(0x86, 2, 1, 1)).layout, Some("cube"));
        assert_eq!(probe(&gtf(0x86, 3, 0, 16)).layout, Some("volume"));
        assert_eq!(probe(&gtf(0x86, 2, 0, 1)[..20]).format, "GTF truncated");
        assert_eq!(probe(b"\xff\xd8\xff\xe0").format, "magic ffd8ffe0");
        assert_eq!(probe(b"TE").format, "truncated");
    }

    #[test]
    fn probe_tex_raw_and_zlib_prefix() {
        // Count zero: raw remainder.
        let mut raw = b"TEX \0\x01\0\0".to_vec();
        raw.extend(dds(b"DXT5", 0));
        let p = probe(&raw);
        assert_eq!(p.format, "TEX/DDS DXT5");
        assert_eq!((p.width, p.height, p.mips), (Some(16), Some(8), Some(5)));
        // Uncompressed DDS split over a zlib chunk then a stored chunk.
        let d = dds(&[0; 4], 32);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&d[..100]).unwrap();
        let first = z.finish().unwrap();
        let mut b = b"TEX \0\x01\0\x02".to_vec();
        b.extend((first.len() as u16).to_be_bytes());
        b.extend(100u16.to_be_bytes());
        b.extend(28u16.to_be_bytes());
        b.extend(28u16.to_be_bytes());
        b.extend(&first);
        b.extend(&d[100..]);
        assert_eq!(probe(&b).format, "TEX/DDS RGB32 flags 0x0");
        // Wrong advertised expansion: strict zlib end check fails, label only.
        let mut bad = b.clone();
        bad[10..12].copy_from_slice(&99u16.to_be_bytes());
        assert_eq!(probe(&bad).format, "TEX prefix not determinable");
        // Chunk table past the end.
        assert_eq!(
            probe(b"TEX \0\x01\x7f\xff").format,
            "TEX prefix not determinable"
        );
    }

    #[test]
    fn outcome_classes_from_decoder_and_resolver() {
        use lbp_texture::Error as E;
        assert_eq!(
            Outcome::from_decoder(&E::Blocked).key(),
            "blocked_protected"
        );
        let u = Outcome::from_decoder(&E::Unsupported("pixel format or layout flags"));
        assert_eq!(
            (u.key(), u.reason()),
            ("unsupported", Some("pixel format or layout flags"))
        );
        assert_eq!(
            Outcome::from_decoder(&E::Invalid("zlib stream")).key(),
            "malformed"
        );
        assert_eq!(Outcome::from_decoder(&E::Budget).key(), "over_budget");
        assert_eq!(
            Outcome::from_resolve("Unresolved: linked bytes unavailable").key(),
            "missing_file"
        );
        assert_eq!(
            Outcome::from_resolve("Ambiguous: conflicting linked resource candidates").key(),
            "ambiguous"
        );
        // Real end-to-end: the existing decoder refuses an authored GTF 0x85.
        let e = lbp_texture::decode(&gtf(0x85, 2, 0, 1)).unwrap_err();
        assert_eq!(Outcome::from_decoder(&e).key(), "unsupported");
    }

    #[test]
    fn buckets_are_closed_ranges() {
        assert_eq!(size_bucket(0), "<4KiB");
        assert_eq!(size_bucket(4095), "<4KiB");
        assert_eq!(size_bucket(4096), "4-16KiB");
        assert_eq!(size_bucket(1_048_576), "1-4MiB");
        assert_eq!(size_bucket(16 * 1024 * 1024), ">=16MiB");
        assert_eq!(dimension_bucket(None, Some(4)), "unknown");
        assert_eq!(dimension_bucket(Some(16), Some(17)), "17-64");
        assert_eq!(dimension_bucket(Some(4096), Some(1)), "1025-4096");
        assert_eq!(dimension_bucket(Some(4097), Some(1)), ">4096");
    }

    #[test]
    fn tally_counts_hashes_and_distinct_guids() {
        let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
        let mut t = Tally::default();
        t.record(
            &set(&["a", "b"]),
            &Outcome::Decoded,
            "GTF 0x86",
            "<4KiB",
            "<=16",
        );
        t.record(
            &set(&["b"]),
            &Outcome::Unsupported("x".into()),
            "GTF 0x85",
            "<4KiB",
            "<=16",
        );
        t.record(&set(&["c"]), &Outcome::Decoded, "GTF 0x86", "<4KiB", "<=16");
        assert_eq!((t.all.hashes, t.all.guids.len()), (3, 3));
        assert_eq!(t.by_outcome["decoded"].hashes, 2);
        assert_eq!(t.by_outcome["decoded"].guids.len(), 3);
        assert_eq!(t.mixed_outcome_guids(), 1);
        assert_eq!(t.by_format["GTF 0x86"]["decoded"].guids.len(), 3);
        assert_eq!(t.by_reason[&("unsupported", "x".into())].hashes, 1);
        let v = Tally::nested(&t.by_size);
        assert_eq!(v["<4KiB"]["unsupported"]["guids"], 1);
    }
}
