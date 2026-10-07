//! PARAM.SFO identity reader; no complete copyrighted SFO contents are exported.
//!
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit types/ParamSFO.java::build, d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! establishes LE header tables, 16-byte records and value formats. Corroborated by
//! the local BCUS98245 SFO. Independent reader; see notices/docs/formats.md.
use lbp_core::{Error, ErrorKind, Result};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct BuildIdentity {
    pub identification_basis: &'static str,
    pub title_id: Option<String>,
    pub title: Option<String>,
    pub app_version: Option<String>,
    pub disc_version: Option<String>,
    pub category: Option<String>,
    pub minimum_system_version: Option<String>,
    pub communication_id: Option<String>,
    pub sfo_format_version: u32,
}

fn range(data: &[u8], offset: usize, size: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| Error::malformed("SFO range overflow"))?;
    data.get(offset..end)
        .ok_or_else(|| Error::malformed("SFO field outside file"))
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        range(data, offset, 4)?.try_into().unwrap(),
    ))
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        range(data, offset, 2)?.try_into().unwrap(),
    ))
}

pub fn read_identity(data: &[u8]) -> Result<BuildIdentity> {
    if data.len() > 1024 * 1024 {
        return Err(Error::limit("SFO exceeds 1 MiB budget"));
    }
    if range(data, 0, 4)? != b"\0PSF" {
        return Err(Error::malformed("PARAM.SFO magic absent"));
    }
    let version = u32_at(data, 4)?;
    if version != 0x101 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!("SFO format version {version:#x}"),
        ));
    }
    let keys = u32_at(data, 8)? as usize;
    let values = u32_at(data, 12)? as usize;
    let count = u32_at(data, 16)? as usize;
    if count > 4096 {
        return Err(Error::limit("SFO entry budget exceeded"));
    }
    let table_end = 20_usize
        .checked_add(
            count
                .checked_mul(16)
                .ok_or_else(|| Error::malformed("SFO table size overflow"))?,
        )
        .ok_or_else(|| Error::malformed("SFO table end overflow"))?;
    if keys < table_end || values < keys || values > data.len() {
        return Err(Error::malformed("SFO table offsets invalid"));
    }
    let mut fields = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..count {
        let at = 20 + index * 16;
        let key_at = keys
            .checked_add(u16_at(data, at)? as usize)
            .ok_or_else(|| Error::malformed("SFO key offset overflow"))?;
        if key_at >= values {
            return Err(Error::malformed("SFO key is outside key table"));
        }
        let key_tail = range(data, key_at, values - key_at)?;
        let terminator = key_tail
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| Error::malformed("SFO key lacks terminator"))?;
        let key = std::str::from_utf8(&key_tail[..terminator])
            .map_err(|_| Error::malformed("SFO key is not UTF-8"))?;
        if !seen.insert(key) {
            return Err(Error::malformed(format!("duplicate SFO key {key}")));
        }
        let format = u16_at(data, at + 2)?;
        let length = u32_at(data, at + 4)? as usize;
        let maximum = u32_at(data, at + 8)? as usize;
        if length > maximum {
            return Err(Error::malformed(
                "SFO value length exceeds allocated capacity",
            ));
        }
        let value_at = values
            .checked_add(u32_at(data, at + 12)? as usize)
            .ok_or_else(|| Error::malformed("SFO value offset overflow"))?;
        range(data, value_at, maximum)?;
        let value = range(data, value_at, length)?;
        if format == 0x404 && length != 4 {
            return Err(Error::malformed("SFO integer length is not four bytes"));
        }
        if matches!(
            key,
            "TITLE_ID"
                | "TITLE"
                | "APP_VER"
                | "VERSION"
                | "CATEGORY"
                | "PS3_SYSTEM_VER"
                | "NP_COMMUNICATION_ID"
        ) {
            if !matches!(format, 0x4 | 0x204) {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!("SFO identity field {key} has non-text encoding"),
                ));
            }
            let text = if format == 0x204 {
                if value.last() != Some(&0) {
                    return Err(Error::malformed(format!(
                        "SFO {key} lacks string terminator"
                    )));
                }
                &value[..value.len() - 1]
            } else {
                value
            };
            if text.contains(&0) {
                return Err(Error::malformed("SFO identity text contains embedded NUL"));
            }
            let text = std::str::from_utf8(text)
                .map_err(|_| Error::malformed("SFO identity text is not UTF-8"))?;
            fields.insert(key.to_owned(), text.to_owned());
        }
    }
    Ok(BuildIdentity {
        identification_basis: "disc_declared_param_sfo",
        title_id: fields.remove("TITLE_ID"),
        title: fields.remove("TITLE"),
        app_version: fields.remove("APP_VER"),
        disc_version: fields.remove("VERSION"),
        category: fields.remove("CATEGORY"),
        minimum_system_version: fields.remove("PS3_SYSTEM_VER"),
        communication_id: fields.remove("NP_COMMUNICATION_ID"),
        sfo_format_version: version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> Vec<u8> {
        let mut d = b"\0PSF".to_vec();
        for n in [0x101_u32, 36, 48, 1] {
            d.extend(n.to_le_bytes());
        }
        d.extend(0_u16.to_le_bytes());
        d.extend(0x204_u16.to_le_bytes());
        for n in [10_u32, 10, 0] {
            d.extend(n.to_le_bytes());
        }
        d.extend(b"TITLE_ID\0");
        d.resize(48, 0);
        d.extend(b"SYNTH0001\0");
        d
    }
    #[test]
    fn reads_identity_and_bounds() {
        let d = example();
        assert_eq!(
            read_identity(&d).unwrap().title_id.as_deref(),
            Some("SYNTH0001")
        );
        for end in 0..d.len() {
            assert!(read_identity(&d[..end]).is_err());
        }
        let mut bad = d.clone();
        bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_identity(&bad).is_err());
        let mut bad = d;
        bad[24..28].copy_from_slice(&11_u32.to_le_bytes());
        assert!(read_identity(&bad).is_err());
    }
}
