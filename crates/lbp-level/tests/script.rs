//! U10 RScript reader and listing: authored synthetic recipes only.
mod common;
use common::{rev, W};
use lbp_level::script::{class, operands, read_payload, Class, MNEMONICS};
use lbp_level::serial::{Limits, Stop};

fn none() -> bool {
    false
}

fn str8(w: &mut W, s: &str) {
    w.s32(s.len() as i32).raw(s.as_bytes());
}

/// Modifiers: fixed i16 from 0x3d9, else i32.
fn mods(w: &mut W, head: u32, bits: u16) {
    if head >= 0x3d9 {
        w.i16(bits as i16);
    } else {
        w.u32(u32::from(bits));
    }
}

fn op(code: u8, dest: u16, a: u16, b: u16) -> u64 {
    u64::from(code) | u64::from(dest) << 16 | u64::from(a) << 32 | u64::from(b) << 48
}

/// A small class `Lamp` with one native function `SetOn` and one script function `Tick`.
/// ASCII table: 0 "Lamp", 1 "SetOn", 2 "Tick", 3 "bool", 4 "on".
fn lamp(head: u32, flags: u8) -> Vec<u8> {
    let mut w = W::new(flags);
    if head <= 0x360 {
        w.res_none();
    }
    str8(&mut w, "Lamp");
    w.res_guid(0x1234); // superclass
    mods(&mut w, head, 1 << 5); // public
                                // type refs: bool
    w.u32(1).u8(1).u8(1).u8(0).u8(0).res_none().u32(3);
    w.u32(1).u32(0).u32(4); // field ref "on"
    w.u32(1).u32(0).u32(1); // function ref "SetOn"
    w.u32(1);
    mods(&mut w, head, 1 << 7);
    w.u32(0).u32(4); // field def private bool on
    w.u32(0); // property defs
              // function defs: SetOn native (no code), Tick code 0..4
    w.u32(2);
    for (name, modifiers, code) in [
        (1u32, (1 << 1) | (1 << 5), (0u16, 0u16)),
        (2, 1 << 5, (0, 4)),
    ] {
        mods(&mut w, head, modifiers);
        w.u32(0).u32(name);
        if head >= 0x3d9 {
            for v in [0u16, 0, code.0, code.1, 0, 0, 0, 0] {
                w.i16(v as i16);
            }
        } else {
            for v in [
                0u32,
                0,
                u32::from(code.0),
                u32::from(code.0 + code.1),
                0,
                0,
                0,
                0,
            ] {
                w.u32(v);
            }
        }
        w.u32(16); // stack size
    }
    w.u32(0); // shared arguments
    let code = [
        op(0x01, 4, 0, 0) | 1 << 63,    // LCb r4 = true
        op(0x95, 8, 0, 0) | 0x01 << 8,  // GET_SP_MEMBER r8 = r0.field#0
        op(0xa4, 12, 0, 0) | 0x01 << 8, // CALL funcref#0
        u64::from(0xa5u8),              // RETURN void
    ];
    w.u32(code.len() as u32);
    for c in code {
        w.u64(c);
    }
    w.u32(2).i16(10).i16(11); // line numbers
    w.u32(0); // locals
              // ASCII table
    let names = ["Lamp", "SetOn", "Tick", "bool", "on"];
    let mut starts = Vec::new();
    let mut table = Vec::new();
    for n in names {
        starts.push(table.len() as u32);
        table.extend_from_slice(n.as_bytes());
        table.push(0);
    }
    w.int_vector(&starts);
    w.u32(table.len() as u32).raw(&table);
    // Wide table: "Hi"
    w.int_vector(&[0]);
    w.u32(3).raw(&[0, b'H', 0, b'i', 0, 0]);
    if head >= 0x30c {
        // longvector: one value 0x0102
        if flags & 2 != 0 {
            w.u32(1).u8(2).u8(0x02).u8(0x01);
        } else {
            w.u32(1).u64(0x0102);
        }
    }
    w.u32(1).f32(0.5); // f32 constants
    w.u32(1).u32(0xabcd); // depending GUIDs
    w.b
}

