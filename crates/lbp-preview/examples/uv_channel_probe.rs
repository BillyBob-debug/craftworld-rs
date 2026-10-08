//! D104: read-only research probe for texture-sampler UV channel words (parameter 4).
//! Counts sampler channel words over every material location of a material census,
//! mesh UV attribute counts over every mesh MAP row, and, for each primitive whose
//! material has a sampler channel word above 1, per-channel coordinate statistics over
//! the primitive's referenced vertices. Output stays under ignored `local-analysis/`;
//! it is research data, not an asset count.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_resource::material::GraphMetadata;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

/// Texture descriptor GUID, hash, and wrap S/T words of one material slot.
type Slot = (Option<u32>, Option<String>, u8, u8);

fn read_all(
    source: &AttachedSource,
    key: &lbp_catalog::attachment::LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(key.size as usize);
    while data.len() < key.size as usize {
        let count = (key.size as usize - data.len()).min(4096);
        let page = source
            .read_hex_checked(key, data.len() as u64, count, control)?
            .bytes;
        if page.len() != count {
            return Err("short page".into());
        }
        data.extend(page);
    }
    Ok(data)
}

/// Output ports (box type 0 port_to) reachable downstream of `start`, bounded.
fn output_ports(graph: &GraphMetadata, start: usize) -> BTreeSet<i64> {
    let n = graph.boxes.len();
    let mut ports = BTreeSet::new();
    let mut seen = vec![false; n];
    let mut pending = vec![start];
    while let Some(i) = pending.pop() {
        if i >= n || std::mem::replace(&mut seen[i], true) {
            continue;
        }
        for w in &graph.wires {
            if w.box_from as usize != i || w.box_to < 0 || w.box_to as usize >= n {
                continue;
            }
            let to = w.box_to as usize;
            if graph.boxes[to].type_raw == 0 {
                ports.insert(w.port_to_raw as i64);
            } else {
                pending.push(to);
            }
        }
    }
    ports
}

fn sampler_channels(graph: &GraphMetadata) -> Vec<(usize, u32, BTreeSet<i64>)> {
    if graph.boxes.len() > 1024 || graph.wires.len() > 4096 {
        return vec![];
    }
    graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 1)
        .map(|(i, b)| (i, b.parameters_raw[4], output_ports(graph, i)))
        .collect()
}

