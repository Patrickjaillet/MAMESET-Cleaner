use std::io::{self, Read};

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
}
