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

/// D346 FOUND (01.33): a badge's overlay slots 1..12, the medals and marks drawn on it.
/// Each slot's mesh is a TOC GUID (r2+0x6c98 down to r2+0x6c6c, the wrappers at
/// 0x6fcd30..0x6fcf98 into 0x6fca08), and its place on the polaroid is the second column
/// of the 13 x 2 table 0x6fe398 builds (0xe535b0, read by 0x6fe974): an offset in the
/// card's own space (the picture faces -Y). Slots 1 and 8..10 sit at the card's origin.
pub const BADGE_OVERLAYS: [(u32, [f32; 3]); 12] = [
    (0x0000_0eb1, [0., 0., 0.]),         // 1 padlock
    (0x0000_d12f, [0., -20., -200.]),    // 2 medal_finish
    (0x0001_3890, [0., -20., -200.]),    // 3 medal_played
    (0x0000_d12d, [150., -40., -150.]),  // 4 medal_collect
    (0x0000_d12e, [-150., -10., -150.]), // 5 medal_ace
    (0x0000_d16d, [-150., -10., -50.]),  // 6 medal_heart
    (0x0001_52b0, [150., -10., -50.]),   // 7 medal_playlist
    (0x0001_3ee0, [0., 0., 0.]),         // 8 badge_rosette
    (0x0002_2f20, [0., 0., 0.]),         // 9 badge_versus
    (0x0002_2f1f, [0., 0., 0.]),         // 10 badge_cutscene
    (0x0002_3a8c, [-150., -40., 150.]),  // 11 medal_lbp1
    (0x0002_edc4, [150., -20., 150.]),   // 12 medal_move
];

/// D346 FOUND (0x6fe974): each overlay turns about the card's Y (its depth axis, so an
/// in-plane tilt) by a random value in [-1, 1] times pi/8 (r2+0x6dd4). D349 FOUND
/// (0x6fcc0c, 0x6fcce4): the value is one draw of the game's shared generator (the
/// word at game+0x400; x = (x 0x2e90edd + 0xe908fc1) & 0x3fffffff, value
/// 2 ((x >> 16) & 0x3fff) 6.1038882e-5 - 1, the factor r2+0x6c68), taken when a shown
/// slot's mesh is first made, in slot order. INFERRED: the generator's state then
/// depends on every earlier draw (about 44 functions use it), so the viewer starts it at
/// [`OVERLAY_SEED`]. Returns each slot's spin in radians for `shown` (bit `n` for slot
/// `n`), 0 for the others.
pub fn overlay_spins(shown: u16, seed: u32) -> [f32; 13] {
    let mut x = seed;
    let mut spins = [0.; 13];
    for (n, spin) in spins.iter_mut().enumerate().skip(1) {
        if shown & (1 << n) != 0 {
            x = x.wrapping_mul(0x2e9_0edd).wrapping_add(0xe90_8fc1) & 0x3fff_ffff;
            let r = 2. * (((x >> 16) & 0x3fff) as f32 * 6.103_888e-5) - 1.;
            *spin = r * std::f32::consts::FRAC_PI_8;
        }
    }
    spins
}

/// D349: the generator state the viewer's overlay spins start from (INFERRED, see
/// [`overlay_spins`]).
pub const OVERLAY_SEED: u32 = 1;

/// D349 FOUND (0x6fcc78, 0x6fec44): with the card's grow flag set (card+0x22c) a new
/// overlay's scale and spin each start at 0 (current and previous) and move towards 1 and
/// their random spin, each update: next = current, plus 0.7 (current - previous)
/// (r2+0x6db0), plus 0.25 (target - current) (r2+0x6d14). The drawn spin is the current one times pi/8,
/// the scale is uniform. All of a badge's new overlays start together (no stagger found).
/// D350 FOUND: the update runs in the planets' combined update-and-draw (0x42f43c calls
/// 0x42a92c, then 0x4226b0 to 0x6fe974); INFERRED: once per game tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OverlayGrow {
    /// Scale, then spin (radians): (current, previous).
    pub scale: (f32, f32),
    pub spin: (f32, f32),
}

impl OverlayGrow {
    /// One update towards scale 1 and spin `spin`.
    pub fn step(&mut self, spin: f32) {
        self.step_to(1., spin);
    }

    fn step_to(&mut self, scale: f32, spin: f32) {
        let next = |(cur, prev): (f32, f32), target: f32| {
            (cur + 0.7 * (cur - prev) + 0.25 * (target - cur), cur)
        };
        self.scale = next(self.scale, scale);
        self.spin = next(self.spin, spin);
    }

