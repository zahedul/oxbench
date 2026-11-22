//! Key pairs for SSH, TLS and signing: Ed25519, ECDSA and RSA, in PEM and OpenSSH formats.

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use p256::elliptic_curve::Generate;
use rand::CryptoRng;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use sha2::{Digest, Sha256};

use crate::{CoreError, CoreResult};

/// OpenSSH wraps its key files at 70 characters a line.
const OPENSSH_LINE_WIDTH: usize = 70;
/// Unencrypted OpenSSH keys pad their private section to this block size.
const OPENSSH_BLOCK_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyAlgorithm {
    #[default]
    Ed25519,
    EcdsaP256,
    EcdsaP384,
    Rsa2048,
    Rsa3072,
    Rsa4096,
}

impl KeyAlgorithm {
    pub const ALL: [KeyAlgorithm; 6] = [
        KeyAlgorithm::Ed25519,
        KeyAlgorithm::EcdsaP256,
        KeyAlgorithm::EcdsaP384,
        KeyAlgorithm::Rsa2048,
        KeyAlgorithm::Rsa3072,
        KeyAlgorithm::Rsa4096,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            KeyAlgorithm::Ed25519 => "Ed25519",
            KeyAlgorithm::EcdsaP256 => "ECDSA P-256",
            KeyAlgorithm::EcdsaP384 => "ECDSA P-384",
            KeyAlgorithm::Rsa2048 => "RSA 2048",
            KeyAlgorithm::Rsa3072 => "RSA 3072",
            KeyAlgorithm::Rsa4096 => "RSA 4096",
        }
    }

    /// Finding large primes takes noticeable time, longer the bigger the key.
    pub const fn is_slow(self) -> bool {
        matches!(
            self,
            KeyAlgorithm::Rsa2048 | KeyAlgorithm::Rsa3072 | KeyAlgorithm::Rsa4096
        )
    }

    /// The file name `ssh-keygen` would use.
    pub const fn default_file_name(self) -> &'static str {
        match self {
            KeyAlgorithm::Ed25519 => "id_ed25519",
            KeyAlgorithm::EcdsaP256 | KeyAlgorithm::EcdsaP384 => "id_ecdsa",
            KeyAlgorithm::Rsa2048 | KeyAlgorithm::Rsa3072 | KeyAlgorithm::Rsa4096 => "id_rsa",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPair {
    pub algorithm: KeyAlgorithm,
    /// `-----BEGIN PRIVATE KEY-----`, unencrypted PKCS #8.
    pub private_pem: String,
    /// `-----BEGIN OPENSSH PRIVATE KEY-----`, unencrypted, as `ssh-keygen -N ""` writes it.
    pub private_openssh: String,
    /// `-----BEGIN PUBLIC KEY-----`, `SubjectPublicKeyInfo`.
    pub public_pem: String,
    /// One line, as in `authorized_keys`.
    pub public_openssh: String,
    /// `SHA256:…`, as `ssh-keygen -l` prints it.
    pub fingerprint: String,
}

/// The parts of a key OpenSSH stores, written in its wire format.
struct OpensshKey {
    public_blob: Vec<u8>,
    /// Everything after the key type in the private section.
    private_fields: Vec<u8>,
    type_name: &'static str,
}

pub fn generate_key_pair<R: CryptoRng + ?Sized>(
    algorithm: KeyAlgorithm,
    comment: &str,
    rng: &mut R,
) -> CoreResult<KeyPair> {
    let (private_pem, public_pem, openssh) = match algorithm {
        KeyAlgorithm::Ed25519 => generate_ed25519(rng)?,
        KeyAlgorithm::EcdsaP256 => generate_p256(rng)?,
        KeyAlgorithm::EcdsaP384 => generate_p384(rng)?,
        KeyAlgorithm::Rsa2048 => generate_rsa(rng, 2048)?,
        KeyAlgorithm::Rsa3072 => generate_rsa(rng, 3072)?,
        KeyAlgorithm::Rsa4096 => generate_rsa(rng, 4096)?,
    };
    let check = rng.next_u32();
    Ok(KeyPair {
        algorithm,
        private_pem,
        public_pem,
        public_openssh: format_openssh_public(&openssh, comment),
        fingerprint: format_fingerprint(&openssh.public_blob),
        private_openssh: format_openssh_private(&openssh, comment, check),
    })
}

type GeneratedKey = (String, String, OpensshKey);

fn generate_ed25519<R: CryptoRng + ?Sized>(rng: &mut R) -> CoreResult<GeneratedKey> {
    let signing_key = ed25519_dalek::SigningKey::generate(rng);
    let verifying_key = signing_key.verifying_key();
    let private_pem = signing_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|error| encoding_error(&error))?;
    let public_pem = verifying_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|error| encoding_error(&error))?;
    let type_name = "ssh-ed25519";
    let public_key = verifying_key.to_bytes();
    let mut public_blob = WireWriter::default();
    public_blob.put_string(type_name.as_bytes());
    public_blob.put_string(&public_key);
    let mut private_fields = WireWriter::default();
    private_fields.put_string(&public_key);
    private_fields.put_string(&signing_key.to_keypair_bytes());
    Ok((
        private_pem.to_string(),
        public_pem,
        OpensshKey {
            public_blob: public_blob.0,
            private_fields: private_fields.0,
            type_name,
        },
    ))
}

