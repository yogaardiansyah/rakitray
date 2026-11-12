use crate::store::StoreError;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Streams a file in 64 KB chunks and verifies its SHA-256 hash.
/// On mismatch, immediately deletes the file from disk and returns an error.
pub fn verify_and_clean_file_sha256(path: &Path, expected_hex: &str) -> Result<(), StoreError> {
    if !path.exists() {
        return Err(StoreError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found for verification: {}", path.display()),
        )));
    }

    let mut file = fs::File::open(path).map_err(StoreError::Io)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(StoreError::Io)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    let calculated_hex = format!("{:x}", result);
    let expected_lower = expected_hex.trim().to_ascii_lowercase();

    // Constant-time-like comparison on hex strings
    if calculated_hex.len() != expected_lower.len()
        || calculated_hex
            .bytes()
            .zip(expected_lower.bytes())
            .fold(0, |acc, (a, b)| acc | (a ^ b))
            != 0
    {
        // Checksum mismatch -> Delete corrupt file immediately
        let _ = fs::remove_file(path);
        return Err(StoreError::ChecksumMismatch {
            expected: expected_lower,
            actual: calculated_hex,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_verify_valid_sha256() {
        let temp_dir = std::env::temp_dir().join("rakit_test_sha_valid");
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("test.bin");

        let data = b"hello rakitray store downloader";
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(data).unwrap();
        drop(file);

        // sha256 of "hello rakitray store downloader"
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = format!("{:x}", hasher.finalize());

        let res = verify_and_clean_file_sha256(&file_path, &expected);
        assert!(res.is_ok());
        assert!(file_path.exists());

        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_verify_mismatch_deletes_file() {
        let temp_dir = std::env::temp_dir().join("rakit_test_sha_invalid");
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("corrupt.bin");

        let data = b"corrupted payload";
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(data).unwrap();
        drop(file);

        let fake_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        let res = verify_and_clean_file_sha256(&file_path, fake_sha);

        assert!(matches!(res, Err(StoreError::ChecksumMismatch { .. })));
        // File MUST be deleted
        assert!(!file_path.exists());
    }
}
