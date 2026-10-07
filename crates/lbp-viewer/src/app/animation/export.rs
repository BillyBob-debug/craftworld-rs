//! Cached metadata projection only. ANMb provenance: docs/animation-parser.md.
//! Opaque animation bytes, locators and keyframes are never exported.
use super::super::diagnostics::{self, reason, stage};
use super::AnimationView;
use serde_json::{json, Value};
use std::path::Path;
pub(super) fn report(view: &AnimationView) -> Result<Value, String> {
    if view.job.is_some() {
        return Err("Wait for animation inspection or cancellation to finish".into());
    }
    let key = view
        .key
        .as_ref()
        .ok_or("No selected animation location to export")?;
    let location = diagnostics::location(key)?;
    if view.metadata.is_none() && view.error.is_empty() {
        return Err("No completed animation diagnostic result to export".into());
    }
    let metadata = view.metadata.as_ref().map(|m| {
        let p = &m.prefix;
        json!({"compression_flags":m.compression_flags,"expanded_bytes":m.expanded_bytes,
            "prefix":{"revision":p.revision,"frames_raw_u16":p.frames_raw_u16,
            "fps_raw_u16":p.fps_raw_u16,"loop_start_raw_u16":p.loop_start_raw_u16,
            "morph_count_raw_u8":p.morph_count_raw_u8,"counts":p.counts,
            "position_offset_bits":p.position_offset_bits,"position_scale_bits":p.position_scale_bits,
            "fat_raw_u8":p.fat_raw_u8,"animation_data_bytes":p.animation_data_bytes,
            "animation_data_start":p.animation_data_start,"parsed_prefix_bytes":p.parsed_prefix_bytes,
            "unparsed_bytes":p.unparsed_bytes}})
    });
    Ok(
        json!({"schema_version":1,"scope":"cached_animation_diagnostic_metadata_not_asset_export",
        "source_state":"validated_at_read_not_rechecked_for_export","location":location,
        "last_request":{"outcome":if view.error.is_empty(){"complete"}else{"failed"},
            "error_kind":reason(&view.error)},"stages":{"animation":stage(metadata,true,&view.error)}}),
    )
}
pub(super) fn save(view: &AnimationView, path: &Path) -> Result<(), String> {
    diagnostics::save(&report(view)?, path)
}
