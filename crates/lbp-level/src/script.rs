//! U10: bounded reader and disassembly listing for compiled Fish scripts (SCRIPT, `FSHb`,
//! `.ff`). Data only: nothing here executes bytecode, and native function semantics are
//! not known (they live in the game executable).
//!
//! Independent specification notes (no code copied/translated), written from
//! ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib:
//! `resources/RScript.java`, `structs/script/*Row.java`, `structs/script/TypeOffset.java`,
//! `structs/script/instructions/*.java`, `enums/{InstructionType,InstructionClass,
//! MachineType,BuiltinType,ModifierType}.java`, `io/serializer/Serializer.java`
//! (`longvector`, `intvector`, `resource`). CONFIRMED_FROM_COMMUNITY_CODE for the layout.
//! Rules as implemented (head = `Revision::version`):
//! - head <= 0x1eb is the inline script format: Unsupported (Toolkit refuses it too).
//! - head <= 0x360: an `upToDateScript` resource (with flags word) first. Toolkit
//!   `Revisions` names 0x33a for this field; the code gate is used here (PC check).
//! - class name `str`, superclass resource (flags word), class modifiers.
//! - Modifiers: fixed BE i16 from head 0x3d9, else i32 (compressed per flag 1); bit i =
//!   ModifierType i (0 STATIC .. 13 EXPORT).
//! - Rows (each an i32 count then inline): type references {u8 machine type, u8 builtin
//!   type, i8 dimension count, u8 array base machine type, script resource, i32 type name
//!   index}; field and function references {i32 type ref, i32 name}; field definitions
//!   {modifiers, i32 type ref, i32 name}; property definitions {.., i32 get fn, i32 set fn};
//!   function definitions {modifiers, i32 type ref, i32 name, 4 ranges (arguments,
//!   bytecode, line numbers, locals), i32 stack size}. From 0x3d9 a range is fixed u16
//!   begin + fixed u16 length; before, i32 begin + i32 end. Shared arguments
//!   {i32 type ref, i32 offset}.
//! - Bytecode: i32 count, then one u64 per instruction (ULEB when flag 1). Opcode = low
//!   byte (0x00..=0xca); operand fields per instruction class are in [`operands`].
//! - Line numbers: i32 count + fixed i16 each. Local variables {modifiers, i32 type ref,
//!   i32 name, i32 offset}.
//! - ASCII strings: intvector of start offsets, then i32 byte size (Toolkit `bytearray`,
//!   not zigzag) + bytes; string i runs
//!   to the next start (or the end), NULs removed. Wide strings: intvector of starts in
//!   units, i32 unit count + UTF-16BE units, same rule.
//! - head >= 0x30c: s64 constants as a `longvector` (flag 2: i32 count, u8 byte width,
//!   then width planes of count bytes, little-endian per element; else i32 count + u64
//!   each). Toolkit `Revisions` names 0x30b; code gate used (PC check).
//! - f32 constants: i32 count + fixed f32 each. head >= 0x1ec: depending GUIDs, i32 count +
//!   u32 each (compressed per flag 1).
//!
//! STRONG_INFERENCE (not in Toolkit, which keeps raw indices): name and type-name
//! indices point into the ASCII string table; `LCsa`/`LCsw` index the ASCII/wide tables;
//! `LCs64` indexes the s64 constants; GET/SET member `field` indexes field references and
//! CALL `call` indexes function references. The census counts indices out of range to
//! test this.
use crate::serial::{Descriptor, Limits, Reader, Result, Revision, Stop, FLAG_VECTORS};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub const MAX_INPUT: usize = 64 << 20;
const MAX_EXPANDED: usize = 256 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TypeRef {
    pub machine_type: u8,
    pub builtin_type: u8,
    pub dimension_count: i8,
    pub array_base_machine_type: u8,
    pub script: Option<Descriptor>,
    pub type_name: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MemberRef {
    pub type_ref: u32,
    pub name: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FieldDef {
    pub modifiers: u32,
    pub type_ref: u32,
    pub name: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PropertyDef {
    pub modifiers: u32,
    pub type_ref: u32,
    pub name: u32,
    pub get_function: i32,
    pub set_function: i32,
}

/// Half-open index range into one of the shared tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub begin: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FunctionDef {
    pub modifiers: u32,
    pub type_ref: u32,
    pub name: u32,
    pub arguments: Span,
    pub bytecode: Span,
    pub line_numbers: Span,
    pub locals: Span,
    pub stack_size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TypeOffset {
    pub type_ref: u32,
    pub offset: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LocalVariable {
    pub modifiers: u32,
    pub type_ref: u32,
    pub name: u32,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Script {
    /// Present (possibly null) only at head <= 0x360.
    pub up_to_date_script: Option<Option<Descriptor>>,
    pub class_name: String,
    pub super_class: Option<Descriptor>,
    pub modifiers: u32,
    pub type_refs: Vec<TypeRef>,
    pub field_refs: Vec<MemberRef>,
    pub function_refs: Vec<MemberRef>,
    pub field_defs: Vec<FieldDef>,
    pub property_defs: Vec<PropertyDef>,
    pub function_defs: Vec<FunctionDef>,
    pub shared_arguments: Vec<TypeOffset>,
    pub bytecode: Vec<u64>,
    pub line_numbers: Vec<i16>,
    pub locals: Vec<LocalVariable>,
    pub strings_a: Vec<String>,
    pub strings_w: Vec<String>,
    pub constants_s64: Option<Vec<u64>>,
    pub constants_f32: Vec<f32>,
    pub depending_guids: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScriptReport {
    pub revision: Revision,
    pub result: std::result::Result<Script, Stop>,
    pub trailing_bytes: usize,
}

/// Reads a whole `FSHb` resource file.
pub fn read(
    bytes: &[u8],
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> std::result::Result<ScriptReport, lbp_resource::binary::Error> {
    let (h, data) =
        lbp_resource::binary::payload(bytes, b"FSHb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    Ok(read_payload(&data, rev, limits, cancel))
}

/// Reads an expanded RScript payload at a known revision.
pub fn read_payload(
    data: &[u8],
    rev: Revision,
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> ScriptReport {
    let mut r = Reader::new(data, rev, limits, cancel);
    let result = script(&mut r);
    let trailing_bytes = if result.is_ok() { r.remaining() } else { 0 };
    ScriptReport {
        revision: rev,
        result,
        trailing_bytes,
    }
}

fn script(r: &mut Reader) -> Result<Script> {
    let rev = r.rev;
    if rev.branch_id != 0 || rev.head >> 16 != 0 || rev.head > crate::LBP2_MAX {
        return r.unsupported(format!(
            "script branch 0x{:x} / head 0x{:x}",
            rev.branch_id, rev.head
        ));
    }
    let v = rev.version();
    if v <= 0x1eb {
        return r.unsupported("inline script format (head <= 0x1eb)");
    }
    let up_to_date_script = if v <= 0x360 {
        Some(r.resource(false)?)
    } else {
        None
    };
    let class_name = str8(r)?;
    let super_class = r.resource(false)?;
    let class_modifiers = modifiers(r)?;
    let type_refs = rows(r, 9, |r| {
        Ok(TypeRef {
            machine_type: r.u8()?,
            builtin_type: r.u8()?,
            dimension_count: r.i8()?,
            array_base_machine_type: r.u8()?,
            script: r.resource(false)?,
            type_name: r.u32()?,
        })
    })?;
    let member = |r: &mut Reader| {
        Ok(MemberRef {
            type_ref: r.u32()?,
            name: r.u32()?,
        })
    };
    let field_refs = rows(r, 2, member)?;
    let function_refs = rows(r, 2, member)?;
    let field_defs = rows(r, 3, |r| {
        Ok(FieldDef {
            modifiers: modifiers(r)?,
            type_ref: r.u32()?,
            name: r.u32()?,
        })
    })?;
    let property_defs = rows(r, 5, |r| {
        Ok(PropertyDef {
            modifiers: modifiers(r)?,
            type_ref: r.u32()?,
            name: r.u32()?,
            get_function: r.i32()?,
            set_function: r.i32()?,
        })
    })?;
    let function_defs = rows(r, 12, |r| {
        let modifiers = modifiers(r)?;
        let type_ref = r.u32()?;
        let name = r.u32()?;
        let mut spans = [Span { begin: 0, end: 0 }; 4];
        if r.rev.version() >= 0x3d9 {
            let mut raw = [0u32; 8];
            for x in &mut raw {
                *x = u32::from(r.u16()?);
            }
            for (i, s) in spans.iter_mut().enumerate() {
                // Stored as begin + length; both u16, so the sum cannot overflow u32.
                *s = Span {
                    begin: raw[2 * i],
                    end: raw[2 * i] + raw[2 * i + 1],
                };
            }
        } else {
            for s in &mut spans {
                *s = Span {
                    begin: r.u32()?,
                    end: r.u32()?,
                };
            }
        }
        Ok(FunctionDef {
            modifiers,
            type_ref,
            name,
            arguments: spans[0],
            bytecode: spans[1],
            line_numbers: spans[2],
            locals: spans[3],
            stack_size: r.i32()?,
        })
    })?;
    let shared_arguments = rows(r, 2, |r| {
        Ok(TypeOffset {
            type_ref: r.u32()?,
            offset: r.i32()?,
        })
    })?;
    let n = r.count(1)?;
    let mut bytecode = Vec::with_capacity(n);
    for i in 0..n {
        if i % 4096 == 0 {
            r.check_cancel()?;
        }
        bytecode.push(r.u64()?);
    }
    let n = r.count(2)?;
    let mut line_numbers = Vec::with_capacity(n);
    for _ in 0..n {
        line_numbers.push(r.i16()?);
    }
    let locals = rows(r, 4, |r| {
        Ok(LocalVariable {
            modifiers: modifiers(r)?,
            type_ref: r.u32()?,
            name: r.u32()?,
            offset: r.i32()?,
        })
    })?;
    let starts = r.int_vector()?;
    let n = size(r)?;
    let table = r.bytes(n)?;
    let strings_a = split(r, &starts, table.len(), |a, b| {
        String::from_utf8_lossy(&table[a..b]).replace('\0', "")
    })?;
    let starts = r.int_vector()?;
    let units = size(r)?;
    let bytes = units.checked_mul(2).ok_or(Stop::Budget {
        what: "wide string table",
        offset: r.pos(),
    })?;
    let wide: Vec<u16> = r
        .bytes(bytes)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_be_bytes(*b))
        .collect();
    let strings_w = split(r, &starts, wide.len(), |a, b| {
        String::from_utf16_lossy(&wide[a..b]).replace('\0', "")
    })?;
    let constants_s64 = if r.rev.version() >= 0x30c {
        Some(long_vector(r)?)
    } else {
        None
    };
    let n = r.count(4)?;
    let mut constants_f32 = Vec::with_capacity(n);
    for _ in 0..n {
        constants_f32.push(r.f32()?);
    }
    let depending_guids = if r.rev.version() >= 0x1ec {
        let n = r.count(1)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.u32()?);
        }
        Some(v)
    } else {
        None
    };
    Ok(Script {
        up_to_date_script,
        class_name,
        super_class,
        modifiers: class_modifiers,
        type_refs,
        field_refs,
        function_refs,
        field_defs,
        property_defs,
        function_defs,
        shared_arguments,
        bytecode,
        line_numbers,
        locals,
        strings_a,
        strings_w,
        constants_s64,
        constants_f32,
        depending_guids,
    })
}

fn rows<T>(
    r: &mut Reader,
    min_size: usize,
    mut row: impl FnMut(&mut Reader) -> Result<T>,
) -> Result<Vec<T>> {
    let n = r.count(min_size)?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(row(r)?);
    }
    Ok(v)
}

fn modifiers(r: &mut Reader) -> Result<u32> {
    if r.rev.version() >= 0x3d9 {
        Ok(u32::from(r.u16()?))
    } else {
        r.u32()
    }
}

/// Toolkit `i32()` size (not zigzag), refused when negative.
fn size(r: &mut Reader) -> Result<usize> {
    let n = r.i32()?;
    usize::try_from(n).or_else(|_| r.invalid("negative size"))
}

/// Toolkit `str()`: s32 (zigzag when compressed) byte length.
fn str8(r: &mut Reader) -> Result<String> {
    let n = r.s32()?;
    let n = usize::try_from(n).or_else(|_| r.invalid("negative string length"))?;
    Ok(String::from_utf8_lossy(r.bytes(n)?).replace('\0', ""))
}

/// Splits a string table at `starts`; string i ends at start i+1 or the table end.
/// Starts past the end or decreasing are Invalid (Toolkit `copyOfRange` would throw).
fn split(
    r: &Reader,
    starts: &[u32],
    len: usize,
    text: impl Fn(usize, usize) -> String,
) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let a = start as usize;
        let b = starts.get(i + 1).map_or(len, |&e| e as usize);
        if a > b || b > len {
            return r.invalid("string table index out of range");
        }
        out.push(text(a, b));
    }
    Ok(out)
}

/// Toolkit `longvector`.
fn long_vector(r: &mut Reader) -> Result<Vec<u64>> {
    if r.rev.compression & FLAG_VECTORS == 0 {
        let n = r.count(1)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.u64()?);
        }
        return Ok(v);
    }
    let n = r.count(0)?;
    if n == 0 {
        return Ok(Vec::new());
    }
    let width = r.u8()? as usize;
    if width > 8 {
        return r.unsupported("longvector byte width above 8");
    }
    let total = match n.checked_mul(width) {
        Some(t) if t <= r.remaining() => t,
        _ => return r.invalid("longvector planes exceed remaining bytes"),
    };
    let planes = r.bytes(total)?;
    let mut v = vec![0u64; n];
    for (i, plane) in planes.chunks_exact(n).enumerate() {
        for (dst, b) in v.iter_mut().zip(plane) {
            *dst |= u64::from(*b) << (8 * i);
        }
    }
    Ok(v)
}

/// Toolkit `InstructionClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Class {
    Nop,
    LoadConst,
    Cast,
    Unary,
    Binary,
    GetBuiltinMember,
    SetBuiltinMember,
    GetMember,
    SetMember,
    GetElement,
    SetElement,
    NewObject,
    NewArray,
    Write,
    Arg,
    Call,
    Return,
    Branch,
}

