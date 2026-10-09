//! D121 opt-in preview lighting (docs/d121-preview-lighting.md). A directional key light
//! plus ambient (Lambert on the generated diagnostic normals), multiplied onto every
//! material mode's finished colour in `dual.rs`. Not game lighting: LBP's renderer,
//! PWorld lights, rim light, exposure and backdrop lighting are not reproduced.
use bevy::prelude::*;
use lbp_preview::level_scene::LevelSettings;

pub const LABEL: &str = "preview lighting (not game lighting)";
pub const LEVEL_LABEL: &str =
    "preview lighting from level settings (approximate, not game shading)";

/// Linear fog in display units (Bevy `FogFalloff::Linear`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub color: Vec3,
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewLight {
    /// Unit direction toward the light (display space = world space up to scale).
    pub toward_light: Vec3,
    pub key: Vec3,
    pub ambient: Vec3,
    pub fog: Option<Fog>,
    pub from_level: bool,
    /// P2l: level values before the preview fit (key = sun colour x multiplier).
    pub raw_key: Vec3,
    pub raw_ambient: Vec3,
    /// P2l: rim colours (rimColor below, rimColor2 above), zero without level settings.
    pub rim: [Vec3; 2],
    /// P2p: level exposure (1 without level settings or when unusable).
    pub exposure: f32,
    /// P2l: LBP-shaped mode (`lbp`); off = D121 Lambert preview.
    pub lbp: bool,
    /// P2q: the scalar (.w) the game sends with the sun, ambient and rim colours; it
    /// weights the gloss term (brdf.cg `light.w`). Zero outside LBP mode.
    pub scales: Vec3,
    /// P2q: level sun multiplier (already folded into `raw_key`); 1 without settings.
    pub multiplier: f32,
    /// Values used and every preview choice, for the panel and QA log.
    pub notes: Vec<String>,
}

impl Default for PreviewLight {
    fn default() -> Self {
        Self::generic()
    }
}

/// Fully lit factor never exceeds 1 per channel (preview choice: no tonemapping here,
/// so brighter factors would clip textures to white).
fn fit(key: Vec3, ambient: Vec3) -> (Vec3, Vec3, f32) {
    let peak = (key + ambient).max_element();
    let scale = if peak > 1. { 1. / peak } else { 1. };
    (key * scale, ambient * scale, scale)
}

fn finite_rgb(v: [f32; 4]) -> Option<Vec3> {
    let c = Vec3::new(v[0], v[1], v[2]);
    (c.is_finite() && c.min_element() >= 0.).then_some(c)
}

impl PreviewLight {
    /// Generic key light from upper left, in front (LBP side-on view looks along -Z).
    pub fn generic() -> Self {
        Self {
            toward_light: Vec3::new(-0.45, 0.75, 0.5).normalize(),
            key: Vec3::splat(0.65),
            ambient: Vec3::splat(0.35),
            fog: None,
            from_level: false,
            raw_key: Vec3::splat(0.65),
            raw_ambient: Vec3::splat(0.35),
            rim: [Vec3::ZERO; 2],
            exposure: 1.,
            lbp: false,
            scales: Vec3::ZERO,
            multiplier: 1.,
            notes: vec!["generic key (upper left, front) 0.65 + ambient 0.35".into()],
        }
    }

    /// Level settings mapping. `centre` is the displayed bounds centre in level units and
    /// `scale` the display units per level unit (scene normalization). Any unusable value
    /// falls back to the generic light, with the reason noted.
    pub fn from_settings(s: &LevelSettings, centre: Vec3, scale: f32) -> Self {
        match Self::try_settings(s, centre, scale) {
            Ok(light) => light,
            Err(e) => {
                let mut light = Self::generic();
                light
                    .notes
                    .push(format!("level settings not used ({e}); generic light"));
                light
            }
        }
    }

