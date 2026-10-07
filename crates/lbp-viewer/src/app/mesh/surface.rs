//! Authored neutral diagnostic surfaces from verified stored geometry, no format rules.
//! Average-depth painter ordering is approximate; no depth buffer or material fidelity.
//! Geometry provenance: docs/mesh-geometry.md; display contract: docs/mesh-surface.md.
use super::{project3, Geometry};
use eframe::egui::{self, Color32};
const MAX_TRIANGLES: usize = 50_000;
const MAX_VERTICES: usize = 1_000_000;
pub(super) struct Face {
    indices: [u32; 3],
    depth: f64,
    tone: u8,
    original: usize,
}
pub(super) struct Scene {
    points: Vec<[f64; 3]>,
    faces: Vec<Face>,
    pub(super) skipped: usize,
}
impl Scene {
    pub(super) fn build(
        g: &Geometry,
        bounds: [[f32; 3]; 2],
        yaw: f64,
        pitch: f64,
    ) -> Result<Self, String> {
        if g.triangles.len() > MAX_TRIANGLES || g.positions.len() > MAX_VERTICES {
            return Err("Solid preview geometry budget exceeded".into());
        }
        if !yaw.is_finite()
            || !pitch.is_finite()
            || bounds.iter().flatten().any(|v| !v.is_finite())
            || (0..3).any(|i| bounds[0][i] > bounds[1][i])
        {
            return Err("Solid preview has invalid camera or bounds".into());
        }
        let mut points = Vec::with_capacity(g.positions.len());
        for &position in &g.positions {
            let p = project3(position, bounds, yaw, pitch);
            if p.iter().any(|v| !v.is_finite()) {
                return Err("Solid preview has nonfinite position".into());
            }
            points.push(p);
        }
        let mut faces = Vec::with_capacity(g.triangles.len());
        let mut skipped = 0;
        for (original, &indices) in g.triangles.iter().enumerate() {
            let triangle = indices.map(|i| points.get(i as usize).copied());
            let [Some(a), Some(b), Some(c)] = triangle else {
                return Err("Solid preview vertex index outside geometry".into());
            };
            let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
            let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let length = n[0].hypot(n[1]).hypot(n[2]);
            if !length.is_finite() {
                return Err("Solid preview normal overflow".into());
            }
            if length == 0. {
                skipped += 1;
                continue;
            }
            // Camera-space fixed light, two-sided; never consumes original normals.
            let light = ((n[0] / length) * 0.3
                + (n[1] / length) * 0.4
                + (n[2] / length) * 0.8660254037844386)
                .abs()
                .clamp(0., 1.);
            let tone = (55. + 155. * light).round() as u8;
            faces.push(Face {
                indices,
                depth: (a[2] + b[2] + c[2]) / 3.,
                tone,
                original,
            });
        }
        faces.sort_unstable_by(|a, b| {
            a.depth
                .total_cmp(&b.depth)
                .then_with(|| a.original.cmp(&b.original))
        });
        Ok(Self {
            points,
            faces,
            skipped,
        })
    }
    pub(super) fn face_count(&self) -> usize {
        self.faces.len()
    }
    pub(super) fn color_mesh(
        &self,
        center: egui::Pos2,
        scale: f64,
        color: Color32,
    ) -> Result<egui::Mesh, String> {
        let mut mesh = self.mesh(center, scale)?;
        for vertex in &mut mesh.vertices {
            vertex.color = color;
        }
        Ok(mesh)
    }
    pub(super) fn textured_mesh(
        &self,
        center: egui::Pos2,
        scale: f64,
        uv: &[[f32; 2]],
        texture: egui::TextureId,
    ) -> Result<egui::Mesh, String> {
        if uv.len() != self.points.len() || uv.iter().flatten().any(|v| !v.is_finite()) {
            return Err("Invalid texture UV extent or values".into());
        }
        let mut mesh = self.mesh(center, scale)?;
        mesh.texture_id = texture;
        for (face, vertices) in self.faces.iter().zip(mesh.vertices.as_chunks_mut::<3>().0) {
            for (index, vertex) in face.indices.iter().zip(vertices) {
                let coordinate = uv[*index as usize];
                vertex.uv = egui::pos2(coordinate[0], coordinate[1]);
                vertex.color = Color32::WHITE;
            }
        }
        Ok(mesh)
    }
    pub(super) fn mesh(&self, center: egui::Pos2, scale: f64) -> Result<egui::Mesh, String> {
        if !scale.is_finite() || scale <= 0. || !center.x.is_finite() || !center.y.is_finite() {
            return Err("Solid preview invalid screen transform".into());
        }
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(self.faces.len() * 3);
        mesh.indices.reserve(self.faces.len() * 3);
        for face in &self.faces {
            let base = mesh.vertices.len() as u32; // <=150000 by build budget.
            for i in face.indices {
                let point = self.points[i as usize];
                let screen = egui::pos2(
                    center.x + (point[0] * scale) as f32,
                    center.y - (point[1] * scale) as f32,
                );
                if !screen.x.is_finite() || !screen.y.is_finite() {
                    return Err("Solid preview screen coordinate overflow".into());
                }
                mesh.colored_vertex(screen, Color32::from_gray(face.tone));
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textured_faces_keep_vertex_uv_ownership_and_do_not_apply_neutral_shading() {
        let view = super::super::tests::cached();
        let geometry = view.geometry.as_ref().unwrap();
        let scene = Scene::build(
            geometry,
            geometry.metadata.prefix.position_bounds.unwrap(),
            0.,
            0.,
        )
        .unwrap();
        let uv = [[0., 0.], [2., 0.], [0., 1.]];
        let texture = egui::TextureId::Managed(17);
        let mesh = scene
            .textured_mesh(egui::pos2(0., 0.), 10., &uv, texture)
            .unwrap();
        assert_eq!(mesh.texture_id, texture);
        for (face, vertices) in scene.faces.iter().zip(mesh.vertices.as_chunks::<3>().0) {
            for (index, vertex) in face.indices.iter().zip(vertices) {
                assert_eq!(
                    vertex.uv,
                    egui::pos2(uv[*index as usize][0], uv[*index as usize][1])
                );
                assert_eq!(vertex.color, Color32::WHITE);
            }
        }
        assert!(scene
            .textured_mesh(egui::pos2(0., 0.), 10., &uv[..2], texture)
            .is_err());
        assert!(scene
            .textured_mesh(egui::pos2(0., 0.), 10., &[[f32::NAN; 2]; 3], texture)
            .is_err());
        let solid = scene
            .color_mesh(egui::pos2(0., 0.), 10., Color32::RED)
            .unwrap();
        assert_eq!(solid.vertices.len(), mesh.vertices.len());
        assert!(solid.vertices.iter().all(|v| v.color == Color32::RED));
        assert_eq!(solid.texture_id, egui::TextureId::default());
    }
    #[test]
    fn depth_ties_two_sided_shading_and_degenerates_have_deterministic_output() {
        let mut view = super::super::tests::cached();
        let g = view.geometry.as_mut().unwrap();
        g.positions = vec![
            [-1., -1., -1.],
            [1., -1., -1.],
            [0., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [0., 1., 1.],
        ];
        g.triangles = vec![[3, 4, 5], [0, 1, 2], [2, 1, 0], [0, 0, 1]];
        g.positions.push([0., -1., -1.]);
        g.triangles.push([0, 1, 6]); // Distinct-index collinear face.
        let scene = Scene::build(g, [[-1.; 3], [1.; 3]], 0., 0.).unwrap();
        assert_eq!(
            scene.faces.iter().map(|f| f.original).collect::<Vec<_>>(),
            vec![1, 2, 0]
        );
        assert_eq!(scene.skipped, 2);
        assert_eq!(scene.faces[0].tone, scene.faces[1].tone);
        let mesh = scene.mesh(egui::pos2(100., 100.), 80.).unwrap();
        assert!(mesh.is_valid());
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (9, 9));
        assert!(mesh
            .vertices
            .iter()
            .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite()));
        assert_eq!(
            mesh.indices,
            scene.mesh(egui::pos2(100., 100.), 80.).unwrap().indices
        );
    }
    #[test]
    fn extreme_bounds_invalid_indices_nonfinite_camera_and_budgets_are_explicit() {
        let mut view = super::super::tests::cached();
        let g = view.geometry.as_mut().unwrap();
        g.positions = vec![
            [-f32::MAX, -f32::MAX, 0.],
            [f32::MAX, -f32::MAX, 0.],
            [0., f32::MAX, 0.],
        ];
        let bounds = [[-f32::MAX; 3], [f32::MAX; 3]];
        let scene = Scene::build(g, bounds, 0.5, 0.3).unwrap();
        assert!(scene.mesh(egui::pos2(0., 0.), 500.).unwrap().is_valid());
        assert!(scene.mesh(egui::pos2(0., 0.), f64::MAX).is_err());
        assert!(Scene::build(g, bounds, f64::NAN, 0.).is_err());
        assert!(Scene::build(g, [[1.; 3], [-1.; 3]], 0., 0.).is_err());
        g.triangles[0] = [u32::MAX, 1, 2];
        assert!(Scene::build(g, bounds, 0., 0.).is_err());
        g.triangles = vec![[0, 1, 2]; MAX_TRIANGLES + 1];
        assert!(Scene::build(g, bounds, 0., 0.).is_err());
        g.triangles.clear();
        g.positions[0][0] = f32::NAN;
        assert!(Scene::build(g, bounds, 0., 0.).is_err());
        g.positions.clear();
        let scene = Scene::build(g, [[0.; 3]; 2], 0., 0.).unwrap();
        assert_eq!(scene.face_count(), 0);
        assert!(scene.mesh(egui::pos2(0., 0.), 1.).unwrap().is_valid());
    }
}
