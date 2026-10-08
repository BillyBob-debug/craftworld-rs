mod adapter;
mod dual;
use dual::ViewerMaterial;
mod browser;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use lbp_catalog::{
    attachment::AttachedSource,
    census_summary::{census_flag, CorpusCounts, LineKind, CENSUS_FLAGS_HELP},
    Catalog, LoadControl,
};
use lbp_preview::level_scene;
use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};

type Context = (Arc<Catalog>, Arc<AttachedSource>);
type OwnedScene = (
    Entity,
    Handle<Mesh>,
    Handle<ViewerMaterial>,
    Option<Handle<Image>>,
);
struct Completed {
    context: Context,
    preview: Result<Option<lbp_preview::GroupedMesh>, String>,
    identity: String,
    guid: String,
    level: Option<Result<LevelLoaded, String>>,
}
type LevelScene = level_scene::LevelScene<level_scene::ScenePayload>;
struct LevelLoaded {
    scene: LevelScene,
    identity: String,
}
/// D113 level mode (docs/d113-level-scene.md). Empty target = single-mesh mode.
#[derive(Default)]
struct LevelMode {
    guid: Option<u32>,
    path: Option<String>,
    report: bool,
    lines: Vec<String>,
    home: Option<Orbit>,
}
impl LevelMode {
    fn active(&self) -> bool {
        self.guid.is_some() || self.path.is_some()
    }
    fn label(&self) -> String {
        match (&self.path, self.guid) {
            (Some(p), _) => p.clone(),
            (None, Some(g)) => format!("{g:08x}"),
            (None, None) => String::new(),
        }
    }
}
struct Job {
    control: LoadControl,
    receiver: Mutex<mpsc::Receiver<Result<Completed, String>>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Resource)]
