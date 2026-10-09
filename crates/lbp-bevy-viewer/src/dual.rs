//! Authored unlit two/three/four-sample extension. Evidence: docs/dual-preview.md,
//! docs/nested-preview.md, docs/four-product-preview.md, docs/third-sample-preview.md.
//! Third samples second (or primary, D65) coordinates through a CPU-derived
//! scale/offset; fourth shares primary coordinates after exact CPU validation, or
//! (D101, docs/d101-multiply-rest.md) primary/second coordinates × xy + zw.
//! D99: `enabled.w` selects the two-texture alpha MIX mode (docs/d99-product-trees.md).
//! D121: opt-in preview lighting multiplies the finished colour by a Lambert factor
//! (docs/d121-preview-lighting.md); off (`light_dir.w` = 0) leaves the colour untouched.
//! P2l: `light_dir.w` = 2 selects the opt-in LBP-shaped factor (docs/p2l-lbp-lighting.md).
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
    /// D121: direction toward the key light (xyz), w = 1 when this material is lit.
    #[uniform(109)]
    light_dir: Vec4,
    /// D121: key light colour (rgb).
    #[uniform(110)]
    light_key: Vec4,
    /// D121: ambient colour (rgb).
    #[uniform(111)]
    light_ambient: Vec4,
    /// P2l: lower rim colour (rgb), used toward N.y = -1.
    #[uniform(112)]
    rim_low: Vec4,
    /// P2l: upper rim colour (rgb), used toward N.y = +1.
    #[uniform(113)]
    rim_high: Vec4,
    /// D121: generated normals passed `adapter::normal_trust`; otherwise stays unlit.
    pub normals_trusted: bool,
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
/// D121: applies (Some) or clears (None) preview lighting; returns whether the material
/// is lit. Untrusted normals keep the unlit colour (counted by the caller).
pub fn set_lighting(
    material: &mut ViewerMaterial,
    light: Option<&crate::lighting::PreviewLight>,
) -> bool {
    let ext = &mut material.extension;
    match light.filter(|_| ext.normals_trusted) {
        Some(l) => {
            ext.light_dir = l.toward_light.extend(if l.lbp { 2. } else { 1. });
            ext.light_key = l.key.extend(0.);
            ext.light_ambient = l.ambient.extend(0.);
            ext.rim_low = l.rim[0].extend(0.);
            ext.rim_high = l.rim[1].extend(0.);
            true
        }
        None => {
            ext.light_dir = Vec4::ZERO;
            ext.light_key = Vec4::ZERO;
            ext.light_ambient = Vec4::ZERO;
            ext.rim_low = Vec4::ZERO;
            ext.rim_high = Vec4::ZERO;
            false
        }
    }
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
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var<uniform> light_dir: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var<uniform> light_key: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var<uniform> light_ambient: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var<uniform> rim_low: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var<uniform> rim_high: vec4<f32>;
// D121 preview Lambert factor (mirrors lighting::PreviewLight::factor/facing_normal).
// Double-sided: the smooth normal is turned into the hemisphere of the face normal that
// points at the viewer, so back faces and mirrored instances light like front faces.
fn preview_light(shading: vec3<f32>, face: vec3<f32>, to_view: vec3<f32>) -> vec3<f32> {
    if dot(shading, shading) <= 0.0 {
        return vec3<f32>(1.0);
    }
    let n = facing_normal(shading, face, to_view);
    if light_dir.w > 1.5 {
        // P2l (mirrors lighting::PreviewLight::lbp_factor): hemisphere ambient, sun,
        // grazing rim lerped by N.y; squared for the game's square-then-sqrt combine.
        let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
        let f = light_ambient.rgb * (n.y * 0.15 + 0.5)
            + light_key.rgb * max(dot(n, light_dir.xyz), 0.0)
            + mix(rim_low.rgb, rim_high.rgb, (n.y + 1.0) * 0.5) * rim;
        return f * f;
    }
    return light_ambient.rgb + light_key.rgb * max(dot(n, light_dir.xyz), 0.0);
}
fn facing_normal(shading: vec3<f32>, face: vec3<f32>, to_view: vec3<f32>) -> vec3<f32> {
    var n = normalize(shading);
    if dot(face, face) > 0.0 {
        var f = normalize(face);
        if dot(f, to_view) < 0.0 {
            f = -f;
        }
        if dot(n, f) < 0.0 {
            n = -n;
        }
    }
    return n;
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    let pbr = pbr_input_from_standard_material(in, front);
    // Screen-space derivatives stay in uniform control flow.
    let face = cross(dpdx(in.world_position.xyz), dpdy(in.world_position.xyz));
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
    var rgb = color.rgb;
    if light_dir.w > 0.5 {
        rgb *= preview_light(in.world_normal, face, pbr.V);
    }
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(rgb, 1.0));
    return out;
}
"#;
#[cfg(test)]
mod tests {
    /// Stand-ins for the Bevy imports (same names and the fields this shader uses).
    const STUBS: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
}
struct FragmentOutput {
    @location(0) color: vec4<f32>,
}
struct StandardMaterialStub {
    base_color: vec4<f32>,
}
struct PbrInputStub {
    material: StandardMaterialStub,
    V: vec3<f32>,
}
fn pbr_input_from_standard_material(in: VertexOutput, front: bool) -> PbrInputStub {
    var p: PbrInputStub;
    p.material.base_color = vec4<f32>(1.0);
    p.V = vec3<f32>(0.0, 0.0, 1.0);
    return p;
}
fn main_pass_post_lighting_processing(p: PbrInputStub, c: vec4<f32>) -> vec4<f32> {
    return c;
}
"#;
    /// D121: the authored shader (all `#ifdef` blocks enabled, imports stubbed) parses
    /// and validates headlessly. Bevy's own composition is exercised only at run time.
    #[test]
    fn authored_shader_validates_with_every_branch_enabled() {
        let body: String = super::SOURCE
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .map(|l| l.replace("#{MATERIAL_BIND_GROUP}", "2") + "\n")
            .collect();
        assert!(body.contains("preview_light(in.world_normal, face, pbr.V)"));
        let source = format!("{STUBS}{body}");
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
