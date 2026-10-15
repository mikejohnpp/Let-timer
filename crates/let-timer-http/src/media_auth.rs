//! HMAC-SHA256 signature generation and verification for expiring media URLs.
//!
//! Because browser elements (`<img>`, `<audio>`, CSS `background-image`) cannot
//! send custom HTTP authorization headers, media assets are authorized via
//! short-lived, signed URLs containing an expiration timestamp (`exp`) and HMAC signature (`sig`).

use sha2::{Digest, Sha256};

/// Compute standard HMAC-SHA256 (RFC 2104).
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hash = Sha256::digest(key);
        key_block[..32].copy_from_slice(&hash);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key_block[i] ^ 0x36;
        opad[i] = key_block[i] ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// Constant-time byte slice comparison to prevent timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut res = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        res |= x ^ y;
    }
    res == 0
}

/// Sign a relative media path with an expiration timestamp.
pub fn sign_media_path(secret: &str, rel_path: &str, exp: u64) -> String {
    let clean_path = rel_path.trim_start_matches('/');
    let message = format!("{clean_path}|{exp}");
    let hmac_bytes = hmac_sha256(secret.as_bytes(), message.as_bytes());
    hmac_bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Build a fully signed media URL with TTL (default 3600 seconds / 1 hour).
pub fn sign_media_url(secret: &str, original_url: &str, ttl_secs: u64) -> String {
    if original_url.starts_with("http://") || original_url.starts_with("https://") {
        return original_url.to_string();
    }

    let prefix = "/api/v1/media/files/";
    let rel_path = if let Some(pos) = original_url.find(prefix) {
        &original_url[pos + prefix.len()..]
    } else {
        original_url.trim_start_matches('/')
    };

    let now_unix = chrono::Utc::now().timestamp() as u64;
    let exp = now_unix + ttl_secs;
    let sig = sign_media_path(secret, rel_path, exp);
    format!(
        "/api/v1/media/files/{}?exp={}&sig={}",
        rel_path.trim_start_matches('/'),
        exp,
        sig
    )
}

/// Verify that a media signature is valid and not expired.
pub fn verify_media_signature(
    secret: &str,
    rel_path: &str,
    exp: u64,
    sig: &str,
    now_unix: u64,
) -> bool {
    if now_unix > exp {
        return false;
    }
    let expected = sign_media_path(secret, rel_path, exp);
    constant_time_eq(expected.as_bytes(), sig.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_rfc4231_test_case_2() {
        // Test with "Jefe" and "what do ya want for nothing?"
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let hmac = hmac_sha256(key, data);
        let hex: String = hmac.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn test_signature_verification_valid_and_expired() {
        let secret = "test_signing_secret_1234567890123456";
        let path = "images/wallpaper.png";
        let exp = 1000u64;

        let sig = sign_media_path(secret, path, exp);

        // Valid signature before expiration
        assert!(verify_media_signature(secret, path, exp, &sig, 500));
        assert!(verify_media_signature(secret, path, exp, &sig, 1000));

        // Expired signature
        assert!(!verify_media_signature(secret, path, exp, &sig, 1001));

        // Tampered signature
        assert!(!verify_media_signature(
            secret,
            path,
            exp,
            "invalid_sig",
            500
        ));

        // Tampered path
        assert!(!verify_media_signature(
            secret,
            "images/other.png",
            exp,
            &sig,
            500
        ));

        // Tampered expiration
        assert!(!verify_media_signature(secret, path, exp + 1, &sig, 500));
    }
}
