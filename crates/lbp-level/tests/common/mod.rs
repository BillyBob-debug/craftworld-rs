//! Authored synthetic encoder for test recipes (no game bytes). Mirrors the rules in
//! crates/lbp-level/src/serial.rs from the writer side.
#![allow(dead_code)]
use lbp_level::Revision;

pub struct W {
    pub b: Vec<u8>,
    pub flags: u8,
}

impl W {
    pub fn new(flags: u8) -> Self {
        Self {
            b: Vec::new(),
            flags,
        }
    }
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.b.push(v);
        self
    }
    pub fn raw(&mut self, v: &[u8]) -> &mut Self {
        self.b.extend_from_slice(v);
        self
    }
    pub fn i16(&mut self, v: i16) -> &mut Self {
        self.raw(&v.to_be_bytes())
    }
    pub fn uleb(&mut self, mut v: u64) -> &mut Self {
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.b.push(byte);
                return self;
            }
            self.b.push(byte | 0x80);
        }
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        if self.flags & 1 != 0 {
            self.uleb(u64::from(v))
        } else {
            self.raw(&v.to_be_bytes())
        }
    }
    pub fn s32(&mut self, v: i32) -> &mut Self {
        if self.flags & 1 != 0 {
            self.uleb(u64::from(((v << 1) ^ (v >> 31)) as u32))
        } else {
            self.raw(&v.to_be_bytes())
        }
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        if self.flags & 1 != 0 {
            self.uleb(v)
        } else {
            self.raw(&v.to_be_bytes())
        }
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.raw(&v.to_be_bytes())
    }
    pub fn m44(&mut self, m: [f32; 16]) -> &mut Self {
        if self.flags & 4 != 0 {
            let id = [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ];
            let mask: u16 = (0..16)
                .filter(|&i| m[i] != id[i])
                .fold(0, |a, i| a | 1 << i);
            self.raw(&mask.to_be_bytes());
            for (i, v) in m.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    self.f32(*v);
                }
            }
            self
        } else {
            for v in m {
                self.f32(v);
            }
            self
        }
    }
    /// Resource with flags word (head > 0x22e, not a descriptor call).
    pub fn res_guid(&mut self, guid: u32) -> &mut Self {
        self.u32(0).u8(2).u32(guid)
    }
    pub fn res_none(&mut self) -> &mut Self {
        self.u32(0).u8(0)
    }
    pub fn int_vector(&mut self, v: &[u32]) -> &mut Self {
        self.u32(v.len() as u32);
        if self.flags & 2 == 0 {
            for x in v {
                self.u32(*x);
            }
            return self;
        }
        if v.is_empty() {
            return self;
        }
        let max = v.iter().copied().max().unwrap();
        let width = (32 - max.leading_zeros()).div_ceil(8) as usize;
        self.u8(width as u8);
        for i in 0..width {
            for x in v {
                self.u8((x >> (8 * i)) as u8);
            }
        }
        self
    }
    /// Thing header from marker through part flags.
    #[allow(clippy::too_many_arguments)]
    pub fn thing_header(
        &mut self,
        uid: i32,
        parent: u32,
        group: u32,
        emitter: u32,
        parts_revision: i32,
        part_flags: u64,
    ) -> &mut Self {
        self.u8(0xaa)
            .u32(uid as u32)
            .u32(parent)
            .u32(group)
            .u32(emitter);
        self.i16(-1)
            .i16(-1)
            .u32(0)
            .u8(0)
            .s32(parts_revision)
            .u64(part_flags)
    }
}

pub fn rev(head: u32, flags: u8) -> Revision {
    Revision {
        head,
        branch_id: 0,
        branch_revision: 0,
        compression: flags,
    }
}

pub fn bit(index: u32) -> u64 {
    1u64 << index
}

pub fn translate(x: f32, y: f32, z: f32) -> [f32; 16] {
    [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, y, z, 1.]
}
