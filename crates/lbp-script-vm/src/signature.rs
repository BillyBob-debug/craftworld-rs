//! Mangled function names, as call sites name natives: `Name__` followed by one code per
//! parameter, e.g. `IsSwitchTriggered__i`, `PlaySound__Q5ThingQ7StringA`.
//!
//! INFERRED (docs/script-natives.md) from the 01.33 native-call census names; Toolkit
//! d6a2ef80 reads these names but does not decode them. `Q<len><name>` is a class or enum
//! by name. The vector letters `p`/`q`/`r` follow the order of Toolkit's BuiltinType
//! V2/V3/V4 (all four floats natively); `[` marks an array. Unrecognised codes are kept as
//! [`Param::Unknown`] so the census can count them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Param {
    Bool,
    Char,
    Int,
    /// `j`: 64-bit integer (INFERRED from `ShiftL__ji`, `MakeAgeString__j`).
    Long,
    Float,
    V2,
    V3,
    V4,
    M44,
    Guid,
    /// `Q<len><name>`: script class, engine class or enum (`Gooey.EGooeyTextStyle`).
    Named(String),
    /// `[` before an element code: array of that element.
    Array(Box<Param>),
    Unknown(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature<'a> {
    pub name: &'a str,
    pub params: Vec<Param>,
}

/// Longest class name accepted in a `Q` code; deepest array nesting.
const MAX_NAME: usize = 256;
const MAX_ARRAY_DEPTH: usize = 8;

/// Splits a mangled name; `None` when there is no `__` or a code is malformed.
pub fn parse(mangled: &str) -> Option<Signature<'_>> {
    // Skip the first byte so a leading "__" is part of the name.
    let split = mangled.get(1..)?.find("__")? + 1;
    let name = &mangled[..split];
    let codes = &mangled[split + 2..];
    let mut params = Vec::new();
    let mut at = 0;
    while at < codes.len() {
        params.push(one(codes, &mut at, 0)?);
    }
    Some(Signature { name, params })
}

fn one(codes: &str, at: &mut usize, depth: usize) -> Option<Param> {
    let c = codes.get(*at..)?.chars().next()?;
    *at += c.len_utf8();
    Some(match c {
        'b' => Param::Bool,
        'c' => Param::Char,
        'i' => Param::Int,
        'j' => Param::Long,
        'f' => Param::Float,
        'p' => Param::V2,
        'q' => Param::V3,
        'r' => Param::V4,
        'm' => Param::M44,
        'g' => Param::Guid,
        '[' if depth < MAX_ARRAY_DEPTH => Param::Array(Box::new(one(codes, at, depth + 1)?)),
        'Q' => {
            let rest = &codes[*at..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let len: usize = rest.get(..digits)?.parse().ok()?;
            if len == 0 || len > MAX_NAME {
                return None;
            }
            let named = rest.get(digits..digits + len)?;
            *at += digits + len;
            Param::Named(named.to_owned())
        }
        other => Param::Unknown(other),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_census_shapes() {
        let s = parse("PlaySound__Q5ThingQ7StringA").unwrap();
        assert_eq!(s.name, "PlaySound");
        assert_eq!(
            s.params,
            vec![Param::Named("Thing".into()), Param::Named("StringA".into())]
        );
        let s = parse("DoText__Q6StringQ21Gooey.EGooeyTextStyle").unwrap();
        assert_eq!(s.params[1], Param::Named("Gooey.EGooeyTextStyle".into()));
        let s = parse("DoHorizontalBreak__ipf").unwrap();
        assert_eq!(s.params, vec![Param::Int, Param::V2, Param::Float]);
        assert_eq!(parse("Yield__").unwrap().params, vec![]);
        assert_eq!(
            parse("Messaging_NewMessage__iQ6String").unwrap().name,
            "Messaging_NewMessage"
        );
        assert_eq!(parse("X__z").unwrap().params, vec![Param::Unknown('z')]);
        assert_eq!(
            parse("ShiftL__ji").unwrap().params,
            vec![Param::Long, Param::Int]
        );
        assert_eq!(
            parse("DoInit__[Q12TweakSettingi").unwrap().params,
            vec![
                Param::Array(Box::new(Param::Named("TweakSetting".into()))),
                Param::Int
            ]
        );
        assert_eq!(parse("Bad__["), None);
        assert_eq!(parse("NoSeparator"), None);
        assert_eq!(parse("Bad__Q9Thing"), None);
        assert_eq!(parse("Bad__Q"), None);
    }
}