    /// D351 FOUND (0x6fcc68, 0x6fec94-0x6fecb0): a slot no longer wanted (card+0x1a0+n
    /// cleared, its scale target set to 0) keeps its mesh and is drawn while it shrinks:
    /// one update towards scale 0, then it is freed (0x6fba24 at 0x6ff324) once its
    /// current scale is 0 or below (r2+0x6c50). Returns whether it is still drawn. Slot 0
    /// (not an overlay) is freed at once instead (0x6fcfd0). No viewer path drops a
    /// single overlay yet (the set is the profile's, fixed at load).
    #[allow(dead_code)] // the game's slot model, exercised by tests
    pub fn leave(&mut self, spin: f32) -> bool {
        self.step_to(0., spin);
        self.scale.0 > 0.
    }

    /// D350 FOUND (0x6fcc78, 0x42b288): a slot made while the card's grow flag is clear
    /// (its first fill after it is made or reset, 0x6fc5c0 / 0x42330c) starts at rest on
    /// scale 1 and its spin.
    pub fn settled(spin: f32) -> Self {
        Self {
            scale: (1., 1.),
            spin: (spin, spin),
        }
    }
}

/// D352 FOUND (0x42b4c8-0x42b534, in the fill loop 0x42a92c): a card's own scale is a
/// spring, at card+0x210 with its speed at card+0x21c: each update speed = 0.7 speed
/// (TOC-0x799c) + 0.25 (target - scale) (TOC-0x7998), then scale += speed; the same
/// constants as an overlay's ([`OverlayGrow`]). The target is the slot's scale times
/// planet+0x114/+0x118 (0x42ae84), 0 once the card's slot is gone. FOUND (0x42bc50 to
/// 0x6fc414): a new card is made at its target with speed 0, so opening the Story search
/// shows the cards at full size at once; only closing animates. FOUND (0x42afa4): a card
/// whose scale and target are 0 or below is erased with its slots. INFERRED: closing
/// sends the target to 0 (the slot's scale, not planet+0x114, which only follows a
/// settings object) and the card is drawn while its scale is above 0, so from 1 it shows
/// at 0.75, 0.3875 and 0.0369, then is gone (the overshoot to -0.2178 is never drawn).
/// Left out: the snap when `*(planets+0x20)+0xb0` is set (scale = target, speed 0) and
/// the 0.05 kick a CPlanets method adds to the speed (0x4279a8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardScale {
    pub scale: f32,
    pub speed: f32,
}

impl CardScale {
    /// A new card, at rest on `target`.
    pub fn made(target: f32) -> Self {
        Self {
            scale: target,
            speed: 0.,
        }
    }

    /// One update towards `target`.
    pub fn step(&mut self, target: f32) {
        self.speed = 0.7 * self.speed + 0.25 * (target - self.scale);
        self.scale += self.speed;
    }

    /// Whether the fill loop erases the card (0x42afa4).
    pub fn erased(&self, target: f32) -> bool {
        self.scale <= 0. && target <= 0.
    }
}

/// D352: the Story card shrinking out after the search closed (`None` while open or once
/// it is erased). Set in the planets' tick (`follow_story_world`), drawn by
/// `grow_badge_overlays`.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct StoryCard(pub Option<CardScale>);

/// D352: whether a scene item is the badge's card itself (the polaroid).
pub fn is_card(item: &crate::thing_view::Item) -> bool {
    item.alternate && item.mesh == Some(POLAROID_MESH)
}

/// D349: whether a scene item is one of a badge's overlays (they can spring in, D350).
pub fn is_overlay(item: &crate::thing_view::Item) -> bool {
    item.alternate
        && item
            .mesh
            .is_some_and(|m| BADGE_OVERLAYS.iter().any(|(g, _)| *g == m))
}

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
    /// D346: the overlays shown (slot 1..12 and mesh), or why one was left out.
    pub overlays: Vec<(usize, Result<lbp_preview::GroupedMesh, String>)>,
}

