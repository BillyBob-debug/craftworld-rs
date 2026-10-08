//! D100: bounded CPU evaluator for static diffuse graphs. Runs only after every fixed
//! route (and the D99 product tree) refused a graph. Nodes follow the Toolkit
//! `GfxAssembler.resolve` rules already recorded in the D-docs (independently restated,
//! nothing copied): TEXTURE_SAMPLE, COLOR, CONSTANT, CONSTANT4, MULTIPLY, ADD, MIX and
//! BLEND (D94 thresholds, D97 mask colour handling). Every other box type is refused.
//! One sampling source: evaluated per texel of that texture (any accepted transform).
//! Two or more: evaluated on the D49 nearest Repeat grid (same UV channel, integer
//! scales 1..16, zero offset; Repeat checked at load). Evidence: docs/d100-grid-graph-bake.md.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_resource::material::{BoxMetadata, GraphInspection, GraphMetadata, WireMetadata};

/// Distinct boxes evaluated (the cached-resolve DAG, output excluded).
const MAX_NODES: usize = 32;
/// Longest wire path from the diffuse root to a leaf.
const MAX_DEPTH: usize = 12;
/// Distinct texture sample boxes.
const MAX_SAMPLES: usize = 6;
/// Bound on CONSTANT words and an unwired MIX weight (keeps every value finite).
const MAX_SCALAR: f32 = 16.;

/// Selection marker; the program is re-derived from the same graph at load time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridGraph {
    /// The box wired to the diffuse output port.
    pub root: usize,
    /// Distinct evaluated boxes.
    pub nodes: u8,
    /// Distinct texture sample boxes.
    pub samples: u8,
    /// Distinct sampling sources (slot, UV channel, transform); 1 = per-texel bake.
    pub sources: u8,
    /// D103: off the integer Repeat grid, a labelled preview approximation
    /// (docs/d103-offgrid-bake.md). `None` = the D100 per-texel or grid bake.
    pub off_grid: Option<crate::OffGrid>,
}
impl GridGraph {
    pub fn label(self) -> String {
        let base = format!(
            "evaluated graph: {} nodes, {} texture samples, {} sources",
            self.nodes, self.samples, self.sources
        );
        match self.off_grid {
            Some(o) => format!("{base} • {}", o.label()),
            None => base,
        }
    }
}