struct Viewer {
    instance_input_enabled: bool,
    instance_rgba: [f32; 4],
    animation_base: bool,
    snapshot: String,
    game: String,
    guid: String,
    status: String,
    context: Option<Context>,
    context_paths: (String, String),
    job: Option<Job>,
    requested: bool,
    scene: Vec<OwnedScene>,
    capture: Option<PathBuf>,
    qa: bool,
    ready_frames: u32,
    synthetic: bool,
    /// `--qa-hide-ui`: draw no egui panels (clean level/asset captures).
    hide_ui: bool,
    browser: browser::Browser,
    census: CorpusCounts,
    level: LevelMode,
}
impl Viewer {
    fn preview_options(&self) -> lbp_preview::PreviewOptions {
        lbp_preview::PreviewOptions {
            animation_base: self.animation_base,
            instance_color: self
                .instance_input_enabled
                .then_some(lbp_preview::DiffuseTint {
                    bits: self.instance_rgba.map(f32::to_bits),
                }),
        }
    }
    fn start(&mut self) {
        self.browser.displayed_guid = None;
        self.job = None;
        self.status = "Loading snapshot / validating read-only source / preparing preview…".into();
        self.ready_frames = 0;
        let paths = (self.snapshot.clone(), self.game.clone());
        if paths != self.context_paths {
            self.context = None;
            self.browser.clear_source();
        }
        let cached = (paths == self.context_paths)
            .then(|| self.context.clone())
            .flatten();
        // Level mode ignores the single-mesh GUID.
        let guid = if self.level.active() {
            String::new()
        } else {
            self.guid.clone()
        };
        let level_target = self
            .level
            .active()
            .then(|| (self.level.guid, self.level.path.clone()));
        let options = self.preview_options();
        let control = LoadControl::default();
        let worker = control.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        // Large reserved stack: the level reader recurses natively (D107).
        let spawned = std::thread::Builder::new()
            .stack_size(level_scene::LEVEL_STACK_BYTES)
            .spawn(move || {
                let run = || {
                    let (catalog, source) = match cached {
                        Some(context) => context,
                        None => {
                            let catalog =
                                Arc::new(lbp_catalog::load(&PathBuf::from(&paths.0), &worker)?);
                            let source = Arc::new(AttachedSource::attach(
                                &PathBuf::from(&paths.1),
                                &catalog,
                                &worker,
                            )?);
                            (catalog, source)
                        }
                    };
                    // Attachment succeeds independently of a particular preview failure.
                    // The catalog remains browsable even when the initial GUID is unsupported.
                    let mut identity = String::new();
                    let preview = (|| {
                        if guid.trim().is_empty() {
                            return Ok(None);
                        }
                        let number = u32::from_str_radix(guid.trim(), 16)
                            .map_err(|_| "GUID must be eight hexadecimal digits")?;
                        if guid.trim().len() != 8 {
                            return Err("GUID must be eight hexadecimal digits".into());
                        }
                        let key = lbp_preview::resolve(&catalog, Some(number), None)?;
                        identity = catalog.by_guid[&format!("{number:08x}")]
                            .iter()
                            .map(|&i| catalog.snapshot.resources[i].map_row.path.as_str())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        lbp_preview::load_mesh_groups_with_options(
                            &source, &catalog, &key, &worker, options,
                        )
                        .map(Some)
                    })();
                    let level = level_target.map(|(level_guid, level_path)| {
                        let (key, identity) = level_scene::resolve_level(
                            &catalog,
                            level_guid,
                            level_path.as_deref(),
                        )?;
                        let scene = level_scene::load_level_scene(
                            &source, &catalog, &key, &worker, options,
                        )?;
                        Ok(LevelLoaded { scene, identity })
                    });
                    worker.check()?;
                    Ok(Completed {
                        context: (catalog, source),
                        preview,
                        identity,
                        guid,
                        level,
                    })
                };
                let _ = tx.send(run());
            });
        if let Err(e) = spawned {
            self.status = format!("Load failed: worker thread: {e}");
            return;
        }
        self.context_paths = (self.snapshot.clone(), self.game.clone());
        self.job = Some(Job {
            control,
            receiver: Mutex::new(rx),
        });
    }
}
#[derive(Resource, Clone, Copy)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}
impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.65,
            distance: 2.,
            target: Vec3::ZERO,
        }
    }
}
fn options() -> Result<Viewer, String> {
    let mut v = Viewer {
        instance_input_enabled: false,
        instance_rgba: [1.; 4],
        animation_base: false,
        snapshot: String::new(),
        game: String::new(),
        guid: String::new(),
        status: String::new(),
        context: None,
        context_paths: default(),
        job: None,
        requested: true,
        scene: Vec::new(),
        capture: None,
        qa: false,
        ready_frames: 0,
        synthetic: false,
        hide_ui: false,
        browser: browser::Browser::default(),
        census: CorpusCounts::default(),
        level: LevelMode::default(),
    };
    let mut print_census = false;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--instance-rgba" {
            let value = args
                .next()
                .ok_or("--instance-rgba requires linear R,G,B,A")?;
            let parts = value
                .split(',')
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid instance RGBA")?;
            v.instance_rgba = parts
                .try_into()
                .map_err(|_| "instance RGBA requires four values")?;
            lbp_preview::DiffuseTint {
                bits: v.instance_rgba.map(f32::to_bits),
            }
            .validate()?;
            v.instance_input_enabled = true;
            continue;
        }
        if flag == "--animation-base" {
            v.animation_base = true;
            continue;
        }
        if flag == "--show-unavailable" {
            v.browser.show_unavailable = true;
            continue;
        }
        if flag == "--synthetic-affine" {
            v.synthetic = true;
            v.guid = "synthetic-affine".into();
            continue;
        }
        if flag == "--synthetic-groups" {
            v.synthetic = true;
            v.guid = "synthetic-groups".into();
            continue;
        }
        if flag == "--synthetic" {
            v.synthetic = true;
            continue;
        }
        if flag == "--qa-hide-ui" {
            v.hide_ui = true;
            continue;
        }
        if flag == "--level-report" {
            v.level.report = true;
            continue;
        }
        if flag == "--print-census" {
            print_census = true;
            continue;
        }
        if let Some(role) = census_flag(&flag) {
            let path = args
                .next()
                .ok_or_else(|| format!("{flag} requires a FILE"))?;
            v.census.load(role, std::path::Path::new(&path))?;
            continue;
        }
        if flag == "--help" {
            println!("lbp-bevy-viewer --snapshot FILE --game-dir DIRECTORY [--select-guid HEX]\n--browse-query TEXT; --show-unavailable: browser filters\n--animation-base: opt-in local animation base-row snapshot (not game time)\n--synthetic: authored depth/UV/repeat/clamp scene\n--synthetic-groups: authored mixed material groups\n--synthetic-affine: authored UV scale/offset/reflection\n--qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS: capture then exit
--qa-hide-ui: draw no panels (clean captures)\n{CENSUS_FLAGS_HELP}\n--print-census: print the corpus-count panel text and exit (no window)\n--level-guid HEX | --level-path MAP_PATH: level scene mode (RENDER_MESH Things at POS)\n--level-report: with a level, print scene counts and exit (no window)\nLeft drag orbit; right/middle drag pan; wheel zoom; R reset; F frame.");
            std::process::exit(0);
        }
        let value = args.next().ok_or("Missing option value")?;
        match flag.as_str() {
            "--snapshot" => v.snapshot = value,
            "--game-dir" => v.game = value,
            "--select-guid" => v.guid = value,
            "--browse-query" => v.browser.query = value,
            "--level-guid" => {
                if value.len() != 8 {
                    return Err("--level-guid requires eight hexadecimal digits".into());
                }
                v.level.guid = Some(
                    u32::from_str_radix(&value, 16)
                        .map_err(|_| "--level-guid requires eight hexadecimal digits")?,
                );
            }
            "--level-path" => v.level.path = Some(value),
            "--qa-capture" => {
                let base = std::fs::canonicalize("local-analysis").map_err(|e| e.to_string())?;
                let path = PathBuf::from(value);
                let parent = std::fs::canonicalize(path.parent().ok_or("Capture parent required")?)
                    .map_err(|e| e.to_string())?;
                if !parent.starts_with(base)
                    || path.exists()
                    || path.extension().is_none_or(|v| v != "png")
                {
                    return Err("Capture must be a new PNG under local-analysis".into());
                }
                v.capture = Some(parent.join(path.file_name().ok_or("Capture filename required")?));
                v.qa = true;
            }
            _ => return Err(format!("Unknown option {flag}")),
        }
    }
    if print_census {
        for (_, line) in v.census.lines() {
            println!("{line}");
        }
        std::process::exit(0);
    }
    if !v.census.is_empty() {
        println!(
            "Corpus counts: {} mesh census file(s), texture census {}",
            v.census.mesh.len(),
            if v.census.texture.is_some() {
                "given"
            } else {
                "not given"
            }
        );
    }
    if v.level.active() && v.synthetic {
        return Err("Level mode cannot be combined with --synthetic".into());
    }
    if v.level.guid.is_some() && v.level.path.is_some() {
        return Err("Give --level-guid or --level-path, not both".into());
    }
    if v.level.report && !v.level.active() {
        return Err("--level-report requires --level-guid or --level-path".into());
    }
    if !v.synthetic && (v.snapshot.is_empty() || v.game.is_empty()) {
        return Err(
            "Provide --snapshot, --game-dir or --synthetic (--select-guid is optional)".into(),
        );
    }
    Ok(v)
}
fn main() {
    let viewer = options().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    if viewer.level.report {
        std::process::exit(match level_report(&viewer) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    App::new()
        .insert_resource(viewer)
        .init_resource::<Orbit>()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.045)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LBP2 Asset Explorer".into(),
                resolution: (1280, 800).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(dual::DualPlugin)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (place_on_second_monitor, load, camera, capture).chain(),
        )
        .add_systems(EguiPrimaryContextPass, panel)
        .run();
}
// Keep test/launcher windows hidden until positioned on a non-primary display.
fn place_on_second_monitor(
    monitors: Query<(Entity, Option<&bevy::window::PrimaryMonitor>), With<bevy::window::Monitor>>,
    mut windows: Query<&mut Window>,
) {
    let second = monitors.iter().find(|(_, primary)| primary.is_none());
    if let Some((monitor, _)) = second {
        for mut window in &mut windows {
            if !window.visible {
                window.position = bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Entity(monitor),
                );
                println!(
                    "Preview window positioned on non-primary monitor {monitor:?} before showing"
                );
                window.visible = true;
            }
        }
    }
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.01,
            far: 100.,
            ..default()
        }),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::default(),
    ));
}
fn clear(
    v: &mut Viewer,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) {
    for (entity, mesh, material, image) in v.scene.drain(..) {
        commands.entity(entity).despawn();
        meshes.remove(mesh.id());
        if let Some(second) = materials
            .get(material.id())
            .and_then(|m| m.extension.image.as_ref())
        {
            images.remove(second.id());
        }
        if let Some(third) = materials
            .get(material.id())
            .and_then(|m| m.extension.third_image.as_ref())
        {
            images.remove(third.id());
        }
        if let Some(fourth) = materials
            .get(material.id())
            .and_then(|m| m.extension.fourth_image.as_ref())
        {
            images.remove(fourth.id());
        }
        materials.remove(material.id());
        if let Some(image) = image {
            images.remove(image.id());
        }
    }
}
fn load(
    mut commands: Commands,
    mut v: ResMut<Viewer>,
    mut orbit: ResMut<Orbit>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ViewerMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if v.requested {
        v.requested = false;
        clear(
            &mut v,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
        );
        if v.synthetic {
            if v.guid == "synthetic-groups" {
                v.scene = synthetic_groups(&mut commands, &mut meshes, &mut materials, &mut images);
                *orbit = Orbit {
                    yaw: 0.,
                    pitch: 0.,
                    distance: 2.,
                    ..default()
                };
                v.status = "Authored primitive groups: red left; green upper right; neutral unsupported lower right. Repeated red binding shares one draw. Shared whole-mesh framing.".into();
                return;
            }
            synthetic(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                v.guid == "synthetic-affine",
            );
            *orbit = Orbit {
                yaw: 0.,
                pitch: 0.,
                distance: 3.,
                ..default()
            };
            v.status="Authored fixture: left UV0 quadrants (red/green above blue/yellow); middle repeat UV 0..2; right clamp UV 0..2. Red front square occludes blue back square.".into();
            if v.guid == "synthetic-affine" {
                v.status = "Authored affine fixture: left raw UV quadrants; middle repeat UV × (2,2) + (0.25,0.25); right clamp UV × (-1,1) + (1,0), horizontally reflected. Raw UV arrays preserved.".into();
            }
            return;
        }
        v.start();
    }
    let result = v
        .job
        .as_ref()
        .and_then(|job| match job.receiver.lock().unwrap().try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Worker disconnected".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
    let Some(result) = result else { return };
    let job = v.job.take().unwrap();
    if job.control.check().is_err() {
        return;
    }
    let attempted_guid = result
        .as_ref()
        .ok()
        .map(|done| done.guid.trim().to_ascii_lowercase());
    let mut evidence = browser::PreviewEvidence::default();
    let rendered = (|| -> Result<(), String> {
        let done = result?;
        v.context = Some(done.context);
        if let Some(level) = done.level {
            let level = level?;
            let plan = plan_level(&level.scene)?;
            spawn_level(
                &mut v,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                plan,
            );
            *orbit = v.level.home.unwrap_or_default();
            v.status = format!(
                "Loaded level {}\n{}",
                level.identity,
                v.level.lines.join("\n")
            );
            println!("{}", v.status);
            return Ok(());
        }
        let Some(preview) = done.preview? else {
            v.status = "Source attached. Choose an asset in the browser.".into();
            return Ok(());
        };
        let g = &preview.geometry;
        let mut details = Vec::new();
        let prepared = prepare(&preview, true, &mut evidence, &mut details)?;
        for p in prepared {
            let (mesh, material, image) = upload(p, &mut meshes, &mut materials, &mut images);
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
                .id();
            v.scene.push((entity, mesh, material, image));
        }
        *orbit = Orbit::default();
        v.status = format!(
            "Loaded {}\n{}\n{} vertices • {} triangles • {} primitives{}{}",
            done.guid,
            done.identity,
            g.positions.len(),
            g.triangles.len(),
            g.metadata.primitives.len(),
            "\n",
            details.join("\n")
        );
        println!("{}", v.status);
        Ok(())
    })();
    if let Some(guid) = attempted_guid.filter(|guid| !guid.is_empty()) {
        v.browser.record(guid, rendered.clone().map(|()| evidence));
        let (viewable, complete, failed) = v.browser.counts();
        println!("Compatibility (source session, distinct GUIDs): {viewable} viewable; {complete} complete; {failed} failed");
    }
    if let Err(e) = rendered {
        v.status = format!("Load failed: {e}");
        eprintln!("{}", v.status);
    }
}
/// CPU-side display preparation shared by the single-mesh and level paths: one entry per
/// supported material bucket (plus one neutral bucket). `normalize` centres and scales
/// the mesh to unit size (single-mesh view); level scenes keep model coordinates.
struct Prepared {
    mesh: Mesh,
    image: Option<Image>,
    second_image: Option<Image>,
    third_image: Option<(Image, lbp_preview::RelativeSampling)>,
    fourth_image: Option<(Image, lbp_preview::RelativeSampling)>,
    color: Color,
    alpha_mix: bool,
}
fn prepare(
    preview: &lbp_preview::GroupedMesh,
    normalize: bool,
    evidence: &mut browser::PreviewEvidence,
    details: &mut Vec<String>,
) -> Result<Vec<Prepared>, String> {
    let g = &preview.geometry;
    let buckets = adapter::buckets(&g.triangles, &g.primitive_triangles, &preview.bindings)?;
    for (range, binding) in g.primitive_triangles.iter().zip(&preview.bindings) {
        if !range.is_empty() {
            evidence.total_groups += 1;
            if let Ok(asset) = binding.as_deref() {
                evidence.unrendered_texture_groups +=
                    usize::from(!asset.additional_texture_slots().is_empty());
                evidence.instance_input |= asset.instance_projection().is_some();
                evidence.animation_snapshot |= asset
                    .texture()
                    .is_some_and(|a| a.selection.animation_base.is_some());
            }
            match binding.as_deref() {
                Ok(lbp_preview::MaterialAsset::Texture(_)) => evidence.textured_groups += 1,
                Ok(lbp_preview::MaterialAsset::Solid(_)) => evidence.solid_groups += 1,
                Err(_) => {}
            }
        }
    }
    for (index, (range, binding)) in g
        .primitive_triangles
        .iter()
        .zip(&preview.bindings)
        .enumerate()
    {
        let label = match binding {
            Ok(a) => a.label(),
            Err(e) => format!("Neutral: {e}"),
        };
        details.push(format!(
            "Primitive {index}: {} triangles • {label}",
            range.len()
        ));
    }
    prepare_buckets(&g.positions, buckets, normalize, evidence, details)
}
/// Material buckets over one position array (model mesh or merged shape batch).
fn prepare_buckets(
    positions: &[[f32; 3]],
    buckets: Vec<adapter::Bucket<lbp_preview::MaterialAsset>>,
    normalize: bool,
    evidence: &mut browser::PreviewEvidence,
    details: &mut Vec<String>,
) -> Result<Vec<Prepared>, String> {
    // Prepare all adapters before allocating GPU assets. Shared full-mesh bounds
    // keep different groups aligned; supported groups share one draw per binding.
    let mut prepared = Vec::new();
    for (asset, triangles) in buckets {
        if triangles.is_empty() {
            continue;
        }
        let build = if normalize {
            adapter::mesh
        } else {
            adapter::mesh_local
        };
        let mut mesh = match build(
            positions,
            &triangles,
            asset.as_ref().and_then(|a| a.sampling_uv()),
        ) {
            Ok(mesh) => mesh,
            Err(e) if e == "Unsupported: no nondegenerate faces" => {
                evidence.skipped_geometry = true;
                details.push(e);
                continue;
            }
            Err(e) => return Err(e),
        };
        let alpha_mix = asset
            .as_ref()
            .and_then(|a| a.texture())
            .is_some_and(|a| a.dual.is_some() && a.selection.texture_alpha_mix.is_some());
        let second_image = if let Some(second) = asset
            .as_ref()
            .and_then(|a| a.texture())
            .and_then(|a| a.dual.as_ref())
        {
            adapter::secondary_uv(&mut mesh, &second.sampling_uv)?;
            Some(adapter::image_axes(
                second.texture.header.width,
                second.texture.header.height,
                second.texture.rgba.clone(),
                second.wrap,
            )?)
        } else {
            None
        };
        let image = asset
            .as_ref()
            .and_then(|a| a.texture())
            .map(|a| {
                let [width, height] = a.display_extent();
                let mut image =
                    adapter::image_axes(width, height, a.display_pixels().to_vec(), a.wrap)?;
                if a.nearest_sampling() {
                    if let bevy::image::ImageSampler::Descriptor(s) = &mut image.sampler {
                        s.mag_filter = bevy::image::ImageFilterMode::Nearest;
                        s.min_filter = bevy::image::ImageFilterMode::Nearest;
                    }
                }
                Ok::<_, String>(image)
            })
            .transpose()?;
        let color = match asset.as_deref() {
            Some(lbp_preview::MaterialAsset::Solid(a)) => {
                let [r, g, b, _] = a.selection.color.values();
                Color::linear_rgba(r, g, b, 1.)
            }
            Some(lbp_preview::MaterialAsset::Texture(_)) => Color::WHITE,
            None => Color::srgb(0.65, 0.68, 0.72),
        };
        let third_image = asset
            .as_ref()
            .and_then(|a| a.texture())
            .and_then(|a| a.third.as_ref())
            .map(|third| {
                let relative = third.relative.ok_or("Missing third-sample coordinates")?;
                adapter::image(
                    third.texture.header.width,
                    third.texture.header.height,
                    third.texture.rgba.clone(),
                    true,
                )
                .map(|image| (image, relative))
            })
            .transpose()?;
        let fourth_image = asset
            .as_ref()
            .and_then(|a| a.texture())
            .and_then(|a| a.fourth.as_ref())
            .map(|fourth| {
                let relative = fourth.relative.ok_or("Missing fourth-sample coordinates")?;
                adapter::image(
                    fourth.texture.header.width,
                    fourth.texture.header.height,
                    fourth.texture.rgba.clone(),
                    true,
                )
                .map(|image| (image, relative))
            })
            .transpose()?;
        prepared.push(Prepared {
            mesh,
            image,
            second_image,
            third_image,
            fourth_image,
            color,
            alpha_mix,
        });
    }
    if prepared.is_empty() {
        return Err("Unavailable: no nondegenerate faces to display".into());
    }
    Ok(prepared)
}
fn upload(
    p: Prepared,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) -> (Handle<Mesh>, Handle<ViewerMaterial>, Option<Handle<Image>>) {
    let image = p.image.map(|i| images.add(i));
    let mut surface = dual::material(StandardMaterial {
        base_color: p.color,
        base_color_texture: image.clone(),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    if let Some(second) = p.second_image {
        dual::enable(&mut surface, images.add(second));
        if p.alpha_mix {
            dual::enable_alpha_mix(&mut surface);
        }
    }
    if let Some((third, relative)) = p.third_image {
        dual::enable_third(&mut surface, images.add(third), relative);
    }
    if let Some((fourth, relative)) = p.fourth_image {
        dual::enable_fourth(&mut surface, images.add(fourth), relative);
    }
    (meshes.add(p.mesh), materials.add(surface), image)
}
/// D113: display plan for a level scene, built without GPU or window. Per scene mesh the
/// prepared buckets (or the display error); per displayed item its mesh and Transform
/// (world matrix after the scene normalization below).
struct LevelPlan {
    prepared: Vec<Result<Vec<Prepared>, String>>,
    placed: Vec<(usize, Transform)>,
    lines: Vec<String>,
    home: Orbit,
}
fn plan_level(scene: &LevelScene) -> Result<LevelPlan, String> {
    let prepared: Vec<_> = scene
        .meshes
        .iter()
        .map(|m| {
            let mut evidence = browser::PreviewEvidence::default();
            let mut details = Vec::new();
            match &m.payload {
                level_scene::ScenePayload::Model(g) => {
                    prepare(g, false, &mut evidence, &mut details)
                }
                // World-space batch; a material that did not select draws neutral grey.
                level_scene::ScenePayload::Shapes { batch, material } => prepare_buckets(
                    &batch.positions,
                    vec![(material.as_ref().ok().cloned(), batch.triangles.clone())],
                    false,
                    &mut evidence,
                    &mut details,
                ),
            }
        })
        .collect();
    let buckets: Vec<Result<usize, String>> = prepared
        .iter()
        .map(|p| p.as_ref().map(Vec::len).map_err(Clone::clone))
        .collect();
    let (placed, home, display) = place_items(scene, &buckets)?;
    let mut lines = scene.counts.lines();
    lines.extend(display);
    Ok(LevelPlan {
        prepared,
        placed,
        lines,
        home,
    })
}
/// Scene normalization and framing. The whole level is centred and scaled to unit size
/// (largest axis = 1) so the shared orbit camera, clip planes and pan limits of the
/// single-mesh view apply unchanged; the home view looks along -Z (LBP side-on) and
/// fits the X/Y extent in a 1280x800 window with the default 45 degree vertical FOV.
fn level_home(lo: Vec3, hi: Vec3) -> Result<(Mat4, Orbit), String> {
    let size = hi - lo;
    let span = size.max_element();
    if !span.is_finite() || span <= 0. {
        return Err("Unavailable: zero-size or nonfinite level bounds".into());
    }
    let normalize =
        Mat4::from_scale(Vec3::splat(1. / span)) * Mat4::from_translation(-(lo + hi) * 0.5);
    let unit = size / span;
    let tan = (PerspectiveProjection::default().fov * 0.5).tan();
    let aspect = 1280. / 800.;
    let distance = ((unit.x * 0.5 / (tan * aspect)).max(unit.y * 0.5 / tan) * 1.1 + unit.z * 0.5)
        .clamp(0.15, 30.);
    Ok((
        normalize,
        Orbit {
            yaw: 0.,
            pitch: 0.,
            distance,
            target: Vec3::ZERO,
        },
    ))
}
/// Items whose mesh has display buckets get a Transform; others are counted by reason.
type Placement = (Vec<(usize, Transform)>, Orbit, Vec<String>);
fn place_items<T>(
    scene: &level_scene::LevelScene<T>,
    buckets: &[Result<usize, String>],
) -> Result<Placement, String> {
    let shown = |i: &level_scene::SceneItem| buckets.get(i.mesh).is_some_and(Result::is_ok);
    let [lo, hi] = scene
        .bounds_where(shown)
        .ok_or("Unavailable: no displayable level items")?;
    let (normalize, home) = level_home(Vec3::from(lo), Vec3::from(hi))?;
    let mut placed = Vec::new();
    let mut entities = 0usize;
    let mut sheared = 0usize;
    let (mut model_things, mut shape_things) = (0usize, 0usize);
    let mut skipped = std::collections::BTreeMap::<String, usize>::new();
    for item in &scene.items {
        let things = item.shapes.max(1);
        let count = match buckets.get(item.mesh) {
            Some(Ok(n)) => *n,
            Some(Err(e)) => {
                *skipped.entry(e.clone()).or_default() += things;
                continue;
            }
            None => {
                *skipped.entry("Invalid: mesh index".into()).or_default() += things;
                continue;
            }
        };
        if item.shapes == 0 {
            model_things += 1;
        } else {
            shape_things += item.shapes;
        }
        let matrix = normalize * Mat4::from_cols_array(&item.world);
        let transform = Transform::from_matrix(matrix);
        if !transform.to_matrix().abs_diff_eq(matrix, 1e-4) {
            sheared += 1;
        }
        entities += count;
        placed.push((item.mesh, transform));
    }
    let mut lines = vec![
        format!("Drawn in viewer: {model_things} RENDER_MESH Things, {shape_things} shape Things ({entities} draw entities)"),
    ];
    for (reason, n) in &skipped {
        lines.push(format!("  display-skipped {n} Things: {reason}"));
    }
    lines.push(format!(
        "Shear dropped by TRS transform: {sheared} Things (placement approximate)"
    ));
    lines.push(format!(
        "Bounds (level units): ({:.1}, {:.1}, {:.1}) .. ({:.1}, {:.1}, {:.1})",
        lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]
    ));
    Ok((placed, home, lines))
}
fn spawn_level(
    v: &mut Viewer,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
    plan: LevelPlan,
) {
    // One set of GPU assets per scene mesh, shared by every instance.
    let uploaded: Vec<Vec<_>> = plan
        .prepared
        .into_iter()
        .map(|p| {
            p.map(|list| {
                list.into_iter()
                    .map(|p| upload(p, meshes, materials, images))
                    .collect()
            })
            .unwrap_or_default()
        })
        .collect();
    for (mesh_index, transform) in plan.placed {
        for (mesh, material, image) in &uploaded[mesh_index] {
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    transform,
                ))
                .id();
            v.scene
                .push((entity, mesh.clone(), material.clone(), image.clone()));
        }
    }
    v.level.lines = plan.lines;
    v.level.home = Some(plan.home);
}
/// `--level-report`: builds the same scene and display plan without a window.
fn level_report(v: &Viewer) -> Result<(), String> {
    let (snapshot, game) = (PathBuf::from(&v.snapshot), PathBuf::from(&v.game));
    let (level_guid, level_path) = (v.level.guid, v.level.path.clone());
    let options = v.preview_options();
    let lines = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<Vec<String>, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(&snapshot, &control)?;
            let source = AttachedSource::attach(&game, &catalog, &control)?;
            let (key, identity) =
                level_scene::resolve_level(&catalog, level_guid, level_path.as_deref())?;
            let scene = level_scene::load_level_scene(&source, &catalog, &key, &control, options)?;
            let plan = plan_level(&scene)?;
            let mut lines = vec![format!("Level: {identity}")];
            lines.extend(plan.lines);
            for m in &scene.meshes {
                let users = scene
                    .items
                    .iter()
                    .filter(|i| std::ptr::eq(&scene.meshes[i.mesh], m))
                    .count();
                lines.push(format!(
                    "  mesh {}: {users} instances{}",
                    m.source.label(),
                    if m.neutral_groups {
                        ", neutral groups"
                    } else {
                        ""
                    }
                ));
            }
            Ok(lines)
        })
        .map_err(|e| format!("level worker thread: {e}"))?
        .join()
        .map_err(|_| "level worker panicked".to_string())??;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