/// D343: loads the polaroid badge of story slot `slot` (a group: [`GROUP_BADGE_ANGLE`]),
/// its picture group drawn with the slot's icon. A missing or undecodable icon leaves
/// the mesh's placeholder picture. D345: `colour` (packed ARGB, the profile's first
/// colour, which the game's badge instance holds at +0x10 and +0x14, part 91 MEASURED)
/// multiplies the frame's solid colour ([`BADGE_FRAME_GROUP`]). INFERRED: neither of the
/// card's gmats has a ThingColor box, so the game applies it outside the material graph;
/// the picture group is left untinted. D346: `overlays` has bit `n` set for each overlay
/// slot `n` (1..12, [`BADGE_OVERLAYS`]) the game shows on this badge.
pub fn load_badge(
    source: &lbp_catalog::attachment::AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &lbp_catalog::LoadControl,
    options: lbp_preview::PreviewOptions,
    slot: &lbp_level::slots::Slot,
    colour: Option<u32>,
    overlays: u16,
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
    let overlays = (1..=BADGE_OVERLAYS.len())
        .filter(|n| overlays & (1 << n) != 0)
        .map(|n| {
            let guid = BADGE_OVERLAYS[n - 1].0;
            let mesh = lbp_preview::resolve(catalog, Some(guid), None).and_then(|key| {
                lbp_preview::load_mesh_groups_with_overrides(
                    source,
                    catalog,
                    &key,
                    control,
                    options,
                    &[],
                )
            });
            (n, mesh)
        })
        .collect();
    Ok(Badge {
        mesh,
        location: slot.location,
        angle: GROUP_BADGE_ANGLE,
        icon,
        frame,
        overlays,
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
    let mut push = |source: u32, mesh: lbp_preview::GroupedMesh, world: Mat4| {
        let payload = ScenePayload::Model(Box::new(mesh));
        scene.meshes.push(level_scene::SceneMesh {
            source: MeshSource::Guid(source),
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
        scene.meshes.last().and_then(|m| m.bounds)
    };
    let card = push(POLAROID_MESH, badge.mesh, world);
    // D346: each overlay at its offset in the card's space, turned about the card's Y.
    let shown = badge.overlays.iter().fold(0u16, |m, (n, _)| m | 1 << n);
    let spins = overlay_spins(shown, OVERLAY_SEED);
    let mut overlays = Vec::new();
    for (n, mesh) in badge.overlays {
        let (guid, at) = BADGE_OVERLAYS[n - 1];
        overlays.push((
            n,
            mesh.map(|m| {
                push(guid, m, world * overlay_matrix(at, spins[n]));
            }),
        ));
    }
    Ok(BadgeBounds {
        earth,
        card,
        centre: translation,
        at: world.w_axis.truncate(),
        overlays,
    })
}

/// D346: the overlays for the `Story badge` line: each slot drawn, or why not.
pub fn overlays_line(overlays: &[(usize, Result<(), String>)]) -> String {
    if overlays.is_empty() {
        return "none".into();
    }
    let each = overlays.iter().map(|(n, r)| match r {
        Ok(()) => format!("{n} {:08x}", BADGE_OVERLAYS[n - 1].0),
        Err(e) => format!("{n} not drawn ({e})"),
    });
    each.collect::<Vec<_>>().join(", ")
}

/// D346: an overlay's matrix in the card's space: its offset, then its spin about Y
/// ([`overlay_spins`]). D349: the spring's scale ([`OverlayGrow`]) is applied on the draw
/// entity, after this. D350: the axis is FOUND (0x6ff06c: the mesh's X and Z rows mix, Y
/// stays), the card's depth axis, so the overlay turns in the card's plane.
pub fn overlay_matrix(at: [f32; 3], spin: f32) -> Mat4 {
    Mat4::from_translation(Vec3::from(at)) * Mat4::from_rotation_y(spin)
}

/// D343: the Earth model and the badge, for the load's line.
#[derive(Debug, Clone)]
pub struct BadgeBounds {
    pub earth: Option<[[f32; 3]; 2]>,
    pub card: Option<[[f32; 3]; 2]>,
    /// The Earth model's and the badge's places in the level.
    pub centre: Vec3,
    pub at: Vec3,
    /// D346: the overlay slots drawn, or why one was not.
    pub overlays: Vec<(usize, Result<(), String>)>,
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

/// D352: [`wanted`], except that the badge's card and overlays stay drawn while the card
/// shrinks out after closing ([`CardScale`]).
pub fn wanted_closing(
    item: &crate::thing_view::Item,
    globe: u32,
    open: bool,
    shrinking: bool,
) -> Option<Visibility> {
    if shrinking && item.thing == globe && (is_card(item) || is_overlay(item)) {
        return Some(Visibility::Inherited);
    }
    wanted(item, globe, open)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d346_overlays_sit_on_the_card() {
        // Finish below the picture, ace and collect at its bottom corners.
        let at = |n: usize| {
            overlay_matrix(BADGE_OVERLAYS[n - 1].1, 0.3)
                .w_axis
                .truncate()
        };
        assert_eq!(at(2), Vec3::new(0., -20., -200.));
        assert_eq!(at(5).x, -150.);
        assert_eq!(at(4).x, 150.);
        for spin in overlay_spins(0x1ffe, OVERLAY_SEED) {
            assert!(spin.abs() <= std::f32::consts::FRAC_PI_8);
            // The spin is about Y: the card's depth axis stays put.
            let y = overlay_matrix([0.; 3], spin).transform_vector3(Vec3::Y);
            assert!((y - Vec3::Y).length() < 1e-6);
        }
    }

    #[test]
    fn d350_overlay_turn_is_the_games_row_mix() {
        // 0x6ff06c-0x6ff0c8: X' = cos X - sin Z and Z' = sin X + cos Z, Y kept (the card's
        // depth axis), so the overlay turns in the card's plane.
        for spin in [0.3_f32, -std::f32::consts::FRAC_PI_8] {
            let m = overlay_matrix([0.; 3], spin);
            let (s, c) = spin.sin_cos();
            assert!(m.x_axis.abs_diff_eq(Vec4::new(c, 0., -s, 0.), 1e-6));
            assert!(m.y_axis.abs_diff_eq(Vec4::Y, 1e-6));
            assert!(m.z_axis.abs_diff_eq(Vec4::new(s, 0., c, 0.), 1e-6));
        }
    }

    #[test]
    fn d350_first_fill_is_settled() {
        let mut g = OverlayGrow::settled(0.3);
        for _ in 0..3 {
            g.step(0.3);
            assert_eq!((g.scale.0, g.spin.0), (1., 0.3));
        }
    }

    #[test]
    fn d351_left_overlay_shrinks_out() {
        let mut g = OverlayGrow::settled(0.3);
        let drawn: Vec<bool> = (0..4).map(|_| g.leave(0.3)).collect();
        // 1 -> 0.75 -> 0.3875 -> 0.036875, then past 0 (the spring overshoots): freed on
        // the fourth update.
        assert_eq!(drawn, [true, true, true, false]);
        assert!(g.scale.0 < 0.);
        assert_eq!(g.spin.0, 0.3);
    }

    #[test]
    fn d352_card_opens_full_and_shrinks_out() {
        // Opening: a new card rests on its target.
        let mut c = CardScale::made(1.);
        c.step(1.);
        assert_eq!(c, CardScale::made(1.));
        // Closing: target 0; drawn while above 0, erased on the fourth update.
        let mut drawn = Vec::new();
        while !c.erased(0.) {
            c.step(0.);
            drawn.push(c.scale);
        }
        let want = [0.75, 0.3875, 0.036875, -0.217_781_25];
        assert_eq!(drawn.len(), want.len());
        for (a, b) in drawn.iter().zip(want) {
            assert!((a - b).abs() < 1e-5, "{drawn:?}");
        }
    }

    #[test]
    fn d349_overlay_spins_are_the_games_generator() {
        // Seed 0: the first draw is 0xe908fc1, whose 14 bits from 16 are 0xe90.
        let spins = overlay_spins(1 << 2 | 1 << 5, 0);
        let pi8 = std::f32::consts::FRAC_PI_8;
        let r = 2. * (0xe90 as f32 * 6.103_888e-5) - 1.;
        assert_eq!(spins[2], r * pi8);
        // Slots not shown take no draw and no spin; the next shown slot takes the next draw.
        assert_eq!(spins[3], 0.);
        assert_eq!(overlay_spins(1 << 5, 0)[5], spins[2]);
        assert_ne!(spins[5], spins[2]);
        // The 14-bit value times the factor spans [-1, 1].
        assert!((2. * (0x3fff as f32 * 6.103_888e-5) - 1. - 1.).abs() < 1e-4);
    }

    #[test]
    fn d349_overlays_spring_in_from_nothing() {
        let mut g = OverlayGrow::default();
        g.step(0.3);
        // First update: 0.25 of the way.
        assert_eq!(g.scale, (0.25, 0.));
        assert!((g.spin.0 - 0.075).abs() < 1e-6);
        let mut peak: f32 = 0.;
        for _ in 0..120 {
            g.step(0.3);
            peak = peak.max(g.scale.0);
        }
        // Underdamped: it overshoots, then settles on scale 1 and the spin.
        assert!(peak > 1.2 && peak < 1.6, "peak {peak}");
        assert!((g.scale.0 - 1.).abs() < 1e-3 && (g.spin.0 - 0.3).abs() < 1e-3);
    }

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
    fn d352_a_shrinking_card_stays_drawn_after_closing() {
        let card = crate::thing_view::Item {
            mesh: Some(POLAROID_MESH),
            ..item(1, true)
        };
        let shown = Some(Visibility::Inherited);
        assert_eq!(wanted_closing(&card, 1, false, true), shown);
        assert_eq!(
            wanted_closing(&card, 1, false, false),
            Some(Visibility::Hidden)
        );
        // The pasted Earth goes at once; only the card shrinks out.
        assert_eq!(
            wanted_closing(&item(1, true), 1, false, true),
            Some(Visibility::Hidden)
        );
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
