//! C1 diagnostic only: no pixel decoder, no original byte export.
//! CONFIRMED_FROM_COMMUNITY_CODE: ennuo/toolkit commit
//! d6a2ef80b5a09999f08f23a803b516096055e5c9, SerializedResource::process,
//! CellGcmTexture constructor and Compressor::decompressData. See docs/textures.md.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

fn u16be(b: &[u8], p: usize) -> u16 {
    u16::from_be_bytes([b[p], b[p + 1]])
}
fn u32be(b: &[u8], p: usize) -> u32 {
    u32::from_be_bytes(b[p..p + 4].try_into().unwrap())
}
fn inflate_exact(b: &[u8], expanded: usize) -> Result<Vec<u8>, String> {
    // One chunk is u16-sized. One extra byte detects falsely small advertised lengths.
    if expanded > 65535 {
        return Err("chunk expansion budget".into());
    }
    let mut z = flate2::Decompress::new(true);
    let mut out = vec![0; expanded + 1];
    let status = z
        .decompress(b, &mut out, flate2::FlushDecompress::Finish)
        .map_err(|e| e.to_string())?;
    if status != flate2::Status::StreamEnd
        || z.total_in() != b.len() as u64
        || z.total_out() != expanded as u64
    {
        return Err("zlib length/end/trailing mismatch".into());
    }
    out.truncate(expanded);
    Ok(out)
}
fn dds_header(b: &[u8]) -> Result<Value, String> {
    if b.len() < 128 || &b[..4] != b"DDS " {
        return Err("missing/truncated DDS header".into());
    }
    let le = |p| u32::from_le_bytes(b[p..p + 4].try_into().unwrap());
    if le(4) != 124 || le(76) != 32 {
        return Err("unsupported DDS header sizes".into());
    }
    Ok(
        json!({"width":le(16),"height":le(12),"depth":le(24),"mips":le(28),
        "fourcc":String::from_utf8_lossy(&b[84..88]),"rgb_bits":le(88),"pixel_flags":le(80),
        "red_mask":le(92),"green_mask":le(96),"blue_mask":le(100),"alpha_mask":le(104),"caps2":le(112)}),
    )
}
fn tex_dds(
    source: &AttachedSource,
    key: &LocationKey,
    payload: u64,
    table: &[u8],
    total: &mut u64,
) -> Result<Value, String> {
    if table.is_empty() {
        return dds_header(&read(source, key, payload, 128, total)?);
    }
    let mut prefix = Vec::new();
    let mut offset = payload;
    for row in table.as_chunks::<4>().0 {
        let (s, e) = (u16be(row, 0) as usize, u16be(row, 2) as usize);
        let b = read(source, key, offset, s, total)?;
        let decoded = if s == e { b } else { inflate_exact(&b, e)? };
        prefix.extend_from_slice(&decoded[..decoded.len().min(128 - prefix.len())]);
        if prefix.len() == 128 {
            return dds_header(&prefix);
        }
        offset += s as u64;
    }
    Err("DDS header exceeds expanded payload".into())
}
fn header(b: &[u8]) -> Result<(usize, Value), String> {
    match b.get(..4) {
        Some(b"TEX ") if b.len() >= 8 => Ok((4, Value::Null)),
        Some(b"GTF ") if b.len() >= 32 => Ok((
            28,
            json!({
                "format_raw": b[4], "format_base": b[4] & !0x60,
                "mips": b[5], "dimension": b[6], "cubemap": b[7],
                "remap": u32be(b,8), "width": u16be(b,12), "height": u16be(b,14),
                "depth": u16be(b,16), "location": b[18], "flags": b[19],
                "pitch": u32be(b,20), "offset": u32be(b,24)
            }),
        )),
        _ => Err("unsupported or truncated clear texture wrapper".into()),
    }
}
fn chunk_totals(table: &[u8], count: usize, remaining: u64) -> Result<Value, String> {
    if count > 32767 || table.len() != count * 4 {
        return Err("chunk table bounds/count".into());
    }
    let (mut stored, mut expanded, mut equal) = (0u64, 0u64, 0usize);
    for row in table.as_chunks::<4>().0 {
        let (s, e) = (u16be(row, 0) as u64, u16be(row, 2) as u64);
        stored += s;
        expanded += e;
        equal += usize::from(s == e);
    }
    if stored > remaining {
        return Err("chunk payload exceeds resource".into());
    }
    Ok(
        json!({"count":count,"stored_bytes":stored,"expanded_bytes":expanded,
        "equal_size_chunks":equal,"zlib_chunks":count-equal,"trailing_bytes":remaining-stored}),
    )
}
fn read(
    source: &AttachedSource,
    key: &LocationKey,
    offset: u64,
    size: usize,
    total: &mut u64,
) -> Result<Vec<u8>, String> {
    if *total + size as u64 > 128 * 1024 * 1024 {
        return Err("global 128 MiB diagnostic read budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex(
            key,
            offset + bytes.len() as u64,
            (size - bytes.len()).min(4096),
        )?;
        *total += page.bytes.len() as u64;
        if page.bytes.is_empty() {
            return Err("truncated read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: texture_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    let local = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !output
        .parent()
        .ok_or("output parent required")?
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(local)
        || output.exists()
    {
        return Err("new output under ignored local-analysis required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut rows = Vec::new();
    let mut total = 0;
    let mut counts = BTreeMap::<String, usize>::new();
    for l in &catalog.snapshot.resource_inspections {
        if !l
            .inspection
            .confirmed_type
            .as_ref()
            .is_some_and(|k| k.name == "texture")
        {
            continue;
        }
        let key = LocationKey {
            path: l.source_path.clone(),
            offset: l.offset,
            size: l.size,
        };
        let result = (|| -> Result<Value, String> {
            let b = read(&source, &key, 0, l.size.min(32) as usize, &mut total)?;
            let (p, gtf) = header(&b)?;
            let count = u16be(&b, p + 2) as usize;
            if count > 32767 {
                return Err("signed Toolkit chunk count exceeded".into());
            }
            let payload = (p + 4 + count * 4) as u64;
            let remaining = l
                .size
                .checked_sub(payload)
                .ok_or("chunk table exceeds resource")?;
            let table = read(&source, &key, (p + 4) as u64, count * 4, &mut total)?;
            let chunks = if count == 0 {
                json!({"count":0,"stored_bytes":remaining,"expanded_bytes":remaining,"equal_size_chunks":0,"zlib_chunks":0,"trailing_bytes":0})
            } else {
                chunk_totals(&table, count, remaining)?
            };
            let (dds, dds_error) = if p == 4 {
                match tex_dds(&source, &key, payload, &table, &mut total) {
                    Ok(v) => (v, Value::Null),
                    Err(e) => (Value::Null, json!(e)),
                }
            } else {
                (Value::Null, Value::Null)
            };
            Ok(
                json!({"wrapper":std::str::from_utf8(&b[..4]).unwrap(),"gtf":gtf,
                "compression_flag":u16be(&b,p),"payload_offset":payload,"chunks":chunks,"dds":dds,"dds_error":dds_error}),
            )
        })();
        let mut row =
            json!({"source_path":l.source_path,"sha1":l.sha1,"offset":l.offset,"size":l.size});
        match result {
            Ok(v) => {
                let wrapper = v["wrapper"].as_str().unwrap().to_string();
                *counts.entry(wrapper).or_default() += 1;
                row["metadata"] = v;
                row["status"] = json!("header_census_only");
            }
            Err(e) => {
                row["status"] = json!("unsupported_or_invalid");
                row["reason"] = json!(e);
            }
        }
        rows.push(row);
    }
    rows.sort_by_key(|r| {
        (
            r["source_path"].as_str().unwrap().to_string(),
            r["offset"].as_u64().unwrap(),
        )
    });
    source.check_stable()?;
    let value = json!({"schema_version":1,"milestone":"C1","toolkit_commit":"d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "validation_bytes":source.validation_bytes,"diagnostic_bytes_read":total,"wrapper_counts":counts,"textures":rows});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "{} texture headers; {total} diagnostic bytes; {counts:?}",
        value["textures"].as_array().unwrap().len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_protected_and_truncated() {
        for b in [b"GTFe".as_slice(), b"TEX ".as_slice(), b"NPD\0".as_slice()] {
            assert!(header(b).is_err());
        }
    }
    #[test]
    fn checked_chunk_ranges_and_stored_cases() {
        let v = chunk_totals(&[0, 4, 0, 4, 0, 2, 0, 8], 2, 6).unwrap();
        assert_eq!(v["expanded_bytes"], 12);
        assert_eq!(v["equal_size_chunks"], 1);
        assert!(chunk_totals(&[0, 4, 0, 4], 1, 3).is_err());
        assert!(chunk_totals(&[], 32768, 0).is_err());
        assert!(chunk_totals(&[0], 1, 9).is_err());
    }
    #[test]
    fn strict_zlib_length_end_and_trailing() {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(b"authored synthetic chunk").unwrap();
        let mut b = z.finish().unwrap();
        assert_eq!(inflate_exact(&b, 24).unwrap(), b"authored synthetic chunk");
        assert!(inflate_exact(&b, 23).is_err());
        assert!(inflate_exact(&b, 25).is_err());
        assert!(inflate_exact(&b[..b.len() - 1], 24).is_err());
        b.push(0);
        assert!(inflate_exact(&b, 24).is_err());
    }
    #[test]
    fn dds_sizes_and_endian() {
        let mut b = vec![0; 128];
        b[..4].copy_from_slice(b"DDS ");
        b[4] = 124;
        b[76] = 32;
        b[16..20].copy_from_slice(&256u32.to_le_bytes());
        assert_eq!(dds_header(&b).unwrap()["width"], 256);
        assert!(dds_header(&b[..127]).is_err());
        b[76] = 0;
        assert!(dds_header(&b).is_err());
    }
}
