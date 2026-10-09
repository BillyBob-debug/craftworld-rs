//! P1b: `--pod-menu CLASS`, the Pod main menu as the game shows it with the Puter open:
//! the script's buttons as pill labels down the right side of a globe, the last one on the
//! moon, and the profile Polaroid between them (placement in `lbp_ui_script::pod`, from the
//! RPCS3 reference). The scene is a level camera looking into -Z with the globe and moon
//! placed so they project to the measured frame positions.
//! Fidelity: the globe and moon are PLACEHOLDER spheres (flat colours) until the PC finds
//! the pod meshes; the Polaroid is a blank card; the workshop background is a flat colour;
//! pill style (white, cyan highlight with a pulse) follows the captures by eye. The window
//! says so unless `--qa-hide-ui`.
//! Keys: Up/Down move the highlight, Enter prints the chosen label (the live menu, running
//! the script per frame, is a later slice).
//! Usage: `--pod-menu CLASS --snapshot SNAPSHOT --game-dir DIR [--ui-script-function NAME]
//! [--qa-hide-ui] [--qa-capture local-analysis/NEW.png]`.
use crate::gooey_ui::{demo_capture, demo_capture_path, DemoCapture};
use crate::ui_script::{run_script, DEFAULT_FUNCTION};
use bevy::prelude::*;
use lbp_ui_script::pod::{layout, PodMenu, ASPECT, GLOBE, MOON};

/// Vertical field of view (UNKNOWN in the game; only the projected positions matter here).
const FOV: f32 = std::f32::consts::FRAC_PI_4;
/// Distance of the spheres from the camera, in display units.
const DEPTH: f32 = 10.;
/// Globe spin, radians per second (slow, by eye from the captures).
const SPIN: f32 = 0.15;

#[derive(Resource)]
struct Menu {
    menu: PodMenu,
    focused: usize,
}

#[derive(Component)]
struct Globe;

#[derive(Component)]
struct PillNode(usize);

#[derive(Component)]
struct OverlayRoot;

/// The point at frame fractions `f` on the plane `DEPTH` in front of the camera.
pub fn frame_to_world(f: [f32; 2]) -> Vec3 {
    let half = (FOV / 2.).tan() * DEPTH;
    Vec3::new(
        (f[0] - 0.5) * 2. * half * ASPECT,
        (0.5 - f[1]) * 2. * half,
        -DEPTH,
    )
}

/// A radius given as a fraction of the frame height, in display units at `DEPTH`.
pub fn height_to_world(r: f32) -> f32 {
    r * 2. * (FOV / 2.).tan() * DEPTH
}

pub fn run_pod_menu() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    let class = value("--pod-menu").ok_or("--pod-menu requires a CLASS")?;
    let snapshot = value("--snapshot").ok_or("--pod-menu requires --snapshot")?;
    let game = value("--game-dir").ok_or("--pod-menu requires --game-dir")?;
    let function = value("--ui-script-function").unwrap_or_else(|| DEFAULT_FUNCTION.into());
    let hide = all.iter().any(|a| a == "--qa-hide-ui");
    let (report, _) = run_script(snapshot.as_ref(), game.as_ref(), &class, &function)?;
    let menu = layout(&report.tree);
    println!(
        "Pod menu: {} pills ({}), polaroid {}",
        menu.pills.len(),
        menu.pills
            .iter()
            .map(|p| format!(
                "{} {:?} at {:.2}/{:.2}",
                p.label, p.on, p.centre[0], p.centre[1]
            ))
            .collect::<Vec<_>>()
            .join(", "),
        menu.polaroid.is_some()
    );
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.42, 0.33, 0.24)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LBP2 Asset Explorer - Pod menu".into(),
                resolution: (1280, 720).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Menu { menu, focused: 0 })
        .insert_resource(DemoCapture(demo_capture_path()))
        .add_systems(
            Startup,
            move |c: Commands,
                  m: ResMut<Assets<Mesh>>,
                  s: ResMut<Assets<StandardMaterial>>,
                  menu: Res<Menu>| setup(c, m, s, menu, hide),
        )
        .add_systems(
            Update,
            (
                crate::place_on_second_monitor,
                spin,
                keys,
                paint,
                demo_capture,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    (centre, radius): ([f32; 2], f32),
    colour: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(height_to_world(radius)).mesh().uv(48, 24))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: colour,
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(frame_to_world(centre)),
        ))
        .id()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    menu: Res<Menu>,
    hide: bool,
) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            ..default()
        }),
        Transform::IDENTITY,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.,
            ..default()
        },
        Transform::from_xyz(-4., 6., 4.).looking_at(Vec3::new(0., 0., -DEPTH), Vec3::Y),
    ));
    let globe = sphere(
        &mut commands,
        &mut meshes,
        &mut materials,
        GLOBE,
        Color::srgb(0.20, 0.42, 0.62),
    );
    commands.entity(globe).insert(Globe);
    sphere(
        &mut commands,
        &mut meshes,
        &mut materials,
        MOON,
        Color::srgb(0.62, 0.62, 0.60),
    );
    let note = (!hide).then_some(
        "Placeholder globe, moon, Polaroid and background; pill positions from the \
         RPCS3 reference. Up/Down move, Enter prints the choice.",
    );
    spawn_overlay(&mut commands, &menu.menu, |f| f, note);
}

