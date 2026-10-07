//! Independent UV reader: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! RMesh::serialize/init/getUVs(channel), MemoryInputStream::v2 (fixed BE floats).
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java
//! Full-channel oracle avoids pinned nonzero-start getter bug. docs/textured-mesh.md.
use super::*;
#[derive(Debug, Serialize)]
pub struct UvChannelMetadata {
    pub channel: usize,
    pub uv_bits_sha256: String,
}
#[derive(Debug, Serialize)]
pub struct UvMetadata {
    pub vertex_count: u32,
    pub channel_count: u32,
    pub channels: Vec<UvChannelMetadata>,
}
/// CPU UV arrays are deliberately not a serializable asset export.
#[derive(Debug)]
pub struct UvCoordinates {
    pub metadata: UvMetadata,
    pub channels: Vec<Vec<[f32; 2]>>,
}
pub fn decode_uvs(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<UvCoordinates, Error> {
    let (header, data) = payload(bytes, &cancel)?;
    let prefix = prefix(&header, &data, &cancel)?;
    let mut r = Reader {
        b: &data,
        p: 0,
        compact: prefix.compression_flags & 1 != 0,
    };
    for _ in 0..7 {
        r.integer()?;
    }
    r.take(32 * 16)?;
    if header.revision >= 0x239 {
        r.floats()?;
        r.floats()?;
        r.word()?;
    }
    for _ in 0..prefix.counts[4] + 2 {
        r.integer()?;
    }
    let stream_bytes = (prefix.counts[0] as usize)
        .checked_mul(16)
        .and_then(|n| n.checked_mul(prefix.counts[4] as usize))
        .ok_or(Error::Budget)?;
    r.take(stream_bytes)?;
    let length = r.count(MAX_EXPANDED as u32)? as usize;
    let attributes = r.take(length)?;
    decode_attributes(attributes, prefix.counts[0], prefix.counts[5], &cancel)
}
fn decode_attributes(
    attributes: &[u8],
    vertices: u32,
    count: u32,
    cancel: &impl Fn() -> bool,
) -> Result<UvCoordinates, Error> {
    if vertices > 100_000 || count > 8 {
        return Err(Error::Budget);
    }
    let stride = (count as usize).checked_mul(8).ok_or(Error::Budget)?;
    let extent = (vertices as usize)
        .checked_mul(stride)
        .ok_or(Error::Budget)?;
    if attributes.len() != extent {
        return Err(Error::Invalid("UV attribute count/extent"));
    }
    let mut channels = Vec::with_capacity(count as usize);
    let mut metadata = Vec::with_capacity(count as usize);
    for channel in 0..count as usize {
        checked(cancel)?;
        let mut values = Vec::with_capacity(vertices as usize);
        let mut hash = Sha256::new();
        for vertex in 0..vertices as usize {
            if vertex % 1024 == 0 {
                checked(cancel)?;
            }
            let start = vertex * stride + channel * 8; // Checked extent above bounds every slice.
            let bytes = &attributes[start..start + 8];
            let value = std::array::from_fn(|i| {
                f32::from_bits(u32::from_be_bytes(
                    bytes[i * 4..i * 4 + 4].try_into().unwrap(),
                ))
            });
            if value.iter().any(|v| !v.is_finite()) {
                return Err(Error::Invalid("nonfinite mesh UV"));
            }
            hash.update(bytes);
            values.push(value);
        }
        channels.push(values);
        metadata.push(UvChannelMetadata {
            channel,
            uv_bits_sha256: format!("{:x}", hash.finalize()),
        });
    }
    checked(cancel)?;
    Ok(UvCoordinates {
        metadata: UvMetadata {
            vertex_count: vertices,
            channel_count: count,
            channels: metadata,
        },
        channels,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interleaved_channels_fixed_floats_negative_zero_and_extent_are_exact() {
        let values = [0.25f32, -0., 4., 5., 0.75, 1., 6., 7.];
        let bytes: Vec<_> = values
            .iter()
            .flat_map(|v| v.to_bits().to_be_bytes())
            .collect();
        let uv = decode_attributes(&bytes, 2, 2, &|| false).unwrap();
        assert_eq!(uv.channels[0][0][1].to_bits(), 0x80000000);
        assert_eq!(uv.channels[0][1], [0.75, 1.]);
        assert_eq!(uv.channels[1], vec![[4., 5.], [6., 7.]]);
        for size in 0..bytes.len() {
            assert!(decode_attributes(&bytes[..size], 2, 2, &|| false).is_err());
        }
        assert!(decode_attributes(&bytes, 2, 1, &|| false).is_err());
        assert!(decode_attributes(&[], 2, 0, &|| false)
            .unwrap()
            .channels
            .is_empty());
    }
    #[test]
    fn nonfinite_values_budgets_and_mid_decode_cancellation_are_explicit() {
        assert!(
            decode_attributes(&f32::NAN.to_bits().to_be_bytes().repeat(2), 1, 1, &|| false)
                .is_err()
        );
        assert!(decode_attributes(&[], 100_001, 1, &|| false).is_err());
        assert!(decode_attributes(&[], 1, 9, &|| false).is_err());
        let calls = std::cell::Cell::new(0);
        let cancelled = || {
            calls.set(calls.get() + 1);
            calls.get() > 2
        };
        assert!(matches!(
            decode_attributes(&vec![0; 2048 * 8], 2048, 1, &cancelled),
            Err(Error::Cancelled)
        ));
    }
}