#[test]
fn u10_reads_both_function_layouts() {
    for (head, flags) in [(0x3f8u32, 7u8), (0x3f8, 0), (0x3d8, 7), (0x360, 0)] {
        let report = read_payload(
            &lamp(head, flags),
            rev(head, flags),
            Limits::default(),
            &none,
        );
        let s = report
            .result
            .unwrap_or_else(|e| panic!("head {head:x} flags {flags}: {e:?}"));
        assert_eq!(report.trailing_bytes, 0, "head {head:x}");
        assert_eq!(s.up_to_date_script.is_some(), head <= 0x360);
        assert_eq!(s.class_name, "Lamp");
        assert_eq!(s.super_class.unwrap().guid, Some(0x1234));
        assert_eq!(s.strings_a, ["Lamp", "SetOn", "Tick", "bool", "on"]);
        assert_eq!(s.strings_w, ["Hi"]);
        assert_eq!(s.function_defs[1].bytecode.begin, 0);
        assert_eq!(s.function_defs[1].bytecode.end, 4);
        assert_eq!(s.bytecode.len(), 4);
        assert_eq!(s.line_numbers, [10, 11]);
        assert_eq!(s.constants_s64.as_deref(), Some(&[0x0102u64][..]));
        assert_eq!(s.constants_f32, [0.5]);
        assert_eq!(s.depending_guids.as_deref(), Some(&[0xabcd][..]));
        let (text, checks) = s.listing();
        assert_eq!(checks, Default::default(), "{text}");
        assert!(text.contains("class Lamp : g00001234 [public]"), "{text}");
        assert!(
            text.contains("function SetOn: returns type#0 [native public]"),
            "{text}"
        );
        assert!(text.contains("CALL"), "{text}");
        assert!(text.contains("; SetOn"), "{text}");
        assert!(text.contains("; on"), "{text}");
        // Deterministic.
        assert_eq!(s.listing().0, text);
    }
}

#[test]
fn u10_every_opcode_has_a_class_and_mnemonic() {
    for op in 0u8..=0xca {
        assert!(class(op).is_some(), "0x{op:02x}");
        assert!(!MNEMONICS[op as usize].is_empty());
    }
    assert_eq!(class(0xcb), None);
    assert_eq!(class(0xff), None);
    assert_eq!(class(0xa6), Some(Class::Branch));
    assert_eq!(MNEMONICS[0xa4], "CALL");
    assert_eq!(MNEMONICS[0xca], "BIT_XORs64");
    // Branch offset is a signed 32-bit field.
    let b = 0xa7u64 | 3 << 16 | u64::from((-5i32) as u32) << 32;
    assert_eq!(operands(b), vec![("src", 3), ("offset", -5)]);
    // Binary: dest bits 16..32, a 32..48, b 48..64.
    assert_eq!(
        operands(op(0x2a, 1, 2, 3)),
        vec![("dest", 1), ("a", 2), ("b", 3)]
    );
}

#[test]
fn u10_untrusted_input_stops_explicitly() {
    let lim = Limits::default();
    let good = lamp(0x3f8, 0);
    // Every truncation stops with Invalid, never a panic.
    for cut in 0..good.len() {
        let r = read_payload(&good[..cut], rev(0x3f8, 0), lim, &none);
        assert!(
            matches!(r.result, Err(Stop::Invalid { .. })),
            "cut {cut}: {:?}",
            r.result
        );
    }
    // Inline format, LBP3 and branch heads are Unsupported.
    let mut branch = rev(0x3f8, 0);
    branch.branch_id = 0x4c44;
    for rv in [rev(0x1eb, 0), rev(0x0021_03f8, 0), branch] {
        let r = read_payload(&good, rv, lim, &none);
        assert!(matches!(r.result, Err(Stop::Unsupported { .. })), "{rv:?}");
    }
    // Unknown opcode and out-of-range indices are counted by the listing, not fatal.
    let mut s = read_payload(&good, rev(0x3f8, 0), lim, &none)
        .result
        .unwrap();
    s.bytecode[0] = 0xee;
    s.bytecode[2] = op(0xa4, 0, 9, 0);
    s.function_defs[0].bytecode.end = 99;
    s.function_defs[0].name = 77;
    let (_, c) = s.listing();
    assert_eq!(c.unknown_opcodes, 1);
    assert_eq!(c.member_index_out_of_range, 1);
    assert_eq!(c.function_range_out_of_range, 1);
    assert_eq!(c.name_index_out_of_range, 1);
    assert_eq!(c.name_out_of_range_by_kind.get("function"), Some(&1));
    assert!(c.name_out_of_range_values.contains(&77));
}