/// The pills and Polaroid as UI nodes at frame fractions (`remap` maps the 16:9 reference
/// fractions to the window), plus an optional fidelity note.
fn spawn_overlay(
    commands: &mut Commands,
    menu: &PodMenu,
    remap: impl Fn([f32; 2]) -> [f32; 2],
    note: Option<&str>,
) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            OverlayRoot,
        ))
        .id();
    let centred = |f: [f32; 2]| Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(f[0] * 100.),
        top: Val::Percent(f[1] * 100.),
        ..default()
    };
    for (i, pill) in menu.pills.iter().enumerate() {
        let node = commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(18.), Val::Px(6.)),
                    border_radius: BorderRadius::all(Val::Px(999.)),
                    ..centred(remap(pill.centre))
                },
                UiTransform::from_translation(Val2::percent(-50., -50.)),
                BackgroundColor(Color::WHITE),
                PillNode(i),
                children![(
                    Text::new(pill.label.clone()),
                    TextFont::from_font_size(22.),
                    TextColor(Color::srgb(0.12, 0.12, 0.14)),
                )],
            ))
            .id();
        commands.entity(root).add_child(node);
    }
    if let Some(f) = menu.polaroid {
        let card = commands
            .spawn((
                Node {
                    width: Val::Vh(10.),
                    height: Val::Vh(12.),
                    padding: UiRect::new(Val::Vh(0.8), Val::Vh(0.8), Val::Vh(0.8), Val::Vh(2.5)),
                    ..centred(remap(f))
                },
                UiTransform::from_translation(Val2::percent(-50., -50.)),
                BackgroundColor(Color::srgb(0.96, 0.95, 0.90)),
                children![(
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.35, 0.35, 0.38)),
                )],
            ))
            .id();
        commands.entity(root).add_child(card);
    }
    if let Some(note) = note {
        let note = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.),
                    bottom: Val::Px(10.),
                    ..default()
                },
                Text::new(note),
                TextFont::from_font_size(14.),
                TextColor(Color::srgba(1., 1., 1., 0.8)),
            ))
            .id();
        commands.entity(root).add_child(note);
    }
}

fn spin(time: Res<Time>, mut globes: Query<&mut Transform, With<Globe>>) {
    for mut t in &mut globes {
        t.rotate_y(SPIN * time.delta_secs());
    }
}

/// Keyboard arrows/Enter or gamepad D-pad/Cross; with the Pod shell only while open.
fn keys(
    input: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    puter: Option<Res<PuterState>>,
    mut menu: ResMut<Menu>,
) {
    let n = menu.menu.pills.len();
    if n == 0 || puter.is_some_and(|p| p.t < 0.99) {
        return;
    }
    let pad = |b| pads.iter().any(|g| g.just_pressed(b));
    if input.just_pressed(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) {
        menu.focused = (menu.focused + 1) % n;
    }
    if input.just_pressed(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) {
        menu.focused = (menu.focused + n - 1) % n;
    }
    if input.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) || pad(GamepadButton::South) {
        println!("Pod menu: chose {}", menu.menu.pills[menu.focused].label);
    }
}

