//! Authored unlit two/three/four-sample extension. Evidence: docs/dual-preview.md,
//! docs/nested-preview.md, docs/four-product-preview.md, docs/third-sample-preview.md.
//! Third samples second (or primary, D65) coordinates through a CPU-derived
//! scale/offset; fourth shares primary coordinates after exact CPU validation, or
//! (D101, docs/d101-multiply-rest.md) primary/second coordinates × xy + zw.
//! D99: `enabled.w` selects the two-texture alpha MIX mode (docs/d99-product-trees.md).
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::{Shader, ShaderRef},
};
pub type ViewerMaterial = ExtendedMaterial<StandardMaterial, SecondTexture>;
const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("c1f79fb1-32b7-4f6f-bef7-71a8c677f05c");
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct SecondTexture {
    #[texture(100)]
    #[sampler(101)]
    pub image: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    pub third_image: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    pub fourth_image: Option<Handle<Image>>,
    #[uniform(102)]
    enabled: Vec4,
    /// Third sample: base coordinates × xy + zw.
    #[uniform(107)]
    third_transform: Vec4,
    /// D101 fourth sample: base coordinates × xy + zw.
    #[uniform(108)]
    fourth_transform: Vec4,
}
impl MaterialExtension for SecondTexture {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}
pub fn material(base: StandardMaterial) -> ViewerMaterial {
    ViewerMaterial {
        base,
        extension: SecondTexture::default(),
    }
}
pub fn enable(material: &mut ViewerMaterial, image: Handle<Image>) {
    material.extension.image = Some(image);
    material.extension.enabled = Vec4::X;
}
/// D99: off-grid two-texture alpha MIX (docs/d99-product-trees.md): the primary and
/// second samples are mixed by the second sample's alpha instead of multiplied.
pub fn enable_alpha_mix(material: &mut ViewerMaterial) {
    material.extension.enabled.w = 1.;
}
pub fn enable_third(
    material: &mut ViewerMaterial,
    image: Handle<Image>,
    relative: lbp_preview::RelativeSampling,
) {
    material.extension.third_image = Some(image);
    material.extension.enabled.y = if relative.from_primary { 2. } else { 1. };
    material.extension.third_transform = Vec4::new(
        relative.scale[0],
        relative.scale[1],
        relative.offset[0],
        relative.offset[1],
    );
}
/// `enabled.z`: 1 = primary coordinates, 2 = second-sample coordinates (D101).
pub fn enable_fourth(
    material: &mut ViewerMaterial,
    image: Handle<Image>,
    relative: lbp_preview::RelativeSampling,
) {
    material.extension.fourth_image = Some(image);
    material.extension.enabled.z = if relative.from_primary { 1. } else { 2. };
    material.extension.fourth_transform = Vec4::new(
        relative.scale[0],
        relative.scale[1],
        relative.offset[0],
        relative.offset[1],
    );
}
pub struct DualPlugin;
impl Plugin for DualPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ViewerMaterial>::default());
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                SHADER.id(),
                Shader::from_wgsl(SOURCE, "authored-dual-preview.wgsl"),
            )
            .unwrap();
    }
}
// Independently authored preview shader; imported Bevy interfaces are pinned by Cargo.lock.
const SOURCE: &str = r#"
#import bevy_pbr::{forward_io::{VertexOutput, FragmentOutput}, pbr_fragment::pbr_input_from_standard_material, pbr_functions::main_pass_post_lighting_processing}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var second_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var second_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> enabled: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var third_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var third_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var fourth_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var fourth_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var<uniform> third_transform: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var<uniform> fourth_transform: vec4<f32>;
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    let pbr = pbr_input_from_standard_material(in, front);
    var color = pbr.material.base_color;
#ifdef VERTEX_UVS_B
    if enabled.x > 0.0 {
        let second = textureSample(second_image, second_sampler, in.uv_b);
        if enabled.w > 0.5 {
            // Toolkit MIX: (port1 - port0) * port1.a + port0, on linear values.
            color = mix(color, second, second.a);
        } else {
            color *= second;
        }
        if enabled.y > 0.0 {
            var third_base = in.uv_b;
#ifdef VERTEX_UVS_A
            if enabled.y > 1.5 {
                third_base = in.uv;
            }
#endif
            color *= textureSample(third_image, third_sampler, third_base * third_transform.xy + third_transform.zw);
        }
#ifdef VERTEX_UVS_A
        if enabled.z > 0.0 {
            var fourth_base = in.uv;
            if enabled.z > 1.5 {
                fourth_base = in.uv_b;
            }
            color *= textureSample(fourth_image, fourth_sampler, fourth_base * fourth_transform.xy + fourth_transform.zw);
        }
#endif
    }
#endif
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(color.rgb, 1.0));
    return out;
}
"#;
