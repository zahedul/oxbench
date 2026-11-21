use std::str::FromStr;

use base64::Engine;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{Algorithm, DecodingKey};
use serde_json::Value;

use super::decode::{BASE64_URL, DecodedJwt};
use crate::{CoreError, CoreResult};

const BASE64_STANDARD: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// What a token's algorithm needs to check its signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// HS256, HS384, HS512.
    Secret,
    /// `RS*`, `PS*`, `ES256`, `ES384` and `EdDSA`: a PEM public key or a JWK.
    PublicKey,
}

/// How an HMAC secret is typed in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SecretEncoding {
    #[default]
    Text,
    Base64,
}

/// `None` for `none` and for algorithms this tool cannot verify.
pub fn key_kind_for(algorithm: &str) -> Option<KeyKind> {
    let algorithm = Algorithm::from_str(algorithm).ok()?;
    match algorithm.family() {
        jsonwebtoken::AlgorithmFamily::Hmac => Some(KeyKind::Secret),
        _ => Some(KeyKind::PublicKey),
    }
}

/// Checks the signature only; expiry and other claims are reported separately.
/// `Ok(false)` means the key is usable but the signature does not match.
pub fn verify_signature(
    token: &DecodedJwt,
    key_text: &str,
    encoding: SecretEncoding,
) -> CoreResult<bool> {
    let algorithm = read_algorithm(token)?;
    if !token.has_signature() {
        return Err(invalid("The token has no signature."));
    }
    if key_text.trim().is_empty() {
        return Err(invalid(match algorithm.family() {
            jsonwebtoken::AlgorithmFamily::Hmac => "Enter the secret to check the signature.",
            _ => "Paste the public key (PEM or JWK) to check the signature.",
        }));
    }
    let key = build_key(token, algorithm, key_text, encoding)?;
    jsonwebtoken::crypto::verify(
        token.signature(),
        token.signing_input().as_bytes(),
        &key,
        algorithm,
    )
    .map_err(|error| describe_key_error(&error, algorithm))
}

fn read_algorithm(token: &DecodedJwt) -> CoreResult<Algorithm> {
    let Some(name) = token.algorithm.as_deref() else {
        return Err(invalid(
            "The header has no \"alg\", so the algorithm is unknown.",
        ));
    };
    if name.eq_ignore_ascii_case("none") {
        return Err(invalid(
            "This token is unsigned (alg \"none\"). There is no signature to check.",
        ));
    }
    Algorithm::from_str(name).map_err(|_| {
        invalid(&format!(
            "{name} cannot be verified here. Supported: HS256/384/512, RS256/384/512, \
             PS256/384/512, ES256, ES384 and EdDSA."
        ))
    })
}

fn build_key(
    token: &DecodedJwt,
    algorithm: Algorithm,
    key_text: &str,
    encoding: SecretEncoding,
) -> CoreResult<DecodingKey> {
    let trimmed = key_text.trim();
    if trimmed.starts_with('{') {
        let jwk = select_jwk(token, trimmed)?;
        return DecodingKey::from_jwk(&jwk).map_err(|error| describe_key_error(&error, algorithm));
    }
    let key = match algorithm.family() {
        jsonwebtoken::AlgorithmFamily::Hmac => {
            return DecodingKey::from_secret(&decode_secret(key_text, encoding)?)
                .map_err(|error| describe_key_error(&error, algorithm));
        }
        jsonwebtoken::AlgorithmFamily::Rsa => DecodingKey::from_rsa_pem(trimmed.as_bytes()),
        jsonwebtoken::AlgorithmFamily::Ec => DecodingKey::from_ec_pem(trimmed.as_bytes()),
        jsonwebtoken::AlgorithmFamily::Ed => DecodingKey::from_ed_pem(trimmed.as_bytes()),
    };
    key.map_err(|error| describe_key_error(&error, algorithm))
}

