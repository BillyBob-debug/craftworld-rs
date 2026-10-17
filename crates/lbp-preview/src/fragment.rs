//! D189: per-fragment evaluation of D100 graphs whose texture samples mix UV0 and UV1
//! outside a separable product (docs/d189-mixed-uv-graphs.md). No bake: one value per
//! fragment needs every leaf at its own coordinates, which a single texture cannot hold.
//! The binding carries the compiled D100 steps (same node rules: MULTIPLY, ADD, MIX,
//! BLEND with D94 thresholds, COLOR/CONSTANT values, folded instance colour as COLOR)
//! encoded for the Bevy viewer's shader, which evaluates them per fragment on linear
//! texels. `evaluate` is the CPU reference of that shader loop (unit tests only compare
//! it with the D100 evaluator). The legacy viewer refuses these bindings.
use crate::grid_graph::{LoadContext, Source, Step};

/// Shader texture bindings for graph textures (distinct slots).
pub const MAX_FRAGMENT_TEXTURES: usize = 6;
/// Shader step array length (the D100 node budget).
pub const MAX_FRAGMENT_STEPS: usize = 32;
pub const FRAGMENT_SAMPLE: u32 = 1;
pub const FRAGMENT_VALUE: u32 = 2;
pub const FRAGMENT_MULTIPLY: u32 = 3;
pub const FRAGMENT_ADD: u32 = 4;
pub const FRAGMENT_MIX: u32 = 5;
pub const FRAGMENT_BLEND: u32 = 6;
/// MIX `code[3]` when the factor is the unwired weight in `param[0]`.
pub const FRAGMENT_WEIGHT: u32 = u32::MAX;

/// One shader step. `code` = (op, a, b, c); inputs are `step << 3 | lane`, lane 0 = all
/// four lanes, 1..=4 = broadcast lane 0..=3. Sample: (op, texture, UV channel, 0) with
/// `param` = (scale u, scale v, offset u, offset v). Value: `param` = RGBA. MIX weight
/// and BLEND thresholds (p0, p1) sit in `param`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentStep {
    pub code: [u32; 4],
    pub param: [f32; 4],
}
pub struct FragmentTexture {
    pub texture: lbp_texture::Texture,
    pub texture_key: lbp_catalog::attachment::LocationKey,
    pub wrap: crate::TextureWrapping,
}
/// Per-fragment graph binding. Texture 0 is the asset's primary texture (with its
/// `wrap`); `textures[k]` is texture k + 1. The asset's `sampling_uv` is raw UV0.
pub struct FragmentGraph {
    pub textures: Vec<FragmentTexture>,
    pub steps: Vec<FragmentStep>,
    /// Raw UV1 per vertex.
    pub uv1: Vec<[f32; 2]>,
}

fn input((p, lane): (usize, Option<u8>)) -> u32 {
    ((p as u32) << 3) | lane.map_or(0, |l| u32::from(l) + 1)
}

/// Distinct texture slots in source order (texture index = position).
pub(crate) fn slots(sources: &[Source]) -> Vec<usize> {
    let mut slots = vec![];
    for s in sources {
        if !slots.contains(&s.slot) {
            slots.push(s.slot);
        }
    }
    slots
}

