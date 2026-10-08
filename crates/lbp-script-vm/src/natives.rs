//! Rs: host bindings for native functions, keyed by (declaring class, mangled name) as the
//! VM reports them. Written from recorded findings, never from game code
//! (docs/script-natives.md lists each binding with its evidence label).
//!
//! The engine side is a [`World`]; every method defaults to an explicit "not available"
//! error so a partial world (tests, the viewer) still runs the pure bindings. [`Bindings`]
//! adapts a world to the VM's [`Host`] and counts calls to natives that have no binding
//! yet, in the order the G10 census ranked them.
use crate::signature;
use crate::{Host, MemberKind, Value};
use std::collections::BTreeMap;

/// What a Thing's switch input reports on one port (INFERRED field split: the census has
/// separate `IsSwitchTriggered`, binary, analogue and player natives).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SwitchInput {
    pub triggered: bool,
    pub binary: bool,
    pub analogue: f32,
    pub player: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Texture,
    Plan,
}

fn not_available(what: &str) -> String {
    format!("world does not provide {what}")
}

/// Engine state the bindings read and change. Thing handles are the host's `u32` ids.
pub trait World {
    fn thing_uid(&mut self, _thing: u32) -> Result<i32, String> {
        Err(not_available("thing UIDs"))
    }
    fn parent(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn first_child(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn next_sibling(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    /// The level's world Thing.
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Err(not_available("the world thing"))
    }
    fn world_position(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("positions"))
    }
    fn centre_of_mass(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("centres of mass"))
    }
    /// Local transform in game memory order (translation at 12..15).
    fn set_local_transform(&mut self, _thing: u32, _m: [f32; 16]) -> Result<(), String> {
        Err(not_available("transforms"))
    }
    fn is_inactive(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("activity state"))
    }
    /// Does the Thing carry this part (`"PJoint"`, `"PSwitch"`, ...)?
    fn has_part(&mut self, _thing: u32, _part: &str) -> Result<bool, String> {
        Err(not_available("parts"))
    }
    fn switch_input(&mut self, _thing: u32, _port: i32) -> Result<SwitchInput, String> {
        Err(not_available("switch inputs"))
    }
    /// LAMS text by key (the strings the U3 text browser reads).
    fn translate(&mut self, _key: i32) -> Result<String, String> {
        Err(not_available("translated text"))
    }
    fn play_sound(&mut self, _thing: Option<u32>, _name: &str) -> Result<(), String> {
        Err(not_available("audio"))
    }
    /// Handle for a resource loaded by GUID.
    fn load_resource(&mut self, _kind: ResourceKind, _guid: u32) -> Result<u32, String> {
        Err(not_available("resources"))
    }
    fn num_players(&mut self) -> Result<i32, String> {
        Err(not_available("players"))
    }
    /// Outcome of a `Test.Check*` / `Test.Fail` native in the game's own test scripts.
    fn test_check(&mut self, _passed: bool, _message: &str, _line: i32) {}
    /// Uniform in [0, 1); only `NonDeterministic*` natives use it.
    fn random_unit(&mut self) -> f32 {
        0.
    }
    fn get_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn set_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
        _value: Value,
    ) -> Result<(), String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn thing_is(&mut self, _thing: u32, _class: &str) -> bool {
        true
    }
    fn write(&mut self, _text: &str) {}
}

/// A native call split by the mangled signature: `receiver` is present when the VM staged
/// one more value than the signature has parameters (member call, INFERRED).
pub struct Call<'a> {
    pub receiver: Option<&'a Value>,
    pub params: &'a [Value],
}