/// Text secrets are used byte for byte, spaces included.
fn decode_secret(key_text: &str, encoding: SecretEncoding) -> CoreResult<Vec<u8>> {
    match encoding {
        SecretEncoding::Text => Ok(key_text.as_bytes().to_vec()),
        SecretEncoding::Base64 => {
            let trimmed = key_text.trim();
            BASE64_STANDARD
                .decode(trimmed)
                .or_else(|_| BASE64_URL.decode(trimmed))
                .map_err(|_| invalid("The secret is not valid Base64."))
        }
    }
}

/// Accepts a single JWK or a JWK Set. From a set, the key matching the header's `kid` is used.
fn select_jwk(token: &DecodedJwt, key_text: &str) -> CoreResult<Jwk> {
    let value: Value = serde_json::from_str(key_text).map_err(|error| {
        invalid(&format!(
            "The key looks like a JWK but is not valid JSON: {error}"
        ))
    })?;
    if value.get("keys").is_none() {
        return serde_json::from_value(value)
            .map_err(|error| invalid(&format!("Not a valid JWK: {error}")));
    }
    let set: JwkSet = serde_json::from_value(value)
        .map_err(|error| invalid(&format!("Not a valid JWK Set: {error}")))?;
    let kid = token.header.get("kid").and_then(Value::as_str);
    if let Some(kid) = kid {
        return set
            .find(kid)
            .cloned()
            .ok_or_else(|| invalid(&format!("The JWK Set has no key with kid \"{kid}\".")));
    }
    match set.keys.as_slice() {
        [only] => Ok(only.clone()),
        _ => Err(invalid(
            "The token has no \"kid\", so a JWK Set with several keys is ambiguous. Paste one key.",
        )),
    }
}

