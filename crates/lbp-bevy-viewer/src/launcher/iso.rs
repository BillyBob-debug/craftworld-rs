//! D600: reads the player's LBP2 ISO (ISO 9660, as RPCS3 boots it) and copies the files the
//! viewer reads (PARAM.SFO, the FARCs, the MAP and the loose game data) into the launcher's
//! own folder, leaving out videos, audio and the extras' packages. The ISO is only read.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

const SECTOR: u64 = 2048;
/// Kinds the viewer never reads: Bink and MP4 video, FMOD and MP3 audio, packages, firmware.
const SKIPPED: &[&str] = &["BIK", "MP4", "FSB", "MP3", "PKG", "PUP"];
/// The disc this version reads (USA, 01.00 on the disc; other regions come later).
pub const TITLE_ID: &str = "BCUS98245";

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub path: String,
    pub lba: u32,
    pub size: u64,
}

fn read_at(f: &mut File, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Lists every file under the ISO's root (primary volume descriptor names, versions cut).
pub fn list(f: &mut File) -> Result<Vec<Entry>, String> {
    let pvd = read_at(f, 16 * SECTOR, SECTOR as usize).map_err(|e| format!("Read ISO: {e}"))?;
    if &pvd[1..6] != b"CD001" || pvd[0] != 1 {
        return Err("That file isn't an ISO 9660 disc image.".into());
    }
    let mut out = Vec::new();
    let root = pvd[156..190].to_vec();
    walk(f, &root, "", &mut out, 0)?;
    Ok(out)
}

fn walk(
    f: &mut File,
    rec: &[u8],
    prefix: &str,
    out: &mut Vec<Entry>,
    depth: u32,
) -> Result<(), String> {
    if depth > 32 {
        return Err("ISO directories nest too deeply.".into());
    }
    let lba = u32::from_le_bytes(rec[2..6].try_into().unwrap()) as u64;
    let size = u32::from_le_bytes(rec[10..14].try_into().unwrap()) as usize;
    if size > 16 << 20 {
        return Err("An ISO directory is too large.".into());
    }
    let data = read_at(f, lba * SECTOR, size).map_err(|e| format!("Read ISO: {e}"))?;
    let mut i = 0;
    while i < data.len() {
        let len = data[i] as usize;
        if len == 0 {
            // Records never cross a sector; the rest of this one is padding.
            i = (i / SECTOR as usize + 1) * SECTOR as usize;
            continue;
        }
        if len < 34 || i + len > data.len() {
            return Err("A damaged ISO directory record.".into());
        }
        let r = &data[i..i + len];
        let name_len = r[32] as usize;
        let flags = r[25];
        let name = &r[33..(33 + name_len).min(len)];
        if name != [0] && name != [1] {
            let name = String::from_utf8_lossy(name);
            let name = name.split(';').next().unwrap_or_default();
            let path = format!("{prefix}/{name}");
            if flags & 2 != 0 {
                walk(f, r, &path, out, depth + 1)?;
            } else {
                let lba = u32::from_le_bytes(r[2..6].try_into().unwrap());
                let size = u32::from_le_bytes(r[10..14].try_into().unwrap()) as u64;
                // A file over 4 GiB is several records of one name (multi-extent, flag 0x80):
                // the extents follow each other on the disc, so the first one's start holds.
                match out.last_mut() {
                    Some(last) if last.path == path => last.size += size,
                    _ => out.push(Entry { path, lba, size }),
                }
            }
        }
        i += len;
    }
    Ok(())
}

/// The files the viewer needs: the game folder minus the kinds it never reads.
pub fn wanted(entries: &[Entry]) -> Vec<&Entry> {
    entries
        .iter()
        .filter(|e| e.path.starts_with("/PS3_GAME/"))
        .filter(|e| !e.path.starts_with("/PS3_GAME/USRDIR/GAMEDATA/VIDEOS/"))
        .filter(|e| {
            let ext = e.path.rsplit_once('.').map(|(_, x)| x.to_ascii_uppercase());
            !ext.is_some_and(|x| SKIPPED.contains(&x.as_str()))
        })
        .collect()
}

/// Bytes of `PS3_GAME/PARAM.SFO`, to check the title before copying.
pub fn param_sfo(f: &mut File, entries: &[Entry]) -> Result<Vec<u8>, String> {
    let e = entries
        .iter()
        .find(|e| e.path == "/PS3_GAME/PARAM.SFO")
        .ok_or("That ISO has no PS3_GAME/PARAM.SFO; it isn't a PS3 game disc.")?;
    if e.size > 1 << 20 {
        return Err("PARAM.SFO is too large.".into());
    }
    read_at(f, e.lba as u64 * SECTOR, e.size as usize).map_err(|e| format!("Read ISO: {e}"))
}

/// Copies the wanted files under `dest`, counting copied bytes into `done`.
pub fn extract(
    iso: &Path,
    dest: &Path,
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
) -> Result<(), String> {
    let mut f = File::open(iso).map_err(|e| format!("Open the ISO: {e}"))?;
    let entries = list(&mut f)?;
    let sfo = param_sfo(&mut f, &entries)?;
    if !contains(&sfo, TITLE_ID.as_bytes()) {
        return Err(format!(
            "That ISO isn't LittleBigPlanet 2 {TITLE_ID} (USA). Other regions come later."
        ));
    }
    let files = wanted(&entries);
    total.store(files.iter().map(|e| e.size).sum(), Ordering::Relaxed);
    let _ = std::fs::remove_dir_all(dest);
    let mut buf = vec![0u8; 4 << 20];
    for e in files {
        let rel: PathBuf = e.path.trim_start_matches('/').split('/').collect();
        let out_path = dest.join(rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|x| format!("Create {}: {x}", parent.display()))?;
        }
        let mut out =
            File::create(&out_path).map_err(|x| format!("Write {}: {x}", out_path.display()))?;
        f.seek(SeekFrom::Start(e.lba as u64 * SECTOR))
            .map_err(|x| format!("Read ISO: {x}"))?;
        let mut left = e.size;
        while left > 0 {
            let n = left.min(buf.len() as u64) as usize;
            f.read_exact(&mut buf[..n])
                .map_err(|x| format!("Read ISO {}: {x}", e.path))?;
            out.write_all(&buf[..n])
                .map_err(|x| format!("Write {}: {x}", out_path.display()))?;
            left -= n as u64;
            done.fetch_add(n as u64, Ordering::Relaxed);
        }
    }
    Ok(())
}

