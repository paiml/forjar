//! forjar#1000: the GitHub App JWT (RS256), signed on the control host.
//!
//! Signing uses the `openssl` crate that the default `tls` feature already
//! brings in, so this adds no dependency. A build without `tls` cannot sign and
//! says so; it never falls back to an unsigned or weaker token.
//!
//! No error here carries key material: a parse failure reports only that the
//! value is not a PEM private key.

use base64::Engine;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// `header.payload.signature` for `app_id`, valid from 60 s before `now` (clock
/// skew) to 9 minutes after it (GitHub allows at most 10).
pub(super) fn app_jwt(app_id: u64, pem: &str, now: u64) -> Result<String, String> {
    let header = b64(br#"{"alg":"RS256","typ":"JWT"}"#);
    let payload = b64(format!(
        r#"{{"iat":{},"exp":{},"iss":"{app_id}"}}"#,
        now.saturating_sub(60),
        now + 540
    )
    .as_bytes());
    let signing_input = format!("{header}.{payload}");
    let signature = sign(pem, signing_input.as_bytes())?;
    Ok(format!("{signing_input}.{}", b64(&signature)))
}

#[cfg(feature = "tls")]
fn sign(pem: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    use openssl::{hash::MessageDigest, pkey::PKey, sign::Signer};
    let key = PKey::private_key_from_pem(pem.as_bytes())
        .map_err(|_| "github_runner: app.private_key is not a PEM private key".to_string())?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key)
        .map_err(|e| format!("github_runner: cannot start RS256 signing: {e}"))?;
    signer
        .update(data)
        .map_err(|e| format!("github_runner: RS256 signing failed: {e}"))?;
    signer
        .sign_to_vec()
        .map_err(|e| format!("github_runner: RS256 signing failed: {e}"))
}

#[cfg(not(feature = "tls"))]
fn sign(_pem: &str, _data: &[u8]) -> Result<Vec<u8>, String> {
    Err("github_runner: RS256 signing needs the `tls` feature, which this build lacks".to_string())
}

#[cfg(all(test, feature = "tls"))]
mod tests {
    use super::*;
    use openssl::{hash::MessageDigest, pkey::PKey, rsa::Rsa, sign::Verifier};

    fn decode(s: &str) -> Vec<u8> {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .unwrap()
    }

    #[test]
    fn jwt_is_a_verifiable_rs256_token_with_the_app_id() {
        let rsa = Rsa::generate(2048).unwrap();
        let pem = String::from_utf8(rsa.private_key_to_pem().unwrap()).unwrap();
        let jwt = app_jwt(123456, &pem, 1_000_000).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(decode(parts[0]), br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = String::from_utf8(decode(parts[1])).unwrap();
        assert_eq!(payload, r#"{"iat":999940,"exp":1000540,"iss":"123456"}"#);
        let key = PKey::from_rsa(rsa).unwrap();
        let mut v = Verifier::new(MessageDigest::sha256(), &key).unwrap();
        v.update(format!("{}.{}", parts[0], parts[1]).as_bytes())
            .unwrap();
        assert!(v.verify(&decode(parts[2])).unwrap());
    }

    #[test]
    fn a_bad_key_is_refused_without_echoing_it() {
        let err = app_jwt(1, "not-a-key-SECRETMARKER", 10).unwrap_err();
        assert!(err.contains("not a PEM private key"));
        assert!(!err.contains("SECRETMARKER"));
    }
}