/// The two ECDSA curves differ only in their types and names, so one macro covers both.
macro_rules! generate_ecdsa {
    ($name:ident, $curve:ident, $type_name:literal, $curve_name:literal) => {
        fn $name<R: CryptoRng + ?Sized>(rng: &mut R) -> CoreResult<GeneratedKey> {
            let secret_key = <$curve::SecretKey as Generate>::generate_from_rng(rng);
            let public_key = secret_key.public_key();
            let private_pem = secret_key
                .to_pkcs8_pem(LineEnding::LF)
                .map_err(|error| encoding_error(&error))?;
            let public_pem = public_key
                .to_public_key_pem(LineEnding::LF)
                .map_err(|error| encoding_error(&error))?;
            let point = public_key.to_sec1_bytes();
            let mut public_blob = WireWriter::default();
            public_blob.put_string($type_name.as_bytes());
            public_blob.put_string($curve_name.as_bytes());
            public_blob.put_string(&point);
            let mut private_fields = WireWriter::default();
            private_fields.put_string($curve_name.as_bytes());
            private_fields.put_string(&point);
            private_fields.put_mpint(&secret_key.to_bytes());
            Ok((
                private_pem.to_string(),
                public_pem,
                OpensshKey {
                    public_blob: public_blob.0,
                    private_fields: private_fields.0,
                    type_name: $type_name,
                },
            ))
        }
    };
}

generate_ecdsa!(generate_p256, p256, "ecdsa-sha2-nistp256", "nistp256");
generate_ecdsa!(generate_p384, p384, "ecdsa-sha2-nistp384", "nistp384");

fn generate_rsa<R: CryptoRng + ?Sized>(rng: &mut R, bits: usize) -> CoreResult<GeneratedKey> {
    let private_key = rsa::PrivateKey::new(rng, bits)
        .map_err(|error| CoreError::InvalidInput(format!("Couldn't generate the key: {error}.")))?;
    let public_key = private_key.to_public_key();
    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|error| encoding_error(&error))?;
    let public_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|error| encoding_error(&error))?;
    let [p, q] = private_key.primes() else {
        return Err(CoreError::InvalidInput(
            "The generated RSA key doesn't have two primes.".to_owned(),
        ));
    };
    let Some(q_inverse) = private_key.crt_coefficient() else {
        return Err(CoreError::InvalidInput(
            "The generated RSA key has no CRT coefficient.".to_owned(),
        ));
    };
    let modulus = private_key.n().to_be_bytes_trimmed_vartime();
    let exponent = private_key.e().to_be_bytes_trimmed_vartime();
    let type_name = "ssh-rsa";
    let mut public_blob = WireWriter::default();
    public_blob.put_string(type_name.as_bytes());
    public_blob.put_mpint(&exponent);
    public_blob.put_mpint(&modulus);
    let mut private_fields = WireWriter::default();
    private_fields.put_mpint(&modulus);
    private_fields.put_mpint(&exponent);
    private_fields.put_mpint(&private_key.d().to_be_bytes_trimmed_vartime());
    private_fields.put_mpint(&q_inverse.to_be_bytes_trimmed_vartime());
    private_fields.put_mpint(&p.to_be_bytes_trimmed_vartime());
    private_fields.put_mpint(&q.to_be_bytes_trimmed_vartime());
    Ok((
        private_pem.to_string(),
        public_pem,
        OpensshKey {
            public_blob: public_blob.0,
            private_fields: private_fields.0,
            type_name,
        },
    ))
}

