//! Bounded read-only mesh compatibility baseline/delta; output stays ignored.
//! Asset unit: distinct catalog GUIDs, with content-hash rows preserving aliases.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs::OpenOptions, io::Write, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 5 {
        return Err("compatibility_census SNAPSHOT GAME-DIR MESH-SAMPLE-MANIFEST NEW-JSON".into());
    }
    let output = Path::new(&args[4]);
    let mut options = lbp_preview::PreviewOptions::default();
    let mut flags = args[5..].iter();
    while let Some(flag) = flags.next() {
        match flag.as_str() {
            "--animation-base" => options.animation_base = true,
            "--instance-rgba" => {
                let values = flags
                    .next()
                    .ok_or("RGBA required")?
                    .split(',')
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "invalid RGBA")?;
                let values: [f32; 4] =
                    values.try_into().map_err(|_| "four RGBA values required")?;
                let color = lbp_preview::DiffuseTint {
                    bits: values.map(f32::to_bits),
                };
                color.validate()?;
                options.instance_color = Some(color);
            }
            _ => return Err("unknown option".into()),
        }
    }
    let animation_base = options.animation_base;
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
    let oracle: Value =
        serde_json::from_reader(std::fs::File::open(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9" {
        return Err("mesh oracle pin differs".into());
    }
    let samples = oracle["samples"].as_array().ok_or("samples required")?;
    if samples.is_empty() || samples.len() > 48 {
        return Err("mesh sample budget".into());
    }
    let mut seen = BTreeSet::new();
    let mut viewable = BTreeSet::new();
    let mut all_diffuse = BTreeSet::new();
    let mut rows = Vec::new();
    let mut mesh_bytes = 0u64;
    for sample in samples {
        control.check()?;
        let sha = sample["sha1"].as_str().ok_or("sha required")?;
        if !seen.insert(sha.to_owned()) {
            return Err("duplicate mesh sample".into());
        }
        let guids: BTreeSet<_> = catalog
            .by_hash
            .get(sha)
            .ok_or("sample absent from catalog")?
            .iter()
            .map(|&i| {
                catalog.snapshot.resources[i]
                    .map_row
                    .guid
                    .to_ascii_lowercase()
            })
            .collect();
        let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
        mesh_bytes = mesh_bytes
            .checked_add(key.size)
            .ok_or("mesh byte overflow")?;
        if mesh_bytes > 64 * 1024 * 1024 {
            return Err("aggregate mesh input budget".into());
        }
        let result =
            lbp_preview::load_mesh_groups_with_options(&source, &catalog, &key, &control, options);
        let row = match result {
            Ok(preview) => {
                let g = &preview.geometry;
                // This confirms geometry availability, not original pose or visual fidelity.
                let geometry_available = g.triangles.iter().any(|t| {
                    let a = g.positions[t[0] as usize].map(f64::from);
                    let b = g.positions[t[1] as usize].map(f64::from);
                    let c = g.positions[t[2] as usize].map(f64::from);
                    let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
                    let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
                    [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ]
                    .iter()
                    .any(|n| *n != 0.)
                });
                let mut supported = 0;
                let mut nonempty = 0;
                let mut groups = Vec::new();
                for (i, (range, binding)) in g
                    .primitive_triangles
                    .iter()
                    .zip(&preview.bindings)
                    .enumerate()
                {
                    if !range.is_empty() {
                        nonempty += 1;
                        supported += usize::from(binding.is_ok());
                    }
                    groups.push(match binding {
                        Ok(asset) => match asset.as_ref() {
                            lbp_preview::MaterialAsset::Texture(a) => json!({"primitive":i,"triangles":range.len(),"status":"diffuse_loaded","alpha_mix":a.selection.alpha_mix.map(|m|json!({"mix_index":m.mix_index,"color_bits":m.color.bits,"color_first":m.color_first,"unused_weight_bits":m.unused_weight_bits,"factor":"same_sample_alpha","filter":"nearest"})),"animation_base":a.selection.animation_base.map(|b|json!({"subtype_raw":b.subtype_raw,"sample_index":b.sample_index,"global_rows_verified":b.global_rows_verified,"static_matrix_bits":b.static_matrix_bits,"row_bits":b.row_bits,"key_counts":b.key_counts,"time":null})),"static_mix":a.selection.blend.map(|b|json!({"mix_index":b.mix_index,"weight_bits":b.weight_bits,"color_bits":b.color.bits,"color_first":b.color_first})),"wrap_uv_raw":a.source_wrap_raw(),"display_wrap_uv_raw":a.wrap.raw(),"border_domain":a.border_domain.map(|b|json!({"source_wrap_raw":b.source_wrap_raw,"min_bits":b.min_bits,"max_bits":b.max_bits,"margin_bits":lbp_preview::INTERIOR_MARGIN.to_bits(),"filter":"nearest","scope":"all selected mesh UVs"})),"wrap_frontend":if a.wrap.uniform_axis().is_some(){"Bevy_and_legacy"}else{"Bevy"},"uv_channel":a.selection.uv_channel,"material":{"path":a.material_key.path,"offset":a.material_key.offset,"size":a.material_key.size},"texture":{"path":a.texture_key.path,"offset":a.texture_key.offset,"size":a.texture_key.size},"width":a.texture.header.width,"height":a.texture.header.height,"product":a.product.as_ref().map(|p|json!({"second_texture":{"path":p.texture_key.path,"offset":p.texture_key.offset,"size":p.texture_key.size},"second_extent":[p.texture.header.width,p.texture.header.height],"display_extent":a.display_extent(),"filter":"nearest"}))}),
                            lbp_preview::MaterialAsset::Solid(a) => json!({"primitive":i,"triangles":range.len(),"status":"solid_diffuse_loaded","unwired_diffuse":a.selection.unwired_diffuse,"color_bits":a.selection.color.bits,"material":{"path":a.material_key.path,"offset":a.material_key.offset,"size":a.material_key.size},"texture":null,"uv_channel":null}),
                        },
                        Err(e) => json!({"primitive":i,"triangles":range.len(),"status":"neutral","reason":e,"material_guid":g.metadata.primitives[i].material_guid,"material_hash":g.metadata.primitives[i].material_hash}),
                    });
                }
                for (i, group) in groups.iter_mut().enumerate() {
                    if let Ok(asset) = &preview.bindings[i] {
                        group["additional_unrendered_texture_slots"] =
                            json!(asset.additional_texture_slots());
                        group["supplied_instance_input"] = asset.instance_projection().map_or(Value::Null, |p| json!({"linear_rgba_bits":p.color.bits,"folded_sampler":p.folded_sampler,"nested_tint_bits":p.nested_tint.map(|c|c.bits),"color_nodes":p.color_nodes,"alpha_subtract_samplers":p.alpha_subtract_samplers,"verified_game_instance":false}));
                        if let Some(dual) = asset.texture().and_then(|a| a.dual.as_ref()) {
                            group["wrap_frontend"] = json!("Bevy");
                            group["independent_sampling"] = json!({"frontend":"Bevy","second_wrap_uv_raw":dual.wrap.raw(),"second_uv_channel":asset.texture().unwrap().selection.second_sample.unwrap().uv_channel,"second_texture":{"path":dual.texture_key.path,"offset":dual.texture_key.offset,"size":dual.texture_key.size},"filter":"linear","second_extent":[dual.texture.header.width,dual.texture.header.height]});
                        }
                        if let Some(third) = asset.texture().and_then(|a| a.third.as_ref()) {
                            group["third_sampling"] = json!({"frontend":"Bevy","shared_with":"second_sampler_coordinates","uv_channel":asset.texture().unwrap().selection.third_sample.unwrap().uv_channel,"texture":{"path":third.texture_key.path,"offset":third.texture_key.offset,"size":third.texture_key.size},"extent":[third.texture.header.width,third.texture.header.height],"filter":"linear"});
                        }
                        if let Some(fourth) = asset.texture().and_then(|a| a.fourth.as_ref()) {
                            group["fourth_sampling"] = json!({"frontend":"Bevy","shared_with":"primary_sampler_coordinates","uv_channel":asset.texture().unwrap().selection.fourth_sample.unwrap().uv_channel,"texture":{"path":fourth.texture_key.path,"offset":fourth.texture_key.offset,"size":fourth.texture_key.size},"extent":[fourth.texture.header.width,fourth.texture.header.height],"filter":"linear"});
                        }
                    }
                }
                if geometry_available {
                    viewable.extend(guids.iter().cloned());
                }
                if geometry_available && nonempty > 0 && supported == nonempty {
                    all_diffuse.extend(guids.iter().cloned());
                }
                json!({"sha1":sha,"guids":guids,"geometry_available":geometry_available,"vertices":g.positions.len(),"triangles":g.triangles.len(),"nonempty_groups":nonempty,"supported_groups":supported,"groups":groups})
            }
            Err(e) => json!({"sha1":sha,"guids":guids,"geometry_available":false,"reason":e}),
        };
        rows.push(row);
    }
    source.check_stable()?;
    let report = json!({"schema_version":10,"supplied_instance_color_bits":options.instance_color.map(|c|c.bits),"animation_base_enabled":animation_base,"scope":"bounded_existing_mesh_samples_diffuse_coverage","asset_unit":"distinct_catalog_guid","mesh_hashes":rows.len(),"mesh_input_bytes":mesh_bytes,"viewable_guids":viewable,"all_diffuse_guids":all_diffuse,"complete_verified_guids":[],"complete_scope":"No visual completeness certification; all_diffuse is only binding coverage","samples":rows});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "{} mesh hashes: {} viewable GUIDs, {} all-diffuse GUIDs; 0 complete visually verified",
        rows.len(),
        report["viewable_guids"].as_array().unwrap().len(),
        report["all_diffuse_guids"].as_array().unwrap().len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
