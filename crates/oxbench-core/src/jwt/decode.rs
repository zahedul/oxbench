use base64::Engine;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use serde_json::Value;

use crate::{CoreError, CoreResult};

/// `Base64URL` as JWTs use it. Padding is not part of the format, but some tools add it anyway.
pub(super) const BASE64_URL: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    Json(Value),
    /// A signed token may carry any bytes; this is the payload shown as text.
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedJwt {
    pub header: Value,
    pub payload: Payload,
    /// The `alg` header value, when present.
    pub algorithm: Option<String>,
    /// The token with whitespace and any `Bearer ` prefix removed.
    pub token: String,
}

impl DecodedJwt {
    /// `header.payload`: the bytes the signature covers.
    pub fn signing_input(&self) -> &str {
        self.token
            .rsplit_once('.')
            .map_or(self.token.as_str(), |(input, _)| input)
    }

    pub fn signature(&self) -> &str {
        self.token
            .rsplit_once('.')
            .map_or("", |(_, signature)| signature)
    }

    pub fn has_signature(&self) -> bool {
        !self.signature().is_empty()
    }
}

/// Decodes a compact JWS (`header.payload.signature`). The signature is not checked here.
pub fn decode_jwt(input: &str) -> CoreResult<DecodedJwt> {
    let token = normalize_token(input);
    if token.is_empty() {
        return Err(invalid("Paste a JWT, such as eyJhbGciOi…"));
    }
    let parts: Vec<&str> = token.split('.').collect();
    let [header_part, payload_part, _signature] = parts.as_slice() else {
        if parts.len() == 5 {
            return Err(invalid(
                "This is an encrypted token (JWE, five parts). It can only be read with the decryption key.",
            ));
        }
        return Err(invalid(&format!(
            "A JWT has three parts separated by dots; this has {}.",
            parts.len()
        )));
    };
    let header_bytes = decode_part(header_part, "header")?;
    let header: Value = serde_json::from_slice(&header_bytes)
        .map_err(|error| invalid(&format!("The header is not valid JSON: {error}")))?;
    let Value::Object(header_fields) = &header else {
        return Err(invalid("The header must be a JSON object."));
    };
    let algorithm = header_fields
        .get("alg")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let payload_bytes = decode_part(payload_part, "payload")?;
    Ok(DecodedJwt {
        payload: read_payload(&payload_bytes),
        header,
        algorithm,
        token,
    })
}

fn normalize_token(input: &str) -> String {
    let trimmed = input.trim();
    let without_scheme = trimmed
        .get(..7)
        .filter(|prefix| prefix.eq_ignore_ascii_case("bearer "))
        .map_or(trimmed, |_| &trimmed[7..]);
    without_scheme
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn decode_part(part: &str, name: &str) -> CoreResult<Vec<u8>> {
    BASE64_URL
        .decode(part)
        .map_err(|error| invalid(&format!("The {name} is not valid Base64URL: {error}.")))
}

fn read_payload(bytes: &[u8]) -> Payload {
    if let Ok(value) = serde_json::from_slice(bytes) {
        return Payload::Json(value);
    }
    Payload::Text(String::from_utf8_lossy(bytes).into_owned())
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The well-known example token from RFC 7519 tooling (HS256, secret `your-256-bit-secret`).
    const EXAMPLE_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
        eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
        SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    fn decodes_header_and_payload() {
        let decoded = decode_jwt(EXAMPLE_TOKEN).unwrap();
        assert_eq!(decoded.header, json!({"alg": "HS256", "typ": "JWT"}));
        assert_eq!(
            decoded.payload,
            Payload::Json(json!({"sub": "1234567890", "name": "John Doe", "iat": 1_516_239_022}))
        );
        assert_eq!(decoded.algorithm.as_deref(), Some("HS256"));
        assert_eq!(
            decoded.signature(),
            "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
        );
        assert!(decoded.signing_input().ends_with("MjM5MDIyfQ"));
    }

    #[test]
    fn accepts_a_bearer_prefix_and_line_breaks() {
        let pasted = format!("Bearer {}\n", EXAMPLE_TOKEN.replace('.', ".\n  "));
        assert_eq!(decode_jwt(&pasted).unwrap().token, EXAMPLE_TOKEN);
    }

    #[test]
    fn keeps_header_order_and_reads_text_payloads() {
        let decoded = decode_jwt("eyJ0eXAiOiJKV1QiLCJhbGciOiJub25lIn0.aGVsbG8.").unwrap();
        assert_eq!(decoded.header.to_string(), r#"{"typ":"JWT","alg":"none"}"#);
        assert_eq!(decoded.payload, Payload::Text("hello".to_owned()));
        assert!(!decoded.has_signature());
    }

    #[test]
    fn explains_malformed_tokens() {
        let message = |input: &str| decode_jwt(input).unwrap_err().to_string();
        assert!(message("").contains("Paste a JWT"));
        assert!(message("abc.def").contains("this has 2"));
        assert!(message("a.b.c.d.e").contains("encrypted token (JWE"));
        assert!(message("!!!.e30.x").contains("header is not valid Base64URL"));
        assert!(message("bm90IGpzb24.e30.x").contains("header is not valid JSON"));
        assert!(message("WzFd.e30.x").contains("must be a JSON object"));
        assert!(message("e30.!!!.x").contains("payload is not valid Base64URL"));
    }
}