/// Encode D100 steps for the shader. Every value is the f32 the D100 compile read
/// (COLOR/CONSTANT words, MIX weight, BLEND thresholds), so nothing is re-rounded.
pub(crate) fn encode(steps: &[Step], sources: &[Source]) -> Result<Vec<FragmentStep>, String> {
    if steps.is_empty() || steps.len() > MAX_FRAGMENT_STEPS {
        return Err("Unsupported: graph evaluator per-fragment step budget".into());
    }
    let slots = slots(sources);
    if slots.len() > MAX_FRAGMENT_TEXTURES {
        return Err(
            "Unsupported: graph evaluator per-fragment path takes at most 6 textures".into(),
        );
    }
    let narrow = |v: [f64; 4]| v.map(|x| x as f32);
    steps
        .iter()
        .map(|step| {
            Ok(match step {
                Step::Sample(s) => {
                    let source = sources.get(*s).ok_or("Invalid: graph evaluator source")?;
                    let texture = slots
                        .iter()
                        .position(|&slot| slot == source.slot)
                        .ok_or("Invalid: graph evaluator slot")?;
                    if source.uv_channel > 1 {
                        return Err("Unsupported: graph evaluator per-fragment UV channel".into());
                    }
                    FragmentStep {
                        code: [FRAGMENT_SAMPLE, texture as u32, source.uv_channel as u32, 0],
                        param: narrow(crate::offgrid::scale_offset(source)?),
                    }
                }
                Step::Value(v) => FragmentStep {
                    code: [FRAGMENT_VALUE, 0, 0, 0],
                    param: narrow(*v),
                },
                Step::Multiply([a, b]) => FragmentStep {
                    code: [FRAGMENT_MULTIPLY, input(*a), input(*b), 0],
                    param: [0.; 4],
                },
                Step::Add([a, b]) => FragmentStep {
                    code: [FRAGMENT_ADD, input(*a), input(*b), 0],
                    param: [0.; 4],
                },
                Step::Mix([a, b], f) => FragmentStep {
                    code: [
                        FRAGMENT_MIX,
                        input(*a),
                        input(*b),
                        (*f).map_or(FRAGMENT_WEIGHT, input),
                    ],
                    param: [(*f).err().unwrap_or(0.) as f32, 0., 0., 0.],
                },
                Step::Blend([a, b, m], [p0, p1]) => FragmentStep {
                    code: [FRAGMENT_BLEND, input(*a), input(*b), input(*m)],
                    param: [*p0 as f32, *p1 as f32, 0., 0.],
                },
            })
        })
        .collect()
}

/// CPU reference of the shader loop (f32, linear values; the root is the last step).
/// `sample(texture, uv)` returns the linear texel at the step's transformed coordinates.
pub fn evaluate(
    steps: &[FragmentStep],
    uv: [[f32; 2]; 2],
    sample: &dyn Fn(u32, [f32; 2]) -> [f32; 4],
) -> Option<[f32; 4]> {
    let mut values: Vec<[f32; 4]> = Vec::with_capacity(steps.len());
    for step in steps.iter().take(MAX_FRAGMENT_STEPS) {
        let get = |code: u32| -> Option<[f32; 4]> {
            let v = *values.get((code >> 3) as usize)?;
            Some(match code & 7 {
                0 => v,
                l => [*v.get(l as usize - 1)?; 4],
            })
        };
        let [op, a, b, c] = step.code;
        let p = step.param;
        let v = match op {
            FRAGMENT_SAMPLE => {
                let base = *uv.get(b as usize)?;
                sample(a, [base[0] * p[0] + p[2], base[1] * p[1] + p[3]])
            }
            FRAGMENT_VALUE => p,
            FRAGMENT_MULTIPLY => {
                let (l, r) = (get(a)?, get(b)?);
                std::array::from_fn(|k| l[k] * r[k])
            }
            FRAGMENT_ADD => {
                let (l, r) = (get(a)?, get(b)?);
                std::array::from_fn(|k| l[k] + r[k])
            }
            FRAGMENT_MIX => {
                let (l, r) = (get(a)?, get(b)?);
                let f = if c == FRAGMENT_WEIGHT {
                    [p[0]; 4]
                } else {
                    get(c)?
                };
                std::array::from_fn(|k| (r[k] - l[k]) * f[k] + l[k])
            }
            FRAGMENT_BLEND => {
                let (l, r) = (get(a)?, get(b)?);
                let f = ((get(c)?[0] - p[0]) / (p[1] - p[0])).clamp(0., 1.);
                std::array::from_fn(|k| (r[k] - l[k]) * f + l[k])
            }
            _ => return None,
        };
        values.push(v);
    }
    values.last().copied()
}

