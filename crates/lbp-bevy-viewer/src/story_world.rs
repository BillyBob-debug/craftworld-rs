//! D331 (docs/level-scripts/part-82.md): the Story view's planet world.
//!
//! FOUND (01.33, 0x43cd64 -> 0x42324c): when a story group is chosen the planet clears
//! Earth's world and pastes the group's planet decorations, a level of its own (The
//! Cosmos, group (4, 27): 0x19049), at Earth's centre and radius. That level holds an
//! Earth with the same mesh as the Pod's (its continents in other materials by a
//! PMaterialOverride), the Earth's decoration and a moon.
//!
//! The viewer loads that level with the Pod's planets level and pastes the models of
//! its Earth and the Things under it onto the Pod's Earth Thing ([`merge`]; the level's
//! moon is left out, INFERRED: it is not in Earth's world): each item is moved by the matrix that takes the
//! decorations level's Earth model onto the Pod's, and is kept as an alternate item (out
//! of the scene bounds, spawned hidden). While the Story search is up
//! ([`story_open`]) those items are shown and the Pod's own Earth and Moon are hidden.
//!
//! D336 (docs/level-scripts/part-83.md): the swap also moves the decorations level's
//! backdrop Thing into the Pod world in place of the Pod's (0xdb2c4) and copies its PWorld
//! lighting factors (0xda074), so the Pod world is lit by that backdrop's settings
//! (0xd9990). The viewer pastes those backdrop items unmoved and swaps the lights.
use bevy::prelude::*;
use lbp_preview::level_scene::{self, MeshSource, ScenePayload};

type LevelScene = level_scene::LevelScene<ScenePayload>;

/// The Pod's Earth and the decorations level's Earth are drawn by this mesh.
pub const EARTH_MESH: u32 = 0x0000_27bd;

/// What [`merge`] pasted, for the load's printed line.
#[derive(Debug, Clone, PartialEq)]
pub struct Merged {
    /// The decorations level's Earth Thing.
    pub story_earth: usize,
    /// Items pasted onto the Pod's Earth, and the meshes they draw.
    pub items: usize,
    pub meshes: usize,
    /// Pasted items by the decorations level's Thing and mesh label.
    pub by_thing: std::collections::BTreeMap<(usize, String), usize>,
    /// D336: the decorations level's backdrop items pasted in place of the Pod's backdrop
    /// (none when either level has no backdrop Thing).
    pub backdrop_items: usize,
    /// D336: the lighting of the decorations level's backdrop plan (its preset 0), which
    /// becomes the Pod world's lighting while Story is up.
    pub settings: Option<level_scene::LevelSettings>,
    /// D336: the decorations level's PWorld lighting factors (0xda074 copies them into the
    /// Pod world): `lighting`, `colourCorrection`, `fog`, `fogTint`, `darkness`.
    pub world_lighting: Option<[f32; 5]>,
}

/// The first model item of `thing` drawing [`EARTH_MESH`].
fn earth_item(scene: &LevelScene, thing: Option<usize>) -> Option<&level_scene::SceneItem> {
    scene.items.iter().find(|i| {
        thing.is_none_or(|t| i.thing == t)
            && i.shapes == 0
            && !i.alternate
            && scene.meshes.get(i.mesh).is_some_and(|m| {
                matches!(m.payload, ScenePayload::Model(_))
                    && m.source.base_guid() == Some(EARTH_MESH)
            })
    })
}