/// One node input: the source box and an optional single-component swizzle
/// (`r/x` 0, `g/y` 1, `b/z` 2, `a/w` 3) broadcast to all four lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphInput {
    pub node: usize,
    pub lane: Option<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixFactor {
    /// Wired port 2.
    Input(GraphInput),
    /// Unwired: parameter word 0.
    Weight(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOp {
    /// Texel: RGB sRGB-decoded, alpha linear.
    Sample {
        slot: usize,
        uv_channel: usize,
        transform: crate::SamplingTransform,
    },
    /// COLOR or CONSTANT4: four parameter words, each within 0..1.
    Color([u32; 4]),
    /// CONSTANT: one word broadcast to four lanes.
    Constant(u32),
    Multiply([GraphInput; 2]),
    Add([GraphInput; 2]),
    /// `(port1 - port0) * factor + port0`.
    Mix {
        inputs: [GraphInput; 2],
        factor: MixFactor,
    },
    /// `port0 + (port1 - port0) * clamp((m - p0)/(p1 - p0), 0, 1)`, `m` = first lane of port 2.
    Blend {
        inputs: [GraphInput; 3],
        threshold_bits: [u32; 2],
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphNode {
    pub index: usize,
    pub op: GraphOp,
}
/// Nodes in evaluation order (inputs before users); the root is last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProgram {
    pub root: usize,
    pub nodes: Vec<GraphNode>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) slot: usize,
    pub(crate) uv_channel: usize,
    pub(crate) transform: crate::SamplingTransform,
}
impl GraphProgram {
    fn sources(&self) -> Vec<Source> {
        let mut sources = vec![];
        for node in &self.nodes {
            if let GraphOp::Sample {
                slot,
                uv_channel,
                transform,
            } = node.op
            {
                let source = Source {
                    slot,
                    uv_channel,
                    transform,
                };
                if !sources.contains(&source) {
                    sources.push(source);
                }
            }
        }
        sources
    }
    fn samples(&self) -> impl Iterator<Item = usize> + '_ {
        self.nodes
            .iter()
            .filter(|n| matches!(n.op, GraphOp::Sample { .. }))
            .map(|n| n.index)
    }
}

fn unanimated(node: &BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
}

fn lane(swizzle: [u8; 5]) -> Result<Option<u8>, String> {
    match swizzle {
        [0, 0, 0, 0, 0] => Ok(None),
        [c, 0, 0, 0, 0] => match c {
            b'r' | b'x' => Ok(Some(0)),
            b'g' | b'y' => Ok(Some(1)),
            b'b' | b'z' => Ok(Some(2)),
            b'a' | b'w' => Ok(Some(3)),
            _ => Err("Unsupported: graph evaluator swizzle".into()),
        },
        _ => Err("Unsupported: graph evaluator needs unswizzled or one-component wires".into()),
    }
}

struct Compiler<'a> {
    graph: &'a GraphMetadata,
    nodes: Vec<GraphNode>,
    /// Result width per compiled node (1 = scalar, 4 = vector).
    widths: Vec<u8>,
    visiting: Vec<usize>,
}
impl Compiler<'_> {
    fn position(&self, index: usize) -> Option<usize> {
        self.nodes.iter().position(|n| n.index == index)
    }
    /// Inputs on exactly `required` ports plus any of `optional`, source port 0.
    fn ports(
        &self,
        index: usize,
        required: &[u8],
        optional: &[u8],
    ) -> Result<Vec<&WireMetadata>, String> {
        let mut incoming: Vec<_> = self
            .graph
            .wires
            .iter()
            .filter(|w| w.box_to as usize == index)
            .collect();
        incoming.sort_by_key(|w| w.port_to_raw);
        let ports: Vec<u8> = incoming.iter().map(|w| w.port_to_raw).collect();
        let allowed = |p: &u8| required.contains(p) || optional.contains(p);
        if incoming.iter().any(|w| w.port_from_raw != 0)
            || ports.windows(2).any(|p| p[0] == p[1])
            || !ports.iter().all(allowed)
            || !required.iter().all(|p| ports.contains(p))
        {
            return Err(format!(
                "Unsupported: graph evaluator box type {} needs inputs on ports {required:?}",
                self.graph.boxes[index].type_raw
            ));
        }
        Ok(incoming)
    }
    fn input(&mut self, wire: &WireMetadata, depth: usize) -> Result<(GraphInput, u8), String> {
        let lane = lane(wire.swizzle_raw)?;
        let node = wire.box_from as usize;
        let width = self.compile(node, depth + 1)?;
        Ok((
            GraphInput { node, lane },
            if lane.is_some() { 1 } else { width },
        ))
    }
    fn compile(&mut self, index: usize, depth: usize) -> Result<u8, String> {
        if let Some(p) = self.position(index) {
            return Ok(self.widths[p]);
        }
        if depth > MAX_DEPTH {
            return Err("Unsupported: graph evaluator depth budget".into());
        }
        if self.visiting.contains(&index) {
            return Err("Unsupported: graph evaluator cycle".into());
        }
        let node = &self.graph.boxes[index];
        let unwired = !self.graph.wires.iter().any(|w| w.box_to as usize == index);
        let p = node.parameters_raw;
        self.visiting.push(index);
        let (op, width) = match node.type_raw {
            1 => {
                let s = crate::linked::sampler(self.graph, index)?;
                (
                    GraphOp::Sample {
                        slot: s.slot,
                        uv_channel: s.uv_channel,
                        transform: s.uv_transform,
                    },
                    4,
                )
            }
            3 | 7 => {
                if !unanimated(node) || !unwired {
                    return Err(
                        "Unsupported: graph evaluator COLOR must be static and unwired".into(),
                    );
                }
                let color = crate::DiffuseTint {
                    bits: [p[0], p[1], p[2], p[3]],
                };
                color.validate()?;
                (GraphOp::Color(color.bits), 4)
            }
            4 => {
                let v = f32::from_bits(p[0]);
                if !unanimated(node) || !unwired || !v.is_finite() || v.abs() > MAX_SCALAR {
                    return Err("Unsupported: graph evaluator CONSTANT must be static, unwired and within -16..16".into());
                }
                (GraphOp::Constant(p[0]), 1)
            }
            10 | 11 | 12 | 16 => {
                if !unanimated(node) {
                    return Err(
                        "Unsupported: graph evaluator operator must be static subtype 0".into(),
                    );
                }
                let (required, optional): (&[u8], &[u8]) = match node.type_raw {
                    12 => (&[0, 1], &[2]),
                    16 => (&[0, 1, 2], &[]),
                    _ => (&[0, 1], &[]),
                };
                let wires: Vec<WireMetadata> = self
                    .ports(index, required, optional)?
                    .into_iter()
                    .cloned()
                    .collect();
                let mut inputs = vec![];
                let mut width = 1;
                for wire in &wires {
                    let (input, w) = self.input(wire, depth)?;
                    width = width.max(w);
                    inputs.push(input);
                }
                let op = match node.type_raw {
                    10 => GraphOp::Multiply([inputs[0], inputs[1]]),
                    11 => GraphOp::Add([inputs[0], inputs[1]]),
                    12 => GraphOp::Mix {
                        inputs: [inputs[0], inputs[1]],
                        factor: match inputs.get(2) {
                            Some(&f) => MixFactor::Input(f),
                            None => {
                                let w = f32::from_bits(p[0]);
                                if !w.is_finite() || w.abs() > MAX_SCALAR {
                                    return Err("Unsupported: graph evaluator MIX weight must be finite within -16..16".into());
                                }
                                MixFactor::Weight(p[0])
                            }
                        },
                    },
                    _ => {
                        crate::blend_thresholds([p[0], p[1]])?;
                        GraphOp::Blend {
                            inputs: [inputs[0], inputs[1], inputs[2]],
                            threshold_bits: [p[0], p[1]],
                        }
                    }
                };
                (op, width)
            }
            other => {
                return Err(format!(
                    "Unsupported: graph evaluator has no rule for box type {other}"
                ))
            }
        };
        self.visiting.pop();
        if self.nodes.len() >= MAX_NODES {
            return Err("Unsupported: graph evaluator node budget".into());
        }
        self.nodes.push(GraphNode { index, op });
        self.widths.push(width);
        Ok(width)
    }
}