fn paint(
    time: Res<Time>,
    menu: Res<Menu>,
    puter: Option<Res<PuterState>>,
    mut root: Query<&mut Visibility, With<OverlayRoot>>,
    mut pills: Query<(&PillNode, &mut BackgroundColor, &mut UiTransform)>,
) {
    // With the Pod shell, the menu shows once the Puter is open.
    let shown = puter.is_none_or(|p| p.t >= 0.99);
    for mut v in &mut root {
        *v = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let pulse = (time.elapsed_secs() * 4.).sin() * 0.5 + 0.5;
    for (pill, mut background, mut transform) in &mut pills {
        let focused = pill.0 == menu.focused;
        *background = BackgroundColor(if focused {
            Color::srgb(0.30 + 0.10 * pulse, 0.85, 0.95)
        } else {
            Color::WHITE
        });
        transform.scale = Vec2::splat(if focused { 1.08 + 0.03 * pulse } else { 1. });
    }
}

/// P2: the level camera that shows the globe where the RPCS3 reference has it (centre
/// GLOBE.0 of a 16:9 frame, radius GLOBE.1 of the height), looking -Z with no tilt, for a
/// vertical field of view `fov`. `centre` and `radius` are the globe's in display units;
/// returns the camera position (the orbit target is straight ahead of it at the globe's
/// depth). Only the projected centre and radius are matched; the game's camera itself is
/// script-driven and UNKNOWN (PlanetsWorld.UpdatePodCamera).
pub fn pod_camera(centre: Vec3, radius: f32, fov: f32) -> Vec3 {
    eye_for(GLOBE.0, centre, radius, GLOBE.1, fov)
}

/// A level camera (looking -Z) that shows a thing centred at `centre` at frame fraction
/// `at` (16:9 reference) with `size` (display units) spanning `size_h` of the frame height.
pub fn eye_for(at: [f32; 2], centre: Vec3, size: f32, size_h: f32, fov: f32) -> Vec3 {
    let t = (fov / 2.).tan();
    let distance = size / (2. * size_h * t);
    // Offset from the frame centre in heights (x scaled from width to height).
    let dx = (at[0] - 0.5) * ASPECT;
    let dy = 0.5 - at[1];
    Vec3::new(
        centre.x - dx * 2. * distance * t,
        centre.y - dy * 2. * distance * t,
        centre.z + distance,
    )
}

/// P2b closed-Puter (default) view of the globe: centre 0.47/0.48 (RPCS3, about 0.02).
/// The radius is INFERRED: a third of the open view's, from the globe-moon spacing (0.09
/// of the width closed, 0.27 open).
pub const GLOBE_CLOSED: ([f32; 2], f32) = ([0.47, 0.48], 0.075);
/// The DualShock in the shell view: centre and width as a fraction of the 16:9 width,
/// Puter closed and open (RPCS3 previews, about 0.02).
pub const PAD_CLOSED: ([f32; 2], f32) = ([0.50, 0.70], 0.15);
pub const PAD_OPEN: ([f32; 2], f32) = ([0.50, 0.86], 0.17);
/// Opening the Puter takes about a second (RPCS3).
pub const DOLLY_SECONDS: f32 = 1.0;
/// Sackboy standing in the closed view: head top and feet as fractions of the 16:9
/// height, centred at 0.50 of the width (RPCS3, about 0.02).
pub const SACKBOY_CLOSED: [f32; 2] = [0.56, 0.82];
/// Sackboy holding the controller, seen from behind, Puter open: centre (RPCS3, 0.02).
pub const SACKBOY_OPEN: [f32; 2] = [0.50, 0.90];

/// Two layers as the game appears to draw them: the Pod shell (tent, DualShock) over the
/// planets view through its window, each seen by its own camera. The viewer has one
/// camera, so the shell is placed each frame by `X = P + scale (X_shell - S)` (P, S the
/// planets and shell eyes): scaling about the eye keeps its projection, and `scale` puts
/// the whole shell in front of the globe. Eyes are display-space, closed then open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PodRig {
    pub planet: [Vec3; 2],
    pub globe_z: f32,
    pub shell: [Vec3; 2],
    pub scale: f32,
    pub fov: f32,
}

