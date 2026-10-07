//! D88: texture routes tinted by a supplied instance color. Runs only on a graph
//! derived by `project_instance_color` (supplied-color opt-in); the default and
//! animation-base-only selectors are unchanged. Evidence: docs/d88-thing-color-textured.md.
use lbp_resource::material::GraphMetadata;

/// Instance-aware diffuse selection on a derived graph. Existing selectors first; with
/// the animation-base opt-in also, one animated sampler × the projected ThingColor (or
/// the D88 folded ThingColor operand), which D78 leaves out for a plain static COLOR.
pub fn select_instance_diffuse(
    derived: &GraphMetadata,
    projection: &crate::InstanceProjection,
    animation_base: bool,
) -> Result<crate::DiffuseSelection, String> {
    let error = match crate::select_diffuse_with_animation_base(derived, animation_base) {
        Ok(selection) => return Ok(selection),
        Err(error) => error,
    };
    if !animation_base || projection.folded_sampler.is_some() {
        return Err(error);
    }
    let Ok(selection) = crate::linked::select_animated_products(derived) else {
        return Err(error);
    };
    let instance_tint = selection.color_index.is_some_and(|c| {
        projection.color_nodes.contains(&c) || projection.folded_operand == Some(c)
    });
    // Exactly animated texture × instance COLOR under the root MULTIPLY: no second,
    // third or fourth leaf, no nested product and no MIX/BLEND state.
    if instance_tint
        && selection.animation_base.is_some()
        && selection.multiply_index.is_some()
        && selection.second_sample.is_none()
        && selection.third_sample.is_none()
        && selection.fourth_sample.is_none()
        && selection.inner_multiply_index.is_none()
        && selection.blend.is_none()
        && selection.blend_mask.is_none()
        && selection.alpha_mix.is_none()
        && selection.texture_alpha_mix.is_none()
        && selection.sampler_scale.is_none()
    {
        Ok(selection)
    } else {
        Err(error)
    }
}