fn format_openssh_public(key: &OpensshKey, comment: &str) -> String {
    let encoded = STANDARD.encode(&key.public_blob);
    let comment = comment.trim();
    if comment.is_empty() {
        return format!("{} {encoded}", key.type_name);
    }
    format!("{} {encoded} {comment}", key.type_name)
}

fn format_fingerprint(public_blob: &[u8]) -> String {
    let digest = Sha256::digest(public_blob);
    format!("SHA256:{}", STANDARD_NO_PAD.encode(digest))
}

/// The `openssh-key-v1` container: unencrypted, one key, with a random check number that
/// appears twice so readers can tell a wrong passphrase from a corrupt file.
fn format_openssh_private(key: &OpensshKey, comment: &str, check: u32) -> String {
    let mut private_section = WireWriter::default();
    private_section.put_u32(check);
    private_section.put_u32(check);
    private_section.put_string(key.type_name.as_bytes());
    private_section.0.extend_from_slice(&key.private_fields);
    private_section.put_string(comment.trim().as_bytes());
    let mut padding = 1_u8;
    while !private_section.0.len().is_multiple_of(OPENSSH_BLOCK_SIZE) {
        private_section.0.push(padding);
        padding += 1;
    }

    let mut container = WireWriter(b"openssh-key-v1\0".to_vec());
    container.put_string(b"none");
    container.put_string(b"none");
    container.put_string(b"");
    container.put_u32(1);
    container.put_string(&key.public_blob);
    container.put_string(&private_section.0);

    let encoded = STANDARD.encode(&container.0);
    let mut pem = String::from("-----BEGIN OPENSSH PRIVATE KEY-----\n");
    for line in encoded.as_bytes().chunks(OPENSSH_LINE_WIDTH) {
        pem.push_str(&String::from_utf8_lossy(line));
        pem.push('\n');
    }
    pem.push_str("-----END OPENSSH PRIVATE KEY-----\n");
    pem
}

fn encoding_error(error: &impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("Couldn't encode the key: {error}."))
}

/// SSH's binary encoding (RFC 4251): big-endian lengths before strings and integers.
#[derive(Default)]
struct WireWriter(Vec<u8>);

