//! Static checks over real scripts that settle VM rules (PC census, docs/script-vm.md).
use lbp_level::script::{class, Class, Script};

/// Branch targets that land inside their own function under each candidate rule. The
/// rule with every target in range (and the other with misses) is the measured answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BranchCheck {
    pub branches: usize,
    pub in_range_from_current: usize,
    pub in_range_from_next: usize,
}

/// Opcode use and call shape, for choosing which natives and opcodes to verify first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptCheck {
    pub branch: BranchCheck,
    /// Instruction count per opcode.
    pub opcodes: Vec<(u8, usize)>,
    /// Instructions whose opcode has no class.
    pub unknown_opcodes: usize,
    /// Functions with code that end without RETURN or B.
    pub functions_without_terminal: usize,
}

pub fn check(script: &Script) -> ScriptCheck {
    let mut c = ScriptCheck::default();
    let mut counts = [0usize; 256];
    for f in &script.function_defs {
        let (b, e) = (f.bytecode.begin as usize, f.bytecode.end as usize);
        let Some(code) = script.bytecode.get(b..e) else {
            continue;
        };
        if let Some(&last) = code.last() {
            if !matches!((last & 0xff) as u8, 0xa5 | 0xa6) {
                c.functions_without_terminal += 1;
            }
        }
        for (k, &bits) in code.iter().enumerate() {
            let op = (bits & 0xff) as u8;
            counts[usize::from(op)] += 1;
            match class(op) {
                None => c.unknown_opcodes += 1,
                Some(Class::Branch) => {
                    let pc = (b + k) as i64;
                    let off = i64::from((bits >> 32) as u32 as i32);
                    let inside = |t: i64| t >= b as i64 && t < e as i64;
                    c.branch.branches += 1;
                    c.branch.in_range_from_current += usize::from(inside(pc + off));
                    c.branch.in_range_from_next += usize::from(inside(pc + 1 + off));
                }
                Some(_) => {}
            }
        }
    }
    c.opcodes = counts
        .iter()
        .enumerate()
        .filter(|(_, &n)| n > 0)
        .map(|(op, &n)| (op as u8, n))
        .collect();
    c
}
