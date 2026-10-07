//! Cached diagnostic projection only. PLNb provenance: docs/plan-parser.md.
//! Inventory scalar provenance: docs/plan-inventory-parser.md.
//! No Thing/inventory payload bytes or source reads. Writer: app::diagnostics.
use super::super::diagnostics::{self, reason, stage};
use super::PlanView;
use serde_json::{json, Value};
use std::path::Path;
pub(super) fn report(view: &PlanView) -> Result<Value, String> {
    if view.job.is_some() {
        return Err("Wait for plan inspection or cancellation to finish".into());
    }
    let key = view
        .key
        .as_ref()
        .ok_or("No selected plan location to export")?;
    let location = diagnostics::location(key)?;
    if view.metadata.is_none() && view.error.is_empty() {
        return Err("No completed plan diagnostic result to export".into());
    }
    if (view.metadata.is_some() && !view.error.is_empty())
        || (view.inventory.is_some() && !view.inventory_error.is_empty())
        || (!view.inventory_mode && (view.inventory.is_some() || !view.inventory_error.is_empty()))
        || (view.metadata.is_none()
            && (view.inventory.is_some() || !view.inventory_error.is_empty()))
    {
        return Err("Inconsistent cached plan diagnostic stages".into());
    }
    if view.inventory_mode
        && view.metadata.is_some()
        && view.inventory.is_none()
        && view.inventory_error.is_empty()
    {
        return Err("No completed inventory diagnostic result to export".into());
    }
    let metadata = view.metadata.as_ref().map(|m| {
        let p = &m.prefix;
        json!({"compression_flags":m.compression_flags,"expanded_bytes":m.expanded_bytes,
            "prefix":{"revision":p.revision,"inner_revision_raw_u32":p.inner_revision_raw_u32,
            "thing_data_bytes":p.thing_data_bytes,"thing_data_start":p.thing_data_start,
            "thing_data_sha256":p.thing_data_sha256,"parsed_prefix_bytes":p.parsed_prefix_bytes,
            "unparsed_bytes":p.unparsed_bytes,"inventory_tail_start":p.inventory_tail_start,
            "inventory_tail_bytes":p.inventory_tail_bytes}})
    });
    let inventory = view.inventory.as_ref().map(|m| {
        json!({"date_added_encoded_u64_hex":m.date_added_encoded_u64_hex,
            "slot_type_raw_u32":m.slot_type_raw_u32,"slot_number_raw_u32":m.slot_number_raw_u32,
            "highlight_sound_guid_raw_u32":m.highlight_sound_guid_raw_u32,
            "colour_raw_u32":m.colour_raw_u32,"type_flags_raw_u32":m.type_flags_raw_u32,
            "sub_type_raw_u32":m.sub_type_raw_u32,"title_key_raw_u32":m.title_key_raw_u32,
            "description_key_raw_u32":m.description_key_raw_u32,
            "inventory_prefix_start":m.inventory_prefix_start,
            "parsed_inventory_prefix_bytes":m.parsed_inventory_prefix_bytes,
            "inventory_prefix_end":m.inventory_prefix_end,
            "unparsed_after_inventory_prefix":m.unparsed_after_inventory_prefix})
    });
    let (outcome, error) = if !view.error.is_empty() {
        ("failed", view.error.as_str())
    } else if !view.inventory_error.is_empty() {
        ("partial", view.inventory_error.as_str())
    } else {
        ("complete", "")
    };
    Ok(
        json!({"schema_version":2,"scope":"cached_plan_diagnostic_metadata_not_asset_export",
        "source_state":"validated_at_read_not_rechecked_for_export","location":location,
        "inspection_mode":if view.inventory_mode{"plan_inventory_prefix"}else{"plan_prefix"},
        "last_request":{"outcome":outcome,"error_kind":reason(error)},
        "stages":{"plan":stage(metadata,true,&view.error),
            "inventory":stage(inventory,view.inventory_mode,&view.inventory_error)}}),
    )
}
pub(super) fn save(view: &PlanView, path: &Path) -> Result<(), String> {
    diagnostics::save(&report(view)?, path)
}