/// Selection-time check: scale/offset samplers on UV0/UV1, at most 6 texture slots and
/// a step list the shader takes.
pub(crate) fn plan(
    program: &crate::GraphProgram,
    sources: &[Source],
) -> Result<crate::offgrid::Plan, String> {
    for s in sources {
        crate::offgrid::scale_offset(s)?;
    }
    let steps = encode(&crate::grid_graph::steps(program, sources)?, sources)?;
    let samples = steps
        .iter()
        .filter(|s| s.code[0] == FRAGMENT_SAMPLE)
        .count();
    Ok(crate::offgrid::Plan {
        kind: crate::OffGrid::Fragment {
            samples: samples as u8,
            textures: slots(sources).len() as u8,
        },
        second_sample: None,
    })
}

fn texture_budget(t: &lbp_texture::Texture) -> Result<(), String> {
    let (w, h) = (t.header.width, t.header.height);
    let count = u64::from(w) * u64::from(h);
    if w == 0 || h == 0 || count.checked_mul(4) != Some(t.rgba.len() as u64) {
        return Err("Invalid: graph evaluator source extent".into());
    }
    // Same GPU budget as the viewer's upload (4096 per axis, 4 Mi pixels).
    if w > 4096 || h > 4096 || count > lbp_texture::MAX_PIXELS as u64 {
        return Err("Unsupported: GPU texture budget (4 Mi pixels)".into());
    }
    Ok(())
}