/// Mnemonics for opcodes 0x00..=0xca (Toolkit `InstructionType` order).
pub const MNEMONICS: [&str; 203] = [
    "NOP",
    "LCb",
    "LCc",
    "LCi",
    "LCf",
    "LCsw",
    "LC_NULLsp",
    "MOVb",
    "LOG_NEGb",
    "MOVc",
    "MOVi",
    "INCi",
    "DECi",
    "NEGi",
    "BIT_NEGi",
    "LOG_NEGi",
    "ABSi",
    "MOVf",
    "NEGf",
    "ABSf",
    "SQRTf",
    "SINf",
    "COSf",
    "TANf",
    "MOVv4",
    "NEGv4",
    "MOVm44",
    "MOVs",
    "MOVrp",
    "MOVcp",
    "MOVsp",
    "MOVo",
    "EQb",
    "NEb",
    "RESERVED0",
    "RESERVED1",
    "LTc",
    "LTEc",
    "GTc",
    "GTEc",
    "EQc",
    "NEc",
    "ADDi",
    "SUBi",
    "MULi",
    "DIVi",
    "MODi",
    "MINi",
    "MAXi",
    "SLAi",
    "SRAi",
    "SRLi",
    "BIT_ORi",
    "BIT_ANDi",
    "BIT_XORi",
    "LTi",
    "LTEi",
    "GTi",
    "GTEi",
    "EQi",
    "NEi",
    "ADDf",
    "SUBf",
    "MULf",
    "DIVf",
    "MINf",
    "MAXf",
    "LTf",
    "LTEf",
    "GTf",
    "GTEf",
    "EQf",
    "NEf",
    "ADDv4",
    "SUBv4",
    "MULSv4",
    "DIVSv4",
    "DOT4v4",
    "DOT3v4",
    "DOT2v4",
    "CROSS3v4",
    "MULm44",
    "EQs",
    "NEs",
    "EQrp",
    "NErp",
    "EQo",
    "NEo",
    "EQsp",
    "NEsp",
    "GET_V4_X",
    "GET_V4_Y",
    "GET_V4_Z",
    "GET_V4_W",
    "GET_V4_LEN2",
    "GET_V4_LEN3",
    "GET_V4_LEN4",
    "GET_M44_XX",
    "GET_M44_XY",
    "GET_M44_XZ",
    "GET_M44_XW",
    "GET_M44_YX",
    "GET_M44_YY",
    "GET_M44_YZ",
    "GET_M44_YW",
    "GET_M44_ZX",
    "GET_M44_ZY",
    "GET_M44_ZZ",
    "GET_M44_ZW",
    "GET_M44_WX",
    "GET_M44_WY",
    "GET_M44_WZ",
    "GET_M44_WW",
    "GET_M44_rX",
    "GET_M44_rY",
    "GET_M44_rZ",
    "GET_M44_rW",
    "GET_M44_cX",
    "GET_M44_cY",
    "GET_M44_cZ",
    "GET_M44_cW",
    "SET_V4_X",
    "SET_V4_Y",
    "SET_V4_Z",
    "SET_V4_W",
    "SET_M44_XX",
    "SET_M44_XY",
    "SET_M44_XZ",
    "SET_M44_XW",
    "SET_M44_YX",
    "SET_M44_YY",
    "SET_M44_YZ",
    "SET_M44_YW",
    "SET_M44_ZX",
    "SET_M44_ZY",
    "SET_M44_ZZ",
    "SET_M44_ZW",
    "SET_M44_WX",
    "SET_M44_WY",
    "SET_M44_WZ",
    "SET_M44_WW",
    "SET_M44_rX",
    "SET_M44_rY",
    "SET_M44_rZ",
    "SET_M44_rW",
    "SET_M44_cX",
    "SET_M44_cY",
    "SET_M44_cZ",
    "SET_M44_cW",
    "GET_SP_MEMBER",
    "GET_RP_MEMBER",
    "SET_SP_MEMBER",
    "SET_RP_MEMBER",
    "GET_ELEMENT",
    "SET_ELEMENT",
    "GET_ARRAY_LEN",
    "NEW_ARRAY",
    "ARRAY_INSERT",
    "ARRAY_APPEND",
    "ARRAY_ERASE",
    "ARRAY_FIND",
    "ARRAY_CLEAR",
    "WRITE",
    "ARG",
    "CALL",
    "RETURN",
    "B",
    "BEZ",
    "BNEZ",
    "CASTsp",
    "INTb",
    "INTc",
    "INTf",
    "FLOATb",
    "FLOATc",
    "FLOATi",
    "BOOLc",
    "BOOLi",
    "BOOLf",
    "GET_OBJ_MEMBER",
    "SET_OBJ_MEMBER",
    "NEW_OBJECT",
    "ARRAY_RESIZE",
    "ARRAY_RESERVE",
    "LCv4",
    "LC_NULLo",
    "CASTo",
    "GET_SP_NATIVE_MEMBER",
    "LCsa",
    "BIT_ORb",
    "BIT_ANDb",
    "BIT_XORb",
    "CALLVo",
    "CALLVsp",
    "ASSERT",
    "LCs64",
    "MOVs64",
    "ADDs64",
    "EQs64",
    "NEs64",
    "BIT_ORs64",
    "BIT_ANDs64",
    "BIT_XORs64",
];