pub type NativeFn = fn(&mut dyn World, &Call<'_>) -> Result<Value, String>;

/// Adapts a [`World`] to the VM's [`Host`].
pub struct Bindings<W> {
    pub world: W,
    /// Calls to natives without a binding, by `Class.mangled`.
    pub unbound: BTreeMap<String, u64>,
}

impl<W: World> Bindings<W> {
    pub fn new(world: W) -> Self {
        Self {
            world,
            unbound: BTreeMap::new(),
        }
    }
}

/// The binding for a native, if one exists.
pub fn lookup(class: &str, mangled: &str) -> Option<NativeFn> {
    TABLE
        .iter()
        .find(|(c, m, _)| *c == class && *m == mangled)
        .map(|(_, _, f)| *f)
}

/// Every bound native as (class, mangled name), in table order.
pub fn bound() -> impl Iterator<Item = (&'static str, &'static str)> {
    TABLE.iter().map(|(c, m, _)| (*c, *m))
}

/// Calls a native through the table, checking arity against the mangled signature.
pub fn dispatch(
    world: &mut dyn World,
    class: &str,
    mangled: &str,
    args: &[Value],
) -> Option<Result<Value, String>> {
    let f = lookup(class, mangled)?;
    let n = signature::parse(mangled).map_or(0, |s| s.params.len());
    let call = if args.len() == n {
        Call {
            receiver: None,
            params: args,
        }
    } else if args.len() == n + 1 {
        Call {
            receiver: args.first(),
            params: &args[1..],
        }
    } else {
        return Some(Err(format!(
            "{class}.{mangled}: {} arguments for {n} parameters",
            args.len()
        )));
    };
    Some(f(world, &call))
}

impl<W: World> Host for Bindings<W> {
    fn call_native(
        &mut self,
        class: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match dispatch(&mut self.world, class, function, args) {
            Some(r) => r.map_err(|e| format!("{class}.{function}: {e}")),
            None => {
                *self
                    .unbound
                    .entry(format!("{class}.{function}"))
                    .or_default() += 1;
                Err(format!("unbound native {class}.{function}"))
            }
        }
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        self.world.get_member(kind, base, class, field)
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        self.world.set_member(kind, base, class, field, value)
    }
    fn thing_is(&mut self, thing: u32, class: &str) -> bool {
        self.world.thing_is(thing, class)
    }
    fn write(&mut self, text: &str) {
        self.world.write(text)
    }
}

// ---- argument helpers ------------------------------------------------------------

fn param<'a>(c: &'a Call<'_>, i: usize) -> Result<&'a Value, String> {
    c.params
        .get(i)
        .ok_or_else(|| format!("missing parameter {i}"))
}

fn float(v: &Value) -> Result<f32, String> {
    match v {
        Value::Float(f) => Ok(*f),
        other => Err(format!("expected f32, got {other:?}")),
    }
}

fn int(v: &Value) -> Result<i32, String> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(format!("expected s32, got {other:?}")),
    }
}

fn vec4(v: &Value) -> Result<[f32; 4], String> {
    match v {
        Value::Vec4(x) => Ok(*x),
        other => Err(format!("expected vector, got {other:?}")),
    }
}

fn text(v: &Value) -> Result<&str, String> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(format!("expected string, got {other:?}")),
    }
}

/// GUID parameter (`g`): carried as an integer or pointer-sized value (INFERRED).
fn guid(v: &Value) -> Result<u32, String> {
    match v {
        Value::Int(i) => Ok(*i as u32),
        Value::Raw(u) | Value::Ref(u) => Ok(*u),
        other => Err(format!("expected GUID, got {other:?}")),
    }
}

fn thing(v: &Value) -> Result<Option<u32>, String> {
    match v {
        Value::Thing(t) => Ok(*t),
        other => Err(format!("expected Thing, got {other:?}")),
    }
}

fn some_thing(v: &Value) -> Result<u32, String> {
    thing(v)?.ok_or_else(|| "null Thing".to_owned())
}

/// The Thing a member native acts on.
fn this(c: &Call<'_>) -> Result<u32, String> {
    some_thing(
        c.receiver
            .ok_or("member native called without a receiver")?,
    )
}

/// For `Get*__Q5Thing`: the Thing parameter (static form or member form alike).
fn subject(c: &Call<'_>) -> Result<u32, String> {
    some_thing(param(c, 0)?)
}

// ---- math (INFERRED: vector-library conventions, radians, column vectors) -----------

/// Column-major with column vectors: column `k` occupies indices `4k..4k+4`, so the
/// translation sits at 12..15 as in level-file world matrices.
fn columns(c0: [f32; 4], c1: [f32; 4], c2: [f32; 4], c3: [f32; 4]) -> Value {
    let mut m = [0.; 16];
    for (k, col) in [c0, c1, c2, c3].iter().enumerate() {
        m[k * 4..k * 4 + 4].copy_from_slice(col);
    }
    Value::Mat44(m)
}

const W: [f32; 4] = [0., 0., 0., 1.];

fn rotate(axis: usize, a: f32) -> Value {
    let (s, c) = a.sin_cos();
    match axis {
        0 => columns([1., 0., 0., 0.], [0., c, s, 0.], [0., -s, c, 0.], W),
        1 => columns([c, 0., -s, 0.], [0., 1., 0., 0.], [s, 0., c, 0.], W),
        _ => columns([c, s, 0., 0.], [-s, c, 0., 0.], [0., 0., 1., 0.], W),
    }
}

