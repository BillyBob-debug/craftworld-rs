//! Read-only probe (P2y): every PLevelSettings holder in a level, with its stored preset
//! count and the values the live game was seen to use for pod_planets (sun x 0.74296,
//! FogNear 5804.64, FogFar 58453.61, fog colour (0.6456, 0.4092, 0.2281); PC Ghidra read,
//! oracles/results/2026-10-09-shading-ghidra.md). Prints summary numbers only.
//! Usage: settings_holders_probe SNAPSHOT GAME-DIRS LEVEL-PATH
//!        settings_holders_probe SNAPSHOT GAME-DIRS --plan GUID-HEX   (a plan resource's Things)
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_level::parts::LevelSettings;
use std::path::Path;

const WANTED: [(&str, f32); 4] = [
    ("sun x 0.74296", 0.74296),
    ("FogNear 5804.64", 5804.64),
    ("FogFar 58453.61", 58453.61),
    ("fog r 0.6456", 0.6456),
];

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= b.abs() * 1e-3
}

fn show(label: &str, s: &LevelSettings) {
    println!(
        "  {label}: sun pos ({:.5}, {:.5}, {:.5}) x {:.0}, sun ({:.3}, {:.3}, {:.3}) mult {:.4}, exposure {:.4}, fog ({:.4}, {:.4}, {:.4}, {:.2}) {:.2}..{:.2}, presets {}",
        s.sun_position[0], s.sun_position[1], s.sun_position[2], s.sun_position_scale,
        s.sun_color[0], s.sun_color[1], s.sun_color[2], s.sun_multiplier, s.exposure,
        s.fog_color[0], s.fog_color[1], s.fog_color[2], s.fog_color[3],
        s.fog_near, s.fog_far, s.presets
    );
    println!(
        "    full vec4: sun {:?} ambient {:?} rim {:?} rim2 {:?}",
        s.sun_color, s.ambient_color, s.rim_color, s.rim_color2
    );
    let values = [s.sun_position[0], s.fog_near, s.fog_far, s.fog_color[0]];
    for ((name, want), v) in WANTED.iter().zip(values) {
        if near(v, *want) {
            println!("    MATCHES live value: {name}");
        }
    }
}

fn list(things: &[lbp_level::thing::ThingRecord]) {
    for t in things {
        if let Some(w) = t.data.world.as_ref() {
            println!(
                "PWorld Thing {}: lighting/colourCorrection/fog/fogTint/darkness {:?}; GlobalSettings target {:?} from {:?} blend {}; backdrop plans {:?} / {:?}",
                t.index,
                w.lighting,
                w.gs_target,
                w.gs_from,
                w.gs_blend,
                w.backdrop_plan.as_ref().and_then(|d| d.guid),
                w.backdrop_new_plan.as_ref().and_then(|d| d.guid)
            );
        }
    }
    let mut holders = 0;
    for t in things {
        let Some(main) = t.data.level_settings.as_ref() else {
            continue;
        };
        holders += 1;
        println!(
            "Thing {} (world holder: {}): {} stored presets",
            t.index,
            t.data.world.is_some(),
            t.data.level_settings_presets.len()
        );
        show("main", main);
        for (i, p) in t.data.level_settings_presets.iter().enumerate() {
            show(&format!("preset {i}"), p);
        }
    }
    println!(
        "{holders} PLevelSettings holder(s) in {} Things",
        things.len()
    );
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        return Err(
            "settings_holders_probe SNAPSHOT GAME-DIRS LEVEL-PATH | --plan GUID-HEX".into(),
        );
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    if args[3] == "--plan" {
        let guid = u32::from_str_radix(args.get(4).ok_or("--plan needs a GUID")?, 16)
            .map_err(|_| "GUID: hex")?;
        let key = lbp_preview::resolve(&catalog, Some(guid), None)?;
        let bytes = lbp_preview::level_scene::read_resource(
            &source,
            &key,
            &control,
            lbp_level::plan::MAX_INPUT,
        )?;
        let report = lbp_level::read_plan(&bytes, lbp_level::Limits::default(), &|| false)
            .map_err(|e| e.to_string())?;
        println!("plan stop: {:?}", report.stop);
        list(&report.things);
        return Ok(());
    }
    let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(&args[3]))?;
    let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    list(&report.things);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