    fn try_settings(s: &LevelSettings, centre: Vec3, scale: f32) -> Result<Self, String> {
        let sun = Vec3::from(s.sun_position) * s.sun_position_scale;
        let toward = (sun - centre)
            .try_normalize()
            .filter(|_| sun.is_finite() && centre.is_finite())
            .ok_or("sun position unusable")?;
        let multiplier = Some(s.sun_multiplier)
            .filter(|m| m.is_finite() && *m >= 0.)
            .ok_or("sun multiplier unusable")?;
        let key = finite_rgb(s.sun_color).ok_or("sun colour unusable")? * multiplier;
        let ambient = finite_rgb(s.ambient_color).ok_or("ambient colour unusable")?;
        let rim = [
            finite_rgb(s.rim_color).unwrap_or(Vec3::ZERO),
            finite_rgb(s.rim_color2).unwrap_or(Vec3::ZERO),
        ];
        let (raw_key, raw_ambient) = (key, ambient);
        let (key, ambient, fitted) = fit(key, ambient);
        let fog = finite_rgb(s.fog_color)
            .filter(|_| {
                s.fog_near.is_finite()
                    && s.fog_far.is_finite()
                    && s.fog_far > s.fog_near
                    && scale.is_finite()
                    && scale > 0.
            })
            .map(|color| Fog {
                color,
                start: s.fog_near * scale,
                end: s.fog_far * scale,
            });
        let mut notes = vec![
            format!(
                "sun position ({:.4}, {:.4}, {:.4}) x {:.1}; toward light ({:.3}, {:.3}, {:.3})",
                s.sun_position[0],
                s.sun_position[1],
                s.sun_position[2],
                s.sun_position_scale,
                toward.x,
                toward.y,
                toward.z
            ),
            format!(
                "sun colour ({:.3}, {:.3}, {:.3}) x multiplier {:.3}; ambient ({:.3}, {:.3}, {:.3})",
                s.sun_color[0],
                s.sun_color[1],
                s.sun_color[2],
                multiplier,
                s.ambient_color[0],
                s.ambient_color[1],
                s.ambient_color[2]
            ),
            format!("preview fit: key and ambient x {fitted:.3} (lit peak 1; exposure {:.3}, rim colours not applied)", s.exposure),
        ];
        notes.push(match fog {
            Some(f) => format!(
                "fog ({:.3}, {:.3}, {:.3}) linear {:.1}..{:.1} level units (Bevy distance fog)",
                f.color.x, f.color.y, f.color.z, s.fog_near, s.fog_far
            ),
            None => "fog not used (unusable colour or range)".into(),
        });
        Ok(Self {
            toward_light: toward,
            key,
            ambient,
            fog,
            from_level: true,
            raw_key,
            raw_ambient,
            rim,
            exposure: Some(s.exposure)
                .filter(|e| e.is_finite() && *e > 0.)
                .unwrap_or(1.),
            lbp: false,
            scales: Vec3::ZERO,
            multiplier,
            notes,
        })
    }

    pub fn label(&self) -> &'static str {
        if self.from_level {
            LEVEL_LABEL
        } else {
            LABEL
        }
    }

    /// P2l `--lbp-lighting`: the same values in an LBP-shaped model (community brdf.cg
    /// reconstruction, docs/p2l-lbp-lighting.md): hemisphere ambient, sun, two rim
    /// colours, unfitted. `turn` rotates the light from level into world space (the Pod
    /// planets root). The uniform mapping (exposure, tint, rim constants) is INFERRED.
    pub fn lbp_shaped(&self, turn: Quat) -> Self {
        let mut light = self.clone();
        let e = self.exposure;
        light.lbp = true;
        // P2p: the scales the game sends with each colour (D-RSX capture of the Pod).
        light.key = self.raw_key * GAME_SUN_SCALE * e;
        light.ambient = self.raw_ambient * GAME_AMBIENT_SCALE * e;
        light.rim = self.rim.map(|r| r * GAME_RIM_SCALE * e);
        light.scales = Vec3::new(
            GAME_SUN_SCALE * self.multiplier * e,
            GAME_AMBIENT_SCALE * e,
            GAME_RIM_SCALE * e,
        );
        light.toward_light = (turn * self.toward_light).normalize_or_zero();
        light.notes.push(format!(
            "LBP-shaped: ambient x (N.y 0.15 + 0.5), sun, rim ({:.3}, {:.3}, {:.3}) below to ({:.3}, {:.3}, {:.3}) above x (N.V + 0.9)^2; game scales sun {GAME_SUN_SCALE} x multiplier, ambient {GAME_AMBIENT_SCALE}, rim {GAME_RIM_SCALE}, all x exposure {e:.3} (RSX capture fit)",
            self.rim[0].x, self.rim[0].y, self.rim[0].z, self.rim[1].x, self.rim[1].y, self.rim[1].z
        ));
        light
    }

    /// CPU mirror of the shader's `preview_light` for an already oriented unit normal.
    #[allow(dead_code)] // shader mirror, exercised by tests
    pub fn factor(&self, normal: Vec3) -> Vec3 {
        self.ambient + self.key * normal.dot(self.toward_light).max(0.)
    }

    /// CPU mirror of the shader's LBP-shaped light for an oriented unit normal and the
    /// unit direction from the surface to the viewer.
    #[allow(dead_code)] // shader mirror, exercised by tests
    pub fn lbp_factor(&self, normal: Vec3, to_view: Vec3) -> Vec3 {
        let up = (normal.y + 1.) / 2.;
        // V from the eye: 0 facing the viewer, 0.81 at grazing. P2m tried V toward the
        // eye; D203 measured the room 2.8x too bright with it, so it was reverted (P2n).
        let rim = (normal.dot(-to_view) + 0.9).clamp(0., 1.).powi(2);
        self.ambient * (normal.y * 0.15 + 0.5)
            + self.key * normal.dot(self.toward_light).max(0.)
            + self.rim[0].lerp(self.rim[1], up) * rim
    }
}