/// Compile the diffuse graph under `root` (bounded, cycle-checked, cached per box as
/// the resolver's LOOKUP does). Public so `diffuse_parity` can compare it with its own
/// reading of the oracle rows.
pub fn grid_graph_program(graph: &GraphMetadata, root: usize) -> Result<GraphProgram, String> {
    if graph.boxes.len() > 1024
        || graph.wires.len() > 4096
        || root >= graph.boxes.len()
        || graph.wires.iter().any(|w| {
            w.box_from < 0
                || w.box_to < 0
                || w.box_from as usize >= graph.boxes.len()
                || w.box_to as usize >= graph.boxes.len()
        })
    {
        return Err("Unsupported: graph evaluator bounds".into());
    }
    let mut compiler = Compiler {
        graph,
        nodes: vec![],
        widths: vec![],
        visiting: vec![],
    };
    if compiler.compile(root, 0)? != 4 {
        return Err("Unsupported: graph evaluator root must be a four-component value".into());
    }
    let program = GraphProgram {
        root,
        nodes: compiler.nodes,
    };
    let samples = program.samples().count();
    if samples == 0 {
        return Err("Unsupported: graph evaluator needs a texture sample".into());
    }
    if samples > MAX_SAMPLES {
        return Err("Unsupported: graph evaluator texture sample budget".into());
    }
    Ok(program)
}

fn grid_scales(sources: &[Source]) -> Result<Vec<[u32; 2]>, String> {
    if sources.len() == 1 {
        return Ok(vec![[1, 1]]);
    }
    let channel = sources[0].uv_channel;
    sources
        .iter()
        .map(|s| {
            if s.uv_channel != channel {
                return Err(());
            }
            crate::product::scales(s.transform).map_err(|_| ())
        })
        .collect::<Result<Vec<_>, ()>>()
        .map_err(|()| {
            "Unsupported: graph evaluator requires same-channel integer Repeat grid samplers".into()
        })
}

/// Fallback after the fixed routes and the product tree refused `graph`. Graphs with a
/// box outside the evaluator's rules keep `earlier`; evaluable graphs that cannot be
/// baked report why.
pub(crate) fn select(
    graph: &GraphMetadata,
    animated: bool,
    earlier: String,
) -> Result<crate::DiffuseSelection, String> {
    if animated {
        return Err(earlier);
    }
    let boxes = &graph.boxes;
    let outputs: Vec<_> = (0..boxes.len())
        .filter(|&i| boxes[i].type_raw == 0)
        .collect();
    if outputs.len() != 1 || graph.wires.len() > 4096 {
        return Err(earlier);
    }
    let output = outputs[0];
    let diffuse: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == output && w.port_to_raw == 0)
        .collect();
    if diffuse.len() != 1
        || diffuse[0].port_from_raw != 0
        || diffuse[0].swizzle_raw != [0; 5]
        || !unanimated(&boxes[output])
        || diffuse[0].box_from < 0
        || diffuse[0].box_from as usize >= boxes.len()
    {
        return Err(earlier);
    }
    let root = diffuse[0].box_from as usize;
    let Ok(program) = grid_graph_program(graph, root) else {
        return Err(earlier);
    };
    let sources = program.sources();
    let first = program.samples().next().expect("one sample");
    let primary = crate::linked::sampler(graph, first)?;
    let mut off_grid = None;
    let mut second_sample = None;
    if grid_scales(&sources).is_err() {
        // D103: off the grid, a labelled resampled bake or a separable split. Keep the
        // fixed route's reason first (stable census groups), then say why the
        // otherwise evaluable graph has no bake.
        match crate::offgrid::plan(&program, &sources, primary.uv_channel) {
            Ok(plan) => {
                if let Some(index) = plan.second_sample {
                    second_sample = Some(crate::linked::sampler(graph, index)?);
                }
                off_grid = Some(plan.kind);
            }
            Err(reason) => {
                return Err(format!(
                    "{earlier} (graph evaluator: {})",
                    reason.trim_start_matches("Unsupported: graph evaluator ")
                ))
            }
        }
    }
    let edges = graph
        .wires
        .iter()
        .filter(|w| program.nodes.iter().any(|n| n.index == w.box_to as usize))
        .count();
    Ok(crate::DiffuseSelection {
        grid_graph: Some(GridGraph {
            root,
            nodes: program.nodes.len() as u8,
            samples: program.samples().count() as u8,
            sources: sources.len() as u8,
            off_grid,
        }),
        second_sample,
        output_index: output,
        sample_index: first,
        slot: primary.slot,
        uv_channel: primary.uv_channel,
        uv_transform: primary.uv_transform,
        ignored_nodes: boxes.len().saturating_sub(program.nodes.len() + 1),
        ignored_wires: graph.wires.len().saturating_sub(edges + 1),
        ..Default::default()
    })
}

/// Under a supplied instance color only the plain ThingColor-to-COLOR substitution is
/// evaluated; folds, nested tints and UV alpha subtraction stay with their own routes.
pub(crate) fn instance_supported(p: &crate::InstanceProjection) -> bool {
    p.nested_tint.is_none()
        && p.folded_sampler.is_none()
        && p.alpha_subtract_samplers.is_empty()
        && p.folded_root.is_none()
        && p.swizzled_color_edges.is_empty()
        && p.folded_operand.is_none()
        && p.sampler_port_folds.is_empty()
}

fn encode(linear: f64) -> u8 {
    let c = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1. / 2.4) - 0.055
    };
    (c * 255.).round().clamp(0., 255.) as u8
}

