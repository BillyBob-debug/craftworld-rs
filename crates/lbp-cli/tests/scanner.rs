//! Integration tests build tiny, entirely synthetic game directories.
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn fixture(root: &Path) {
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
    fs::write(game.join("PARAM.SFO"), sfo).unwrap();
    let mut map = 0x100_u32.to_be_bytes().to_vec();
    map.extend(4_u32.to_be_bytes());
    for (path, hash, guid, size) in [
        ("a.tex", 1_u8, u32::MAX, 3_u32),
        ("alias.tex", 1, u32::MAX, 4),
        ("loose.raw", 2, 2, 2),
        ("../escape.bin", 3, 3, 1),
    ] {
        map.extend((path.len() as u32).to_be_bytes());
        map.extend(path.as_bytes());
        map.extend(0_i64.to_be_bytes());
        map.extend(size.to_be_bytes());
        map.extend([hash; 20]);
        map.extend(guid.to_be_bytes());
    }
    fs::write(game.join("USRDIR/OUTPUT/BLURAYGUIDS.MAP"), map).unwrap();
    let mut farc = b"abc".to_vec();
    farc.extend([1; 20]);
    farc.extend(0_u32.to_be_bytes());
    farc.extend(3_u32.to_be_bytes());
    farc.extend(1_u32.to_be_bytes());
    farc.extend(b"FARC");
    fs::write(game.join("USRDIR/DATA.FARC"), farc).unwrap();
    fs::write(game.join("USRDIR/loose.raw"), b"xy").unwrap();
}
fn run(root: &Path, output: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lbp-cli"))
        .arg("--game-dir")
        .arg(root)
        .arg("--json")
        .arg(output)
        .arg("--tables-only")
        .args(extra)
        .output()
        .unwrap()
}
#[test]
fn deterministic_catalog_preserves_aliases_and_input_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disc");
    fixture(&root);
    let map_path = root.join("PS3_GAME/USRDIR/OUTPUT/BLURAYGUIDS.MAP");
    let before = fs::read(&map_path).unwrap();
    let a = temp.path().join("a.json");
    let b = temp.path().join("b.json");
    let result = run(&root, &a, &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(run(&root.join("PS3_GAME"), &b, &[]).status.success());
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert_eq!(before, fs::read(map_path).unwrap());
    let data: Value = serde_json::from_slice(&fs::read(a).unwrap()).unwrap();
    assert_eq!(data["build"]["title_id"], "SYNTH0001");
    assert_eq!(data["counts"]["map_rows"], 4);
    assert_eq!(data["counts"]["duplicate_guid_rows"], 1);
    assert_eq!(data["counts"]["unsafe_map_paths"], 1);
    assert_eq!(data["counts"]["map_rows_with_archive_candidates"], 2);
    assert_eq!(data["counts"]["map_rows_with_loose_candidates"], 1);
    assert_eq!(data["counts"]["map_rows_missing_in_selected_directory"], 1);
    assert_eq!(data["counts"]["declared_size_mismatch_candidates"], 1);
    assert_eq!(data["resources"][0]["map_row"]["guid"], "ffffffff");
    assert_eq!(data["resources"][0]["confirmed_type"], Value::Null);
    assert_eq!(data["resources"][0]["dependency_status"], "not_inspected");
}
#[test]
fn refuses_overwrite_game_outputs_and_excessive_counts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disc");
    fixture(&root);
    let existing = temp.path().join("existing.json");
    fs::write(&existing, b"sentinel").unwrap();
    assert!(!run(&root, &existing, &[]).status.success());
    assert_eq!(fs::read(existing).unwrap(), b"sentinel");
    let inside = root.join("new.json");
    assert!(!run(&root, &inside, &[]).status.success());
    assert!(!inside.exists());
    for extra in [
        ["--max-entries", "1"],
        ["--max-table-bytes", "8"],
        ["--max-files", "1"],
    ] {
        let out = temp.path().join("limited.json");
        assert!(!run(&root, &out, &extra).status.success());
        assert!(!out.exists());
    }
}

fn a2_fixture(root: &Path) {
    fixture(root);
    let mut resource = b"MATb".to_vec();
    resource.extend(951_u32.to_be_bytes());
    resource.extend(22_u32.to_be_bytes());
    resource.extend([0, 0, 0, 0, 7, 1]);
    resource.extend(b"BODY");
    resource.extend(2_u32.to_be_bytes());
    for _ in 0..2 {
        resource.push(3);
        resource.extend(u32::MAX.to_be_bytes());
        resource.extend([1; 20]);
        resource.extend(1_u32.to_be_bytes());
    }
    let size = resource.len() as u32;
    resource.extend([1; 20]);
    resource.extend(0_u32.to_be_bytes());
    resource.extend(size.to_be_bytes());
    resource.extend(1_u32.to_be_bytes());
    resource.extend(b"FARC");
    fs::write(root.join("PS3_GAME/USRDIR/DATA.FARC"), resource).unwrap();
    fs::write(root.join("PS3_GAME/USRDIR/loose.raw"), b"????").unwrap();
}
fn run_a2(root: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lbp-cli"))
        .arg("--game-dir")
        .arg(root)
        .arg("--json")
        .arg(out)
        .args(extra)
        .output()
        .unwrap()
}
#[test]
fn metadata_is_per_location_and_dependency_aliases_are_retained() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disc");
    a2_fixture(&root);
    let out = temp.path().join("a2.json");
    let result = run_a2(&root, &out, &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let data: Value = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(data["schema_version"], 2);
    assert_eq!(data["partial_metadata"], false);
    assert_eq!(data["resource_inspections"].as_array().unwrap().len(), 2);
    assert_eq!(
        data["resources"][0]["archive_candidates"][0]["inspection_index"],
        data["resources"][1]["archive_candidates"][0]["inspection_index"]
    );
    assert_eq!(data["resources"][0]["type_hint"]["name"], "texture");
    assert_eq!(data["resources"][0]["confirmed_type"], "physics_material");
    assert_eq!(data["counts"]["dependency_entries"], 2);
    assert_eq!(
        data["dependency_links"][0]["guid_map_row_indices"],
        serde_json::json!([0, 1])
    );
    assert_eq!(
        data["dependency_links"][0]["status"],
        "guid_and_hash_catalog_candidates"
    );
    let repeat = temp.path().join("repeat.json");
    assert!(run_a2(&root.join("PS3_GAME"), &repeat, &[])
        .status
        .success());
    assert_eq!(fs::read(out).unwrap(), fs::read(repeat).unwrap());
}
#[test]
fn metadata_failures_publish_partial_json_with_exit_two_and_respect_global_budget() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disc");
    a2_fixture(&root);
    let out = temp.path().join("limited.json");
    assert_eq!(
        run_a2(&root, &out, &["--scan-read-budget", "4"])
            .status
            .code(),
        Some(2)
    );
    let data: Value = serde_json::from_slice(&fs::read(out).unwrap()).unwrap();
    assert_eq!(data["partial_metadata"], true);
    assert_eq!(data["counts"]["metadata_bytes_read"], 4);
    assert_eq!(data["counts"]["dependency_entries"], 0);
    let malformed = temp.path().join("malformed.json");
    fixture(&root);
    assert_eq!(run_a2(&root, &malformed, &[]).status.code(), Some(2));
    let data: Value = serde_json::from_slice(&fs::read(malformed).unwrap()).unwrap();
    assert_eq!(data["metadata_status_counts"]["malformed"], 2);
}