/// Instruction class of an opcode (Toolkit `InstructionType.INSTRUCTION_CLASSES`).
pub fn class(opcode: u8) -> Option<Class> {
    use Class::*;
    Some(match opcode {
        0x00 | 0x1b | 0x52 | 0x53 => Nop,
        0x01..=0x06 | 0xb8 | 0xb9 | 0xbc | 0xc3 => LoadConst,
        0x07..=0x1a | 0x1c..=0x1f | 0xaa..=0xb2 | 0xc4 => Unary,
        0x20..=0x51 | 0x54..=0x59 | 0xbd..=0xbf | 0xc5..=0xca => Binary,
        0x5a..=0x78 | 0x9b => GetBuiltinMember,
        0x79..=0x94 => SetBuiltinMember,
        0x95 | 0x96 | 0xb3 | 0xbb => GetMember,
        0x97 | 0x98 | 0xb4 => SetMember,
        0x99 | 0xa0 => GetElement,
        0x9a | 0x9d..=0x9f | 0xa1 | 0xb6 | 0xb7 => SetElement,
        0x9c => NewArray,
        0xa2 | 0xc2 => Write,
        0xa3 => Arg,
        0xa4 | 0xc0 | 0xc1 => Call,
        0xa5 => Return,
        0xa6..=0xa8 => Branch,
        0xa9 | 0xba => Cast,
        0xb5 => NewObject,
        _ => return None,
    })
}

