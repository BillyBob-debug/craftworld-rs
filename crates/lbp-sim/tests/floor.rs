//! `PhysicsScene::floor_below` on hand-built colliders (no game data).
use lbp_sim::scene::{Frame, PhysicsScene, SceneCollider};

fn slab(position: [f32; 2], angle: f32, z: f32, w: f32, h: f32) -> SceneCollider {
    let [a, b, c, d] = [[-w, -h], [w, -h], [w, h], [-w, h]];
    SceneCollider {
        thing: 0,
        body: None,
        frame: Frame { position, angle },
        triangles: vec![[a, b, c], [a, c, d]],
        z,
        half_thickness: 50.,
        density: 1.,
    }
}

#[test]
fn floor_below_finds_the_highest_top_under_the_point() {
    let scene = PhysicsScene {
        colliders: vec![
            // Ground: top at y 0.
            slab([0., -100.], 0., 0., 1000., 100.),
            // A step: top at y 200, x -100..100.
            slab([0., 150.], 0., 0., 100., 50.),
            // A ceiling above the point is ignored.
            slab([0., 900.], 0., 0., 1000., 50.),
            // Another layer is ignored.
            slab([0., 450.], 0., 200., 1000., 50.),
            // A 45-degree turned square centred at x 500: its top corner is at y 100 * sqrt 2.
            slab([500., 0.], std::f32::consts::FRAC_PI_4, 0., 100., 100.),
        ],
        ..Default::default()
    };
    assert_eq!(scene.floor_below([0., 500.], 0.), Some(200.));
    assert_eq!(scene.floor_below([300., 500.], 0.), Some(0.));
    assert_eq!(scene.floor_below([0., 100.], 0.), Some(0.));
    let corner = scene.floor_below([500., 500.], 0.).unwrap();
    assert!((corner - 100. * 2f32.sqrt()).abs() < 1e-3, "{corner}");
    assert_eq!(scene.floor_below([5000., 500.], 0.), None);
}
