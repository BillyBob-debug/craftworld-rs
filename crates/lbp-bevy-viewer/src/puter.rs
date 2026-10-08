//! P1: the Puter screen. `--puter-screen CLASS` runs one of the user's own UI scripts
//! headlessly (as `--ui-script`, stub natives) and draws the frame tree it builds onto a
//! screen in the 3D view instead of a flat window: the gooey UI renders through a second
//! camera into a 1280 x 720 texture, shown unlit on a quad. It works in any 3D mode (single
//! mesh, level), so the screen can be placed in the pod room once that is found.
//! Keyboard: arrows move focus, Enter activates (the tree is the script's one run, so
//! activation changes only the focus outline). Mouse picking on the quad is not wired.
//! Placement is a viewer choice, not the game's (UNKNOWN until the PC reference captures):
//! `--puter-at X,Y,Z[,WIDTH]` in display units, facing +Z (the default camera); the default
//! sits to the right of a single mesh.
//! Usage: `--puter-screen CLASS [--puter-function NAME] [--puter-at X,Y,Z[,WIDTH]]` with
//! `--snapshot` and `--game-dir`.
use crate::gooey_ui::{add_gooey, GooeyTarget};
use crate::ui_script::{run_script, DEFAULT_FUNCTION, ROOT};
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use lbp_ui_script::tree::ui_tree;

/// Screen texture size: the Gooey host answers 1280 x 720 to the scripts' screen queries.
pub const SCREEN: [u32; 2] = [1280, 720];

/// Where the screen quad sits: centre in display units and width (height follows 16:9).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PuterPlacement {
    pub centre: Vec3,
    pub width: f32,
}

impl Default for PuterPlacement {
    fn default() -> Self {
        Self {
            centre: Vec3::new(1.1, 0., 0.),
            width: 1.2,
        }
    }
}

/// `X,Y,Z[,WIDTH]`, all finite, width positive.
pub fn parse_placement(value: &str) -> Result<PuterPlacement, String> {
    let n: Vec<f32> = value
        .split(',')
        .map(|x| x.trim().parse::<f32>())
        .collect::<Result<_, _>>()
        .map_err(|_| "--puter-at takes X,Y,Z[,WIDTH]")?;
    let width = match n.len() {
        3 => PuterPlacement::default().width,
        4 => n[3],
        _ => return Err("--puter-at takes X,Y,Z[,WIDTH]".into()),
    };
    if !n.iter().all(|v| v.is_finite()) || width <= 0. {
        return Err("--puter-at values must be finite and WIDTH positive".into());
    }
    Ok(PuterPlacement {
        centre: Vec3::new(n[0], n[1], n[2]),
        width,
    })
}

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

/// Adds the Puter screen when `--puter-screen` is given; otherwise does nothing.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let Some(class) = value(&all, "--puter-screen") else {
        return Ok(());
    };
    let snapshot = value(&all, "--snapshot").ok_or("--puter-screen requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--puter-screen requires --game-dir")?;
    let function = value(&all, "--puter-function").unwrap_or_else(|| DEFAULT_FUNCTION.into());
    let placement = match value(&all, "--puter-at") {
        Some(v) => parse_placement(&v)?,
        None => PuterPlacement::default(),
    };
    let (report, _header) = run_script(snapshot.as_ref(), game.as_ref(), &class, &function)?;
    let root = report.tree;
    // The count lines are printed by run_script; the screen shows only the script's tree.
    add_gooey(
        app,
        Box::new(move |input| ui_tree(&root, ROOT, &[], input)),
        Vec::new(),
        None,
        None,
    );
    // PostStartup: the viewer's own 3D camera (Startup) exists first, so the egui panel
    // keeps attaching to it rather than to the texture camera.
    app.insert_resource(placement)
        .add_systems(PostStartup, spawn_screen);
    Ok(())
}

fn spawn_screen(
    mut commands: Commands,
    placement: Res<PuterPlacement>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target: ResMut<GooeyTarget>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SCREEN[0],
            height: SCREEN[1],
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let handle = images.add(image);
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: -1,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.03, 0.05)),
                ..default()
            },
            RenderTarget::Image(handle.clone().into()),
        ))
        .id();
    target.0 = Some(camera);
    let height = placement.width * SCREEN[1] as f32 / SCREEN[0] as f32;
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(placement.width, height))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(handle),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(placement.centre),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p1_puter_placement_parses_and_rejects_bad_values() {
        assert_eq!(
            parse_placement("1, 2, -3").unwrap(),
            PuterPlacement {
                centre: Vec3::new(1., 2., -3.),
                width: 1.2
            }
        );
        assert_eq!(parse_placement("0,0,0,2").unwrap().width, 2.);
        for bad in [
            "1,2",
            "1,2,3,4,5",
            "1,x,3",
            "1,2,NaN",
            "0,0,0,0",
            "0,0,0,-1",
        ] {
            assert!(parse_placement(bad).is_err(), "{bad}");
        }
    }
}