fn panel(mut contexts: EguiContexts, mut v: ResMut<Viewer>, mut orbit: ResMut<Orbit>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if v.hide_ui {
        return Ok(());
    }
    if !v.synthetic {
        let mut browser_ui = egui::Ui::new(
            ctx.clone(),
            "browser_root".into(),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(ctx.viewport_rect()),
        );
        egui::Panel::left("asset_browser")
            .default_size(390.)
            .min_size(260.)
            .show(&mut browser_ui, |ui| {
                ui.heading("Browse assets");
                let context = v.context.clone();
                if let Some((catalog, _)) = context {
                    let selected = v.guid.clone();
                    if let Some(guid) = v.browser.panel(ui, &catalog, &selected) {
                        v.guid = guid;
                        v.level.guid = None;
                        v.level.path = None;
                        v.level.lines.clear();
                        v.level.home = None;
                        v.requested = true;
                    }
                } else {
                    ui.label("Attach a snapshot and game directory to browse.");
                }
            });
    }
    egui::Window::new("Asset preview").default_pos(if v.synthetic { [10., 10.] } else { [900., 10.] }).default_width(310.).show(ctx, |ui| {
        if !v.synthetic {
            ui.label("Snapshot");
            ui.text_edit_singleline(&mut v.snapshot);
            ui.label("Extracted game directory");
            ui.text_edit_singleline(&mut v.game);
            ui.label("Mesh GUID (blank: attach and browse)");
            ui.text_edit_singleline(&mut v.guid);
            if ui.button("Load / reload").clicked() { v.requested = true; }
            if v.job.is_some() && ui.button("Cancel").clicked() {
                v.job = None;
                v.status = "Cancelled".into();
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() { *orbit = v.level.home.unwrap_or_default(); }
            if ui.button("Frame selected").clicked() {
                let home = v.level.home.unwrap_or_default();
                orbit.target = home.target;
                orbit.distance = home.distance;
            }
        });
        ui.separator();
        if !v.synthetic {
            if ui.checkbox(&mut v.animation_base,"Preview stored material animation base values").changed() {
                v.browser.clear_source();v.requested=true;
            }
            if v.animation_base {ui.small("Stored base rows matched to global declarations; no game time or motion inferred. Reload clears verification.");}
            let mut input_changed = ui.checkbox(&mut v.instance_input_enabled,"Supply instance color for preview").changed();
            if v.instance_input_enabled {
                ui.small("Supplied linear RGBA; actual game instance values are unverified.");
                ui.horizontal(|ui| {
                    for (name, value) in ["R", "G", "B", "A"].into_iter().zip(v.instance_rgba.iter_mut()) {
                        ui.label(name);
                        input_changed |= ui.add(egui::DragValue::new(value).range(0.0..=1.0).speed(0.01)).changed();
                    }
                });
            }
            if input_changed { v.browser.clear_source(); v.requested = true; }
            v.browser.review_panel(ui);
        }
        egui::ScrollArea::vertical().max_height(260.).show(ui, |ui| { ui.label(&v.status); });
        ui.separator();
        ui.label("Left drag: orbit\nRight / middle drag: pan\nWheel: zoom • R: reset • F: frame");
        ui.small("Unlit • opaque • sRGB • double-sided\nGenerated diagnostic normals; no bone/morph pose.\nTop mip only. Original game lighting, alpha and axis conventions unverified.");
    });
    if !v.synthetic || !v.census.is_empty() {
        census_window(ctx, &v.census);
    }
    if v.level.active() {
        egui::Window::new("Level scene counts")
            .default_pos([420., 460.])
            .default_width(420.)
            .show(ctx, |ui| {
                ui.label(format!("Level: {}", v.level.label()));
                for line in &v.level.lines {
                    ui.label(line);
                }
                if v.level.lines.is_empty() {
                    ui.small("Loading or failed; see status.");
                }
                ui.small("Models: static bind pose at each Thing's POS matrix; bones, animation and decorations not applied.");
                ui.small(format!("Shapes: {}.", level_scene::SHAPE_PREVIEW_LABEL));
            });
    }
    Ok(())
}
/// Corpus-count window; text and validation come from `lbp_catalog::census_summary`.
fn census_window(ctx: &egui::Context, census: &CorpusCounts) {
    egui::Window::new("Corpus counts (census files)")
        .default_pos([420., 10.])
        .default_width(380.)
        .default_open(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(420.)
                .show(ui, |ui| {
                    for (kind, text) in census.lines() {
                        match kind {
                            LineKind::Heading => {
                                ui.separator();
                                ui.strong(text);
                            }
                            LineKind::Value => {
                                ui.label(text);
                            }
                            LineKind::Note => {
                                ui.small(text);
                            }
                        }
                    }
                });
        });
}
// Bevy injects independent input resources and the camera query into this system.
#[allow(clippy::too_many_arguments)]
fn camera(
    mut contexts: EguiContexts,
    v: Res<Viewer>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut orbit: ResMut<Orbit>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let blocked = v.qa
        || contexts
            .ctx_mut()
            .is_ok_and(|ctx| ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input());
    if !blocked {
        if keys.just_pressed(KeyCode::KeyR) {
            *orbit = v.level.home.unwrap_or_default();
        }
        if keys.just_pressed(KeyCode::KeyF) {
            let home = v.level.home.unwrap_or_default();
            orbit.target = home.target;
            orbit.distance = home.distance;
        }
        if buttons.pressed(MouseButton::Left) {
            orbit.yaw -= motion.delta.x * 0.008;
            orbit.pitch = (orbit.pitch + motion.delta.y * 0.008).clamp(-1.5, 1.5);
        }
        let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
        if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
            let distance = orbit.distance;
            orbit.target +=
                rotation * Vec3::new(-motion.delta.x, motion.delta.y, 0.) * distance * 0.0015;
            orbit.target = orbit.target.clamp(Vec3::splat(-10.), Vec3::splat(10.));
        }
        orbit.distance = (orbit.distance * (-scroll.delta.y * 0.12).exp()).clamp(0.15, 30.);
    }
    let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(orbit.target + rotation * Vec3::Z * orbit.distance)
                .looking_at(orbit.target, Vec3::Y);
    }
}
fn capture(mut commands: Commands, mut v: ResMut<Viewer>, time: Res<Time>) {
    if v.job.is_none() && !v.requested {
        v.ready_frames += 1;
    }
    if v.ready_frames > 120 && time.elapsed_secs() > 10. {
        if let Some(path) = v.capture.take() {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path))
                .observe(
                    |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                        exit.write(AppExit::Success);
                    },
                );
        }
    }
}
fn synthetic(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
    affine: bool,
) {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 64,
    ];
    for (x, extent, repeat) in [(-0.75, 1., false), (0., 2., true), (0.75, 2., false)] {
        let raw_uv = if affine {
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
        } else {
            vec![[0., 0.], [extent, 0.], [extent, extent], [0., extent]]
        };
        let transform = lbp_preview::UvTransform {
            bits: if !affine || x < 0. {
                [1., 1., 0., 0.]
            } else if x == 0. {
                [2., 2., 0.25, 0.25]
            } else {
                [-1., 1., 1., 0.]
            }
            .map(f32::to_bits),
        };
        let sampling_uv = transform.sampling(&raw_uv, || false).unwrap();
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.3, 0.3, 0.],
                [0.3, 0.3, 0.],
                [0.3, -0.3, 0.],
                [-0.3, -0.3, 0.],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, sampling_uv)
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 2, 1, 0, 3, 2]));
        let mut image = adapter::image(2, 2, rgba.clone(), repeat).unwrap();
        if let bevy::image::ImageSampler::Descriptor(ref mut s) = image.sampler {
            s.mag_filter = bevy::image::ImageFilterMode::Nearest;
            s.min_filter = bevy::image::ImageFilterMode::Nearest;
        }
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(dual::material(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                unlit: true,
                cull_mode: None,
                ..default()
            }))),
            Transform::from_xyz(x, 0.35, 0.),
        ));
    }
    // Submit foreground before background: visibility must come from depth, not submission order.
    for (z, scale, color) in [
        (0.2, 0.3, Color::srgb(1., 0., 0.)),
        (0., 0.6, Color::srgb(0., 0., 1.)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(scale, scale))),
            MeshMaterial3d(materials.add(dual::material(StandardMaterial {
                base_color: color,
                unlit: true,
                cull_mode: None,
                ..default()
            }))),
            Transform::from_xyz(0., -0.55, z),
        ));
    }
}