fn describe_key_error(error: &jsonwebtoken::errors::Error, algorithm: Algorithm) -> CoreError {
    let message = match error.kind() {
        ErrorKind::InvalidRsaKey(detail) => format!("Not a usable RSA public key: {detail}."),
        ErrorKind::InvalidEcdsaKey => "Not a usable EC public key for this curve.".to_owned(),
        ErrorKind::InvalidEddsaKey => "Not a usable Ed25519 public key.".to_owned(),
        ErrorKind::InvalidKeyFormat => format!(
            "The key is not in a format {algorithm:?} accepts. Use a PEM public key or a JWK."
        ),
        ErrorKind::InvalidAlgorithm => {
            format!("This key's type does not match the token's algorithm ({algorithm:?}).")
        }
        ErrorKind::Base64(_) => "The signature or key is not valid Base64.".to_owned(),
        _ => format!("Could not check the signature: {error}."),
    };
    CoreError::InvalidInput(message)
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::decode_jwt;
    use jsonwebtoken::EncodingKey;

    // Throwaway keys generated for these tests only.
    const RSA_PRIVATE: &str = include_str!("fixtures/rsa-private.pem");
    const RSA_PUBLIC: &str = include_str!("fixtures/rsa-public.pem");
    const EC_PRIVATE: &str = include_str!("fixtures/ec-private.pem");
    const EC_PUBLIC: &str = include_str!("fixtures/ec-public.pem");
    const ED_PRIVATE: &str = include_str!("fixtures/ed-private.pem");
    const ED_PUBLIC: &str = include_str!("fixtures/ed-public.pem");

    const EXAMPLE_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
        eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
        SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    fn sign(algorithm: Algorithm, key: &EncodingKey, header: &str) -> DecodedJwt {
        let header = BASE64_URL.encode(header);
        let payload = BASE64_URL.encode(r#"{"sub":"42"}"#);
        let input = format!("{header}.{payload}");
        let signature = jsonwebtoken::crypto::sign(input.as_bytes(), key, algorithm).unwrap();
        decode_jwt(&format!("{input}.{signature}")).unwrap()
    }

    fn verify(token: &DecodedJwt, key: &str) -> CoreResult<bool> {
        verify_signature(token, key, SecretEncoding::Text)
    }

    #[test]
    fn verifies_hmac_with_a_text_or_base64_secret() {
        let token = decode_jwt(EXAMPLE_TOKEN).unwrap();
        assert!(verify(&token, "your-256-bit-secret").unwrap());
        assert!(!verify(&token, "wrong secret").unwrap());
        let base64_secret = BASE64_STANDARD.encode("your-256-bit-secret");
        assert!(verify_signature(&token, &base64_secret, SecretEncoding::Base64).unwrap());
        assert!(verify_signature(&token, "***", SecretEncoding::Base64).is_err());
    }

    #[test]
    fn verifies_rsa_pkcs1_and_pss_signatures() {
        let key = EncodingKey::from_rsa_pem(RSA_PRIVATE.as_bytes()).unwrap();
        for (algorithm, header) in [
            (Algorithm::RS256, r#"{"alg":"RS256"}"#),
            (Algorithm::PS512, r#"{"alg":"PS512"}"#),
        ] {
            let token = sign(algorithm, &key, header);
            assert!(verify(&token, RSA_PUBLIC).unwrap(), "{algorithm:?}");
        }
    }

    #[test]
    fn verifies_ecdsa_and_eddsa_signatures() {
        let ec_key = EncodingKey::from_ec_pem(EC_PRIVATE.as_bytes()).unwrap();
        let ec_token = sign(Algorithm::ES256, &ec_key, r#"{"alg":"ES256"}"#);
        assert!(verify(&ec_token, EC_PUBLIC).unwrap());

        let eddsa_key = EncodingKey::from_ed_pem(ED_PRIVATE.as_bytes()).unwrap();
        let eddsa_token = sign(Algorithm::EdDSA, &eddsa_key, r#"{"alg":"EdDSA"}"#);
        assert!(verify(&eddsa_token, ED_PUBLIC).unwrap());
        assert!(verify(&eddsa_token, EC_PUBLIC).is_err());
    }

    #[test]
    fn a_tampered_payload_fails() {
        let key = EncodingKey::from_ec_pem(EC_PRIVATE.as_bytes()).unwrap();
        let token = sign(Algorithm::ES256, &key, r#"{"alg":"ES256"}"#);
        let (_, signature) = token.token.rsplit_once('.').unwrap();
        let header = token.token.split('.').next().unwrap();
        let forged_payload = BASE64_URL.encode(r#"{"sub":"admin"}"#);
        let forged = decode_jwt(&format!("{header}.{forged_payload}.{signature}")).unwrap();
        assert!(!verify(&forged, EC_PUBLIC).unwrap());
    }

    #[test]
    fn picks_the_matching_key_from_a_jwk_set() {
        let key = EncodingKey::from_rsa_pem(RSA_PRIVATE.as_bytes()).unwrap();
        let token = sign(Algorithm::RS256, &key, r#"{"alg":"RS256","kid":"main"}"#);
        let mut jwk = Jwk::from_encoding_key(&key, Algorithm::RS256).unwrap();
        jwk.common.key_id = Some("main".to_owned());
        let single = serde_json::to_string(&jwk).unwrap();
        assert!(verify(&token, &single).unwrap());
        let set = format!(r#"{{"keys":[{single}]}}"#);
        assert!(verify(&token, &set).unwrap());
        let other_set = set.replace("\"main\"", "\"other\"");
        let error = verify(&token, &other_set).unwrap_err();
        assert!(error.to_string().contains("no key with kid \"main\""));
    }

    #[test]
    fn explains_why_a_token_cannot_be_checked() {
        let message = |token: &str, key: &str| {
            verify(&decode_jwt(token).unwrap(), key)
                .unwrap_err()
                .to_string()
        };
        let unsigned = "eyJhbGciOiJub25lIn0.e30.";
        assert!(message(unsigned, "x").contains("unsigned"));
        assert!(message(EXAMPLE_TOKEN, "  ").contains("Enter the secret"));
        let rs256 = "eyJhbGciOiJSUzI1NiJ9.e30.c2ln";
        assert!(message(rs256, "not a pem").contains("RS256"));
        let es512 = "eyJhbGciOiJFUzUxMiJ9.e30.c2ln";
        assert!(message(es512, "x").contains("ES512 cannot be verified here"));
    }

    #[test]
    fn maps_algorithms_to_key_kinds() {
        assert_eq!(key_kind_for("HS512"), Some(KeyKind::Secret));
        assert_eq!(key_kind_for("ES256"), Some(KeyKind::PublicKey));
        assert_eq!(key_kind_for("none"), None);
    }
}