/// Toolkit `MachineType` names (7 is unassigned).
pub fn machine_type_name(t: u8) -> Option<&'static str> {
    [
        "void",
        "bool",
        "char",
        "s32",
        "f32",
        "v4",
        "m44",
        "",
        "raw_ptr",
        "ref_ptr",
        "safe_ptr",
        "object_ref",
        "s64",
        "f64",
    ]
    .get(t as usize)
    .copied()
    .filter(|s| !s.is_empty())
}

/// Toolkit `ModifierType` names by bit.
pub const MODIFIERS: [&str; 14] = [
    "static",
    "native",
    "ephemeral",
    "pinned",
    "const",
    "public",
    "protected",
    "private",
    "property",
    "abstract",
    "virtual",
    "override",
    "divergent",
    "export",
];

pub fn modifier_names(bits: u32) -> Vec<&'static str> {
    MODIFIERS
        .iter()
        .enumerate()
        .filter(|(i, _)| bits & (1 << i) != 0)
        .map(|(_, n)| *n)
        .collect()
}

/// Operand fields of one instruction, by class (Toolkit `structs/script/instructions`).
/// Registers are 16-bit fields; `type` is the MachineType byte (bits 8..16).
pub fn operands(bits: u64) -> Vec<(&'static str, i64)> {
    let op = (bits & 0xff) as u8;
    let h16 = |s: u32| ((bits >> s) & 0xffff) as i64;
    let ty = ((bits >> 8) & 0xff) as i64;
    let hi32 = (bits >> 32) as u32;
    let Some(c) = class(op) else {
        return Vec::new();
    };
    match c {
        Class::Nop => vec![],
        Class::LoadConst => match op {
            0x01 => vec![("dest", h16(16)), ("value", (bits >> 63) as i64)],
            0x02 => vec![("dest", h16(16)), ("value", h16(48))],
            0x03 => vec![("dest", h16(16)), ("value", i64::from(hi32 as i32))],
            0x04 => vec![("dest", h16(16)), ("bits", i64::from(hi32))],
            0x06 | 0xb9 => vec![("dest", h16(16))],
            _ => vec![("dest", h16(16)), ("index", i64::from(hi32))],
        },
        Class::Unary => vec![("dest", h16(16)), ("src", h16(32))],
        Class::Binary => vec![("dest", h16(16)), ("a", h16(32)), ("b", h16(48))],
        Class::GetBuiltinMember => vec![("dest", h16(16)), ("base", h16(32))],
        Class::SetBuiltinMember => vec![("base", h16(32)), ("src", h16(16))],
        Class::GetMember => vec![
            ("dest", h16(16)),
            ("base", h16(32)),
            ("field", h16(48)),
            ("type", ty),
        ],
        Class::SetMember => vec![
            ("base", h16(32)),
            ("field", h16(48)),
            ("src", h16(16)),
            ("type", ty),
        ],
        Class::GetElement => vec![
            ("dest", h16(16)),
            ("base", h16(32)),
            ("src", h16(48)),
            ("type", ty),
        ],
        Class::SetElement => vec![
            ("base", h16(32)),
            ("index", h16(48)),
            ("src", h16(16)),
            ("type", ty),
        ],
        Class::NewObject => vec![("dest", h16(16)), ("type_ref", h16(48))],
        Class::NewArray => vec![("dest", h16(16)), ("type_ref", h16(48)), ("size", h16(32))],
        Class::Write => vec![("src", h16(32)), ("type", ty)],
        Class::Arg => vec![("arg", h16(32)), ("src", h16(16)), ("type", ty)],
        Class::Call => vec![("dest", h16(16)), ("call", h16(32)), ("type", ty)],
        Class::Return => vec![("src", h16(32)), ("type", ty)],
        Class::Branch => vec![("src", h16(16)), ("offset", i64::from(hi32 as i32))],
        Class::Cast => vec![("dest", h16(16)), ("src", h16(32)), ("type_ref", h16(48))],
    }
}

