//! Authored unlit two/three/four-sample extension. Evidence: docs/dual-preview.md,
//! docs/nested-preview.md, docs/four-product-preview.md, docs/third-sample-preview.md.
//! Third samples second (or primary, D65) coordinates through a CPU-derived
//! scale/offset; fourth shares primary coordinates after exact CPU validation, or
//! (D101, docs/d101-multiply-rest.md) primary/second coordinates × xy + zw.
//! D99: `enabled.w` selects the two-texture alpha MIX mode (docs/d99-product-trees.md).
//! D121: opt-in preview lighting multiplies the finished colour by a Lambert factor
//! (docs/d121-preview-lighting.md); off (`light_dir.w` = 0) leaves the colour untouched.
//! P2l: `light_dir.w` = 2 selects the opt-in LBP-shaped factor (docs/p2l-lbp-lighting.md).
//! P2q: under that factor, an optional bump map on UV1 tilts the normal and a static
//! gloss colour adds a specular term (docs/p2q-bump-gloss.md).
//! D189: `graph_info.x` > 0 evaluates a D100 graph per fragment over raw UV0/UV1
//! (docs/d189-mixed-uv-graphs.md); graph textures 0..2 reuse bindings 100/103/105.
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
    /// P2q: bump map (linear, UV1).
    #[texture(114)]
    #[sampler(115)]
    pub bump_image: Option<Handle<Image>>,
    /// P2q: x = BumpLevel, y = 1 when the bump map is bound.
    #[uniform(116)]
    bump_params: Vec4,
    /// P2q: gloss colour (rgb) and CosinePower (w; 0 = no gloss).
    #[uniform(117)]
    gloss: Vec4,
    /// P2t: game fog: start, slope (display units), luma CC, on (w).
    #[uniform(118)]
    lbp_fog: Vec4,
    /// P2t: game fog colour (output space).
    #[uniform(119)]
    lbp_fog_color: Vec4,
    /// D189: graph textures 3..5 (0..2 are `image`, `third_image`, `fourth_image`).
    #[texture(120)]
    #[sampler(121)]
    pub graph_image3: Option<Handle<Image>>,
    #[texture(122)]
    #[sampler(123)]
    pub graph_image4: Option<Handle<Image>>,
    #[texture(124)]
    #[sampler(125)]
    pub graph_image5: Option<Handle<Image>>,
    /// D189: x = step count (0 = off).
    #[uniform(126)]
    graph_info: UVec4,
    /// D189: `lbp_preview::FragmentStep::code` per step.
    #[uniform(127)]
    graph_code: [UVec4; 32],
    /// D189: `lbp_preview::FragmentStep::param` per step.
    #[uniform(128)]
    graph_param: [Vec4; 32],
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
/// D189: per-fragment graph over raw UV0 (`uv`) and UV1 (`uv_b`); `images[k]` is graph
/// texture k (at most 6), `steps` the encoded D100 program (root last, at most 32).
pub fn enable_graph(
    material: &mut ViewerMaterial,
    images: Vec<Handle<Image>>,
    steps: &[lbp_preview::FragmentStep],
) -> Result<(), String> {
    if images.is_empty()
        || images.len() > lbp_preview::MAX_FRAGMENT_TEXTURES
        || steps.is_empty()
        || steps.len() > lbp_preview::MAX_FRAGMENT_STEPS
    {
        return Err("Invalid: per-fragment graph budget".into());
    }
    let ext = &mut material.extension;
    let mut images = images.into_iter();
    ext.image = images.next();
    ext.third_image = images.next();
    ext.fourth_image = images.next();
    ext.graph_image3 = images.next();
    ext.graph_image4 = images.next();
    ext.graph_image5 = images.next();
    for (k, step) in steps.iter().enumerate() {
        ext.graph_code[k] = UVec4::from_array(step.code);
        ext.graph_param[k] = Vec4::from_array(step.param);
    }
    ext.graph_info = UVec4::new(steps.len() as u32, 0, 0, 0);
    Ok(())
}
/// P2q: bump map sampled on UV1, offset (.yw - 0.502) × `level` (docs/p2q-bump-gloss.md).
pub fn enable_bump(material: &mut ViewerMaterial, image: Handle<Image>, level: f32) {
    material.extension.bump_image = Some(image);
    material.extension.bump_params = Vec4::new(level, 1., 0., 0.);
}
/// P3d: gloss colour map on `graph_image5` (free unless a D189 graph is bound) with the
/// static gloss white; `uv_mode` 1 = primary UV, 2 = UV1. Call only without a graph.
pub fn enable_spec_map(material: &mut ViewerMaterial, image: Handle<Image>, uv_mode: u32) {
    material.extension.graph_image5 = Some(image);
    material.extension.graph_info.y = uv_mode;
}
/// P3a: alpha cut-out: fragments whose colour alpha is below `level` are discarded (LBP
/// mode materials with a stored AlphaTestLevel, docs/p3a-alpha-test.md).
pub fn enable_alpha_test(material: &mut ViewerMaterial, level: f32) {
    material.extension.bump_params.z = level;
}
/// P2q: static gloss colour with exponent `power` (CosinePower).
pub fn enable_gloss(material: &mut ViewerMaterial, color: [f32; 3], power: f32) {
    material.extension.gloss = Vec3::from(color).extend(power.max(1.));
}
/// D121: applies (Some) or clears (None) preview lighting; returns whether the material
/// is lit. Untrusted normals keep the unlit colour (counted by the caller).
pub fn set_lighting(
    material: &mut ViewerMaterial,
    light: Option<&crate::lighting::PreviewLight>,
) -> bool {
    let ext = &mut material.extension;
    // P2s: LBP mode lights every draw; untrusted smooth normals use the face normal (3).
    match light.filter(|l| ext.normals_trusted || l.lbp) {
        Some(l) => {
            let mode = match (l.lbp, ext.normals_trusted) {
                (false, _) => 1.,
                (true, true) => 2.,
                (true, false) => 3.,
            };
            ext.light_dir = l.toward_light.extend(mode);
            // P2q: .w carries the game's scalar per light (brdf.cg `light.w`).
            ext.light_key = l.key.extend(l.scales.x);
            ext.light_ambient = l.ambient.extend(l.scales.y);
            ext.rim_low = l.rim[0].extend(l.scales.z);
            ext.rim_high = l.rim[1].extend(l.scales.w);
            ext.lbp_fog = l.lbp_fog();
            ext.graph_info.z = u32::from(l.shadows);
            ext.lbp_fog_color = l.lbp_fog_color().extend(1.);
            true
        }
        None => {
            ext.light_dir = Vec4::ZERO;
            ext.light_key = Vec4::ZERO;
            ext.light_ambient = Vec4::ZERO;
            ext.rim_low = Vec4::ZERO;
            ext.rim_high = Vec4::ZERO;
            ext.lbp_fog = Vec4::ZERO;
            ext.graph_info.z = 0;
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
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::{forward_io::{VertexOutput, FragmentOutput}, pbr_fragment::pbr_input_from_standard_material, pbr_functions::main_pass_post_lighting_processing, shadows::fetch_directional_shadow}
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
@group(#{MATERIAL_BIND_GROUP}) @binding(114) var bump_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(115) var bump_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(116) var<uniform> bump_params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(117) var<uniform> gloss: vec4<f32>;
// P2q: screen-space derivatives for the bump frame, taken in uniform control flow.
struct BumpInput {
    dp1: vec3<f32>,
    dp2: vec3<f32>,
    duv1: vec2<f32>,
    duv2: vec2<f32>,
    offset: vec2<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(118) var<uniform> lbp_fog: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(119) var<uniform> lbp_fog_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var graph_image3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(121) var graph_sampler3: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(122) var graph_image4: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(123) var graph_sampler4: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(124) var graph_image5: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(125) var graph_sampler5: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(126) var<uniform> graph_info: vec4<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(127) var<uniform> graph_code: array<vec4<u32>, 32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(128) var<uniform> graph_param: array<vec4<f32>, 32>;
// D189 graph texture k at uv with explicit gradients (non-uniform control flow).
fn graph_texture(k: u32, uv: vec2<f32>, gx: vec2<f32>, gy: vec2<f32>) -> vec4<f32> {
    switch k {
        case 0u: { return textureSampleGrad(second_image, second_sampler, uv, gx, gy); }
        case 1u: { return textureSampleGrad(third_image, third_sampler, uv, gx, gy); }
        case 2u: { return textureSampleGrad(fourth_image, fourth_sampler, uv, gx, gy); }
        case 3u: { return textureSampleGrad(graph_image3, graph_sampler3, uv, gx, gy); }
        case 4u: { return textureSampleGrad(graph_image4, graph_sampler4, uv, gx, gy); }
        default: { return textureSampleGrad(graph_image5, graph_sampler5, uv, gx, gy); }
    }
}
// D189 input: step (code >> 3), lane 0 = all four, 1..4 = broadcast lane - 1.
fn graph_lane(v: vec4<f32>, code: u32) -> vec4<f32> {
    let lane = code & 7u;
    if lane == 0u {
        return v;
    }
    return vec4<f32>(v[min(lane - 1u, 3u)]);
}
// D189 per-fragment D100 program (mirrors lbp_preview::evaluate_fragment): linear texels,
// MULTIPLY/ADD per lane, MIX (r - l) * f + l, BLEND l + (r - l) * clamp((m.x - p0)/(p1 - p0)).
fn graph_color(uv0: vec2<f32>, uv1: vec2<f32>, d0: vec4<f32>, d1: vec4<f32>) -> vec4<f32> {
    var values: array<vec4<f32>, 32>;
    let count = min(graph_info.x, 32u);
    for (var k = 0u; k < count; k++) {
        let code = graph_code[k];
        let p = graph_param[k];
        let a = graph_lane(values[min(code.y >> 3u, 31u)], code.y);
        let b = graph_lane(values[min(code.z >> 3u, 31u)], code.z);
        let c = graph_lane(values[min(code.w >> 3u, 31u)], code.w);
        var v = vec4<f32>(0.0);
        switch code.x {
            case 1u: {
                var base = uv0;
                var d = d0;
                if code.z == 1u {
                    base = uv1;
                    d = d1;
                }
                v = graph_texture(code.y, base * p.xy + p.zw, d.xy * p.xy, d.zw * p.xy);
            }
            case 2u: { v = p; }
            case 3u: { v = a * b; }
            case 4u: { v = a + b; }
            case 5u: {
                var f = c;
                if code.w == 0xffffffffu {
                    f = vec4<f32>(p.x);
                }
                v = (b - a) * f + a;
            }
            case 6u: { v = (b - a) * clamp((c.x - p.x) / (p.y - p.x), 0.0, 1.0) + a; }
            default: {}
        }
        values[k] = v;
    }
    return values[max(count, 1u) - 1u];
}
// D121 preview Lambert factor (mirrors lighting::PreviewLight::factor/facing_normal).
// Double-sided: the smooth normal is turned into the hemisphere of the face normal that
// points at the viewer, so back faces and mirrored instances light like front faces.
fn preview_light(n: vec3<f32>, to_view: vec3<f32>, shadow: f32) -> vec3<f32> {
    if dot(n, n) <= 0.0 {
        return vec3<f32>(1.0);
    }
    if light_dir.w > 1.5 {
        // P2l (mirrors lighting::PreviewLight::lbp_factor): hemisphere ambient, sun,
        // grazing rim lerped by N.y (P2n: P2m's eye-facing rim reverted, D203); squared
        // for the game's square-then-sqrt combine.
        let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
        let f = light_ambient.rgb * (n.y * 0.15 + 0.5)
            + light_key.rgb * (max(dot(n, light_dir.xyz), 0.0) * shadow)
            + mix(rim_low.rgb, rim_high.rgb, (n.y + 1.0) * 0.5) * rim;
        return f * f;
    }
    return light_ambient.rgb + light_key.rgb * max(dot(n, light_dir.xyz), 0.0);
}
// P2q gloss (LBP mode only), brdf.cg combine `2 * light.w * light^2 * spec`: light.w
// sums the game's per-light scalars with the same weights as the colours, light^2 is
// the squared factor, spec = gloss colour x pow(saturate(R.L), CosinePower) with R the
// mirrored view ray.
fn gloss_term(n: vec3<f32>, to_view: vec3<f32>, factor: vec3<f32>, tint: vec3<f32>, shadow: f32) -> vec3<f32> {
    if light_dir.w < 1.5 || gloss.w <= 0.0 || dot(n, n) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
    let w = light_ambient.w * (n.y * 0.15 + 0.5)
        + light_key.w * (max(dot(n, light_dir.xyz), 0.0) * shadow)
        + mix(rim_low.w, rim_high.w, (n.y + 1.0) * 0.5) * rim;
    let r = reflect(-to_view, n);
    let spec = gloss.rgb * tint * (pow(clamp(dot(r, light_dir.xyz), 0.0, 1.0), gloss.w) * shadow);
    return 2.0 * w * factor * spec;
}
// Facing normal, tilted by the bump offset in a cotangent frame built from the
// position and UV1 derivatives (LBP mode with a bound bump map only).
fn lit_normal(shading: vec3<f32>, face: vec3<f32>, to_view: vec3<f32>, bump: BumpInput) -> vec3<f32> {
    if dot(shading, shading) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n = facing_normal(shading, face, to_view);
    if light_dir.w < 1.5 || bump_params.y < 0.5 {
        return n;
    }
    let dp2perp = cross(bump.dp2, n);
    let dp1perp = cross(n, bump.dp1);
    let t = dp2perp * bump.duv1.x + dp1perp * bump.duv2.x;
    let b = dp2perp * bump.duv1.y + dp1perp * bump.duv2.y;
    let size = max(dot(t, t), dot(b, b));
    if size <= 1e-20 {
        return n;
    }
    let scale = inverseSqrt(size);
    return normalize(n + (t * bump.offset.x + b * bump.offset.y) * scale);
}
// P2r (RSX capture, all lit Pod draws): the game writes sqrt(colour x tintcol) with
// tintcol (4,4,4) into a plain RGBA8 target, so that sqrt is the only output gamma.
// Bevy sRGB-encodes our linear output, so return the linear value whose encoding is it.
const GAME_TINT: f32 = 4.0;
// P2t: then the game's fog in that output space, by view depth (lbp_fog).
fn lbp_output(c: vec3<f32>, depth: f32) -> vec3<f32> {
    var v = clamp(sqrt(max(c * GAME_TINT, vec3<f32>(0.0))), vec3<f32>(0.0), vec3<f32>(1.0));
    if lbp_fog.w > 0.5 {
        let k = clamp((depth - lbp_fog.x) * lbp_fog.y, 0.0, 1.0);
        let fogged = (dot(v, vec3<f32>(0.3, 0.6, 0.1)) * lbp_fog.z + 1.0) * lbp_fog_color.rgb;
        v = clamp(mix(v, fogged, k), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let low = v / 12.92;
    let high = pow((v + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, v <= vec3<f32>(0.04045));
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
    let dp1 = dpdx(in.world_position.xyz);
    let dp2 = dpdy(in.world_position.xyz);
    let face = cross(dp1, dp2);
    var bump = BumpInput(dp1, dp2, vec2<f32>(0.0), vec2<f32>(0.0), vec2<f32>(0.0));
#ifdef VERTEX_UVS_B
    bump.duv1 = dpdx(in.uv_b);
    bump.duv2 = dpdy(in.uv_b);
    if bump_params.y > 0.5 {
        let texel = textureSample(bump_image, bump_sampler, in.uv_b);
        bump.offset = (texel.yw - vec2<f32>(0.502)) * bump_params.x;
    }
#endif
    var color = pbr.material.base_color;
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_B
    // D189: gradients taken here, in uniform control flow, for textureSampleGrad.
    let graph_d0 = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    let graph_d1 = vec4<f32>(dpdx(in.uv_b), dpdy(in.uv_b));
    if graph_info.x > 0u {
        let g = graph_color(in.uv, in.uv_b, graph_d0, graph_d1);
        color = vec4<f32>(clamp(g.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
#endif
#endif
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
    if bump_params.z > 0.0 && color.a < bump_params.z {
        discard;
    }
    var rgb = color.rgb;
    if light_dir.w > 0.5 {
        // P2s: mode 3 (untrusted smooth normals) shades with the face normal.
        let shading = select(in.world_normal, face, light_dir.w > 2.5);
        let n = lit_normal(shading, face, pbr.V, bump);
        // P3g: Bevy's cascaded sun-shadow map (one directional light, set up by the viewer),
        // LBP-lit level meshes only; 1 = lit.
        var shadow = 1.0;
        if light_dir.w > 1.5 && graph_info.z > 0u {
            let view_z = (view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
            shadow = fetch_directional_shadow(0u, in.world_position, in.world_normal, view_z, in.position.xy);
        }
        let factor = preview_light(n, pbr.V, shadow);
        // P3d: gloss colour map (port 2 sampler) on graph slot 5, unused without a graph.
        var spec_tint = vec3<f32>(1.0);
#ifdef VERTEX_UVS_A
        if graph_info.y == 1u {
            spec_tint = textureSample(graph_image5, graph_sampler5, in.uv).rgb;
        }
#endif
#ifdef VERTEX_UVS_B
        if graph_info.y == 2u {
            spec_tint = textureSample(graph_image5, graph_sampler5, in.uv_b).rgb;
        }
#endif
        rgb = rgb * factor + gloss_term(n, pbr.V, factor, spec_tint, shadow);
        if light_dir.w > 1.5 {
            let depth = -(view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
            rgb = lbp_output(rgb, depth);
        }
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
struct ViewStub {
    view_from_world: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewStub;
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
fn fetch_directional_shadow(id: u32, p: vec4<f32>, n: vec3<f32>, z: f32, xy: vec2<f32>) -> f32 {
    return 1.0;
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
        assert!(body.contains("lit_normal(shading, face, pbr.V, bump)"));
        assert!(body.contains("gloss_term(n, pbr.V, factor, spec_tint, shadow)"));
        assert!(body.contains("rgb = lbp_output(rgb, depth);"));
        assert!(body.contains("graph_color(in.uv, in.uv_b, graph_d0, graph_d1)"));
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
