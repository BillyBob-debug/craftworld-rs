//! Mangled function names, as call sites name natives: `Name__` followed by one code per
//! parameter, e.g. `IsSwitchTriggered__i`, `PlaySound__Q5ThingQ7StringA`.
//!
//! INFERRED (docs/script-natives.md) from the 01.33 native-call census names; Toolkit
//! d6a2ef80 reads these names but does not decode them. `Q<len><name>` is a class or enum
//! by name. The vector letters `p`/`q`/`r` follow the order of Toolkit's BuiltinType
//! V2/V3/V4 (all four floats natively). Unrecognised codes are kept as
//! [`Param::Unknown`] so the census can count them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Param {
    Bool,
    Char,
    Int,
    Float,
    V2,
    V3,
    V4,
    M44,
    Guid,
    /// `Q<len><name>`: script class, engine class or enum (`Gooey.EGooeyTextStyle`).
    Named(String),
    Unknown(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature<'a> {
    pub name: &'a str,
    pub params: Vec<Param>,
}

/// Longest class name accepted in a `Q` code.
const MAX_NAME: usize = 256;

/// Splits a mangled name; `None` when there is no `__` or a `Q` length is malformed.
pub fn parse(mangled: &str) -> Option<Signature<'_>> {
    // Skip the first byte so a leading "__" is part of the name.
    let split = mangled.get(1..)?.find("__")? + 1;
    let name = &mangled[..split];
    let codes = &mangled[split + 2..];
    let mut params = Vec::new();
    let mut chars = codes.char_indices();
    while let Some((at, c)) = chars.next() {
        params.push(match c {
            'b' => Param::Bool,
            'c' => Param::Char,
            'i' => Param::Int,
            'f' => Param::Float,
            'p' => Param::V2,
            'q' => Param::V3,
            'r' => Param::V4,
            'm' => Param::M44,
            'g' => Param::Guid,
            'Q' => {
                let rest = &codes[at + 1..];
                let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
                let len: usize = rest.get(..digits)?.parse().ok()?;
                if len == 0 || len > MAX_NAME {
                    return None;
                }
                let named = rest.get(digits..digits + len)?;
                // Advance past the digits and the name.
                for _ in 0..digits + named.chars().count() {
                    chars.next();
                }
                Param::Named(named.to_owned())
            }
            other => Param::Unknown(other),
        });
    }
    Some(Signature { name, params })
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
        assert_eq!(parse("NoSeparator"), None);
        assert_eq!(parse("Bad__Q9Thing"), None);
        assert_eq!(parse("Bad__Q"), None);
    }
}
