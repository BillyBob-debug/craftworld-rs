//! Entirely authored directory/metadata recipes. No original asset bytes.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey, HEX_PAGE_BYTES},
    Catalog, LoadControl, Snapshot,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn hash(n: u8) -> String {
    format!("{n:02x}").repeat(20)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fixture(root: &Path) -> Snapshot {
    let game = root.join("PS3_GAME");
    fs::create_dir_all(game.join("USRDIR/OUTPUT")).unwrap();
    let mut sfo = b"\0PSF".to_vec();
    for n in [0x101_u32, 36, 48, 1] {
        sfo.extend(n.to_le_bytes());
    }
    sfo.extend(0_u16.to_le_bytes());
    sfo.extend(0x204_u16.to_le_bytes());
    for n in [10_u32, 10, 0] {
        sfo.extend(n.to_le_bytes());
    }
    sfo.extend(b"TITLE_ID\0");
    sfo.resize(48, 0);
    sfo.extend(b"SYNTH0001\0");
    fs::write(game.join("PARAM.SFO"), &sfo).unwrap();
    let mut map = 0x100_u32.to_be_bytes().to_vec();
    map.extend(5_u32.to_be_bytes());
    let definitions = [
        ("a.raw", 1_u8, 1_u32, 10050_u32),
        ("alias.raw", 1, 2, 10050),
        ("loose.raw", 3, 3, 9),
        ("conflict.raw", 5, 4, 10050),
        ("../escape", 6, 5, 1),
    ];
    for (path, h, guid, size) in definitions {
        map.extend((path.len() as u32).to_be_bytes());
        map.extend(path.as_bytes());
        map.extend(0_i64.to_be_bytes());
        map.extend(size.to_be_bytes());
        map.extend([h; 20]);
        map.extend(guid.to_be_bytes());
    }
    fs::write(game.join("USRDIR/OUTPUT/BLURAYGUIDS.MAP"), &map).unwrap();
    let mut farc = vec![b'X'; 10050];
    farc[..4].copy_from_slice(b"TEST");
    farc.extend(b"NPD\0protected synthetic");
    let protected_size = (farc.len() - 10050) as u32;
    let orphan_offset = farc.len() as u32;
    farc.extend(b"ORPHAN!");
    let table = farc.len() as u64;
    let entries = [
        (1_u8, 0_u32, 10050_u32),
        (2, 10050, protected_size),
        (4, orphan_offset, 7),
        (5, 0, 10050),
    ];
    for (h, offset, size) in entries {
        farc.extend([h; 20]);
        farc.extend(offset.to_be_bytes());
        farc.extend(size.to_be_bytes());
    }
    farc.extend(4_u32.to_be_bytes());
    farc.extend(b"FARC");
    fs::write(game.join("USRDIR/DATA.FARC"), &farc).unwrap();
    fs::write(game.join("USRDIR/loose.raw"), b"LOOSE1234").unwrap();
    let rows:Vec<_>=definitions.iter().enumerate().map(|(i,(path,h,guid,size))| {
        let index=match i {0|1=>Some(0),3=>Some(3),_=>None};
        json!({"map_row":{"row_index":i,"path":path,"timestamp_raw":0,"declared_size":size,"sha1":hash(*h),"guid":format!("{guid:08x}")},
            "type_hint":null,"confirmed_type":null,"header_status":if i==4 {"missing"}else{"not_inspected"},"dependency_status":"not_inspected","map_path_status":"relative_path_candidate",
            "resolution":if index.is_some(){"archive_candidate"}else if i==2{"loose_candidate"}else{"missing_in_selected_directory"},
            "archive_candidates":index.map(|e|vec![json!({"archive":"USRDIR/DATA.FARC","entry_index":e,"offset":0,"size":10050,"declared_size_matches":true,"inspection_index":null})]).unwrap_or_default(),
            "loose_candidates":if i==2 {vec![json!({"path":"USRDIR/loose.raw","size":9,"declared_size_matches":true,"integrity":"not_verified","inspection_index":null})]}else{vec![]}})
    }).collect();
    serde_json::from_value(json!({"schema_version":2,"milestone":"A1_directory_tables","partial_metadata":false,
        "build":lbp_io::sfo::read_identity(&sfo).unwrap(),"build_fingerprint":{"path":"PARAM.SFO","size":sfo.len(),"sha256":digest(&sfo)},
        "map":{"path":"USRDIR/OUTPUT/BLURAYGUIDS.MAP","revision":256,"row_count":5,"sha256":digest(&map)},
        "source_scope":"synthetic","payload_integrity":"not_verified","counts":{"map_rows":5,"archive_entries":4,"archives":1,"dependency_entries":0},
        "metadata_status_counts":{},"confirmed_type_counts":{},"dependency_resolution_counts":{},"dependency_links":[],"resource_inspections":[],"resources":rows,
        "archives":[{"path":"USRDIR/DATA.FARC","file_size":farc.len(),"table_offset":table,"entry_count":4,"duplicate_hash_rows":0,
            "entries":entries.iter().enumerate().map(|(i,(h,offset,size))| json!({"entry_index":i,"sha1":hash(*h),"offset":offset,"size":size})).collect::<Vec<_>>()}]})).unwrap()
}
fn catalog(s: Snapshot) -> Catalog {
    Catalog::build(s, &LoadControl::default()).unwrap()
}
fn key(c: &Catalog, e: usize) -> LocationKey {
    let a = &c.snapshot.archives[0];
    let row = &a.entries[e];
    LocationKey {
        path: a.path.clone(),
        offset: row.offset,
        size: u64::from(row.size),
    }
}

#[test]
fn source_binding_and_hex_preserve_aliases_conflicts_orphans_and_input_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let c = catalog(fixture(temp.path()));
    let path = temp.path().join("PS3_GAME/USRDIR/DATA.FARC");
    let before = fs::read(&path).unwrap();
    let attached = AttachedSource::attach(temp.path(), &c, &LoadControl::default()).unwrap();
    assert_eq!(attached.entry_count, 4);
    assert_eq!(c.by_hash[&hash(1)], vec![0, 1]);
    let k = key(&c, 0);
    let first = attached.read_hex(&k, 0, HEX_PAGE_BYTES).unwrap();
    assert_eq!(first.bytes.len(), 4096);
    assert_eq!(&first.bytes[..4], b"TEST");
    assert_eq!(attached.read_hex(&k, 10048, 4096).unwrap().bytes, b"XX");
    assert!(attached.read_hex(&k, 10051, 1).is_err());
    assert!(attached.read_hex(&k, 0, 4097).is_err());
    assert_eq!(attached.read_hex(&k, 10050, 1).unwrap().bytes.len(), 0);
    assert_eq!(attached.read_hex(&key(&c, 3), 0, 4).unwrap().bytes, b"TEST");
    assert_eq!(
        attached.read_hex(&key(&c, 2), 0, 4096).unwrap().bytes,
        b"ORPHAN!"
    );
    assert!(attached
        .read_hex(&key(&c, 1), 8, 4)
        .err()
        .unwrap()
        .contains("Protected marker"));
    assert_eq!(
        attached
            .read_hex(
                &LocationKey {
                    path: "USRDIR/loose.raw".into(),
                    offset: 0,
                    size: 9
                },
                0,
                4096
            )
            .unwrap()
            .bytes,
        b"LOOSE1234"
    );
    for malicious in [
        LocationKey {
            path: "../outside".into(),
            offset: 0,
            size: 9,
        },
        LocationKey {
            path: k.path.clone(),
            offset: u64::MAX,
            size: 10050,
        },
    ] {
        assert!(attached.read_hex(&malicious, 0, 1).is_err());
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        AttachedSource::attach(&temp.path().join("PS3_GAME"), &c, &LoadControl::default()).is_ok()
    );
}