impl WireWriter {
    fn put_u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }

    fn put_string(&mut self, bytes: &[u8]) {
        // Keys and comments are far below 4 GiB, so the length always fits.
        let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        self.put_u32(length);
        self.0.extend_from_slice(bytes);
    }

    /// A non-negative integer from big-endian bytes, with a leading zero byte when the top
    /// bit is set, so it isn't read as negative.
    fn put_mpint(&mut self, big_endian: &[u8]) {
        let first_nonzero = big_endian
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(big_endian.len());
        let digits = &big_endian[first_nonzero..];
        let needs_sign_byte = digits.first().is_some_and(|&byte| byte & 0x80 != 0);
        if needs_sign_byte {
            let mut padded = Vec::with_capacity(digits.len() + 1);
            padded.push(0);
            padded.extend_from_slice(digits);
            self.put_string(&padded);
            return;
        }
        self.put_string(digits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};

    fn generate(algorithm: KeyAlgorithm) -> KeyPair {
        generate_key_pair(algorithm, "me@laptop", &mut StdRng::seed_from_u64(1)).unwrap()
    }

    fn decode_public_blob(public_openssh: &str) -> Vec<u8> {
        let encoded = public_openssh.split(' ').nth(1).unwrap();
        STANDARD.decode(encoded).unwrap()
    }

    #[test]
    fn ed25519_keys_round_trip_through_pem() {
        let pair = generate(KeyAlgorithm::Ed25519);
        assert!(
            pair.private_pem
                .starts_with("-----BEGIN PRIVATE KEY-----\n")
        );
        let signing_key = ed25519_dalek::SigningKey::from_pkcs8_pem(&pair.private_pem).unwrap();
        let verifying_key =
            ed25519_dalek::VerifyingKey::from_public_key_pem(&pair.public_pem).unwrap();
        assert_eq!(signing_key.verifying_key(), verifying_key);
        let blob = decode_public_blob(&pair.public_openssh);
        assert_eq!(&blob[blob.len() - 32..], verifying_key.as_bytes());
    }

    #[test]
    fn openssh_public_lines_name_the_type_and_comment() {
        let pair = generate(KeyAlgorithm::Ed25519);
        let parts: Vec<&str> = pair.public_openssh.split(' ').collect();
        assert_eq!(parts[0], "ssh-ed25519");
        assert_eq!(parts[2], "me@laptop");
        assert!(pair.fingerprint.starts_with("SHA256:"));
        assert_eq!(pair.fingerprint.len(), "SHA256:".len() + 43);
        let without_comment =
            generate_key_pair(KeyAlgorithm::Ed25519, "  ", &mut StdRng::seed_from_u64(1)).unwrap();
        assert_eq!(without_comment.public_openssh.split(' ').count(), 2);
    }

    #[test]
    fn ecdsa_keys_use_uncompressed_points() {
        let pair = generate(KeyAlgorithm::EcdsaP256);
        assert!(pair.public_openssh.starts_with("ecdsa-sha2-nistp256 "));
        let blob = decode_public_blob(&pair.public_openssh);
        // The 65-byte uncompressed point closes the blob, starting with 0x04.
        assert_eq!(blob[blob.len() - 65], 0x04);
        let secret_key = p256::SecretKey::from_pkcs8_pem(&pair.private_pem).unwrap();
        let public_key = p256::PublicKey::from_public_key_pem(&pair.public_pem).unwrap();
        assert_eq!(secret_key.public_key(), public_key);
        let p384_pair = generate(KeyAlgorithm::EcdsaP384);
        assert!(p384_pair.public_openssh.starts_with("ecdsa-sha2-nistp384 "));
    }

    #[test]
    fn rsa_keys_round_trip_through_pem() {
        let pair = generate(KeyAlgorithm::Rsa2048);
        assert!(pair.public_openssh.starts_with("ssh-rsa AAAAB3NzaC1yc2E"));
        let private_key = rsa::PrivateKey::from_pkcs8_pem(&pair.private_pem).unwrap();
        let public_key = rsa::PublicKey::from_public_key_pem(&pair.public_pem).unwrap();
        assert_eq!(private_key.to_public_key(), public_key);
        assert_eq!(public_key.n().bit_width(), 2048);
    }

    #[test]
    fn openssh_private_keys_have_the_documented_layout() {
        let pair = generate(KeyAlgorithm::Ed25519);
        let lines: Vec<&str> = pair.private_openssh.lines().collect();
        assert_eq!(lines[0], "-----BEGIN OPENSSH PRIVATE KEY-----");
        assert_eq!(lines[lines.len() - 1], "-----END OPENSSH PRIVATE KEY-----");
        assert!(
            lines[1..lines.len() - 1]
                .iter()
                .all(|line| line.len() <= 70)
        );
        let container = STANDARD.decode(lines[1..lines.len() - 1].concat()).unwrap();
        assert!(
            container
                .starts_with(b"openssh-key-v1\0\0\0\0\x04none\0\0\0\x04none\0\0\0\0\0\0\0\x01")
        );
    }

    #[test]
    fn mpints_gain_a_sign_byte_only_when_needed() {
        let mut writer = WireWriter::default();
        writer.put_mpint(&[0x00, 0x7f]);
        writer.put_mpint(&[0x80]);
        writer.put_mpint(&[0x00]);
        assert_eq!(
            writer.0,
            [0, 0, 0, 1, 0x7f, 0, 0, 0, 2, 0, 0x80, 0, 0, 0, 0]
        );
    }
}