/// CPU mirror of the shader's double-sided orientation: the smooth normal is turned into
/// the hemisphere of the face normal that points at the viewer.
#[allow(dead_code)] // shader mirror, exercised by tests
pub fn facing_normal(smooth: Vec3, face: Vec3, to_view: Vec3) -> Vec3 {
    let n = smooth.normalize_or_zero();
    let Some(mut f) = face.try_normalize() else {
        return n;
    };
    if f.dot(to_view) < 0. {
        f = -f;
    }
    if n.dot(f) < 0. {
        -n
    } else {
        n
    }
}

/// P2p: per-colour scales in the game's Pod shader constants (.w of suncol, ambcol and
/// both rim colours), fitted to the PC's RenderDoc capture of RPCS3 with pod_planets
/// preset 0 (multiplier 1.0206, exposure 1.2165): sun 0.372 = 0.3 x 1.0206 x 1.2165,
/// ambient 0.547 = 0.45 x 1.2165, rim 0.730 = 0.6 x 1.2165 (each within 0.001).
/// CONFIRMED_FROM_LOCAL_DATA for that frame; the split into constant x exposure (x
/// multiplier) is INFERRED from one level, as is the shader applying .w to .rgb.
pub const GAME_SUN_SCALE: f32 = 0.3;
pub const GAME_AMBIENT_SCALE: f32 = 0.45;
pub const GAME_RIM_SCALE: f32 = 0.6;

/// Viewer-side toggle state.
#[derive(Default)]
pub struct LightingState {
    pub enabled: bool,
    /// P2l `--lbp-lighting`.
    pub lbp: bool,
    /// P2l: the Pod planets root rotation the LBP light was last turned by.
    pub turn: Quat,
    /// P2l: scene entities from this index on (the Pod shell layer and figure) are lit by
    /// their own level's settings, turned by the shell root.
    pub shell: Option<(usize, PreviewLight)>,
    /// P2l: the shell root rotation last applied.
    pub shell_turn: Quat,
    pub light: PreviewLight,
    /// Set when the toggle or scene changes; `apply` runs once per change.
    pub dirty: bool,
    /// Lighting was applied last time (so turning it off restores materials).
    pub applied: bool,
    pub lines: Vec<String>,
}