/// Pastes the model items of the decorations level `story` onto Thing `globe` of
/// `scene` (the Pod's Earth), as alternate items. INFERRED: "at Earth's centre and
/// radius" is the matrix taking the decorations level's Earth model onto the Pod's.
pub fn merge(scene: &mut LevelScene, story: LevelScene, globe: usize) -> Result<Merged, String> {
    let pod = earth_item(scene, Some(globe)).ok_or("the Pod's Earth has no Earth model")?;
    let deco = earth_item(&story, None).ok_or("the decorations level has no Earth model")?;
    let story_earth = deco.thing;
    let to = Mat4::from_cols_array(&pod.world) * Mat4::from_cols_array(&deco.world).inverse();
    if !to.is_finite() {
        return Err("the Earth placement matrix is not finite".into());
    }
    let backdrop = story.counts.backdrop.thing;
    // Earth's world: the Earth Thing and the Things under it (not the level's moon).
    let in_earth = |mut thing: usize| {
        for _ in 0..=story.parents.len() {
            if thing == story_earth {
                return true;
            }
            match story.parents.get(thing).copied().flatten() {
                Some(p) => thing = p,
                None => return false,
            }
        }
        false
    };
    let model = |m: &level_scene::SceneMesh<ScenePayload>| {
        matches!(m.payload, ScenePayload::Model(_))
            && matches!(
                m.source,
                MeshSource::Guid(_) | MeshSource::Sha1(_) | MeshSource::BoneSkin { .. }
            )
    };
    let kept: Vec<_> = story
        .items
        .iter()
        .filter(|i| {
            i.shapes == 0
                && !i.alternate
                && in_earth(i.thing)
                && Some(i.thing) != backdrop
                && story.meshes.get(i.mesh).is_some_and(model)
        })
        .cloned()
        .collect();
    // D336: the backdrop moves into the Pod world unmoved (0xdb2c4 re-homes the Thing
    // and its plan), so its items keep their level matrices.
    let pod_backdrop = scene.counts.backdrop.thing;
    let backdrop_kept: Vec<_> = match (backdrop, pod_backdrop) {
        (Some(b), Some(_)) => story
            .items
            .iter()
            .filter(|i| i.thing == b && !i.alternate)
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    let settings = story.backdrop_presets.first().copied().or(story.settings);
    let world_lighting = story.world_lighting;
    let mut remap = std::collections::HashMap::new();
    let labels: Vec<String> = story.meshes.iter().map(|m| m.source.label()).collect();
    let mut meshes: Vec<_> = story.meshes.into_iter().map(Some).collect();
    let mut items = 0;
    let mut by_thing = std::collections::BTreeMap::new();
    for mut item in kept {
        *by_thing
            .entry((item.thing, labels[item.mesh].clone()))
            .or_insert(0) += 1;
        let next = scene.meshes.len();
        let index = *remap.entry(item.mesh).or_insert_with(|| {
            scene
                .meshes
                .push(meshes[item.mesh].take().expect("each mesh moved once"));
            next
        });
        item.mesh = index;
        item.world = (to * Mat4::from_cols_array(&item.world)).to_cols_array();
        item.thing = globe;
        item.uid = None;
        item.alternate = true;
        scene.items.push(item);
        items += 1;
    }
    let mut backdrop_items = 0;
    for mut item in backdrop_kept {
        let next = scene.meshes.len();
        let index = *remap.entry(item.mesh).or_insert_with(|| {
            scene
                .meshes
                .push(meshes[item.mesh].take().expect("each mesh moved once"));
            next
        });
        item.mesh = index;
        item.thing = pod_backdrop.expect("checked above");
        item.uid = None;
        item.alternate = true;
        scene.items.push(item);
        backdrop_items += 1;
    }
    Ok(Merged {
        story_earth,
        items,
        meshes: remap.len(),
        by_thing,
        backdrop_items,
        settings,
        world_lighting,
    })
}

/// D343 (part 89 FOUND): a story group's or level's badge is `cosmos_polaroid.mol`; the
/// game binds the slot's icon into its group 0 texture slot each frame (0x6fe974).
pub const POLAROID_MESH: u32 = 0x0000_6a1a;

/// D345 (MEASURED, the mesh's gmats): group 1 (gmat 0x6a16 is group 0's picture) is the
/// card's frame, a solid colour (gmat 0x6a17).
pub const BADGE_FRAME_GROUP: usize = 1;

/// D344 (MEASURED, RPCS3 memory in the Story view): the angle each of Earth's six group
/// results holds at +0x210 (the value 0x6ffff4 scales by) is 0.27, 1.5 times part 81's
/// 0.18 (0x16afb0, which only seeds a settings block).
pub const GROUP_BADGE_ANGLE: f32 = 0.27;

/// D343: the default result's badge: the polaroid with its slot's icon, and where it sits.
pub struct Badge {
    pub mesh: lbp_preview::GroupedMesh,
    /// The slot's `location`, a unit vector in the planet's own space.
    pub location: [f32; 4],
    pub angle: f32,
    /// What went on the picture: the icon's key, or why the placeholder stayed.
    pub icon: Result<String, String>,
    /// D345: whether the frame took the profile's colour (`None`: no colour given).
    pub frame: Option<Result<(), String>>,
}

/// D343: loads the polaroid badge of story slot `slot` (a group: [`GROUP_BADGE_ANGLE`]),
/// its picture group drawn with the slot's icon. A missing or undecodable icon leaves
/// the mesh's placeholder picture. D345: `colour` (packed ARGB, the profile's first
/// colour, which the game's badge instance holds at +0x10 and +0x14, part 91 MEASURED)
/// multiplies the frame's solid colour ([`BADGE_FRAME_GROUP`]). INFERRED: neither of the
/// card's gmats has a ThingColor box, so the game applies it outside the material graph;
/// the picture group is left untinted.
pub fn load_badge(
    source: &lbp_catalog::attachment::AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &lbp_catalog::LoadControl,
    options: lbp_preview::PreviewOptions,
    slot: &lbp_level::slots::Slot,
    colour: Option<u32>,
) -> Result<Badge, String> {
    let key = lbp_preview::resolve(catalog, Some(POLAROID_MESH), None)?;
    let mut mesh =
        lbp_preview::load_mesh_groups_with_overrides(source, catalog, &key, control, options, &[])?;
    let icon = (|| {
        use lbp_preview::story::{decode_icon_texture, ResourceKey};
        let key = slot
            .icon
            .as_ref()
            .and_then(ResourceKey::of)
            .ok_or("the slot has no icon")?;
        let (texture, at) = decode_icon_texture(source, catalog, key, control)?;
        lbp_preview::retexture_group(&mut mesh, 0, texture, at, || control.check().is_err())?;
        Ok(key.label())
    })();
    let frame = colour.map(|c| {
        lbp_preview::tint_solid_group(
            &mut mesh,
            BADGE_FRAME_GROUP,
            lbp_preview::level_scene::editor_color_tint(c),
        )
    });
    Ok(Badge {
        mesh,
        location: slot.location,
        angle: GROUP_BADGE_ANGLE,
        icon,
        frame,
    })
}

/// D343 (FOUND 0x6ffff4, the path badges take): a badge's matrix in its planet's own
/// space, scale `s` = radius x angle / 240: X is `s` x normalised (0, 1, 0) x `location`
/// ((1, 0, 0) when they are parallel), Y is -`s` x `location` (into the planet), Z is
/// `s` x (`location` x X), at `location` times the radius. The polaroid adds no scale,
/// lift or spin of its own (0x6fcfd0, 0x6fe450).
pub fn badge_matrix(location: [f32; 4], radius: f32, angle: f32) -> Option<Mat4> {
    let d = Vec3::new(location[0], location[1], location[2]);
    if !d.is_finite() || d == Vec3::ZERO {
        return None;
    }
    let x = Vec3::Y.cross(d).try_normalize().unwrap_or(Vec3::X);
    let z = d.cross(x).try_normalize().unwrap_or(Vec3::Y);
    let s = radius * angle / 240.;
    Some(Mat4::from_cols(
        (x * s).extend(0.),
        (-d * s).extend(0.),
        (z * s).extend(0.),
        (d * radius).extend(1.),
    ))
}

/// D343: adds the badge to Thing `globe` (the Pod's Earth) as an alternate item, so it
/// shows while the Story search is up. Its place is in Earth's own space: the Thing's
/// rotation and scale about the Earth model's centre (INFERRED from 0x6ffa64: the
/// planet's instances are drawn by its node matrices). D344: at the Thing's scale (0.961)
/// the planet's radius of 1750 lands on the Earth model's surface (about 1620); the Earth
/// model's own 1.35 put the card 1.46 radii out and 1.18 times too tall.
pub fn add_badge(
    scene: &mut LevelScene,
    badge: Badge,
    globe: usize,
) -> Result<BadgeBounds, String> {
    let pod = earth_item(scene, Some(globe)).ok_or("the Pod's Earth has no Earth model")?;
    let earth = scene.meshes.get(pod.mesh).and_then(|m| m.bounds);
    let local = badge_matrix(badge.location, lbp_runtime::EARTH_RADIUS, badge.angle)
        .ok_or("the badge's location is not a direction")?;
    // The planet's turn (part 82) takes the point into the Thing's frame, whose rotation
    // is not the Earth model's (the model is placed by a bone): the badge takes the
    // Thing's rotation and scale, and the Earth model's centre.
    let (_, _, translation) = Mat4::from_cols_array(&pod.world).to_scale_rotation_translation();
    let thing = scene
        .positions
        .get(globe)
        .copied()
        .flatten()
        .ok_or("the Pod's Earth Thing has no POS")?;
    let (scale, rotation, _) = Mat4::from_cols_array(&thing).to_scale_rotation_translation();
    let world = Mat4::from_scale_rotation_translation(scale, rotation, translation) * local;
    let payload = ScenePayload::Model(Box::new(badge.mesh));
    scene.meshes.push(level_scene::SceneMesh {
        source: MeshSource::Guid(POLAROID_MESH),
        bounds: level_scene::payload_bounds(&payload),
        payload,
        neutral_groups: false,
    });
    scene.items.push(level_scene::SceneItem {
        mesh: scene.meshes.len() - 1,
        world: world.to_cols_array(),
        thing: globe,
        uid: None,
        shapes: 0,
        decoration: false,
        alternate: true,
    });
    let card = scene.meshes.last().and_then(|m| m.bounds);
    Ok(BadgeBounds {
        earth,
        card,
        centre: translation,
        at: world.w_axis.truncate(),
    })
}

/// D343: the Earth model and the badge, for the load's line.
#[derive(Debug, Clone, Copy)]
pub struct BadgeBounds {
    pub earth: Option<[[f32; 3]; 2]>,
    pub card: Option<[[f32; 3]; 2]>,
    /// The Earth model's and the badge's places in the level.
    pub centre: Vec3,
    pub at: Vec3,
}

/// D336: a draw entity of the backdrop Thing: the Pod world's own backdrop, or the
/// decorations level's pasted in its place (shown while the Story search is up).
#[derive(Component, Debug, Clone, Copy)]
pub struct Backdrop {
    pub pasted: bool,
}

/// D336: the Pod's lights (planets, shell) and the Story's, swapped while Story is up.
#[derive(Resource, Debug, Clone)]
pub struct StoryLight {
    pub pod: (crate::lighting::PreviewLight, crate::lighting::PreviewLight),
    pub story: (crate::lighting::PreviewLight, crate::lighting::PreviewLight),
}

/// D336: the visibility of a backdrop entity: the pasted one while Story is up.
pub fn backdrop_wanted(backdrop: Backdrop, open: bool) -> Visibility {
    if backdrop.pasted == open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// The Story search is up ([`lbp_runtime::LevelWorld::story_open`]).
pub fn story_open(world: &lbp_runtime::LevelWorld) -> bool {
    world.story_open()
}

/// The visibility of a draw entity of the Pod's Earth or Moon: the pasted items while
/// the Story search is up, the Pod's own otherwise.
pub fn wanted(item: &crate::thing_view::Item, globe: u32, open: bool) -> Option<Visibility> {
    let moon = item.thing == crate::pod_menu::MOON_THING as u32;
    if item.thing != globe && !moon {
        return None;
    }
    let pasted = item.thing == globe && item.alternate;
    Some(if pasted == open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d343_badge_matrix_is_the_games_basis() {
        // A badge facing +Z: X = up x d = +X, Y = -d, Z = d x X = up, at d x radius.
        let m = badge_matrix([0., 0., 1., 0.], 1750., GROUP_BADGE_ANGLE).unwrap();
        let s = 1750. * 0.27 / 240.;
        assert!(m.x_axis.abs_diff_eq(Vec4::new(s, 0., 0., 0.), 1e-4));
        assert!(m.y_axis.abs_diff_eq(Vec4::new(0., 0., -s, 0.), 1e-4));
        assert!(m.z_axis.abs_diff_eq(Vec4::new(0., s, 0., 0.), 1e-4));
        assert!(m.w_axis.abs_diff_eq(Vec4::new(0., 0., 1750., 1.), 1e-3));
        // At a pole up x d is zero: X falls back to +X (0x6ffff4's TOC constants).
        let pole = badge_matrix([0., 1., 0., 0.], 1750., GROUP_BADGE_ANGLE).unwrap();
        assert!(pole.x_axis.abs_diff_eq(Vec4::new(s, 0., 0., 0.), 1e-4));
        assert!(badge_matrix([0.; 4], 1750., 0.18).is_none());
    }

    fn item(thing: u32, alternate: bool) -> crate::thing_view::Item {
        crate::thing_view::Item {
            thing,
            decoration: false,
            world: Mat4::IDENTITY,
            mesh: Some(EARTH_MESH),
            alternate,
        }
    }

    #[test]
    fn d331_story_shows_the_pasted_world_and_hides_the_pods_planets() {
        let shown = Some(Visibility::Inherited);
        let hidden = Some(Visibility::Hidden);
        // Earth is Thing 1, the Moon Thing 4 (`pod_menu::MOON_THING`).
        assert_eq!(wanted(&item(1, true), 1, true), shown);
        assert_eq!(wanted(&item(1, false), 1, true), hidden);
        assert_eq!(wanted(&item(4, false), 1, true), hidden);
        assert_eq!(wanted(&item(1, true), 1, false), hidden);
        assert_eq!(wanted(&item(1, false), 1, false), shown);
        assert_eq!(wanted(&item(4, false), 1, false), shown);
        assert_eq!(wanted(&item(7, false), 1, true), None);
    }

    #[test]
    fn d336_story_swaps_the_pods_backdrop_for_the_pasted_one() {
        let pod = Backdrop { pasted: false };
        let story = Backdrop { pasted: true };
        assert_eq!(backdrop_wanted(pod, false), Visibility::Inherited);
        assert_eq!(backdrop_wanted(story, false), Visibility::Hidden);
        assert_eq!(backdrop_wanted(pod, true), Visibility::Hidden);
        assert_eq!(backdrop_wanted(story, true), Visibility::Inherited);
    }
}