impl PodRig {
    /// `globe`: the globe's display centre and radius. `pad`: the DualShock's display
    /// centre and width in the shell's display space; `shell_near`: the shell's smallest
    /// z (its far side from the camera).
    pub fn new(globe: (Vec3, f32), pad: (Vec3, f32), shell_near: f32, fov: f32) -> Self {
        let (g, r) = globe;
        let planet = [
            eye_for(GLOBE_CLOSED.0, g, r, GLOBE_CLOSED.1, fov),
            pod_camera(g, r, fov),
        ];
        let (c, w) = pad;
        let shell = [
            eye_for(PAD_CLOSED.0, c, w, PAD_CLOSED.1 * ASPECT, fov),
            eye_for(PAD_OPEN.0, c, w, PAD_OPEN.1 * ASPECT, fov),
        ];
        // The shell's far side at half the closed eye's distance to the globe.
        let depth = (shell[0].z - shell_near).max(1e-6);
        let scale = 0.5 * (planet[0].z - g.z) / depth;
        Self {
            planet,
            globe_z: g.z,
            shell,
            scale,
            fov,
        }
    }

    fn at(pair: [Vec3; 2], t: f32) -> Vec3 {
        let t = t.clamp(0., 1.);
        pair[0].lerp(pair[1], t * t * (3. - 2. * t))
    }

    /// The planets eye at opening progress `t` (0 closed, 1 open, eased).
    pub fn planet_eye(&self, t: f32) -> Vec3 {
        Self::at(self.planet, t)
    }

    /// Where a shell display-space point appears at `t`, as 16:9 frame fractions.
    pub fn shell_frac(&self, t: f32, q: Vec3) -> [f32; 2] {
        let c = Mat4::perspective_rh(self.fov, ASPECT, 0.001, 1000.)
            .project_point3(q - Self::at(self.shell, t));
        [(c.x + 1.) / 2., (1. - c.y) / 2.]
    }

    /// Feet position (shell display space) for an upright figure `h` display units tall
    /// that the closed shell eye sees centred in x with head top and feet at
    /// `SACKBOY_CLOSED`. Solves depth from the projected height, then height from the feet.
    pub fn fit_figure(&self, h: f32) -> Vec3 {
        let tan = (self.fov / 2.).tan();
        let [top, feet] = SACKBOY_CLOSED;
        let depth = h / ((feet - top) * 2. * tan);
        self.shell[0] + Vec3::new(0., (0.5 - feet) * 2. * tan * depth, -depth)
    }

    /// The shell root transform at `t`.
    pub fn shell_root(&self, t: f32) -> Transform {
        let p = self.planet_eye(t);
        let s = Self::at(self.shell, t);
        Transform::from_translation(p - s * self.scale).with_scale(Vec3::splat(self.scale))
    }
}

/// Opening progress of the Puter: 0 closed (tent view), 1 open (menu on the globe).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PuterState {
    pub t: f32,
    pub open: bool,
}

/// Maps a 16:9 reference frame fraction into a window of `aspect`, keeping offsets from
/// the centre in heights (a 16:10 window shows a little more above and below).
pub fn reference_to_window(f: [f32; 2], aspect: f32) -> [f32; 2] {
    [0.5 + (f[0] - 0.5) * ASPECT / aspect, f[1]]
}