fn stats(uv: &[[f32; 2]], verts: &BTreeSet<usize>, other: &[&[[f32; 2]]]) -> Value {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    let mut unit = 0usize;
    let mut distinct = BTreeSet::new();
    let mut equal = vec![0usize; other.len()];
    for &v in verts {
        let Some(p) = uv.get(v) else { continue };
        for a in 0..2 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
        if p.iter().all(|c| (0.0..=1.0).contains(c)) {
            unit += 1;
        }
        distinct.insert((p[0].to_bits(), p[1].to_bits()));
        for (k, o) in other.iter().enumerate() {
            if o.get(v) == Some(p) {
                equal[k] += 1;
            }
        }
    }
    json!({"min":lo,"max":hi,"span":[hi[0]-lo[0],hi[1]-lo[1]],"vertices":verts.len(),"in_unit_square":unit,"distinct":distinct.len(),"equal_to_channel":equal})
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 && !(args.len() == 7 && args[5] == "--blob-dir") {
        return Err(
            "uv_channel_probe SNAPSHOT GAME-DIR MATERIAL-CENSUS NEW-JSON [--blob-dir NEW-DIR]"
                .into(),
        );
    }
    // Optional: decompressed shader blob (prefix blob range) per material, for a
    // read-only container scan; written only to a new directory.
    let blob_dir = args.get(6).map(Path::new);
    if let Some(d) = blob_dir {
        std::fs::create_dir(d).map_err(|e| e.to_string())?;
    }
    let mut blob_index = vec![];
    let output = Path::new(&args[4]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new ignored output required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let path_of = |sha: &str| -> String {
        catalog
            .by_hash
            .get(sha)
            .and_then(|v| v.first())
            .map(|&i| catalog.snapshot.resources[i].map_row.path.clone())
            .unwrap_or_default()
    };

    // 1. Sampler channel words over all material locations with MAP rows.
    let census: Value =
        serde_json::from_reader(std::fs::File::open(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let locations = census["locations"].as_array().ok_or("locations required")?;
    if locations.len() > 20_000 {
        return Err("location budget".into());
    }
    let mut word_samplers: BTreeMap<u32, usize> = BTreeMap::new();
    let mut word_materials: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut high: BTreeMap<String, Value> = BTreeMap::new();
    let mut graphs: BTreeMap<String, Option<GraphMetadata>> = BTreeMap::new();
    let mut slots: BTreeMap<String, Vec<Slot>> = BTreeMap::new();
    let mut key_sha: std::collections::HashMap<lbp_catalog::attachment::LocationKey, String> =
        Default::default();
    let mut bytes = 0u64;
    for location in locations {
        let Some(sha) = location["sha1_candidates"][0].as_str() else {
            continue;
        };
        if !catalog.by_hash.contains_key(sha) || graphs.contains_key(sha) {
            continue;
        }
        let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
        key_sha.insert(key.clone(), sha.to_owned());
        if key.size > lbp_resource::material::MAX_INPUT as u64 {
            continue;
        }
        bytes = bytes.checked_add(key.size).ok_or("overflow")?;
        if bytes > 128 * 1024 * 1024 {
            return Err("total budget".into());
        }
        let data = read_all(&source, &key, &control)?;
        let graph = lbp_resource::material::inspect_graph(&data, || control.check().is_err())
            .ok()
            .map(|g| {
                let pre = &g.material.prefix;
                if let Some(d) = blob_dir {
                    if let Ok((_, expanded)) = lbp_resource::binary::payload(
                        &data,
                        b"GMTb",
                        lbp_resource::material::MAX_INPUT,
                        lbp_resource::material::MAX_EXPANDED,
                        &|| false,
                    ) {
                        if let Some(blob) =
                            expanded.get(pre.blob_start..pre.blob_start + pre.blob_bytes)
                        {
                            if std::fs::write(d.join(format!("{sha}.bin")), blob).is_ok() {
                                blob_index.push(json!({"sha1":sha,"path":path_of(sha),"shader_end_offsets":pre.shader_end_offsets,"words":g.graph.boxes.iter().filter(|b| b.type_raw == 1).map(|b| b.parameters_raw[4]).collect::<Vec<_>>(),"samplers":sampler_channels(&g.graph).iter().map(|(i, w, p)| json!({"box":i,"slot":g.graph.boxes[*i].parameters_raw[5],"word":w,"subtype":g.graph.boxes[*i].subtype_raw,"output_ports":p})).collect::<Vec<_>>()}));
                            }
                        }
                    }
                }
                slots.insert(
                    sha.to_owned(),
                    pre.textures
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            (
                                t.guid,
                                t.sha1.clone(),
                                pre.wrap_s_raw.get(i).copied().unwrap_or(255),
                                pre.wrap_t_raw.get(i).copied().unwrap_or(255),
                            )
                        })
                        .collect(),
                );
                g.graph
            });
        if let Some(g) = &graph {
            let samplers = sampler_channels(g);
            for (_, word, _) in &samplers {
                *word_samplers.entry(*word).or_default() += 1;
                word_materials
                    .entry(*word)
                    .or_default()
                    .insert(sha.to_owned());
            }
            if samplers.iter().any(|(_, w, _)| *w > 1) {
                let mut types: BTreeMap<u32, usize> = BTreeMap::new();
                for b in &g.boxes {
                    *types.entry(b.type_raw).or_default() += 1;
                }
                high.insert(sha.to_owned(), json!({
                    "path": path_of(sha),
                    "box_type_counts": types,
                    "samplers": samplers.iter().map(|(i, w, p)| json!({"box":i,"subtype":g.boxes[*i].subtype_raw,"channel_word":w,"texture_slot":g.boxes[*i].parameters_raw[5],"output_ports":p})).collect::<Vec<_>>(),
                }));
            }
        }
        graphs.insert(sha.to_owned(), graph);
    }

    // 2. Mesh UV attribute counts and primitives bound to high-word materials.
    let mut mesh_hashes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "mesh" {
            mesh_hashes
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    let mut attribute_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut bound_word: BTreeMap<String, BTreeMap<u32, usize>> = BTreeMap::new();
    let mut primitives = vec![];
    let mut last_channel = BTreeMap::<String, usize>::new();
    let mut failed = 0usize;
    let dump_dir = output.with_extension("dump");
    std::fs::create_dir(&dump_dir).map_err(|e| e.to_string())?;
    let mut dump_index = 0usize;
    for sha in mesh_hashes.keys() {
        control.check()?;
        let Ok(key) = lbp_preview::resolve(&catalog, None, Some(sha)) else {
            failed += 1;
            continue;
        };
        if key.size > lbp_mesh::MAX_INPUT as u64 {
            failed += 1;
            continue;
        }
        let data = read_all(&source, &key, &control)?;
        let (Ok(geometry), Ok(uvs)) = (
            lbp_mesh::decode_geometry(&data, || control.check().is_err()),
            lbp_mesh::decode_uvs(&data, || control.check().is_err()),
        ) else {
            failed += 1;
            continue;
        };
        let channels = uvs.channels.len() as u32;
        *attribute_counts.entry(channels).or_default() += 1;
        // Shape of the last channel over the whole mesh, for meshes with 3+ channels.
        if channels >= 3 {
            let last = &uvs.channels[channels as usize - 1];
            let all: BTreeSet<usize> = (0..last.len()).collect();
            let s = stats(last, &all, &[&uvs.channels[0]]);
            let class = if s["equal_to_channel"][0] == json!(last.len()) {
                "equals_uv0"
            } else if s["in_unit_square"] == json!(last.len()) {
                "inside_unit_square"
            } else {
                "outside_unit_square"
            };
            *last_channel.entry(class.into()).or_default() += 1;
        }
        let mut words_here = BTreeSet::new();
        for (p, primitive) in geometry.metadata.primitives.iter().enumerate() {
            let Ok(mkey) = lbp_preview::resolve(
                &catalog,
                primitive.material_guid,
                primitive.material_hash.as_deref(),
            ) else {
                continue;
            };
            let Some(msha) = key_sha.get(&mkey).cloned() else {
                continue;
            };
            let Some(Some(graph)) = graphs.get(&msha) else {
                continue;
            };
            let samplers = sampler_channels(graph);
            for (_, w, _) in &samplers {
                words_here.insert(*w);
            }
            if !samplers.iter().any(|(_, w, _)| *w > 1) {
                continue;
            }
            let range = geometry.primitive_triangles[p].clone();
            let verts: BTreeSet<usize> = geometry.triangles[range]
                .iter()
                .flatten()
                .map(|&v| v as usize)
                .collect();
            let refs: Vec<&[[f32; 2]]> = uvs.channels.iter().map(Vec::as_slice).collect();
            let mut dumped = vec![];
            for (bi, w, ports) in samplers.iter().filter(|(_, w, _)| *w > 1) {
                let b = &graph.boxes[*bi];
                let slot = b.parameters_raw[5] as usize;
                let Some((guid, tsha, ws, wt)) =
                    slots.get(&msha).and_then(|v| v.get(slot)).cloned()
                else {
                    continue;
                };
                let Ok(tkey) = lbp_preview::resolve(&catalog, guid, tsha.as_deref()) else {
                    continue;
                };
                let tpath = tsha
                    .as_deref()
                    .map(|h| path_of(&h.to_ascii_lowercase()))
                    .or_else(|| {
                        guid.and_then(|g| catalog.by_guid.get(&format!("{g:08x}")))
                            .and_then(|v| v.first())
                            .map(|&i| catalog.snapshot.resources[i].map_row.path.clone())
                    })
                    .unwrap_or_default();
                let Ok(texture) = lbp_texture::decode_reader(
                    tkey.size,
                    |offset, count| {
                        source
                            .read_hex_checked(&tkey, offset, count, &control)
                            .map(|p| p.bytes)
                            .map_err(lbp_texture::Error::Input)
                    },
                    || control.check().is_err(),
                ) else {
                    continue;
                };
                let n = dump_index;
                dump_index += 1;
                let stem = dump_dir.join(format!("s{n:02}"));
                let mut f = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(stem.with_extension("rgba"))
                    .map_err(|e| e.to_string())?;
                f.write_all(&texture.rgba).map_err(|e| e.to_string())?;
                let f32s = |k: usize| f32::from_bits(b.parameters_raw[k]);
                let tris: Vec<Vec<[[f32; 2]; 3]>> = uvs
                    .channels
                    .iter()
                    .map(|c| {
                        geometry.triangles[geometry.primitive_triangles[p].clone()]
                            .iter()
                            .map(|t| t.map(|v| c.get(v as usize).copied().unwrap_or([f32::NAN; 2])))
                            .collect()
                    })
                    .collect();
                let meta = json!({"mesh":path_of(sha),"primitive":p,"material":path_of(&msha),"sampler_box":bi,"channel_word":w,"output_ports":ports,"subtype":b.subtype_raw,"params_f32":[f32s(0),f32s(1),f32s(2),f32s(3),f32s(6),f32s(7)],"texture":tpath,"wrap":[ws,wt],"width":texture.header.width,"height":texture.header.height,"triangles_by_channel":tris});
                let mut f = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(stem.with_extension("json"))
                    .map_err(|e| e.to_string())?;
                serde_json::to_writer(&mut f, &meta).map_err(|e| e.to_string())?;
                dumped.push(json!({"dump":format!("s{n:02}"),"texture":tpath,"sampler_box":bi}));
            }
            primitives.push(json!({
                "mesh_sha1": sha,
                "mesh_path": path_of(sha),
                "mesh_guids": mesh_hashes[sha],
                "primitive": p,
                "attribute_count": channels,
                "material_sha1": msha,
                "material_path": path_of(&msha),
                "dumps": dumped,
                "channels": uvs.channels.iter().map(|c| stats(c, &verts, &refs)).collect::<Vec<_>>(),
            }));
        }
        for w in words_here {
            *bound_word
                .entry(format!("{channels}_attributes"))
                .or_default()
                .entry(w)
                .or_default() += 1;
        }
    }
    source.check_stable()?;
    if let Some(d) = blob_dir {
        std::fs::write(
            d.join("index.json"),
            serde_json::to_vec(&blob_index).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    let report = json!({
        "schema_version": 1,
        "tool": "uv_channel_probe",
        "scope": "research: sampler parameter-4 words over material census locations with MAP rows; mesh UV attribute counts over mesh MAP row hashes",
        "material_hashes": graphs.len(),
        "sampler_count_by_channel_word": word_samplers,
        "material_count_by_channel_word": word_materials.iter().map(|(w, s)| (w.to_string(), s.len())).collect::<BTreeMap<_, _>>(),
        "high_word_materials": high,
        "mesh_hashes": mesh_hashes.len(),
        "mesh_failed": failed,
        "mesh_hashes_by_attribute_count": attribute_counts,
        "meshes_by_attribute_count_and_bound_word": bound_word,
        "last_channel_class_for_3plus": last_channel,
        "high_word_primitives": primitives,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "{} materials, {} meshes ({} failed); words {:?}; attributes {:?}; {} high-word primitives",
        graphs.len(),
        mesh_hashes.len(),
        failed,
        word_samplers,
        attribute_counts,
        report["high_word_primitives"]
            .as_array()
            .map_or(0, Vec::len)
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
