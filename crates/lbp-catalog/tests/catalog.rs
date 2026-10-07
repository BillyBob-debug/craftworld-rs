use lbp_catalog::{demo_snapshot, load, Catalog, Filter, LoadControl, MAX_SNAPSHOT_BYTES};

fn catalog() -> Catalog {
    Catalog::build(demo_snapshot(), &LoadControl::default()).unwrap()
}

#[test]
fn aliases_exact_identifiers_and_unknown_states_remain_distinct() {
    let c = catalog();
    assert_eq!(c.filtered(&Filter::default()), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(c.by_hash[&"2".repeat(40)], vec![1, 2]);
    assert_eq!(c.inspection_resources[1], vec![1, 2]);
    for (query, expected) in [
        ("GUID:0x00001002".to_owned(), vec![1]),
        (format!("hash:{}", "2".repeat(40)), vec![1, 2]),
        ("CHECKER".to_owned(), vec![1, 2]),
        ("guid:1002".to_owned(), vec![]),
    ] {
        assert_eq!(
            c.filtered(&Filter {
                query,
                ..Default::default()
            }),
            expected
        );
    }
    for (status, expected) in [
        ("missing", vec![3]),
        ("blocked_encrypted", vec![4]),
        ("unknown_magic", vec![5]),
    ] {
        assert_eq!(
            c.filtered(&Filter {
                status: Some(status.into()),
                ..Default::default()
            }),
            expected
        );
    }
    assert_eq!(
        c.filtered(&Filter {
            kind: Some("unconfirmed".into()),
            ..Default::default()
        }),
        vec![3, 5]
    );
    assert_eq!(
        c.filtered(&Filter {
            archive: Some("USRDIR/SYNTHETIC.FARC".into()),
            ..Default::default()
        }),
        vec![0, 1, 2, 4, 5]
    );
}

#[test]
fn reference_union_preserves_raw_duplicate_edges_and_cycles() {
    let mut s = demo_snapshot();
    s.dependency_links[0].sha1_map_row_indices = vec![1, 2];
    let mut duplicate = s.resource_inspections[0].inspection.dependencies[0].clone();
    duplicate.index = 2;
    s.resource_inspections[0]
        .inspection
        .dependencies
        .push(duplicate);
    let mut edge = s.dependency_links[0].clone();
    edge.dependency_index = 2;
    s.dependency_links.push(edge);
    let mut back = s.resource_inspections[0].inspection.dependencies[0].clone();
    back.index = 0;
    back.guid = Some("00001001".into());
    s.resource_inspections[1].inspection.dependencies.push(back);
    let mut edge = s.dependency_links[0].clone();
    edge.inspection_index = 1;
    edge.dependency_index = 0;
    edge.guid_map_row_indices = vec![0];
    edge.sha1_map_row_indices.clear();
    s.dependency_links.push(edge);
    s.counts.insert("dependency_entries".into(), 4);
    let c = Catalog::build(s, &LoadControl::default()).unwrap();
    assert_eq!(c.targets(0), vec![1, 2]);
    assert_eq!(c.reverse[1], vec![0, 2]);
    assert_eq!(c.reverse[2], vec![0, 2]);
    assert_eq!(c.reverse[0], vec![3]);
    assert_eq!(c.outgoing[0], vec![0, 1, 2]);
    assert!(c.targets(1).is_empty());
}

#[test]
fn invalid_snapshots_fail_before_indexing() {
    let mut cases = Vec::new();
    let mut s = demo_snapshot();
    s.schema_version = 1;
    cases.push(s);
    let mut s = demo_snapshot();
    s.counts.insert("map_rows".into(), 999);
    cases.push(s);
    let mut s = demo_snapshot();
    s.resources[1].map_row.row_index = 0;
    cases.push(s);
    let mut s = demo_snapshot();
    s.resources[0].map_row.sha1 = "invalid".into();
    cases.push(s);
    let mut s = demo_snapshot();
    s.resources[0].archive_candidates[0].inspection_index = Some(999);
    cases.push(s);
    let mut s = demo_snapshot();
    s.dependency_links[0].guid_map_row_indices = vec![999];
    cases.push(s);
    let mut s = demo_snapshot();
    s.dependency_links[1].dependency_index = 0;
    cases.push(s);
    let mut s = demo_snapshot();
    s.archives[0].entries[0].offset = u64::MAX;
    cases.push(s);
    for s in cases {
        assert!(Catalog::build(s, &LoadControl::default()).is_err());
    }
}

#[test]
fn loading_is_read_only_bounded_and_cancellable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authored.json");
    let bytes = serde_json::to_vec(&demo_snapshot()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let control = LoadControl::default();
    assert_eq!(load(&path, &control).unwrap().snapshot.resources.len(), 6);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let control = LoadControl::default();
    control.cancel();
    assert!(load(&path, &control)
        .unwrap_err_string()
        .contains("cancelled"));
    assert!(Catalog::build(demo_snapshot(), &control).is_err());
    std::fs::write(&path, b"{\"schema_version\":2").unwrap();
    assert!(load(&path, &LoadControl::default()).is_err());
    std::fs::File::create(&path)
        .unwrap()
        .set_len(MAX_SNAPSHOT_BYTES + 1)
        .unwrap();
    assert!(load(&path, &LoadControl::default()).is_err());
}

trait ErrorString {
    fn unwrap_err_string(self) -> String;
}
impl ErrorString for Result<Catalog, String> {
    fn unwrap_err_string(self) -> String {
        match self {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        }
    }
}
