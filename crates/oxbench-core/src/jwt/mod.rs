//! JWT Decoder logic: split and decode a token, explain its claims and timing, and verify its
//! signature with a secret, a PEM public key or a JWK.

mod claims;
mod decode;
mod verify;

pub use claims::{ClaimRow, TokenTiming, check_timing, describe_claims};
pub use decode::{DecodedJwt, Payload, decode_jwt};
pub use verify::{KeyKind, SecretEncoding, key_kind_for, verify_signature};