/// Loader branch for a `Fragment` selection (called from `grid_graph::load`).
pub(crate) fn load(
    cx: &LoadContext,
    material_key: lbp_catalog::attachment::LocationKey,
    instance_projection: Option<crate::InstanceProjection>,
    selection: crate::DiffuseSelection,
    program: &crate::GraphProgram,
    sources: &[Source],
) -> Result<crate::MaterialAsset, String> {
    let steps = encode(&crate::grid_graph::steps(program, sources)?, sources)?;
    let slots = slots(sources);
    if slots.first() != Some(&selection.slot) {
        return Err("Invalid: graph evaluator primary slot".into());
    }
    let prefix = &cx.graph.material.prefix;
    let mut textures = vec![];
    for &slot in &slots {
        // Repeat, Mirror or ClampToEdge: sampler address modes; BORDER/clamp (5) and
        // mirror-once stay refused (no validated per-fragment domain).
        let raw = [prefix.wrap_s_raw.get(slot), prefix.wrap_t_raw.get(slot)];
        if raw.iter().any(|r| !matches!(r, Some(1..=3))) {
            return Err(
                "Unsupported: graph evaluator per-fragment path needs Repeat, Mirror or ClampToEdge on every texture"
                    .into(),
            );
        }
        let wrap = crate::TextureWrapping::from_raw(raw[0].copied(), raw[1].copied())?;
        let (texture_key, texture) = crate::grid_graph::decode(cx, slot, false)?;
        texture_budget(&texture)?;
        textures.push(FragmentTexture {
            texture,
            texture_key,
            wrap,
        });
    }
    let additional_texture_slots =
        crate::instance::additional_texture_slots(cx.original_graph, &slots)?;
    let coordinates = cx.uv.decode(cx.control)?;
    let uv0 = crate::linked::selected_uv(&coordinates, 0, cx.vertices)?;
    let uv1 = crate::linked::selected_uv(&coordinates, 1, cx.vertices)?;
    if uv0.iter().chain(&uv1).flatten().any(|v| !v.is_finite()) {
        return Err("Invalid: nonfinite raw UV".into());
    }
    let uv = if selection.uv_channel == 0 {
        uv0.clone()
    } else {
        uv1.clone()
    };
    let mut textures = textures.into_iter();
    let primary = textures.next().ok_or("Invalid: graph evaluator textures")?;
    cx.control.check()?;
    cx.source.check_stable()?;
    Ok(crate::MaterialAsset::Texture(Box::new(crate::Asset {
        surface: None,
        instance_projection,
        additional_texture_slots,
        border_domain: None,
        fourth: None,
        third: None,
        dual: None,
        product: None,
        tinted_rgba: None,
        sampling_uv: uv0,
        uv,
        uv_metadata: coordinates.metadata,
        texture: primary.texture,
        material_key,
        texture_key: primary.texture_key,
        wrap: primary.wrap,
        selection,
        runtime_texture: None,
        overlay_alpha: false,
        fragment: Some(Box::new(FragmentGraph {
            textures: textures.collect(),
            steps,
            uv1,
        })),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, GraphMetadata, WireMetadata};

    fn node(kind: u32, params: [f32; 8]) -> BoxMetadata {
        BoxMetadata {
            type_raw: kind,
            subtype_raw: 0,
            declared_parameter_count: 8,
            parameters_raw: params.map(f32::to_bits),
            position_bits: [0; 4],
            texture_slot_raw: None,
            texture_slot_in_range: None,
            animations: std::array::from_fn(|_| AnimationMetadata {
                base_value_bits: [0; 4],
                component_value_count: 0,
                component_bytes_sha256: String::new(),
                name_hex: "000000".into(),
                components_raw: 0,
                derived_key_count: 0,
            }),
        }
    }
    fn sample(slot: u32, channel: u32, scale: [f32; 2], offset: [f32; 2]) -> BoxMetadata {
        let mut b = node(
            1,
            [scale[0], scale[1], offset[0], offset[1], 0., 0., 0., 0.],
        );
        b.parameters_raw[4] = channel;
        b.parameters_raw[5] = slot;
        b
    }
    fn edge(from: i32, to: i32, port: u8) -> WireMetadata {
        WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw: port,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        }
    }
    fn graph(boxes: Vec<BoxMetadata>, wires: Vec<WireMetadata>) -> GraphMetadata {
        GraphMetadata {
            box_count: boxes.len(),
            wire_count: wires.len(),
            boxes,
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        }
    }
    const NO: [f32; 8] = [0.; 8];

    /// Shape of 16046-like rows: BLEND(T0 UV0 × T1 UV1, T2 UV0 × COLOR, mask T3 UV1).
    fn mixed_blend() -> GraphMetadata {
        graph(
            vec![
                node(0, NO),
                node(16, [0.3, 0.7, 0., 0., 0., 0., 0., 0.]),
                node(10, NO),
                sample(0, 0, [2., 2.], [0., 0.]),
                sample(1, 1, [1., 1.], [0.25, 0.]),
                node(10, NO),
                sample(2, 0, [3.5, 1.], [0., 0.]),
                node(3, [0.5, 0.25, 1., 1., 0., 0., 0., 0.]),
                sample(3, 1, [1., 1.], [0., 0.]),
            ],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(5, 1, 1),
                edge(8, 1, 2),
                edge(3, 2, 0),
                edge(4, 2, 1),
                edge(6, 5, 0),
                edge(7, 5, 1),
            ],
        )
    }

    #[test]
    fn d189_mixed_uv_blend_and_mix_select_the_fragment_path() {
        let s = crate::select_diffuse(&mixed_blend()).unwrap();
        let g = s.grid_graph.unwrap();
        assert_eq!(
            g.off_grid,
            Some(crate::OffGrid::Fragment {
                samples: 4,
                textures: 4
            })
        );
        assert!(g.label().contains("per fragment"));
        assert!(s.second_sample.is_none());
        // Static MIX(T0 UV0, T1 UV1, weight) (15658-like) and a two-texture BLEND whose
        // mask is the UV1 sample (108243-like) take the same path.
        let mix = graph(
            vec![
                node(0, NO),
                node(12, [0.4, 0., 0., 0., 0., 0., 0., 0.]),
                sample(0, 0, [1., 1.], [0., 0.]),
                sample(1, 1, [1., 1.], [0., 0.]),
            ],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1)],
        );
        let g = crate::select_diffuse(&mix).unwrap().grid_graph.unwrap();
        assert!(matches!(
            g.off_grid,
            Some(crate::OffGrid::Fragment { samples: 2, .. })
        ));
        let blend2 = graph(
            vec![
                node(0, NO),
                node(16, [0., 1., 0., 0., 0., 0., 0., 0.]),
                sample(0, 0, [1., 1.], [0., 0.]),
                sample(1, 1, [1., 1.], [0., 0.]),
            ],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1), edge(3, 1, 2)],
        );
        let g = crate::select_diffuse(&blend2).unwrap().grid_graph.unwrap();
        assert!(matches!(
            g.off_grid,
            Some(crate::OffGrid::Fragment {
                samples: 2,
                textures: 2
            })
        ));
        // Matrix samplers stay refused with the existing reason.
        let mut matrix = mixed_blend();
        matrix.boxes[4].subtype_raw = 1;
        assert!(crate::select_diffuse(&matrix)
            .unwrap_err()
            .ends_with("(graph evaluator: off-grid bake needs scale/offset samplers)"));
    }

    #[test]
    fn d189_encoded_steps_evaluate_like_the_d100_evaluator() {
        let g = mixed_blend();
        let program = crate::grid_graph_program(&g, 1).unwrap();
        let sources = program_sources(&program);
        let steps = crate::grid_graph::steps(&program, &sources).unwrap();
        let encoded = encode(&steps, &sources).unwrap();
        assert_eq!(encoded.len(), steps.len());
        // Synthetic texels: a smooth function of (texture, coordinates).
        let texel = |t: u32, uv: [f32; 2]| -> [f32; 4] {
            let x = (uv[0] * 1.7 + uv[1] * 0.3 + t as f32 * 0.21).fract();
            let y = (uv[1] * 0.9 + t as f32 * 0.37).fract();
            [x, y, (x + y) * 0.5, (x * y).sqrt()]
        };
        let slots = slots(&sources);
        for i in 0..50 {
            let uv0 = [i as f32 * 0.037, 1. - i as f32 * 0.011];
            let uv1 = [i as f32 * 0.023 + 0.1, i as f32 * 0.019];
            let got = evaluate(&encoded, [uv0, uv1], &texel).unwrap();
            let mut values = vec![[0f64; 4]; steps.len()];
            crate::grid_graph::evaluate(&steps, &mut values, None, &|s| {
                let src = &sources[s];
                let [sx, sy, ox, oy] = crate::offgrid::scale_offset(src).unwrap();
                let base = if src.uv_channel == 0 { uv0 } else { uv1 };
                let uv = [
                    (f64::from(base[0]) * sx + ox) as f32,
                    (f64::from(base[1]) * sy + oy) as f32,
                ];
                let t = slots.iter().position(|&x| x == src.slot).unwrap() as u32;
                texel(t, uv).map(f64::from)
            });
            let want = *values.last().unwrap();
            for k in 0..4 {
                assert!((f64::from(got[k]) - want[k]).abs() < 1e-5, "{i} {k}");
            }
        }
        // Budgets: more than 6 textures or 32 steps are refused.
        let many: Vec<Source> = (0..7)
            .map(|slot| Source {
                slot,
                uv_channel: 0,
                transform: crate::SamplingTransform::default(),
            })
            .collect();
        let samples: Vec<Step> = (0..7).map(Step::Sample).collect();
        assert!(encode(&samples, &many).unwrap_err().contains("at most 6"));
        let long: Vec<Step> = (0..33).map(|_| Step::Value([0.; 4])).collect();
        assert!(encode(&long, &many).unwrap_err().contains("step budget"));
        // Malformed codes are rejected by the reference loop (no out-of-range input).
        let bad = [FragmentStep {
            code: [FRAGMENT_ADD, 5 << 3, 0, 0],
            param: [0.; 4],
        }];
        assert_eq!(evaluate(&bad, [[0.; 2]; 2], &texel), None);
    }

    fn program_sources(program: &crate::GraphProgram) -> Vec<Source> {
        let mut out = vec![];
        for n in &program.nodes {
            if let crate::GraphOp::Sample {
                slot,
                uv_channel,
                transform,
            } = n.op
            {
                let s = Source {
                    slot,
                    uv_channel,
                    transform,
                };
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    }
}
