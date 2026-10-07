//! Cached texture metadata projection. Explicit field whitelist excludes pixels.
//! Existing header provenance/oracle scope: docs/texture-preview.md, tex-preview.md,
//! bc3-preview.md. No new codec/layout inference; docs/texture-export.md.
use super::super::diagnostics::{self, reason, stage};
use super::Preview;
use serde_json::{json, Value};
use std::path::Path;
pub(super) fn report(preview: &Preview) -> Result<Value, String> {
    if preview.job.is_some() {
        return Err("Wait for texture inspection or cancellation to finish".into());
    }
    let key = preview
        .key
        .as_ref()
        .ok_or("No selected texture location to export")?;
    let location = diagnostics::location(key)?;
    if preview.image.is_none() && preview.error.is_empty() {
        return Err("No completed texture diagnostic result to export".into());
    }
    let metadata = preview.image.as_ref().map(|texture| {
        json!({
        "header":texture.header,"expanded_bytes":texture.expanded_bytes,
        "top_mip_block_bytes":texture.top_mip_block_bytes,
        "unparsed_after_top_mip":texture.unparsed_after_top_mip})
    });
    Ok(
        json!({"schema_version":1,"scope":"cached_texture_diagnostic_metadata_not_asset_export",
        "source_state":"validated_at_read_not_rechecked_for_export","location":location,
        "last_request":{"outcome":if preview.error.is_empty(){"complete"}else{"failed"},
            "error_kind":reason(&preview.error)},
        "stages":{"texture":stage(metadata,true,&preview.error)}}),
    )
}
pub(super) fn save(preview: &Preview, path: &Path) -> Result<(), String> {
    diagnostics::save(&report(preview)?, path)
}
