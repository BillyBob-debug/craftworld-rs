//! Authored signatures and filler only; never protected original data.
#[test]
fn signatures_apply_to_four_bytes_and_long_preflights_without_interior_matches() {
    for magic in [
        b"ANMe", b"GMTe", b"MSHe", b"TEXe", b"GTFe", b"NPD\0", b"SCE\0", b"\x7fPKG",
    ] {
        for n in 0..4 {
            assert!(!lbp_resource::is_protected_prefix(&magic[..n]));
        }
        assert!(lbp_resource::is_protected_prefix(magic));
        assert!(lbp_resource::is_protected_prefix(
            &[magic.as_slice(), &[0; 64]].concat()
        ));
        assert!(!lbp_resource::is_protected_prefix(
            &[b"text".as_slice(), magic].concat()
        ));
    }
    assert!(!lbp_resource::is_protected_prefix(b"ANMbsynthetic"));
}
