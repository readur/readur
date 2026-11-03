//! Hex rendering for digests.
//!
//! Content hashes are persisted (duplicate detection compares them against
//! stored `file_hash` values) and reused as ETags, so the encoding must not
//! drift: lowercase hex, two characters per byte. sha2 0.11 digests no longer
//! implement `LowerHex`, hence this helper instead of `format!("{:x}", ..)`.

use std::fmt::Write;

/// Lowercase hex encoding of `bytes`, two characters per byte.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::to_hex;
    use sha2::{Digest, Sha256};

    #[test]
    fn matches_the_published_sha256_test_vector() {
        // FIPS 180-2, appendix B.1: SHA-256("abc").
        assert_eq!(
            to_hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn pads_each_byte_to_two_lowercase_digits() {
        assert_eq!(to_hex(&[0x00, 0x0a, 0xff]), "000aff");
        assert_eq!(to_hex(&[]), "");
    }
}