#[test]
fn attachment_rejects_changed_fingerprints_fields_ranges_and_redirection() {
    for case in 0..6 {
        let temp = tempfile::tempdir().unwrap();
        let mut s = fixture(temp.path());
        match case {
            0 => s.build_fingerprint.as_mut().unwrap().sha256 = "0".repeat(64),
            1 => s.map.as_mut().unwrap().sha256 = "0".repeat(64),
            2 => s.resources[0].map_row.path = "different.raw".into(),
            3 => s.archives[0].entries[0].offset = 1,
            4 => s.resources[0].archive_candidates[0].offset = 1,
            _ => s.resources[2].loose_candidates[0].path = "../outside".into(),
        }
        let c = catalog(s);
        assert!(
            AttachedSource::attach(temp.path(), &c, &LoadControl::default()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn changed_files_and_cancellation_prevent_further_pages() {
    let temp = tempfile::tempdir().unwrap();
    let c = catalog(fixture(temp.path()));
    let attached = AttachedSource::attach(temp.path(), &c, &LoadControl::default()).unwrap();
    let control = LoadControl::default();
    control.cancel();
    assert!(AttachedSource::attach(temp.path(), &c, &control).is_err());
    assert!(attached
        .read_hex_checked(&key(&c, 0), 0, 4, &control)
        .err()
        .unwrap()
        .contains("cancelled"));
    fs::write(
        temp.path().join("PS3_GAME/USRDIR/OUTPUT/BLURAYGUIDS.MAP"),
        b"changed",
    )
    .unwrap();
    assert!(attached
        .read_hex(&key(&c, 0), 0, 4)
        .err()
        .unwrap()
        .contains("changed"));
}

#[test]
fn known_protected_inspection_stops_without_opening_source_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut s = fixture(temp.path());
    let e = &s.archives[0].entries[1];
    s.resource_inspections.push(serde_json::from_value(json!({"source_path":"USRDIR/DATA.FARC","offset":e.offset,"size":e.size,"sha1":e.sha1,
        "inspection":{"status":"blocked_encrypted","reason":"authored protected marker","magic_hex":"4e504400","confirmed_type":null,"serialization":"protected","header":null,
            "dependency_status":"not_inspected","dependency_count_raw":null,"dependencies":[],"metadata_bytes_read":4}})).unwrap());
    let c = catalog(s);
    let attached = AttachedSource::attach(temp.path(), &c, &LoadControl::default()).unwrap();
    fs::remove_file(temp.path().join("PS3_GAME/USRDIR/DATA.FARC")).unwrap(); // authored temporary fixture only
    assert!(attached
        .read_hex(&key(&c, 1), 0, 4)
        .err()
        .unwrap()
        .contains("Protected resource"));
}

#[test]
fn attachment_matches_case_changed_extracted_paths_without_weakening_identity() {
    let temp = tempfile::tempdir().unwrap();
    let c = catalog(fixture(temp.path()));
    let game = temp.path().join("PS3_GAME");
    fs::rename(game.join("USRDIR"), game.join("renaming-authored-only")).unwrap();
    fs::rename(game.join("renaming-authored-only"), game.join("usrdir")).unwrap();
    let attached = AttachedSource::attach(temp.path(), &c, &LoadControl::default()).unwrap();
    assert_eq!(attached.read_hex(&key(&c, 0), 0, 4).unwrap().bytes, b"TEST");
}
