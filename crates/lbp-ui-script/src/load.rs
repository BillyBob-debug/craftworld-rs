//! Loading the user's scripts for the Gooey host: every magic-confirmed `script` MAP row of
//! a catalog, grouped by content hash and keyed by its MAP GUIDs, so the VM can resolve
//! superclasses and GUID type references (U12).
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_level::script::{self, Script};
use lbp_level::Limits;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const MAX_FILES: usize = 10_000;
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const PAGE: usize = 4096;

/// Scripts with their GUID (if known), and the count not read.
pub type Loaded = (Vec<(Option<u32>, Script)>, usize);

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size as u64 > MAX_FILE_BYTES {
        return Err("archived size exceeds input budget".into());
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

/// Every magic-confirmed `script` MAP row, grouped by content hash; one script per GUID
/// (a hash under several GUIDs is loaded once per GUID).
pub fn from_catalog(snapshot: &Path, game_dir: &Path) -> Result<Loaded, String> {
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(snapshot, &control)?;
    let source = AttachedSource::attach(game_dir, &catalog, &control)?;
    load(&source, &catalog, &control)
}

/// [`from_catalog`] over an already attached catalog.
pub fn load(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<Loaded, String> {
    let mut by_hash: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "script" {
            let e = by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default();
            if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
                e.insert(g);
            }
        }
    }
    if by_hash.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let (mut out, mut not_read, mut total) = (Vec::new(), 0, 0u64);
    for (sha, guids) in &by_hash {
        control.check()?;
        let parsed = lbp_preview::resolve(catalog, None, Some(sha)).and_then(|key| {
            total += key.size;
            if total > MAX_TOTAL_BYTES {
                return Err("total byte budget".to_owned());
            }
            read_all(source, &key, control)
        });
        let Some(s) = parsed.ok().as_deref().and_then(|b| {
            script::read(b, Limits::default(), &|| control.check().is_err())
                .ok()
                .and_then(|r| r.result.ok())
        }) else {
            not_read += 1;
            continue;
        };
        if guids.is_empty() {
            out.push((None, s));
        } else {
            for g in guids {
                out.push((Some(*g), s.clone()));
            }
        }
    }
    Ok((out, not_read))
}
