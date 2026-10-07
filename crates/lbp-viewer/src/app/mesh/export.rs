//! Cached diagnostic mesh projection, not geometry export. Explicit field lists
//! exclude position/index/bone arrays, GPU state and images. Existing prefix and
//! topology provenance: docs/mesh-prefix.md, mesh-geometry.md; docs/mesh-export.md.
use super::super::diagnostics::{self, reason, stage};
use super::MeshView;
use serde_json::{json, Value};
use std::path::Path;
pub(super) fn report(view: &MeshView) -> Result<Value, String> {
    if view.job.is_some() {
        return Err("Wait for mesh inspection or cancellation to finish".into());
    }
    let key = view
        .key
        .as_ref()
        .ok_or("No selected mesh location to export")?;
    let location = diagnostics::location(key)?;
    if view.geometry.is_none() && view.error.is_empty() {
        return Err("No completed mesh diagnostic result to export".into());
    }
    let metadata = view.geometry.as_ref().map(|g| {
        let m = &g.metadata; let p = &m.prefix;
        let primitives: Vec<_> = m.primitives.iter().map(|v| json!({
            "material_flags":v.material_flags,"material_kind":v.material_kind,
            "material_guid":v.material_guid,"material_hash":v.material_hash,
            "alternatives_flags":v.alternatives_flags,"alternatives_kind":v.alternatives_kind,
            "alternatives_guid":v.alternatives_guid,"alternatives_hash":v.alternatives_hash,
            "min_vertex_raw":v.min_vertex_raw,"max_vertex_raw":v.max_vertex_raw,
            "first_index":v.first_index,"index_count":v.index_count,"region":v.region})).collect();
        json!({"vertex_count":g.positions.len(),
            "prefix":{"revision":p.revision,"compression_flags":p.compression_flags,"counts":p.counts,
                "morph_names_hex":p.morph_names_hex,"min_uv_bits":p.min_uv_bits,"max_uv_bits":p.max_uv_bits,
                "area_scale_bits":p.area_scale_bits,"stream_offsets":p.stream_offsets,
                "position_bounds":p.position_bounds,"position_bounds_bits":p.position_bounds_bits,
                "position_bytes_sha256":p.position_bytes_sha256,"attribute_bytes":p.attribute_bytes,
                "attribute_sha256":p.attribute_sha256,"index_bytes":p.index_bytes,"index_sha256":p.index_sha256,
                "expanded_bytes":p.expanded_bytes,"parsed_prefix_bytes":p.parsed_prefix_bytes,
                "unparsed_bytes":p.unparsed_bytes},
            "primitives":primitives,"bone_count":m.bone_count,"primitive_type":m.primitive_type,
            "triangle_count":m.triangle_count,"triangle_indices_sha256":m.triangle_indices_sha256,
            "parsed_geometry_bytes":m.parsed_geometry_bytes,"unparsed_after_geometry":m.unparsed_after_geometry})
    });
    Ok(
        json!({"schema_version":1,"scope":"cached_mesh_diagnostic_metadata_not_asset_export",
        "source_state":"validated_at_read_not_rechecked_for_export","location":location,
        "last_request":{"outcome":if view.error.is_empty(){"complete"}else{"failed"},
            "error_kind":reason(&view.error)},"stages":{"mesh":stage(metadata,true,&view.error)}}),
    )
}
pub(super) fn save(view: &MeshView, path: &Path) -> Result<(), String> {
    diagnostics::save(&report(view)?, path)
}
