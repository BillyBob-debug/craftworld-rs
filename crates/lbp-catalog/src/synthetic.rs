//! Authored metadata-only demonstration. Contains no game bytes or extracted paths.
use crate::Snapshot;
use serde_json::{json, Value};
fn hash(byte: &str) -> String {
    byte.repeat(40)
}
fn inspection(
    kind: Option<(&str, u32)>,
    status: &str,
    magic: &str,
    dependencies: Vec<Value>,
) -> Value {
    json!({"status":status,"reason":if status=="blocked_encrypted" {Some("Synthetic protected marker; no payload exists")}else{None},
        "magic_hex":magic,"confirmed_type":kind.map(|(name,id)|json!({"name":name,"id":id})),
        "serialization":if status=="metadata_parsed" {"binary"}else{"unparsed"},
        "header":if status=="metadata_parsed" {json!({"revision":931,"branch_id":0,"branch_revision":0,
            "compression_flags":7,"compressed":true,"dependency_offset":64,"payload_offset":18,"scope":"ordinary_binary"})}else{Value::Null},
        "dependency_status":if status=="metadata_parsed" {"parsed"}else{"not_inspected"},
        "dependency_count_raw":if status=="metadata_parsed" {Some(dependencies.len())}else{None},
        "dependencies":dependencies,"metadata_bytes_read":if status=="metadata_parsed" {32}else{4}})
}
pub fn demo_snapshot() -> Snapshot {
    let archive = "USRDIR/SYNTHETIC.FARC";
    let definitions = [
        (
            "00001001",
            "1",
            "synthetic/stitched_cube.mol",
            Some("mesh"),
            "metadata_parsed",
            Some(0),
        ),
        (
            "00001002",
            "2",
            "synthetic/checker.tex",
            Some("texture"),
            "identified_unparsed",
            Some(1),
        ),
        (
            "00001003",
            "2",
            "synthetic/checker_alias.tex",
            Some("texture"),
            "identified_unparsed",
            Some(1),
        ),
        (
            "00001004",
            "3",
            "synthetic/unavailable.mat",
            None,
            "missing",
            None,
        ),
        (
            "00001005",
            "4",
            "synthetic/protected.ff",
            Some("script"),
            "blocked_encrypted",
            Some(2),
        ),
        (
            "00001006",
            "5",
            "synthetic/unknown.resource",
            None,
            "unknown_magic",
            Some(3),
        ),
    ];
    let resources:Vec<_>=definitions.iter().enumerate().map(|(row,(guid,h,path,kind,status,index))|json!({
        "map_row":{"row_index":row,"path":path,"timestamp_raw":0,"declared_size":80,"sha1":hash(h),"guid":guid},
        "type_hint":kind.map(|name|json!({"name":name,"basis":"extension_inference"})),"confirmed_type":kind,
        "header_status":status,"dependency_status":if index==&Some(0){"parsed"}else{"not_inspected"},
        "map_path_status":"relative_path_candidate","resolution":if index.is_some(){"archive_candidate"}else{"missing_in_selected_directory"},
        "archive_candidates":index.map(|i|vec![json!({"archive":archive,"entry_index":i,"offset":i*100,"size":80,
            "declared_size_matches":true,"inspection_index":i})]).unwrap_or_default(),"loose_candidates":[]})).collect();
    let locations:Vec<_>=[
        inspection(Some(("mesh",2)),"metadata_parsed","4d534862",vec![
            json!({"index":0,"flags":2,"guid":"00001002","sha1":null,"type_id":1,"known_type":{"id":1,"name":"texture"}}),
            json!({"index":1,"flags":2,"guid":"ffff0001","sha1":null,"type_id":999,"known_type":null})]),
        inspection(Some(("texture",1)),"identified_unparsed","47544620",vec![]),
        inspection(Some(("script",11)),"blocked_encrypted","46534865",vec![]),
        inspection(None,"unknown_magic","3f3f3f3f",vec![]),
    ].into_iter().enumerate().map(|(i,inspection)|json!({"source_path":archive,"offset":i*100,"size":80,
        "sha1":hash(["1","2","4","5"][i]),"inspection":inspection})).collect();
    serde_json::from_value(json!({"schema_version":2,"milestone":"A2_resource_metadata","partial_metadata":false,
        "build":{"title_id":"SYNTHETIC","title":"Authored demonstration","app_version":"demo","disc_version":"demo"},
        "source_scope":"synthetic_metadata_only","payload_integrity":"no_payload_bytes_exist",
        "counts":{"map_rows":6,"unique_map_hashes":5,"archives":1,"archive_entries":4,"dependency_entries":2,
            "map_rows_missing_in_selected_directory":1,"uninspected_resources":1},
        "metadata_status_counts":{"metadata_parsed":1,"identified_unparsed":1,"blocked_encrypted":1,"unknown_magic":1},
        "confirmed_type_counts":{"mesh":1,"texture":1,"script":1},
        "dependency_resolution_counts":{"guid_catalog_candidate":1,"unresolved_in_selected_catalog":1},
        "archives":[{"path":archive,"file_size":520,"table_offset":400,"entry_count":4,"duplicate_hash_rows":0,
            "entries":[{"entry_index":0,"sha1":hash("1"),"offset":0,"size":80},
                {"entry_index":1,"sha1":hash("2"),"offset":100,"size":80},
                {"entry_index":2,"sha1":hash("4"),"offset":200,"size":80},
                {"entry_index":3,"sha1":hash("5"),"offset":300,"size":80}]}],
        "resources":resources,"resource_inspections":locations,
        "dependency_links":[{"inspection_index":0,"dependency_index":0,"guid_map_row_indices":[1],"sha1_map_row_indices":[],
            "hash_archive_candidate_count":0,"status":"guid_catalog_candidate"},
            {"inspection_index":0,"dependency_index":1,"guid_map_row_indices":[],"sha1_map_row_indices":[],
            "hash_archive_candidate_count":0,"status":"unresolved_in_selected_catalog"}]
    })).expect("authored demo matches schema projection")
}