fn synthetic_groups(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) -> Vec<OwnedScene> {
    let positions = [
        [-1., 1., 0.],
        [0., 1., 0.],
        [0., 0., 0.],
        [-1., 0., 0.],
        [1., 1., 0.],
        [1., 0., 0.],
        [0., -1., 0.],
        [1., -1., 0.],
    ];
    let triangles = [
        [0, 2, 1],
        [0, 3, 2],
        [1, 5, 4],
        [1, 2, 5],
        [2, 7, 5],
        [2, 6, 7],
    ];
    let asset = |rgba: Vec<u8>| {
        Arc::new(lbp_preview::Asset {
            instance_projection: None,
            additional_texture_slots: vec![],
            border_domain: None,
            fourth: None,
            third: None,
            dual: None,
            product: None,
            tinted_rgba: None,
            uv: vec![[0.5, 0.5]; 8],
            sampling_uv: vec![[0.5, 0.5]; 8],
            uv_metadata: lbp_mesh::UvMetadata {
                vertex_count: 8,
                channel_count: 1,
                channels: vec![],
            },
            texture: lbp_texture::Texture {
                header: lbp_texture::Header {
                    pixel_format: lbp_texture::PixelFormat::BGRA8,
                    width: 1,
                    height: 1,
                    mips: 1,
                    wrapper: "authored",
                    flags: None,
                    remap: None,
                    format_raw: None,
                    no_srgb: false,
                    dds: None,
                },
                expanded_bytes: 4,
                top_mip_block_bytes: 4,
                unparsed_after_top_mip: 0,
                rgba,
            },
            material_key: lbp_catalog::attachment::LocationKey {
                path: "authored".into(),
                offset: 0,
                size: 0,
            },
            texture_key: lbp_catalog::attachment::LocationKey {
                path: "authored".into(),
                offset: 0,
                size: 0,
            },
            wrap: lbp_preview::TextureWrapping::uniform(false),
            selection: lbp_preview::DiffuseSelection::default(),
        })
    };
    let red = asset(vec![255, 0, 0, 255]);
    let green = asset(vec![0, 255, 0, 255]);
    let buckets = adapter::buckets(
        &triangles,
        &[0..1, 1..2, 2..4, 4..6],
        &[
            Ok(red.clone()),
            Ok(red),
            Ok(green),
            Err("Unsupported: authored complex graph".into()),
        ],
    )
    .unwrap();
    buckets
        .into_iter()
        .map(|(asset, triangles)| {
            let mesh = meshes.add(
                adapter::mesh(
                    &positions,
                    &triangles,
                    asset.as_ref().map(|a| a.sampling_uv.as_slice()),
                )
                .unwrap(),
            );
            let image = asset
                .map(|a| images.add(adapter::image(1, 1, a.texture.rgba.clone(), false).unwrap()));
            let material = materials.add(dual::material(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.65, 0.68, 0.72)
                },
                base_color_texture: image.clone(),
                unlit: true,
                cull_mode: None,
                ..default()
            }));
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
                .id();
            (entity, mesh, material, image)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_only_becomes_visible_on_non_primary_monitor() {
        let mut app = App::new();
        app.add_systems(Update, place_on_second_monitor);
        let monitor = || bevy::window::Monitor {
            name: None,
            physical_height: 1080,
            physical_width: 1920,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: None,
            scale_factor: 1.,
            video_modes: Vec::new(),
        };
        app.world_mut()
            .spawn((monitor(), bevy::window::PrimaryMonitor));
        let window = app
            .world_mut()
            .spawn(Window {
                visible: false,
                ..default()
            })
            .id();
        app.update();
        assert!(!app.world().get::<Window>(window).unwrap().visible);
        let second = app.world_mut().spawn(monitor()).id();
        app.update();
        let placed = app.world().get::<Window>(window).unwrap();
        assert!(placed.visible);
        assert_eq!(
            placed.position,
            bevy::window::WindowPosition::Centered(bevy::window::MonitorSelection::Entity(second))
        );
        app.world_mut().get_mut::<Window>(window).unwrap().position =
            bevy::window::WindowPosition::At(IVec2::new(20, 30));
        app.update();
        assert_eq!(
            app.world().get::<Window>(window).unwrap().position,
            bevy::window::WindowPosition::At(IVec2::new(20, 30))
        );
    }
    #[test]
    fn level_placement_normalizes_frames_side_on_and_counts_display_skips() {
        let mut scene = level_scene::LevelScene::<u8>::default();
        for (source, bounds) in [
            (
                level_scene::MeshSource::Guid(1),
                Some([[-1., -1., -1.], [1., 1., 1.]]),
            ),
            (level_scene::MeshSource::Guid(2), Some([[0.; 3], [1.; 3]])),
        ] {
            scene.meshes.push(level_scene::SceneMesh {
                source,
                payload: 0,
                bounds,
                neutral_groups: false,
            });
        }
        let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0., 0.)).to_cols_array();
        for (mesh, x, shapes) in [(0, 0., 0), (0, 98., 0), (1, 500., 3)] {
            scene
                .push_item(level_scene::SceneItem {
                    mesh,
                    world: at(x),
                    thing: 0,
                    uid: None,
                    shapes,
                })
                .unwrap();
        }
        let (placed, home, lines) =
            place_items(&scene, &[Ok(2), Err("Unavailable: test".into())]).unwrap();
        assert_eq!(placed.len(), 2);
        // Displayed bounds x -1..99 -> unit span centred at the origin.
        let a = placed[0].1.translation;
        let b = placed[1].1.translation;
        assert!((a.x + 0.49).abs() < 1e-5 && (b.x - 0.49).abs() < 1e-5);
        assert!((placed[0].1.scale - Vec3::splat(0.01)).length() < 1e-6);
        assert_eq!((home.yaw, home.pitch, home.target), (0., 0., Vec3::ZERO));
        assert!(home.distance > 0.15 && home.distance < 2.);
        assert!(lines[0].starts_with(
            "Drawn in viewer: 2 RENDER_MESH Things, 0 shape Things (4 draw entities)"
        ));
        assert!(lines
            .iter()
            .any(|l| l == "  display-skipped 3 Things: Unavailable: test"));
        assert!(level_home(Vec3::ZERO, Vec3::ZERO).is_err());
    }
    #[test]
    fn dropping_replaced_job_cancels_and_disconnects_stale_result() {
        let control = LoadControl::default();
        let (tx, rx) = mpsc::sync_channel(1);
        let job = Job {
            control: control.clone(),
            receiver: Mutex::new(rx),
        };
        drop(job);
        assert!(control.check().is_err());
        assert!(tx.send(Err("stale".into())).is_err());
    }
    #[test]
    fn replacing_scene_removes_entities_and_all_owned_gpu_assets() {
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ViewerMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mesh = meshes.add(Rectangle::new(1., 1.));
        let image = images.add(adapter::image(1, 1, vec![255; 4], false).unwrap());
        let material = materials.add(dual::material(StandardMaterial {
            base_color_texture: Some(image.clone()),
            ..default()
        }));
        let entity = world.spawn_empty().id();
        let mut v = Viewer {
            instance_input_enabled: false,
            instance_rgba: [1.; 4],
            animation_base: false,
            snapshot: String::new(),
            game: String::new(),
            guid: String::new(),
            status: String::new(),
            context: None,
            context_paths: default(),
            job: None,
            requested: false,
            scene: vec![(entity, mesh, material, Some(image))],
            capture: None,
            qa: false,
            ready_frames: 0,
            synthetic: false,
            hide_ui: false,
            browser: browser::Browser::default(),
            census: CorpusCounts::default(),
            level: LevelMode::default(),
        };
        let mut entities = vec![entity];
        for _ in 0..3 {
            let mesh = meshes.add(Rectangle::new(1., 1.));
            let image = images.add(adapter::image(1, 1, vec![255; 4], false).unwrap());
            let mut surface = dual::material(StandardMaterial {
                base_color_texture: Some(image.clone()),
                ..default()
            });
            let second = images.add(adapter::image(1, 1, vec![128; 4], true).unwrap());
            dual::enable(&mut surface, second);
            let third = images.add(adapter::image(1, 1, vec![64; 4], true).unwrap());
            dual::enable_third(
                &mut surface,
                third,
                lbp_preview::RelativeSampling {
                    from_primary: false,
                    scale: [1., 1.],
                    offset: [0., 0.],
                },
            );
            let fourth = images.add(adapter::image(1, 1, vec![32; 4], true).unwrap());
            dual::enable_fourth(
                &mut surface,
                fourth,
                lbp_preview::RelativeSampling {
                    from_primary: true,
                    scale: [1., 1.],
                    offset: [0., 0.],
                },
            );
            let material = materials.add(surface);
            let entity = world.spawn_empty().id();
            entities.push(entity);
            v.scene.push((entity, mesh, material, Some(image)));
        }
        clear(
            &mut v,
            &mut Commands::new(&mut queue, &world),
            &mut meshes,
            &mut materials,
            &mut images,
        );
        queue.apply(&mut world);
        assert!(
            v.scene.is_empty() && meshes.is_empty() && materials.is_empty() && images.is_empty()
        );
        assert!(entities
            .into_iter()
            .all(|entity| world.get_entity(entity).is_err()));
    }
}
