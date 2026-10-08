//! D88: texture routes tinted by a supplied instance color. Runs only on a graph
//! derived by `project_instance_color` (supplied-color opt-in); the default and
//! animation-base-only selectors are unchanged. Evidence: docs/d88-thing-color-textured.md.
use lbp_resource::material::GraphMetadata;

/// Instance-aware diffuse selection on a derived graph. Existing selectors first. One
/// animated sampler × COLOR (D102: accepted by the base selector) stays limited to the
/// D88 form: the COLOR is the projected ThingColor or the D88 folded operand and no
/// nested sampler was folded.
pub fn select_instance_diffuse(
    derived: &GraphMetadata,
    projection: &crate::InstanceProjection,
    animation_base: bool,
) -> Result<crate::DiffuseSelection, String> {
    // D96 (docs/d96-thing-sampler-ports.md): folded sampler ports only with direct
    // samplers; the nested-tint rewrite has no oracle parity for a folded leaf.
    if !projection.sampler_port_folds.is_empty() && projection.folded_sampler.is_some() {
        return Err("Unsupported: folded sampler port inputs under a nested instance tint".into());
    }
    let selection = match crate::select_diffuse_with_animation_base(derived, animation_base) {
        // D92 (docs/d92-multi-animated.md): two or more animated leaves under a
        // supplied color only as a direct ThingColor tint; not with a folded nested
        // sampler or alpha subtraction (no oracle parity for those rewrites).
        Ok(selection)
            if selection.animation_base_more[0].is_some()
                && (projection.folded_sampler.is_some()
                    || !projection.alpha_subtract_samplers.is_empty()) =>
        {
            return Err("Unsupported: several animated samplers under a folded or alpha-subtracted instance input".into());
        }
        // D99 product trees fold static COLOR leaves only; no oracle parity under a
        // supplied instance color (docs/d99-product-trees.md).
        Ok(selection) if selection.product_tree.is_some() => {
            return Err("Unsupported: product tree under a supplied instance color".into());
        }
        // D100 (docs/d100-grid-graph-bake.md): evaluated graphs only with the plain
        // ThingColor-to-COLOR substitution (no folds, nested tint or UV subtraction).
        Ok(selection)
            if selection.grid_graph.is_some()
                && !crate::grid_graph::instance_supported(projection) =>
        {
            return Err(
                "Unsupported: graph evaluator under a folded or rewritten instance input".into(),
            );
        }
        Ok(selection) => selection,
        Err(error) => return Err(error),
    };
    let instance_tint = selection.color_index.is_some_and(|c| {
        projection.color_nodes.contains(&c) || projection.folded_operand == Some(c)
    });
    if crate::animation_base::single_tinted_leaf(&selection)
        && (!instance_tint || projection.folded_sampler.is_some())
    {
        return Err(
            "Unsupported: animated sampler × COLOR under a supplied color needs the direct instance tint"
                .into(),
        );
    }
    Ok(selection)
}
