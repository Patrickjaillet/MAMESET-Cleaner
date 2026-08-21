use std::io::{self, Read};

use sha1::{Digest, Sha1};

pub fn crc32_of_bytes(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

pub fn crc32_of_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

/// Result of hashing a stream once: CRC32 is always computed (cheap, and
/// every DAT format provides it), SHA1 only when `compute_sha1` is
/// requested — both from the same single read pass, so enabling SHA1 never
/// costs a second decompression of the same data.
pub struct StreamDigest {
    pub crc32: u32,
    pub sha1: Option<String>,
}

pub fn hash_reader<R: Read + ?Sized>(reader: &mut R, compute_sha1: bool) -> io::Result<StreamDigest> {
    let mut crc_hasher = crc32fast::Hasher::new();
    let mut sha1_hasher = compute_sha1.then(Sha1::new);
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        crc_hasher.update(&buffer[..bytes_read]);
        if let Some(hasher) = sha1_hasher.as_mut() {
            hasher.update(&buffer[..bytes_read]);
        }
    }

    Ok(StreamDigest {
        crc32: crc_hasher.finalize(),
        sha1: sha1_hasher.map(|hasher| format!("{:x}", hasher.finalize())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_known_crc32_of_bytes() {
        assert_eq!(crc32_of_bytes(b""), 0);
        assert_eq!(crc32_of_bytes(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn computes_same_crc32_through_a_reader() {
        let data = b"MAMESET Cleaner";
        let expected = crc32_of_bytes(data);
        let mut cursor = io::Cursor::new(data);
        assert_eq!(crc32_of_reader(&mut cursor).unwrap(), expected);
    }

    #[test]
    fn hash_reader_skips_sha1_when_not_requested() {
        let data = b"MAMESET Cleaner";
        let mut cursor = io::Cursor::new(data);
        let digest = hash_reader(&mut cursor, false).unwrap();
        assert_eq!(digest.crc32, crc32_of_bytes(data));
        assert!(digest.sha1.is_none());
    }

    #[test]
    fn hash_reader_computes_known_sha1_when_requested() {
        // sha1("123456789") is a well-known test vector.
        let data = b"123456789";
        let mut cursor = io::Cursor::new(data);
        let digest = hash_reader(&mut cursor, true).unwrap();
        assert_eq!(digest.crc32, crc32_of_bytes(data));
        assert_eq!(
            digest.sha1.as_deref(),
            Some("f7c3bc1d808e04732adf679965ccc34ca7ae3441")
        );
    }
}
