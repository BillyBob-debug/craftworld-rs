//! Authored cached material envelope. Shared private output safeguards live in
//! app::diagnostics. No parser or original-source reads; docs/material-export.md.
use super::super::diagnostics::{self, reason, stage};
use super::MaterialView;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn report(view: &MaterialView) -> Result<Value, String> {
    if view.job.is_some() {
        return Err("Wait for material inspection or cancellation to finish".into());
    }
    let key = view
        .key
        .as_ref()
        .ok_or("No selected material location to export")?;
    let location = diagnostics::location(key)?;
    if view.metadata.is_none() && view.error.is_empty() {
        return Err("No completed material diagnostic result to export".into());
    }
    let prefix = view
        .metadata
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let graph = view
        .graph
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let tail = view
        .tail
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let outcome = if !view.error.is_empty() {
        "failed"
    } else if !view.graph_error.is_empty() || !view.tail_error.is_empty() {
        "partial"
    } else {
        "complete"
    };
    Ok(
        json!({"schema_version":1,"scope":"cached_material_diagnostic_metadata_not_asset_export",
        "source_state":"validated_at_read_not_rechecked_for_export",
        "location":location,
        "last_request":{"outcome":outcome,"error_kind":reason(&view.error)},
        "stages":{"prefix":stage(prefix,true,if view.metadata.is_none(){&view.error}else{""}),
            "graph":stage(graph,view.graph_requested,&view.graph_error),
            "tail":stage(tail,view.tail_requested,&view.tail_error)}}),
    )
}
pub(super) fn save(view: &MaterialView, path: &Path) -> Result<(), String> {
    diagnostics::save(&report(view)?, path)
}