/// Counted problems found while listing (indices are inferred, see module notes).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ListingChecks {
    pub unknown_opcodes: usize,
    pub name_index_out_of_range: usize,
    /// Out-of-range name indices by row kind (type, fieldref, funcref, field, property,
    /// function, member) and the first distinct values seen (at most 16).
    pub name_out_of_range_by_kind: BTreeMap<&'static str, usize>,
    pub name_out_of_range_values: BTreeSet<u32>,
    pub function_range_out_of_range: usize,
    pub constant_index_out_of_range: usize,
    pub member_index_out_of_range: usize,
}

impl ListingChecks {
    fn name_miss(&mut self, kind: &'static str, index: u32) {
        self.name_index_out_of_range += 1;
        *self.name_out_of_range_by_kind.entry(kind).or_default() += 1;
        if self.name_out_of_range_values.len() < 16 {
            self.name_out_of_range_values.insert(index);
        }
    }
}

impl Script {
    /// ASCII string by index (inferred table for names).
    pub fn name(&self, index: u32) -> Option<&str> {
        self.strings_a.get(index as usize).map(String::as_str)
    }

    /// Deterministic text listing: header, tables, then each function's bytecode.
    pub fn listing(&self) -> (String, ListingChecks) {
        let mut c = ListingChecks::default();
        let mut o = String::new();
        let name = |kind: &'static str, i: u32, c: &mut ListingChecks| match self.name(i) {
            Some(s) => s.to_owned(),
            None => {
                c.name_miss(kind, i);
                format!("<name #{i}>")
            }
        };
        let desc = |d: &Option<Descriptor>| match d {
            None => "none".to_owned(),
            Some(d) => match (d.guid, d.sha1) {
                (Some(g), _) => format!("g{g:08x}"),
                (None, Some(h)) => format!("h{}", crate::serial::hex(&h)),
                _ => "?".to_owned(),
            },
        };
        let _ = writeln!(
            o,
            "class {} : {} [{}]",
            self.class_name,
            desc(&self.super_class),
            modifier_names(self.modifiers).join(" ")
        );
        for (i, t) in self.type_refs.iter().enumerate() {
            let n = name("type", t.type_name, &mut c);
            let _ = writeln!(
                o,
                "type #{i}: {n} machine={} builtin={} dims={} base={} script={}",
                machine_type_name(t.machine_type).unwrap_or("?"),
                t.builtin_type,
                t.dimension_count,
                machine_type_name(t.array_base_machine_type).unwrap_or("?"),
                desc(&t.script)
            );
        }
        for (i, f) in self.field_refs.iter().enumerate() {
            let n = name("fieldref", f.name, &mut c);
            let _ = writeln!(o, "fieldref #{i}: type#{} {n}", f.type_ref);
        }
        for (i, f) in self.function_refs.iter().enumerate() {
            let n = name("funcref", f.name, &mut c);
            let _ = writeln!(o, "funcref #{i}: type#{} {n}", f.type_ref);
        }
        for f in &self.field_defs {
            let n = name("field", f.name, &mut c);
            let _ = writeln!(
                o,
                "field {n}: type#{} [{}]",
                f.type_ref,
                modifier_names(f.modifiers).join(" ")
            );
        }
        for p in &self.property_defs {
            let n = name("property", p.name, &mut c);
            let _ = writeln!(
                o,
                "property {n}: type#{} get={} set={} [{}]",
                p.type_ref,
                p.get_function,
                p.set_function,
                modifier_names(p.modifiers).join(" ")
            );
        }
        for f in &self.function_defs {
            let n = name("function", f.name, &mut c);
            let _ = writeln!(
                o,
                "\nfunction {n}: returns type#{} [{}] stack={} args={}..{} code={}..{}",
                f.type_ref,
                modifier_names(f.modifiers).join(" "),
                f.stack_size,
                f.arguments.begin,
                f.arguments.end,
                f.bytecode.begin,
                f.bytecode.end
            );
            let (a, b) = (f.bytecode.begin as usize, f.bytecode.end as usize);
            if a > b || b > self.bytecode.len() {
                c.function_range_out_of_range += 1;
                let _ = writeln!(o, "  <bytecode range out of bounds>");
                continue;
            }
            for (pc, &bits) in self.bytecode[a..b].iter().enumerate() {
                let _ = writeln!(o, "  {pc:04}  {}", self.instruction(bits, &mut c));
            }
        }
        (o, c)
    }

    fn instruction(&self, bits: u64, c: &mut ListingChecks) -> String {
        let op = (bits & 0xff) as u8;
        let Some(m) = MNEMONICS.get(op as usize) else {
            c.unknown_opcodes += 1;
            return format!("?? 0x{op:02x} bits={bits:016x}");
        };
        let mut s = format!("{m:<12}");
        for (k, v) in operands(bits) {
            let _ = write!(s, " {k}={v}");
        }
        let fields = operands(bits);
        let get = |k: &str| fields.iter().find(|(n, _)| *n == k).map(|(_, v)| *v);
        let index = get("index").unwrap_or(0) as usize;
        // Resolved values (inferred tables).
        let extra = match op {
            0x04 => Some(format!("{}", f32::from_bits((bits >> 32) as u32))),
            0x05 => self.strings_w.get(index).map(|t| format!("{t:?}")),
            0xbc => self.strings_a.get(index).map(|t| format!("{t:?}")),
            0xc3 => self
                .constants_s64
                .as_ref()
                .and_then(|t| t.get(index))
                .map(|v| format!("{}", *v as i64)),
            _ => None,
        };
        if matches!(op, 0x05 | 0xbc | 0xc3) && extra.is_none() {
            c.constant_index_out_of_range += 1;
        }
        let member = match class(op) {
            Some(Class::GetMember | Class::SetMember) if op != 0xbb => {
                get("field").map(|i| self.field_refs.get(i as usize).map(|f| f.name))
            }
            Some(Class::Call) if op == 0xa4 => {
                get("call").map(|i| self.function_refs.get(i as usize).map(|f| f.name))
            }
            _ => None,
        };
        let extra = match member {
            Some(Some(n)) => Some(match self.name(n) {
                Some(t) => t.to_owned(),
                None => {
                    c.name_miss("member", n);
                    "<name?>".to_owned()
                }
            }),
            Some(None) => {
                c.member_index_out_of_range += 1;
                Some("<ref out of range>".to_owned())
            }
            None => extra,
        };
        if let Some(e) = extra {
            let _ = write!(s, "  ; {e}");
        }
        s
    }
}