fn len3(v: [f32; 4]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// `Test.*(..., message, line)`: reports one check; the message and line are the last two
/// parameters (INFERRED from the `Q7StringAi` suffix every Test native shares).
fn report(w: &mut dyn World, c: &Call<'_>, passed: bool) -> Result<Value, String> {
    let n = c.params.len();
    let message = text(param(c, n.wrapping_sub(2))?)?;
    let line = int(param(c, n.wrapping_sub(1))?)?;
    w.test_check(passed, message, line);
    Ok(Value::Void)
}

/// `CheckEqual` on vectors compares the components the type has (V2: 2, V3: 3, V4: 4).
fn equal_components(w: &mut dyn World, c: &Call<'_>, k: usize) -> Result<Value, String> {
    let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
    report(w, c, a[..k] == b[..k])
}

fn equal_values(w: &mut dyn World, c: &Call<'_>) -> Result<Value, String> {
    let (a, b) = (param(c, 0)?, param(c, 1)?);
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return Err(format!("CheckEqual on {a:?} and {b:?}"));
    }
    report(w, c, a == b)
}

fn boolean(v: &Value) -> Result<bool, String> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(format!("expected bool, got {other:?}")),
    }
}

const TABLE: &[(&str, &str, NativeFn)] = &[
    // System.Yield__ is handled inside the VM (it suspends the call).
    ("Math", "floor__f", |_, c| {
        Ok(Value::Float(float(param(c, 0)?)?.floor()))
    }),
    ("Math", "clamp__fff", |_, c| {
        let (x, lo, hi) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        Ok(Value::Float(x.max(lo).min(hi)))
    }),
    ("Math", "atan2__ff", |_, c| {
        Ok(Value::Float(
            float(param(c, 0)?)?.atan2(float(param(c, 1)?)?),
        ))
    }),
    ("Math", "rotateX__f", |_, c| {
        Ok(rotate(0, float(param(c, 0)?)?))
    }),
    ("Math", "rotateY__f", |_, c| {
        Ok(rotate(1, float(param(c, 0)?)?))
    }),
    ("Math", "rotateZ__f", |_, c| {
        Ok(rotate(2, float(param(c, 0)?)?))
    }),
    ("Math", "translate__r", |_, c| {
        let t = vec4(param(c, 0)?)?;
        Ok(columns(
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [t[0], t[1], t[2], 1.],
        ))
    }),
    ("Math", "scale__r", |_, c| {
        let s = vec4(param(c, 0)?)?;
        Ok(columns(
            [s[0], 0., 0., 0.],
            [0., s[1], 0., 0.],
            [0., 0., s[2], 0.],
            W,
        ))
    }),
    ("Math", "mulPerElem__rr", |_, c| {
        let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
        Ok(Value::Vec4(std::array::from_fn(|i| a[i] * b[i])))
    }),
    ("Math", "length3__r", |_, c| {
        Ok(Value::Float(len3(vec4(param(c, 0)?)?)))
    }),
    ("Math", "normalize3__r", |_, c| {
        let v = vec4(param(c, 0)?)?;
        let l = len3(v);
        if l == 0. {
            return Err("normalize3 of a zero vector".into());
        }
        Ok(Value::Vec4([v[0] / l, v[1] / l, v[2] / l, v[3]]))
    }),
    ("Math", "length__p", |_, c| {
        let v = vec4(param(c, 0)?)?;
        Ok(Value::Float((v[0] * v[0] + v[1] * v[1]).sqrt()))
    }),
    ("Math", "NonDeterministicRandomBetween__ff", |w, c| {
        let (a, b) = (float(param(c, 0)?)?, float(param(c, 1)?)?);
        Ok(Value::Float(a + (b - a) * w.random_unit()))
    }),
    // ---- strings ----
    ("String", "Concat__Q6StringQ6String", |_, c| {
        let s = format!("{}{}", text(param(c, 0)?)?, text(param(c, 1)?)?);
        Ok(Value::Str(s.into()))
    }),
    ("String", "StringFromInt__i", |_, c| {
        Ok(Value::Str(int(param(c, 0)?)?.to_string().into()))
    }),
    ("String", "InfinitySymbol__", |_, _| {
        Ok(Value::Str("\u{221e}".into()))
    }),
    ("LAMS", "Translate__i", |w, c| {
        Ok(Value::Str(w.translate(int(param(c, 0)?)?)?.into()))
    }),
    // ---- thing tree and transforms ----
    ("Thing", "GetThingUID__", |w, c| {
        Ok(Value::Int(w.thing_uid(this(c)?)?))
    }),
    ("Thing", "GetParent__Q5Thing", |w, c| {
        Ok(Value::Thing(w.parent(subject(c)?)?))
    }),
    ("Thing", "GetFirstChild__Q5Thing", |w, c| {
        Ok(Value::Thing(w.first_child(subject(c)?)?))
    }),
    ("Thing", "GetNextSibling__Q5Thing", |w, c| {
        Ok(Value::Thing(w.next_sibling(subject(c)?)?))
    }),
    ("Thing", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    ("System", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    ("Thing", "GetWorldPosition__", |w, c| {
        Ok(Value::Vec4(w.world_position(this(c)?)?))
    }),
    ("Thing", "GetCOM__", |w, c| {
        Ok(Value::Vec4(w.centre_of_mass(this(c)?)?))
    }),
    ("Thing", "SetLocalPosition__m", |w, c| {
        let m = match param(c, 0)? {
            Value::Mat44(m) => *m,
            other => return Err(format!("expected m44, got {other:?}")),
        };
        w.set_local_transform(this(c)?, m)?;
        Ok(Value::Void)
    }),
    ("Thing", "IsInactive__", |w, c| {
        Ok(Value::Bool(w.is_inactive(this(c)?)?))
    }),
    // ---- switches (the Thing's own inputs, by port) ----
    ("Thing", "IsSwitchTriggered__i", |w, c| {
        Ok(Value::Bool(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.triggered,
        ))
    }),
    ("Thing", "GetSwitchActivationBinary__i", |w, c| {
        Ok(Value::Bool(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.binary,
        ))
    }),
    ("Thing", "GetSwitchActivationAnalogue__i", |w, c| {
        Ok(Value::Float(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.analogue,
        ))
    }),
    ("Thing", "GetSwitchActivationPlayer__i", |w, c| {
        Ok(Value::Int(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.player,
        ))
    }),
    // ---- parts ----
    ("PartList", "HasPJoint__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PJoint")?))
    }),
    ("PartList", "HasPSwitch__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSwitch")?))
    }),
    ("PartList", "HasPTrigger__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PTrigger")?))
    }),
    ("PartList", "HasPShape__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PShape")?))
    }),
    ("PartList", "HasPSpriteLight__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSpriteLight")?))
    }),
    // ---- audio, resources, game ----
    ("Audio", "PlaySound__Q5ThingQ7StringA", |w, c| {
        let t = thing(param(c, 0)?)?;
        w.play_sound(t, text(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("Resource", "LoadTextureByFilename__g", |w, c| {
        Ok(Value::Ref(w.load_resource(
            ResourceKind::Texture,
            guid(param(c, 0)?)?,
        )?))
    }),
    ("Resource", "LoadPlanByFilename__g", |w, c| {
        Ok(Value::Ref(
            w.load_resource(ResourceKind::Plan, guid(param(c, 0)?)?)?,
        ))
    }),
    ("Game", "GetNumPlayers__", |w, _| {
        Ok(Value::Int(w.num_players()?))
    }),
    // ---- the game's own test scripts ----
    ("Test", "Check__bQ7StringAi", |w, c| {
        let ok = boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "CheckFalse__bQ7StringAi", |w, c| {
        let ok = !boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "Fail__Q7StringAi", |w, c| report(w, c, false)),
    ("Test", "Succeed__Q7StringAi", |w, c| report(w, c, true)),
    ("Test", "CheckEqual__iiQ7StringAi", equal_values),
    ("Test", "CheckEqual__bbQ7StringAi", equal_values),
    ("Test", "CheckEqual__jjQ7StringAi", equal_values),
    ("Test", "CheckEqual__ffQ7StringAi", equal_values),
    ("Test", "CheckEqual__ppQ7StringAi", |w, c| {
        equal_components(w, c, 2)
    }),
    ("Test", "CheckEqual__qqQ7StringAi", |w, c| {
        equal_components(w, c, 3)
    }),
    ("Test", "CheckEqual__rrQ7StringAi", |w, c| {
        equal_components(w, c, 4)
    }),
    ("Test", "CheckSimilar__fffQ7StringAi", |w, c| {
        let (a, b, eps) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        report(w, c, (a - b).abs() <= eps)
    }),
];

/// Natives the VM itself handles; they never reach a host.
pub const VM_HANDLED: [(&str, &str); 1] = [("System", "Yield__")];

#[cfg(test)]
mod tests {
    use super::*;

    struct Tree;
    impl World for Tree {
        fn parent(&mut self, t: u32) -> Result<Option<u32>, String> {
            Ok(t.checked_sub(1))
        }
        fn thing_uid(&mut self, t: u32) -> Result<i32, String> {
            Ok(t as i32 + 100)
        }
        fn switch_input(&mut self, _t: u32, port: i32) -> Result<SwitchInput, String> {
            Ok(SwitchInput {
                analogue: port as f32 / 2.,
                ..Default::default()
            })
        }
    }

    fn call(class: &str, m: &str, args: &[Value]) -> Result<Value, String> {
        dispatch(&mut Tree, class, m, args).expect("bound")
    }

    #[test]
    fn every_entry_has_a_signature_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (c, m) in bound() {
            let s = signature::parse(m).unwrap_or_else(|| panic!("{m}"));
            assert!(
                !s.params
                    .iter()
                    .any(|p| matches!(p, signature::Param::Unknown(_))),
                "{m}"
            );
            assert!(seen.insert((c, m)), "duplicate {c}.{m}");
        }
    }

    #[test]
    fn math() {
        let Value::Mat44(m) =
            call("Math", "translate__r", &[Value::Vec4([1., 2., 3., 0.])]).unwrap()
        else {
            panic!()
        };
        assert_eq!(&m[12..16], &[1., 2., 3., 1.]);
        let Value::Mat44(r) = call(
            "Math",
            "rotateZ__f",
            &[Value::Float(std::f32::consts::FRAC_PI_2)],
        )
        .unwrap() else {
            panic!()
        };
        // Column 0 (the image of +x) is +y.
        assert!((r[0]).abs() < 1e-6 && (r[1] - 1.).abs() < 1e-6);
        assert_eq!(
            call("Math", "length3__r", &[Value::Vec4([3., 4., 0., 9.])]),
            Ok(Value::Float(5.))
        );
        assert_eq!(
            call(
                "Math",
                "clamp__fff",
                &[Value::Float(5.), Value::Float(0.), Value::Float(1.)]
            ),
            Ok(Value::Float(1.))
        );
        assert!(call("Math", "normalize3__r", &[Value::Vec4([0.; 4])]).is_err());
        assert_eq!(
            call(
                "String",
                "Concat__Q6StringQ6String",
                &[Value::Str("a".into()), Value::Str("b".into())]
            ),
            Ok(Value::Str("ab".into()))
        );
    }

    #[test]
    fn receiver_split_and_world() {
        // Member form: receiver plus the port.
        assert_eq!(
            call(
                "Thing",
                "GetSwitchActivationAnalogue__i",
                &[Value::Thing(Some(7)), Value::Int(1)]
            ),
            Ok(Value::Float(0.5))
        );
        assert_eq!(
            call("Thing", "GetThingUID__", &[Value::Thing(Some(7))]),
            Ok(Value::Int(107))
        );
        assert!(call("Thing", "GetThingUID__", &[]).is_err());
        assert_eq!(
            call("Thing", "GetParent__Q5Thing", &[Value::Thing(Some(0))]),
            Ok(Value::Thing(None))
        );
        assert!(call("Thing", "GetThingUID__", &[Value::Thing(None)]).is_err());
        // Arity outside n or n + 1.
        assert!(call("Math", "floor__f", &[]).is_err());
        // A world that lacks a feature says so.
        assert!(call("LAMS", "Translate__i", &[Value::Int(3)])
            .unwrap_err()
            .contains("translated text"));
        assert!(dispatch(&mut Tree, "Gooey", "EndFrame__", &[]).is_none());
    }

    #[test]
    fn test_natives_report_checks() {
        #[derive(Default)]
        struct Log(Vec<(bool, String, i32)>);
        impl World for Log {
            fn test_check(&mut self, passed: bool, message: &str, line: i32) {
                self.0.push((passed, message.into(), line));
            }
        }
        let mut log = Log::default();
        let msg = || Value::Str("m".into());
        let mut run =
            |m: &str, args: Vec<Value>| dispatch(&mut log, "Test", m, &args).expect("bound");
        let v = |a: [f32; 4]| Value::Vec4(a);
        run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(2), Value::Int(2), msg(), Value::Int(9)],
        )
        .unwrap();
        run(
            "CheckEqual__ppQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckEqual__rrQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckSimilar__fffQ7StringAi",
            vec![
                Value::Float(1.),
                Value::Float(1.05),
                Value::Float(0.1),
                msg(),
                Value::Int(3),
            ],
        )
        .unwrap();
        // Mixed types are a fault, not a failed check.
        assert!(run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(1), Value::Float(1.), msg(), Value::Int(1)]
        )
        .is_err());
        let passed: Vec<_> = log.0.iter().map(|(p, _, l)| (*p, *l)).collect();
        assert_eq!(passed, [(true, 9), (true, 9), (false, 9), (true, 3)]);
    }
}
