use flate2::read::ZlibDecoder;
use md5::{Digest, Md5};
use sanctuary_core::SanctuaryError;
use std::io::{Read, Take};

const BLTE_MAGIC: &[u8; 4] = b"BLTE";
const CHUNK_DESCRIPTOR_BYTES: usize = 24;
const MAX_CHUNKS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlteDecodeOptions {
    pub max_output_bytes: usize,
    pub max_recursion_depth: usize,
    pub verify_chunk_md5: bool,
}

impl Default for BlteDecodeOptions {
    fn default() -> Self {
        Self {
            max_output_bytes: 512 * 1024 * 1024,
            max_recursion_depth: 8,
            verify_chunk_md5: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ChunkDescriptor {
    compressed_size: usize,
    decompressed_size: usize,
    digest: [u8; 16],
}

pub fn decode_blte(bytes: &[u8]) -> Result<Vec<u8>, SanctuaryError> {
    decode_blte_with_options(bytes, BlteDecodeOptions::default())
}

pub fn decode_blte_with_options(
    bytes: &[u8],
    options: BlteDecodeOptions,
) -> Result<Vec<u8>, SanctuaryError> {
    if options.max_output_bytes == 0 {
        return Err(blte_error("output limit must be greater than zero"));
    }
    decode_inner(bytes, options, 0)
}

fn decode_inner(
    bytes: &[u8],
    options: BlteDecodeOptions,
    depth: usize,
) -> Result<Vec<u8>, SanctuaryError> {
    if depth > options.max_recursion_depth {
        return Err(blte_error("recursive BLTE depth limit exceeded"));
    }
    if bytes.get(..4) != Some(BLTE_MAGIC.as_slice()) {
        return Err(blte_error("missing BLTE magic"));
    }
    let header_size = read_u32_be(bytes, 4, "header size")? as usize;
    if header_size == 0 {
        let chunk = bytes
            .get(8..)
            .ok_or_else(|| blte_error("headerless BLTE has no chunk"))?;
        return decode_chunk(chunk, None, options, depth);
    }
    if header_size < 12 || header_size > bytes.len() {
        return Err(blte_error("BLTE header size is outside the input"));
    }
    if bytes.get(8).copied() != Some(0x0f) {
        return Err(blte_error("BLTE chunk-table marker is not 0x0f"));
    }
    let chunk_count = read_u24_be(bytes, 9)?;
    if chunk_count == 0 || chunk_count > MAX_CHUNKS {
        return Err(blte_error(
            "BLTE chunk count is invalid or exceeds the limit",
        ));
    }
    let table_size = chunk_count
        .checked_mul(CHUNK_DESCRIPTOR_BYTES)
        .ok_or_else(|| blte_error("BLTE chunk table size overflow"))?;
    let table_end = 12usize
        .checked_add(table_size)
        .ok_or_else(|| blte_error("BLTE chunk table offset overflow"))?;
    if table_end > header_size {
        return Err(blte_error("BLTE chunk table extends beyond the header"));
    }

    let mut descriptors = Vec::with_capacity(chunk_count);
    for index in 0..chunk_count {
        let offset = 12 + index * CHUNK_DESCRIPTOR_BYTES;
        let compressed_size = read_u32_be(bytes, offset, "compressed chunk size")? as usize;
        let decompressed_size = read_u32_be(bytes, offset + 4, "decompressed chunk size")? as usize;
        if compressed_size == 0 {
            return Err(blte_error("BLTE chunk has zero compressed size"));
        }
        if decompressed_size > options.max_output_bytes {
            return Err(blte_error("BLTE chunk exceeds the output limit"));
        }
        let digest_slice = bytes
            .get(offset + 8..offset + 24)
            .ok_or_else(|| blte_error("truncated BLTE chunk digest"))?;
        let mut digest = [0u8; 16];
        digest.copy_from_slice(digest_slice);
        descriptors.push(ChunkDescriptor {
            compressed_size,
            decompressed_size,
            digest,
        });
    }

    let mut cursor = header_size;
    let total_expected = descriptors
        .iter()
        .try_fold(0usize, |total, descriptor| {
            total.checked_add(descriptor.decompressed_size)
        })
        .ok_or_else(|| blte_error("BLTE decompressed-size sum overflow"))?;
    if total_expected > options.max_output_bytes {
        return Err(blte_error("BLTE output exceeds the configured limit"));
    }
    let mut output = Vec::with_capacity(total_expected.min(16 * 1024 * 1024));
    for descriptor in descriptors {
        let end = cursor
            .checked_add(descriptor.compressed_size)
            .ok_or_else(|| blte_error("BLTE chunk offset overflow"))?;
        let chunk = bytes
            .get(cursor..end)
            .ok_or_else(|| blte_error("truncated BLTE chunk body"))?;
        if options.verify_chunk_md5 {
            let actual = Md5::digest(chunk);
            if actual[..] != descriptor.digest[..] {
                return Err(blte_error("BLTE chunk MD5 mismatch"));
            }
        }
        let remaining = options
            .max_output_bytes
            .checked_sub(output.len())
            .ok_or_else(|| blte_error("BLTE output limit exhausted"))?;
        let mut chunk_options = options;
        chunk_options.max_output_bytes = remaining;
        let decoded = decode_chunk(
            chunk,
            Some(descriptor.decompressed_size),
            chunk_options,
            depth,
        )?;
        output.extend_from_slice(&decoded);
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(blte_error(
            "BLTE contains trailing bytes after declared chunks",
        ));
    }
    Ok(output)
}

fn decode_chunk(
    chunk: &[u8],
    expected_size: Option<usize>,
    options: BlteDecodeOptions,
    depth: usize,
) -> Result<Vec<u8>, SanctuaryError> {
    let (&mode, payload) = chunk
        .split_first()
        .ok_or_else(|| blte_error("BLTE chunk is empty"))?;
    let decoded = match mode {
        b'N' => bounded_copy(payload, options.max_output_bytes)?,
        b'Z' => decode_zlib(payload, options.max_output_bytes)?,
        b'F' => {
            if depth >= options.max_recursion_depth {
                return Err(blte_error("recursive BLTE depth limit exceeded"));
            }
            decode_inner(payload, options, depth + 1)?
        }
        b'E' => {
            return Err(SanctuaryError::UnsupportedBuild(
                "encrypted BLTE chunks require a legitimate TACT key source and are not supported by OpenSanctuary".into(),
            ));
        }
        other => {
            return Err(blte_error(&format!(
                "unsupported BLTE chunk mode 0x{other:02x}"
            )));
        }
    };
    if let Some(expected_size) = expected_size
        && decoded.len() != expected_size
    {
        return Err(blte_error(&format!(
            "BLTE chunk decoded to {} bytes but table declares {expected_size}",
            decoded.len()
        )));
    }
    Ok(decoded)
}

fn bounded_copy(bytes: &[u8], limit: usize) -> Result<Vec<u8>, SanctuaryError> {
    if bytes.len() > limit {
        return Err(blte_error("raw BLTE chunk exceeds the output limit"));
    }
    Ok(bytes.to_vec())
}

fn decode_zlib(bytes: &[u8], limit: usize) -> Result<Vec<u8>, SanctuaryError> {
    let decoder = ZlibDecoder::new(bytes);
    let take_limit = (limit as u64).saturating_add(1);
    read_limited(decoder.take(take_limit), limit)
}

fn read_limited<R: Read>(mut reader: Take<R>, limit: usize) -> Result<Vec<u8>, SanctuaryError> {
    let mut output = Vec::new();
    reader
        .read_to_end(&mut output)
        .map_err(|error| blte_error(&format!("zlib decode failed: {error}")))?;
    if output.len() > limit {
        return Err(blte_error("zlib BLTE chunk exceeds the output limit"));
    }
    Ok(output)
}

fn read_u32_be(bytes: &[u8], offset: usize, label: &str) -> Result<u32, SanctuaryError> {
    let raw = bytes
        .get(offset..offset.saturating_add(4))
        .filter(|raw| raw.len() == 4)
        .ok_or_else(|| blte_error(&format!("truncated {label}")))?;
    Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u24_be(bytes: &[u8], offset: usize) -> Result<usize, SanctuaryError> {
    let raw = bytes
        .get(offset..offset.saturating_add(3))
        .filter(|raw| raw.len() == 3)
        .ok_or_else(|| blte_error("truncated BLTE chunk count"))?;
    Ok((usize::from(raw[0]) << 16) | (usize::from(raw[1]) << 8) | usize::from(raw[2]))
}

fn blte_error(detail: &str) -> SanctuaryError {
    SanctuaryError::ContentStoreUnreadable(format!("invalid BLTE: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;

    fn headerless(mode: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = b"BLTE\0\0\0\0".to_vec();
        bytes.push(mode);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn table(chunks: &[(u8, Vec<u8>, usize)]) -> Vec<u8> {
        let header_size = 12 + chunks.len() * CHUNK_DESCRIPTOR_BYTES;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BLTE_MAGIC);
        bytes.extend_from_slice(&(header_size as u32).to_be_bytes());
        bytes.push(0x0f);
        let count = chunks.len() as u32;
        bytes.extend_from_slice(&[(count >> 16) as u8, (count >> 8) as u8, count as u8]);
        let mut bodies = Vec::new();
        for (mode, payload, decoded_len) in chunks {
            let mut body = vec![*mode];
            body.extend_from_slice(payload);
            bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&(*decoded_len as u32).to_be_bytes());
            let digest = Md5::digest(&body);
            bytes.extend_from_slice(&digest);
            bodies.extend_from_slice(&body);
        }
        bytes.extend_from_slice(&bodies);
        bytes
    }

    #[test]
    fn decodes_headerless_and_table_raw_chunks() {
        assert_eq!(decode_blte(&headerless(b'N', b"hello")).unwrap(), b"hello");
        let bytes = table(&[(b'N', b"abc".to_vec(), 3), (b'N', b"def".to_vec(), 3)]);
        assert_eq!(decode_blte(&bytes).unwrap(), b"abcdef");
    }

    #[test]
    fn decodes_zlib_and_recursive_chunks() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"compressed-content").unwrap();
        let compressed = encoder.finish().unwrap();
        let zlib = table(&[(b'Z', compressed, "compressed-content".len())]);
        assert_eq!(decode_blte(&zlib).unwrap(), b"compressed-content");

        let nested = headerless(b'N', b"nested-content");
        let recursive = table(&[(b'F', nested, "nested-content".len())]);
        assert_eq!(decode_blte(&recursive).unwrap(), b"nested-content");
    }

    #[test]
    fn rejects_digest_mismatch_and_encrypted_chunks() {
        let mut bytes = table(&[(b'N', b"abc".to_vec(), 3)]);
        bytes[20] ^= 0xff;
        assert!(decode_blte(&bytes).is_err());
        let encrypted = headerless(b'E', b"ciphertext");
        assert!(matches!(
            decode_blte(&encrypted),
            Err(SanctuaryError::UnsupportedBuild(_))
        ));
    }

    #[test]
    fn enforces_output_and_recursion_limits() {
        let options = BlteDecodeOptions {
            max_output_bytes: 3,
            ..Default::default()
        };
        assert!(decode_blte_with_options(&headerless(b'N', b"four"), options).is_err());

        let nested = headerless(b'N', b"ok");
        let recursive = headerless(b'F', &nested);
        let options = BlteDecodeOptions {
            max_recursion_depth: 0,
            ..Default::default()
        };
        assert!(decode_blte_with_options(&recursive, options).is_err());
    }
}