fn lcm(values: impl Iterator<Item = u64>) -> Result<u32, String> {
    let mut result = 1u64;
    for v in values {
        let (mut a, mut b) = (result, v);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        result = result
            .checked_div(a)
            .and_then(|r| r.checked_mul(v))
            .filter(|r| (1..=4096).contains(r))
            .ok_or("Unsupported: graph evaluator grid budget")?;
    }
    Ok(result as u32)
}

/// Bake `program` over `textures` (one per source, in `GraphProgram::sources` order)
/// with per-source grid `scales`. RGB in linear space, alpha linear, nearest texels;
/// output clamped at 8-bit encoding (same policy as the D49/D83/D89/D97 bakes).
fn bake(
    program: &GraphProgram,
    sources: &[Source],
    textures: &[([u32; 2], &[u8])],
    scales: &[[u32; 2]],
    cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    if textures.len() != sources.len() || scales.len() != sources.len() {
        return Err("Invalid: graph evaluator source count".into());
    }
    for (d, rgba) in textures {
        if d.contains(&0)
            || d.iter().any(|n| *n > 4096)
            || u64::from(d[0]) * u64::from(d[1]) > lbp_texture::MAX_PIXELS as u64
            || u64::from(d[0]) * u64::from(d[1]) * 4 != rgba.len() as u64
        {
            return Err("Invalid: graph evaluator source extent".into());
        }
    }
    if scales.iter().flatten().any(|s| !(1..=16).contains(s)) {
        return Err("Unsupported: graph evaluator scale budget".into());
    }
    let width = lcm(textures
        .iter()
        .zip(scales)
        .map(|((d, _), s)| u64::from(d[0]) * u64::from(s[0])))?;
    let height = lcm(textures
        .iter()
        .zip(scales)
        .map(|((d, _), s)| u64::from(d[1]) * u64::from(s[1])))?;
    let count = u64::from(width) * u64::from(height);
    if count > lbp_texture::MAX_PIXELS as u64 {
        return Err("Unsupported: graph evaluator pixel budget".into());
    }
    let steps = steps(program, sources)?;
    if cancelled() {
        return Err("Cancelled".into());
    }
    let linear: [f64; 256] = std::array::from_fn(|v| crate::blend_mask::srgb_decode(v as u8));
    let mut values = vec![[0f64; 4]; steps.len()];
    let mut texels = vec![0usize; textures.len()];
    let mut output = Vec::with_capacity(count as usize * 4);
    for y in 0..height {
        for x in 0..width {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            for (t, ((d, _), s)) in texels.iter_mut().zip(textures.iter().zip(scales)) {
                let px = (u64::from(x) * u64::from(d[0]) * u64::from(s[0]) / u64::from(width))
                    % u64::from(d[0]);
                let py = (u64::from(y) * u64::from(d[1]) * u64::from(s[1]) / u64::from(height))
                    % u64::from(d[1]);
                *t = (py as usize * d[0] as usize + px as usize) * 4;
            }
            let sample = |source: usize| {
                let (rgba, i) = (textures[source].1, texels[source]);
                [
                    linear[usize::from(rgba[i])],
                    linear[usize::from(rgba[i + 1])],
                    linear[usize::from(rgba[i + 2])],
                    f64::from(rgba[i + 3]) / 255.,
                ]
            };
            evaluate(&steps, &mut values, None, &sample);
            output.extend_from_slice(&encode_texel(values[steps.len() - 1])?);
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((width, height, output))
}

/// sRGB-encoded RGB and linear alpha, each clamped at 8-bit encoding (D100 policy).
pub(crate) fn encode_texel(v: [f64; 4]) -> Result<[u8; 4], String> {
    if v.iter().any(|c| !c.is_finite()) {
        return Err("Invalid: graph evaluator produced a non-finite value".into());
    }
    Ok([
        encode(v[0]),
        encode(v[1]),
        encode(v[2]),
        (v[3] * 255.).round().clamp(0., 255.) as u8,
    ])
}

/// One evaluation step; inputs refer to earlier step positions with an optional lane.
pub(crate) enum Step {
    /// Index into the program's source list.
    Sample(usize),
    Value([f64; 4]),
    Multiply([(usize, Option<u8>); 2]),
    Add([(usize, Option<u8>); 2]),
    Mix([(usize, Option<u8>); 2], Result<(usize, Option<u8>), f64>),
    Blend([(usize, Option<u8>); 3], [f64; 2]),
}

/// Lane-selected value of an earlier step (a one-letter swizzle broadcasts its lane).
pub(crate) fn lane_value(values: &[[f64; 4]], (p, lane): (usize, Option<u8>)) -> [f64; 4] {
    let v: [f64; 4] = values[p];
    lane.map_or(v, |l| [v[usize::from(l)]; 4])
}

/// Evaluate `steps` in order into `values`; `needed` (D103 split targets) skips steps
/// that no requested factor depends on. `sample` returns the linear RGBA of a source.
pub(crate) fn evaluate(
    steps: &[Step],
    values: &mut [[f64; 4]],
    needed: Option<&[bool]>,
    sample: &dyn Fn(usize) -> [f64; 4],
) {
    for k in 0..steps.len() {
        if needed.is_some_and(|n| !n[k]) {
            continue;
        }
        let get = |input: (usize, Option<u8>)| lane_value(values, input);
        values[k] = match &steps[k] {
            Step::Sample(source) => sample(*source),
            Step::Value(v) => *v,
            Step::Multiply([a, b]) => {
                let (a, b) = (get(*a), get(*b));
                std::array::from_fn(|c| a[c] * b[c])
            }
            Step::Add([a, b]) => {
                let (a, b) = (get(*a), get(*b));
                std::array::from_fn(|c| a[c] + b[c])
            }
            Step::Mix([l, r], f) => {
                let (l, r) = (get(*l), get(*r));
                let f = match f {
                    Ok(input) => get(*input),
                    Err(w) => [*w; 4],
                };
                std::array::from_fn(|c| (r[c] - l[c]) * f[c] + l[c])
            }
            Step::Blend([l, r, m], thresholds) => {
                let (l, r) = (get(*l), get(*r));
                let f = crate::blend_factor(get(*m)[0], *thresholds);
                std::array::from_fn(|c| (r[c] - l[c]) * f + l[c])
            }
        };
    }
}

/// Compile `program` into evaluation steps over `sources` (inputs before users, root last).
pub(crate) fn steps(program: &GraphProgram, sources: &[Source]) -> Result<Vec<Step>, String> {
    // Steps refer to earlier positions; samples to their source.
    let position = |index: usize| {
        program
            .nodes
            .iter()
            .position(|n| n.index == index)
            .ok_or("Invalid: graph evaluator input")
    };
    let mut steps = vec![];
    for (k, node) in program.nodes.iter().enumerate() {
        let at = |i: GraphInput| -> Result<(usize, Option<u8>), String> {
            let p = position(i.node)?;
            if p >= k || i.lane.is_some_and(|l| l > 3) {
                return Err("Invalid: graph evaluator order".into());
            }
            Ok((p, i.lane))
        };
        steps.push(match node.op {
            GraphOp::Sample {
                slot,
                uv_channel,
                transform,
            } => Step::Sample(
                sources
                    .iter()
                    .position(|s| {
                        *s == Source {
                            slot,
                            uv_channel,
                            transform,
                        }
                    })
                    .ok_or("Invalid: graph evaluator source")?,
            ),
            GraphOp::Color(bits) => Step::Value(bits.map(|b| f64::from(f32::from_bits(b)))),
            GraphOp::Constant(bits) => Step::Value([f64::from(f32::from_bits(bits)); 4]),
            GraphOp::Multiply([a, b]) => Step::Multiply([at(a)?, at(b)?]),
            GraphOp::Add([a, b]) => Step::Add([at(a)?, at(b)?]),
            GraphOp::Mix {
                inputs: [a, b],
                factor,
            } => Step::Mix(
                [at(a)?, at(b)?],
                match factor {
                    MixFactor::Input(f) => Ok(at(f)?),
                    MixFactor::Weight(w) => Err(f64::from(f32::from_bits(w))),
                },
            ),
            GraphOp::Blend {
                inputs: [a, b, m],
                threshold_bits,
            } => Step::Blend(
                [at(a)?, at(b)?, at(m)?],
                crate::blend_thresholds(threshold_bits)?,
            ),
        });
    }
    if program.nodes.last().map(|n| n.index) != Some(program.root) {
        return Err("Invalid: graph evaluator root".into());
    }
    Ok(steps)
}

/// Inputs shared with `linked::load_primitive`.
pub(crate) struct LoadContext<'a> {
    pub(crate) source: &'a AttachedSource,
    pub(crate) catalog: &'a Catalog,
    pub(crate) geometry: &'a lbp_mesh::Geometry,
    pub(crate) mesh_bytes: &'a [u8],
    pub(crate) graph: &'a GraphInspection,
    pub(crate) original_graph: &'a GraphMetadata,
    pub(crate) control: &'a LoadControl,
}

pub(crate) fn decode(
    cx: &LoadContext,
    slot: usize,
    repeat: bool,
) -> Result<(LocationKey, lbp_texture::Texture), String> {
    cx.control.check()?;
    let prefix = &cx.graph.material.prefix;
    let descriptor = prefix
        .textures
        .get(slot)
        .ok_or("Unsupported: graph evaluator texture slot")?;
    if descriptor.kind == 0 || descriptor.flags != 0 {
        return Err("Unsupported: absent texture or texture descriptor flags".into());
    }
    if repeat
        && (prefix.wrap_s_raw.get(slot) != Some(&1) || prefix.wrap_t_raw.get(slot) != Some(&1))
    {
        return Err("Unsupported: graph evaluator grid requires Repeat on every texture".into());
    }
    let key = crate::linked::resolve(cx.catalog, descriptor.guid, descriptor.sha1.as_deref())?;
    let texture = lbp_texture::decode_reader(
        key.size,
        |offset, count| {
            cx.source
                .read_hex_checked(&key, offset, count, cx.control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || cx.control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    Ok((key, texture))
}

/// Loader branch for a `grid_graph` selection (called from `linked::load_primitive`).
pub(crate) fn load(
    cx: LoadContext,
    material_key: LocationKey,
    instance_projection: Option<crate::InstanceProjection>,
    wrap: crate::TextureWrapping,
    selection: crate::DiffuseSelection,
) -> Result<crate::MaterialAsset, String> {
    let marker = selection
        .grid_graph
        .ok_or("Invalid: graph evaluator selection")?;
    if instance_projection
        .as_ref()
        .is_some_and(|p| !instance_supported(p))
    {
        return Err(
            "Unsupported: graph evaluator under a folded or rewritten instance input".into(),
        );
    }
    let program = grid_graph_program(&cx.graph.graph, marker.root)?;
    let sources = program.sources();
    if sources.len() != usize::from(marker.sources) {
        return Err("Invalid: graph evaluator program changed".into());
    }
    if let Some(kind) = marker.off_grid {
        return crate::offgrid::load(
            &cx,
            material_key,
            instance_projection,
            selection,
            &program,
            &sources,
            kind,
        );
    }
    let scales = grid_scales(&sources)?;
    let grid = sources.len() > 1;
    let mut slots: Vec<usize> = vec![];
    for s in &sources {
        if !slots.contains(&s.slot) {
            slots.push(s.slot);
        }
    }
    if slots[0] != selection.slot {
        return Err("Invalid: graph evaluator primary slot".into());
    }
    let additional_texture_slots =
        crate::instance::additional_texture_slots(cx.original_graph, &slots)?;
    let coordinates = lbp_mesh::decode_uvs(cx.mesh_bytes, || cx.control.check().is_err())
        .map_err(|e| e.to_string())?;
    let uv = crate::linked::selected_uv(
        &coordinates,
        selection.uv_channel,
        cx.geometry.positions.len(),
    )?;
    if grid && wrap != crate::TextureWrapping::uniform(true) {
        return Err("Unsupported: graph evaluator grid requires Repeat on every texture".into());
    }
    let mut decoded = vec![];
    for &slot in &slots {
        decoded.push(decode(&cx, slot, grid)?);
    }
    let cancelled = || cx.control.check().is_err();
    let (width, height, rgba) = {
        let textures: Vec<([u32; 2], &[u8])> = sources
            .iter()
            .map(|s| {
                let t = &decoded[slots.iter().position(|&slot| slot == s.slot).unwrap()].1;
                ([t.header.width, t.header.height], t.rgba.as_slice())
            })
            .collect();
        bake(&program, &sources, &textures, &scales, &cancelled)?
    };
    let prefix = &cx.graph.material.prefix;
    let (sampling_uv, border_domain, tinted_rgba, product);
    let mut decoded = decoded.into_iter();
    let (texture_key, texture) = decoded.next().ok_or("Invalid: graph evaluator textures")?;
    if grid {
        sampling_uv = crate::SamplingTransform::default().sampling(&uv, cancelled)?;
        border_domain = None;
        tinted_rgba = None;
        let (key, second) = match decoded.next() {
            Some(pair) => pair,
            // Two sources on one slot (different scales): the product keeps its own copy.
            None => decode(&cx, slots[0], true)?,
        };
        product = Some(Box::new(crate::linked::ProductAsset {
            texture: second,
            texture_key: key,
            width,
            height,
            rgba,
        }));
    } else {
        if [width, height] != [texture.header.width, texture.header.height] {
            return Err("Invalid: graph evaluator texel extent".into());
        }
        sampling_uv = selection.uv_transform.sampling(&uv, cancelled)?;
        border_domain = crate::BorderDomain::confirm(
            [
                prefix.wrap_s_raw.get(selection.slot).copied(),
                prefix.wrap_t_raw.get(selection.slot).copied(),
            ],
            &sampling_uv,
            cancelled,
        )?;
        tinted_rgba = Some(rgba);
        product = None;
    }
    cx.control.check()?;
    cx.source.check_stable()?;
    Ok(crate::MaterialAsset::Texture(Box::new(crate::Asset {
        instance_projection,
        additional_texture_slots,
        border_domain,
        fourth: None,
        third: None,
        dual: None,
        product,
        tinted_rgba,
        sampling_uv,
        uv,
        uv_metadata: coordinates.metadata,
        texture,
        material_key,
        texture_key,
        wrap,
        selection,
    })))
}

/// Display label for a `grid_graph` asset.
pub(crate) fn label(a: &crate::Asset) -> String {
    let g = a.selection.grid_graph.expect("grid graph label");
    if g.off_grid.is_some() {
        return crate::offgrid::label(a, g);
    }
    let sampling = if a.product.is_some() {
        format!("nearest Repeat grid {:?}", a.display_extent())
    } else {
        format!(
            "{} • {} • per-texel, nearest",
            a.selection.uv_transform.label(),
            a.border_domain
                .map_or_else(|| a.wrap.label(), |b| b.label())
        )
    };
    format!(
        "Diffuse only • {} • UV{} {} • ignores {} nodes / {} wires",
        g.label(),
        a.selection.uv_channel,
        sampling,
        a.selection.ignored_nodes,
        a.selection.ignored_wires
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};

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
    fn sample(slot: u32, scale: f32) -> BoxMetadata {
        let mut b = node(1, [scale, scale, 0., 0., 0., 0., 0., 0.]);
        b.parameters_raw[4] = 0;
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

    /// BLEND(port0 = T2 × T3, port1 = T2, mask = T1.red): three distinct samples.
    fn blend_of_product() -> GraphMetadata {
        graph(
            vec![
                node(0, NO),
                node(16, [0.15, 0.16, 0., 0., 0., 0., 0., 0.]),
                sample(0, 1.),
                sample(1, 2.),
                node(10, NO),
                sample(2, 1.),
            ],
            vec![
                edge(1, 0, 0),
                edge(4, 1, 0),
                edge(3, 1, 1),
                edge(2, 1, 2),
                edge(3, 4, 0),
                edge(5, 4, 1),
            ],
        )
    }

    #[test]
    fn d100_three_sample_blend_of_a_product_selects_on_the_grid() {
        let g = blend_of_product();
        let earlier = crate::select_diffuse(&g);
        let s = earlier.unwrap();
        let marker = s.grid_graph.unwrap();
        assert_eq!(
            (marker.root, marker.nodes, marker.samples, marker.sources),
            (1, 5, 3, 3)
        );
        assert_eq!((s.sample_index, s.slot), (3, 1));
        assert!(s.second_sample.is_none() && s.texture_blend.is_none());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        let program = grid_graph_program(&g, 1).unwrap();
        assert_eq!(program.nodes.last().unwrap().index, 1);
        assert!(matches!(
            program.nodes.last().unwrap().op,
            GraphOp::Blend { inputs: [a, b, m], .. }
                if a.node == 4 && b.node == 3 && m.node == 2 && m.lane.is_none()
        ));
        // Off the grid (different channel) the evaluable graph reports why.
        let mut off = blend_of_product();
        off.boxes[5].parameters_raw[4] = 1;
        assert_eq!(
            crate::select_diffuse(&off).unwrap_err(),
            "Unsupported: BLEND inputs must be COLOR or texture samples (graph evaluator: \
             off-grid samplers mix UV channels outside a separable product)"
        );
        // An unknown box keeps the fixed route's reason.
        let mut unknown = blend_of_product();
        unknown.boxes[5] = node(18, NO);
        assert_eq!(
            crate::select_diffuse(&unknown).unwrap_err(),
            "Unsupported: BLEND inputs must be COLOR or texture samples"
        );
        // ThingColor is not evaluated on the default route.
        let mut thing = blend_of_product();
        thing.boxes[5] = node(2, NO);
        thing.boxes[5].parameters_raw = [8, 0, 0, 0, 0, 0, 0, 0];
        assert!(crate::select_diffuse(&thing)
            .unwrap_err()
            .contains("BLEND inputs"));
        // Animated operator, cycle, wide swizzle: refused.
        let mut animated = blend_of_product();
        animated.boxes[4].subtype_raw = 1;
        assert!(grid_graph_program(&animated, 1).is_err());
        let mut cycle = blend_of_product();
        cycle.wires.push(edge(1, 4, 2));
        assert!(grid_graph_program(&cycle, 1).is_err());
        let mut wide = blend_of_product();
        wide.wires[4].swizzle_raw = [b'r', b'g', 0, 0, 0];
        assert!(grid_graph_program(&wide, 1).is_err());
        // Animation-base pass never evaluates.
        assert_eq!(select(&g, true, "kept".into()).unwrap_err(), "kept");
    }

    #[test]
    fn d100_thing_color_forms_only_under_a_supplied_instance_color() {
        // BLEND(port0 = T2 × ThingColor, port1 = T2, mask = T1.red).
        let mut g = blend_of_product();
        g.boxes[5] = node(2, NO);
        g.boxes[5].parameters_raw = [8, 0, 0, 0, 0, 0, 0, 0];
        assert!(crate::select_diffuse(&g).is_err());
        let color = crate::DiffuseTint {
            bits: [0.25f32, 0.5, 0.75, 0.5].map(f32::to_bits),
        };
        let (derived, projection) = crate::project_instance_color(&g, color).unwrap();
        let projection = projection.unwrap();
        assert!(instance_supported(&projection));
        let s = crate::select_instance_diffuse(&derived, &projection, false).unwrap();
        assert_eq!(s.grid_graph.unwrap().sources, 2);
        let program = grid_graph_program(&derived, 1).unwrap();
        assert!(program
            .nodes
            .iter()
            .any(|n| n.index == 5 && n.op == GraphOp::Color(color.bits)));
        // A rewritten projection (UV alpha subtraction) is refused.
        let mut rewritten = projection.clone();
        rewritten.alpha_subtract_samplers.push(3);
        assert!(crate::select_instance_diffuse(&derived, &rewritten, false).is_err());
    }

    #[test]
    fn d100_add_of_texture_and_product_and_budgets() {
        // ADD(T1, T2 × T3): the D89 refusal now selects on the grid.
        let g = graph(
            vec![
                node(0, NO),
                node(11, NO),
                sample(0, 1.),
                node(10, NO),
                sample(1, 4.),
                sample(2, 3.),
            ],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(3, 1, 1),
                edge(4, 3, 0),
                edge(5, 3, 1),
            ],
        );
        let s = crate::select_diffuse(&g).unwrap();
        assert_eq!(s.grid_graph.unwrap().sources, 3);
        // Scalar root refused; COLOR outside 0..1 refused; large CONSTANT refused.
        let mut scalar = g.clone();
        scalar.wires[0].swizzle_raw = [0; 5];
        scalar.wires[1].swizzle_raw = [b'a', 0, 0, 0, 0];
        scalar.wires[2].swizzle_raw = [b'r', 0, 0, 0, 0];
        assert!(grid_graph_program(&scalar, 1)
            .unwrap_err()
            .contains("four-component"));
        let mut constant = g.clone();
        constant.boxes[2] = node(4, [17., 0., 0., 0., 0., 0., 0., 0.]);
        assert!(grid_graph_program(&constant, 1).is_err());
        constant.boxes[2] = node(4, [0.5, 0., 0., 0., 0., 0., 0., 0.]);
        assert!(grid_graph_program(&constant, 1).is_ok());
        let mut color = g.clone();
        color.boxes[2] = node(3, [1.5, 0., 0., 1., 0., 0., 0., 0.]);
        assert!(grid_graph_program(&color, 1).is_err());
        // Node budget: a chain of 40 MULTIPLY boxes.
        let mut boxes = vec![node(0, NO)];
        let mut wires = vec![edge(1, 0, 0)];
        for i in 0..40 {
            boxes.push(node(10, NO));
            wires.push(edge(i + 2, i + 1, 0));
            wires.push(edge(42, i + 1, 1));
        }
        boxes.push(sample(0, 1.));
        boxes.push(sample(1, 1.));
        wires.push(edge(41, 40, 0));
        let deep = graph(boxes, wires);
        assert!(grid_graph_program(&deep, 1).is_err());
    }

    #[test]
    fn d100_texel_semantics_match_the_fixed_bakes() {
        // Two-texture product: RGB equals the D49 linear product table; BLEND equals D97.
        let a = [255u8, 128, 0, 128, 0, 255, 255, 255];
        let b = [255u8, 255, 255, 128, 255, 0, 255, 255, 255, 255, 255, 255];
        let g = graph(
            vec![node(0, NO), node(10, NO), sample(0, 2.), sample(1, 1.)],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1)],
        );
        let program = grid_graph_program(&g, 1).unwrap();
        let sources = program.sources();
        let scales = grid_scales(&sources).unwrap();
        assert_eq!(scales, [[2, 2], [1, 1]]);
        let (w, h, p) = bake(
            &program,
            &sources,
            &[([2, 1], &a), ([3, 1], &b)],
            &[[2, 1], [1, 1]],
            &|| false,
        )
        .unwrap();
        assert_eq!((w, h), (12, 1));
        // Same RGB as product::tests (alpha is f64-rounded here: 64 either way).
        assert_eq!(&p[..4], &[255, 128, 0, 64]);
        assert_eq!(&p[16..20], &[0, 0, 255, 255]);
        // D97 two-texture BLEND example: COLOR green to texture red by mask red.
        let red = [255u8, 0, 0, 255];
        let mask = [0u8, 7, 7, 7, 255, 7, 7, 7];
        let g = graph(
            vec![
                node(0, NO),
                node(16, [0., 1., 0., 0., 0., 0., 0., 0.]),
                node(3, [0., 1., 0., 1., 0., 0., 0., 0.]),
                sample(0, 2.),
                sample(1, 1.),
            ],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1), edge(4, 1, 2)],
        );
        let program = grid_graph_program(&g, 1).unwrap();
        let sources = program.sources();
        let (w, h, p) = bake(
            &program,
            &sources,
            &[([1, 1], &red), ([2, 1], &mask)],
            &[[2, 1], [1, 1]],
            &|| false,
        )
        .unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(p, [0, 255, 0, 255, 255, 0, 0, 255]);
        // Alpha-swizzled MIX factor and an unwired weight.
        let top = [255u8, 255, 255, 0, 0, 0, 255, 128];
        let g = graph(
            vec![node(0, NO), node(12, NO), sample(0, 2.), sample(1, 1.)],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1), {
                let mut w = edge(3, 1, 2);
                w.swizzle_raw = [b'a', 0, 0, 0, 0];
                w
            }],
        );
        let program = grid_graph_program(&g, 1).unwrap();
        let sources = program.sources();
        let run = |cancel: &dyn Fn() -> bool| {
            bake(
                &program,
                &sources,
                &[([1, 1], &red), ([2, 1], &top)],
                &[[2, 1], [1, 1]],
                cancel,
            )
        };
        // Same bytes as the D83 alpha MIX test.
        assert_eq!(
            run(&|| false).unwrap().2,
            [255, 0, 0, 255, 187, 0, 188, 191]
        );
        assert_eq!(run(&|| true).unwrap_err(), "Cancelled");
        // Budgets: grid over 4096, bad extent.
        assert!(bake(
            &program,
            &sources,
            &[
                ([4096, 1], &vec![0; 4096 * 4]),
                ([4095, 1], &vec![0; 4095 * 4])
            ],
            &[[1, 1], [1, 1]],
            &|| false,
        )
        .is_err());
        assert!(bake(
            &program,
            &sources,
            &[([1, 1], &red), ([2, 1], &top[..4])],
            &[[1, 1], [1, 1]],
            &|| false,
        )
        .is_err());
    }

    #[test]
    fn d100_single_source_graphs_bake_per_texel() {
        // BLEND(COLOR, COLOR, T.alpha): one source, per-texel, no grid requirement.
        let mut t = sample(0, 3.);
        t.parameters_raw[2] = 0.25f32.to_bits();
        let g = graph(
            vec![
                node(0, NO),
                node(16, [0., 1., 0., 0., 0., 0., 0., 0.]),
                node(3, [1., 0., 0., 1., 0., 0., 0., 0.]),
                node(3, [0., 0., 1., 1., 0., 0., 0., 0.]),
                t,
            ],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1), {
                let mut w = edge(4, 1, 2);
                w.swizzle_raw = [b'w', 0, 0, 0, 0];
                w
            }],
        );
        let s = crate::select_diffuse(&g).unwrap();
        assert_eq!(s.grid_graph.unwrap().sources, 1);
        let program = grid_graph_program(&g, 1).unwrap();
        let sources = program.sources();
        let scales = grid_scales(&sources).unwrap();
        let texels = [9u8, 9, 9, 0, 9, 9, 9, 255];
        let (w, h, p) = bake(&program, &sources, &[([2, 1], &texels)], &scales, &|| false).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(p, [255, 0, 0, 255, 0, 0, 255, 255]);
    }
}