/// Summary lines (panel and QA log). `unlit` of `total` draw entities kept unlit.
pub fn summary(light: &PreviewLight, unlit: usize, total: usize) -> Vec<String> {
    let mut lines = vec![format!("Lighting: {}", light.label())];
    lines.extend(light.notes.iter().map(|n| format!("  {n}")));
    lines.push(format!(
        "  {unlit} of {total} draw entities unlit (untrusted generated normals or fixture)"
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> LevelSettings {
        LevelSettings {
            sun_position: [0., 1., 0.],
            sun_position_scale: 1000.,
            sun_color: [1., 0.5, 0.25, 1.],
            ambient_color: [0.5, 0.5, 0.5, 1.],
            sun_multiplier: 2.,
            exposure: 1.,
            fog_color: [0.1, 0.2, 0.3, 1.],
            fog_near: 100.,
            fog_far: 300.,
            rim_color: [0.; 4],
            rim_color2: [0.; 4],
            presets: 0,
            backdrop_mesh: None,
        }
    }

    #[test]
    fn lambert_factor_is_ambient_plus_clamped_key() {
        let l = PreviewLight::generic();
        assert!((l.factor(l.toward_light) - Vec3::ONE).abs().max_element() < 1e-6);
        assert_eq!(l.factor(-l.toward_light), Vec3::splat(0.35));
        assert!(l.toward_light.is_normalized());
    }

    #[test]
    fn back_faces_and_mirrored_winding_light_like_front_faces() {
        let to_view = Vec3::Z;
        // Front face, smooth normal toward the viewer: kept.
        assert_eq!(facing_normal(Vec3::Z, Vec3::Z, to_view), Vec3::Z);
        // Back face seen from behind (face normal away from the viewer): flipped.
        assert_eq!(facing_normal(-Vec3::Z, -Vec3::Z, to_view), Vec3::Z);
        // Mirrored instance: winding reversed, face normal sign arbitrary; still faces view.
        assert_eq!(facing_normal(-Vec3::Z, Vec3::Z, to_view), Vec3::Z);
        // Grazing smooth normal on a front face keeps its tangential part.
        let n = facing_normal(Vec3::new(1., 0., 0.1), Vec3::Z, to_view);
        assert!(n.z > 0. && n.x > 0.9);
        // No face normal: smooth normal unchanged.
        assert_eq!(facing_normal(Vec3::X, Vec3::ZERO, to_view), Vec3::X);
    }

    #[test]
    fn level_settings_map_sun_ambient_fog_and_fit_to_unit_peak() {
        let l = PreviewLight::from_settings(&settings(), Vec3::ZERO, 0.01);
        assert!(l.from_level);
        assert_eq!(l.label(), LEVEL_LABEL);
        assert!((l.toward_light - Vec3::Y).length() < 1e-6);
        // key (2, 1, 0.5) + ambient 0.5: red peak 2.5 fitted to 1.
        assert!((l.key - Vec3::new(0.8, 0.4, 0.2)).length() < 1e-6);
        assert!((l.ambient - Vec3::splat(0.2)).length() < 1e-6);
        assert!(l.factor(Vec3::Y).max_element() <= 1. + 1e-6);
        let fog = l.fog.unwrap();
        assert!((fog.start - 1.).abs() < 1e-6 && (fog.end - 3.).abs() < 1e-6);
        // Sun direction is taken from the scene centre.
        let l = PreviewLight::from_settings(&settings(), Vec3::new(1000., 0., 0.), 0.01);
        assert!((l.toward_light - Vec3::new(-1., 1., 0.).normalize()).length() < 1e-6);
    }

    #[test]
    fn unusable_settings_fall_back_to_generic_and_say_why() {
        let mut s = settings();
        s.sun_position = [0.; 3];
        let l = PreviewLight::from_settings(&s, Vec3::ZERO, 1.);
        assert!(!l.from_level);
        assert_eq!(l.toward_light, PreviewLight::generic().toward_light);
        assert!(l.notes.last().unwrap().contains("sun position unusable"));
        let mut s = settings();
        s.ambient_color[1] = f32::NAN;
        assert!(!PreviewLight::from_settings(&s, Vec3::ZERO, 1.).from_level);
        // Bad fog range keeps the light but drops fog.
        let mut s = settings();
        s.fog_far = s.fog_near;
        let l = PreviewLight::from_settings(&s, Vec3::ZERO, 1.);
        assert!(l.from_level && l.fog.is_none());
    }

    #[test]
    fn p2l_lbp_shape_uses_raw_values_hemisphere_and_grazing_rim() {
        let mut s = settings();
        s.rim_color = [0.2, 0.1, 0.0, 1.];
        s.rim_color2 = [0.0, 0.1, 0.4, 1.];
        s.exposure = 2.;
        let light = PreviewLight::from_settings(&s, Vec3::ZERO, 1.).lbp_shaped(Quat::IDENTITY);
        assert!(light.lbp);
        // Unfitted, game scales: sun colour x multiplier 2 x 0.3 x exposure 2.
        let key = Vec3::new(2., 1., 0.5) * 0.6;
        assert!((light.key - key).length() < 1e-5);
        // P2q: the scalars the game sends in .w (sun x multiplier, ambient, rim).
        assert!((light.scales - Vec3::new(1.2, 0.9, 1.2)).length() < 1e-5);
        // Ambient 0.5 x 0.45 x 2; rims x 0.6 x 2.
        let ambient = 0.45;
        let rim_high = Vec3::new(0.0, 0.1, 0.4) * 1.2;
        let up = Vec3::Y;
        // Facing the viewer straight on: no rim; sun overhead lights fully.
        let f = light.lbp_factor(up, up);
        assert!((f - (Vec3::splat(ambient * 0.65) + key)).length() < 1e-5);
        // Grazing view: rim (0.9^2) with the upper colour for an upward normal.
        let g = light.lbp_factor(up, Vec3::X);
        assert!((g - f - rim_high * 0.81).length() < 1e-5);
        // Downward normal: lower ambient (0.35) and no sun.
        let d = light.lbp_factor(-up, -up);
        assert!((d - Vec3::splat(ambient * 0.35)).length() < 1e-5);
        // The turn moves the light with the scene.
        let t = PreviewLight::from_settings(&s, Vec3::ZERO, 1.)
            .lbp_shaped(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        assert!((t.toward_light - Vec3::NEG_X).length() < 1e-5);
    }

    #[test]
    fn summary_labels_never_claim_game_lighting() {
        for light in [
            PreviewLight::generic(),
            PreviewLight::from_settings(&settings(), Vec3::ZERO, 1.),
        ] {
            let lines = summary(&light, 2, 5);
            assert!(lines[0].contains("not game"));
            assert!(lines.last().unwrap().contains("2 of 5"));
        }
    }
}
