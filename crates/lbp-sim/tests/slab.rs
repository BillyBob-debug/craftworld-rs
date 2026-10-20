//! `ZSlab::overlaps`: touching layers stay apart, overlapping ones collide (D1153).
use lbp_sim::{ZSlab, SLAB_TOUCH_EPSILON};

fn slab(z: f32, half_thickness: f32) -> ZSlab {
    ZSlab { z, half_thickness }
}

#[test]
fn rounded_touching_slabs_do_not_collide() {
    // Rookie Test: the lift (z 0, half 90) against the back slab stored at z 99.99994, half 10.
    assert!(!slab(0., 90.).overlaps(&slab(99.99994, 10.)));
    assert!(!slab(99.99994, 10.).overlaps(&slab(0., 90.)));
    assert!(!slab(0., 90.).overlaps(&slab(100., 10.)));
}

#[test]
fn overlapping_slabs_collide() {
    assert!(slab(0., 90.).overlaps(&slab(0., 10.)));
    assert!(slab(0., 90.).overlaps(&slab(99.9 - SLAB_TOUCH_EPSILON, 10.)));
    assert!(slab(-0.00012207031, 90.).overlaps(&slab(0., 90.)));
}

#[test]
fn separate_slabs_do_not_collide() {
    assert!(!slab(0., 10.).overlaps(&slab(50., 10.)));
}
