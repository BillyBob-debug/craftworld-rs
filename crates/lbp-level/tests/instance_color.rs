//! B176 instance color census: authored in-memory Things only (no game bytes).
use lbp_level::instance_color::{ColorSummary, InstanceColors, DEFAULT_WHITE};
use lbp_level::parts::{GeneratedMesh, RenderMesh, Shape, ShapeColor};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;

fn guid(g: u32) -> Descriptor {
    Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(g),
        sha1: None,
    }
}

fn mesh_thing(mesh: Option<Descriptor>, color: u32) -> ThingRecord {
    let mut t = ThingRecord::default();
    t.data.render_mesh = Some(RenderMesh {
        mesh,
        bone_things: Vec::new(),
        anim: None,
        anim_pos: 0.0,
        anim_speed: 0.0,
        anim_loop: false,
        editor_color: color,
        cast_shadows: 0,
        visibility_flags: 0,
        poppet_render_scale: 1.0,
    });
    t
}

fn shape_thing(gfx: u32, physics: u32, color: ShapeColor) -> ThingRecord {
    let mut t = ThingRecord::default();
    t.data.shape = Some(Shape {
        vertex_count: 0,
        requires_z: false,
        vertices: Vec::new(),
        loops: Vec::new(),
        material: Some(guid(physics)),
        old_material: None,
        thickness: 0.0,
        mass_depth: 0.0,
        color,
        brightness: 0.0,
        bevel_size: 0.0,
        com: None,
        behavior: 0,
        flags: 0,
        contacts: 0,
    });
    t.data.generated_mesh = Some(GeneratedMesh {
        gfx_material: Some(guid(gfx)),
        bevel: None,
        uv_offset: [0.0; 4],
        plan_guid: None,
        play_mode_visible: true,
    });
    t
}

#[test]
fn histograms_count_things_and_files() {
    let mut c = InstanceColors::default();
    c.add_things(
        "level-a",
        &[
            mesh_thing(Some(guid(0x10)), DEFAULT_WHITE),
            mesh_thing(Some(guid(0x10)), 0xff20_4080),
            mesh_thing(Some(guid(0x11)), DEFAULT_WHITE),
            mesh_thing(None, DEFAULT_WHITE),
            shape_thing(0x20, 0x30, ShapeColor::Packed(0xff00_ff00)),
        ],
    );
    c.add_things(
        "level-b",
        &[
            mesh_thing(Some(guid(0x10)), DEFAULT_WHITE),
            mesh_thing(Some(guid(0x12)), 0x8000_0000),
            shape_thing(0x20, 0x31, ShapeColor::Vector([1.0, 0.5, 0.0, 1.0])),
        ],
    );
    let keys = c.meshes.keys();
    let m10 = &keys["guid:00000010"];
    assert_eq!((m10.things, m10.files, m10.distinct_colors), (3, 2, 2));
    assert_eq!(
        m10.colors["ffffffff"],
        ColorSummary {
            things: 2,
            files: 2
        }
    );
    assert_eq!(m10.colors["ff204080"].files, 1);
    let t = c.meshes.totals();
    assert_eq!(t.keys, 3);
    assert_eq!(t.things, 6);
    assert_eq!(t.things_without_key, 1);
    assert_eq!(t.things_default_white, 3);
    assert_eq!(t.distinct_colors, 3);
    assert_eq!(t.keys_single_color, 2);
    assert_eq!(t.keys_only_default_white, 1);
    assert_eq!(t.keys_multi_color, 1);

    let g = c.shapes_by_gfx_material.keys();
    assert_eq!(g.len(), 1);
    let g20 = &g["guid:00000020"];
    assert_eq!((g20.things, g20.files, g20.distinct_colors), (2, 2, 2));
    assert!(g20.colors.contains_key("ff00ff00"));
    assert!(g20.colors.contains_key("v4:1,0.5,0,1"));
    assert_eq!(c.shapes_by_physics_material.totals().keys, 2);
}