/// `--pod-menu CLASS` with a level (`--level-guid`/`--level-path`): the pills over the
/// level view in the main viewer instead of the standalone placeholder scene. Pair with
/// `--pod-camera N` (the globe's Thing) so the globe sits where the pills expect it.
pub fn install_overlay(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    let Some(class) = value("--pod-menu") else {
        return Ok(());
    };
    let snapshot = value("--snapshot").ok_or("--pod-menu requires --snapshot")?;
    let game = value("--game-dir").ok_or("--pod-menu requires --game-dir")?;
    let function = value("--ui-script-function").unwrap_or_else(|| DEFAULT_FUNCTION.into());
    let (report, _) = run_script(snapshot.as_ref(), game.as_ref(), &class, &function)?;
    let menu = layout(&report.tree);
    println!("Pod menu overlay: {} pills", menu.pills.len());
    app.insert_resource(Menu { menu, focused: 0 })
        .add_systems(Startup, |mut commands: Commands, menu: Res<Menu>| {
            spawn_overlay(
                &mut commands,
                &menu.menu,
                |f| reference_to_window(f, 1280. / 800.),
                None,
            )
        })
        .add_systems(Update, (keys, paint).chain());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2b_rig_keeps_each_layer_where_its_reference_puts_it() {
        let fov = std::f32::consts::FRAC_PI_4;
        let globe = (Vec3::new(-0.3, 0.2, 0.0), 0.25);
        let pad = (Vec3::new(0.01, -0.2, -0.1), 0.12);
        let rig = PodRig::new(globe, pad, -0.4, fov);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.001, 1000.);
        let frac = |eye: Vec3, p: Vec3| {
            let c = projection.project_point3(p - eye);
            [(c.x + 1.) / 2., (1. - c.y) / 2.]
        };
        for (t, globe_at, pad_at) in [
            (0., GLOBE_CLOSED.0, PAD_CLOSED.0),
            (1., GLOBE.0, PAD_OPEN.0),
        ] {
            let eye = rig.planet_eye(t);
            let g = frac(eye, globe.0);
            assert!((g[0] - globe_at[0]).abs() < 1e-3 && (g[1] - globe_at[1]).abs() < 1e-3);
            // The shell, placed by its root transform, projects as from its own eye.
            let placed = rig.shell_root(t).transform_point(pad.0);
            let p = frac(eye, placed);
            assert!(
                (p[0] - pad_at[0]).abs() < 1e-3 && (p[1] - pad_at[1]).abs() < 1e-3,
                "{t} {p:?}"
            );
            // ... and in front of the globe.
            assert!(placed.z > globe.0.z);
        }
        let closed = rig.shell_frac(0., pad.0);
        assert!((closed[0] - PAD_CLOSED.0[0]).abs() < 1e-3);
        assert!((closed[1] - PAD_CLOSED.0[1]).abs() < 1e-3);
        let feet = rig.fit_figure(0.05);
        let [x, low] = rig.shell_frac(0., feet);
        let top = rig.shell_frac(0., feet + Vec3::Y * 0.05)[1];
        assert!((x - 0.5).abs() < 1e-4 && (low - SACKBOY_CLOSED[1]).abs() < 1e-4);
        assert!((top - SACKBOY_CLOSED[0]).abs() < 1e-4, "{top}");
        let far = rig.shell_root(0.).transform_point(Vec3::new(0., 0., -0.4));
        assert!(far.z > globe.0.z);
    }

    #[test]
    fn p2_pod_camera_projects_the_globe_to_its_reference_place() {
        let fov = std::f32::consts::FRAC_PI_4;
        let (centre, radius) = (Vec3::new(-0.3, 0.2, 0.1), 0.25);
        let eye = pod_camera(centre, radius, fov);
        let view = Mat4::from_translation(-eye);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.01, 100.);
        let clip = projection.project_point3(view.transform_point3(centre));
        let at = [(clip.x + 1.) / 2., (1. - clip.y) / 2.];
        assert!((at[0] - GLOBE.0[0]).abs() < 1e-4 && (at[1] - GLOBE.0[1]).abs() < 1e-4);
        let edge = projection.project_point3(view.transform_point3(centre + Vec3::Y * radius));
        assert!((((edge.y - clip.y) / 2.) - GLOBE.1).abs() < 1e-4);
        assert_eq!(reference_to_window([0.5, 0.3], 1.6), [0.5, 0.3]);
        // A narrower window keeps offsets in heights, so they grow as width fractions.
        assert!(reference_to_window([0.35, 0.4], 1.6)[0] < 0.35);
    }

    #[test]
    fn p1b_frame_points_project_back_to_their_fractions() {
        let projection = Mat4::perspective_rh(FOV, ASPECT, 0.1, 100.);
        for f in [GLOBE.0, MOON.0, [0.5, 0.5], [0.1, 0.9]] {
            let clip = projection.project_point3(frame_to_world(f));
            let back = [(clip.x + 1.) / 2., (1. - clip.y) / 2.];
            assert!((back[0] - f[0]).abs() < 1e-4 && (back[1] - f[1]).abs() < 1e-4);
        }
        // A radius of the full height spans the visible height at DEPTH.
        let top = projection.project_point3(Vec3::new(0., height_to_world(0.5), -DEPTH));
        assert!((top.y - 1.).abs() < 1e-4);
    }
}