pub fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &[u8], lba: u32, size: u32, dir: bool) -> Vec<u8> {
        let len = 33 + name.len() + (1 - name.len() % 2);
        let mut r = vec![0u8; len];
        r[0] = len as u8;
        r[2..6].copy_from_slice(&lba.to_le_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }

    /// A tiny image: root at sector 18 with PS3_GAME (sector 19) holding PARAM.SFO and a
    /// video, written to a temporary file and listed back.
    #[test]
    fn lists_and_filters_a_small_image() {
        let mut img = vec![0u8; 24 * SECTOR as usize];
        let s = |n: usize| n * SECTOR as usize;
        img[s(16)] = 1;
        img[s(16) + 1..s(16) + 6].copy_from_slice(b"CD001");
        let root = record(&[0], 18, SECTOR as u32, true);
        img[s(16) + 156..s(16) + 156 + root.len()].copy_from_slice(&root);
        let mut dir = record(&[0], 18, 2048, true);
        dir.extend(record(&[1], 18, 2048, true));
        dir.extend(record(b"PS3_GAME", 19, 2048, true));
        img[s(18)..s(18) + dir.len()].copy_from_slice(&dir);
        let mut game = record(&[0], 19, 2048, true);
        game.extend(record(&[1], 18, 2048, true));
        game.extend(record(b"PARAM.SFO;1", 20, 9, false));
        game.extend(record(b"INTRO.BIK;1", 21, 4, false));
        img[s(19)..s(19) + game.len()].copy_from_slice(&game);
        img[s(20)..s(20) + 9].copy_from_slice(b"BCUS98245");
        let path = std::env::temp_dir().join(format!("lbp-iso-test-{}.iso", std::process::id()));
        std::fs::write(&path, &img).unwrap();
        let mut f = File::open(&path).unwrap();
        let entries = list(&mut f).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, ["/PS3_GAME/PARAM.SFO", "/PS3_GAME/INTRO.BIK"]);
        let kept: Vec<&str> = wanted(&entries).iter().map(|e| e.path.as_str()).collect();
        assert_eq!(kept, ["/PS3_GAME/PARAM.SFO"]);
        assert!(contains(
            &param_sfo(&mut f, &entries).unwrap(),
            TITLE_ID.as_bytes()
        ));
        drop(f);
        let _ = std::fs::remove_file(&path);
    }
}
