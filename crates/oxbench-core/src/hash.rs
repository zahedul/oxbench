//! Message digests and HMACs over text or streamed bytes.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use digest::{Digest, DynDigest, KeyInit, Mac};
use hmac::SimpleHmac;

use crate::{CoreError, CoreResult};

/// Large enough for fast streaming, small enough to report progress often.
const READ_CHUNK_BYTES: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Blake3,
}

impl HashAlgorithm {
    pub const ALL: [HashAlgorithm; 10] = [
        HashAlgorithm::Md5,
        HashAlgorithm::Sha1,
        HashAlgorithm::Sha224,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
        HashAlgorithm::Sha3_256,
        HashAlgorithm::Sha3_384,
        HashAlgorithm::Sha3_512,
        HashAlgorithm::Blake3,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha224 => "SHA-224",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha384 => "SHA-384",
            HashAlgorithm::Sha512 => "SHA-512",
            HashAlgorithm::Sha3_256 => "SHA3-256",
            HashAlgorithm::Sha3_384 => "SHA3-384",
            HashAlgorithm::Sha3_512 => "SHA3-512",
            HashAlgorithm::Blake3 => "BLAKE3",
        }
    }

    /// Collisions can be crafted, so these suit checksums but not signatures or certificates.
    pub const fn is_broken(self) -> bool {
        matches!(self, HashAlgorithm::Md5 | HashAlgorithm::Sha1)
    }

    /// BLAKE3 has its own keyed mode with a fixed 32-byte key instead of HMAC.
    pub const fn supports_hmac(self) -> bool {
        !matches!(self, HashAlgorithm::Blake3)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DigestEncoding {
    #[default]
    HexLower,
    HexUpper,
    Base64,
}

impl DigestEncoding {
    pub const ALL: [DigestEncoding; 3] = [
        DigestEncoding::HexLower,
        DigestEncoding::HexUpper,
        DigestEncoding::Base64,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            DigestEncoding::HexLower => "hex",
            DigestEncoding::HexUpper => "HEX",
            DigestEncoding::Base64 => "Base64",
        }
    }
}

/// How an HMAC key is typed in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyEncoding {
    #[default]
    Text,
    Hex,
    Base64,
}

impl KeyEncoding {
    pub const ALL: [KeyEncoding; 3] = [KeyEncoding::Text, KeyEncoding::Hex, KeyEncoding::Base64];

    pub const fn label(self) -> &'static str {
        match self {
            KeyEncoding::Text => "Text",
            KeyEncoding::Hex => "Hex",
            KeyEncoding::Base64 => "Base64",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashOutput {
    pub algorithm: HashAlgorithm,
    pub bytes: Vec<u8>,
}

/// Every algorithm's digest of `data`.
pub fn hash_all(data: &[u8]) -> Vec<HashOutput> {
    let mut hashers = start_hashers();
    for (_, hasher) in &mut hashers {
        hasher.update(data);
    }
    finish_hashers(hashers)
}

/// Every algorithm's digest of everything `reader` yields, reading it once. `on_progress`
/// receives the total bytes read so far.
pub fn hash_reader(
    mut reader: impl Read,
    mut on_progress: impl FnMut(u64),
) -> CoreResult<Vec<HashOutput>> {
    let mut hashers = start_hashers();
    let mut buffer = vec![0; READ_CHUNK_BYTES];
    let mut total_read: u64 = 0;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(CoreError::InvalidInput(format!(
                    "Could not read the file: {error}"
                )));
            }
        };
        let chunk = &buffer[..read];
        for (_, hasher) in &mut hashers {
            hasher.update(chunk);
        }
        total_read += read as u64;
        on_progress(total_read);
    }
    Ok(finish_hashers(hashers))
}

/// Every algorithm's digest of a file, read once in chunks.
pub fn hash_file(path: &Path, on_progress: impl FnMut(u64)) -> CoreResult<Vec<HashOutput>> {
    let file = File::open(path).map_err(|error| {
        CoreError::InvalidInput(format!("Could not open {}: {error}", path.display()))
    })?;
    hash_reader(file, on_progress)
}

/// HMAC of `data` for every algorithm that supports it.
pub fn hmac_all(key: &[u8], data: &[u8]) -> Vec<HashOutput> {
    HashAlgorithm::ALL
        .into_iter()
        .filter(|algorithm| algorithm.supports_hmac())
        .filter_map(|algorithm| {
            let bytes = compute_hmac(algorithm, key, data)?;
            Some(HashOutput { algorithm, bytes })
        })
        .collect()
}

pub fn decode_key(text: &str, encoding: KeyEncoding) -> CoreResult<Vec<u8>> {
    match encoding {
        KeyEncoding::Text => Ok(text.as_bytes().to_vec()),
        KeyEncoding::Hex => decode_hex(text.trim())
            .ok_or_else(|| invalid("The key is not valid hex: use pairs of 0-9 and a-f.")),
        KeyEncoding::Base64 => {
            decode_base64(text.trim()).ok_or_else(|| invalid("The key is not valid Base64."))
        }
    }
}

pub fn encode_digest(bytes: &[u8], encoding: DigestEncoding) -> String {
    match encoding {
        DigestEncoding::HexLower => encode_hex(bytes),
        DigestEncoding::HexUpper => encode_hex(bytes).to_uppercase(),
        DigestEncoding::Base64 => STANDARD.encode(bytes),
    }
}

/// Whether `expected` is this digest, as pasted from a checksum file or a release page:
/// hex in any case or Base64, optionally as `sha256:…` or followed by a file name.
pub fn matches_digest(expected: &str, digest: &[u8]) -> bool {
    let Some(token) = expected.split_whitespace().next() else {
        return false;
    };
    let value = token.rsplit_once(':').map_or(token, |(_, value)| value);
    if value.len() == digest.len() * 2 {
        return decode_hex(value).is_some_and(|bytes| bytes == digest);
    }
    decode_base64(value).is_some_and(|bytes| bytes == digest)
}

fn start_hashers() -> Vec<(HashAlgorithm, Hasher)> {
    HashAlgorithm::ALL
        .into_iter()
        .map(|algorithm| (algorithm, Hasher::new(algorithm)))
        .collect()
}

fn finish_hashers(hashers: Vec<(HashAlgorithm, Hasher)>) -> Vec<HashOutput> {
    hashers
        .into_iter()
        .map(|(algorithm, hasher)| HashOutput {
            algorithm,
            bytes: hasher.finish(),
        })
        .collect()
}

/// BLAKE3 does not implement the `RustCrypto` `digest` traits, so it is kept apart.
enum Hasher {
    RustCrypto(Box<dyn DynDigest + Send>),
    Blake3(Box<blake3::Hasher>),
}

impl Hasher {
    fn new(algorithm: HashAlgorithm) -> Self {
        let digest: Box<dyn DynDigest + Send> = match algorithm {
            HashAlgorithm::Md5 => Box::new(md5::Md5::new()),
            HashAlgorithm::Sha1 => Box::new(sha1::Sha1::new()),
            HashAlgorithm::Sha224 => Box::new(sha2::Sha224::new()),
            HashAlgorithm::Sha256 => Box::new(sha2::Sha256::new()),
            HashAlgorithm::Sha384 => Box::new(sha2::Sha384::new()),
            HashAlgorithm::Sha512 => Box::new(sha2::Sha512::new()),
            HashAlgorithm::Sha3_256 => Box::new(sha3::Sha3_256::new()),
            HashAlgorithm::Sha3_384 => Box::new(sha3::Sha3_384::new()),
            HashAlgorithm::Sha3_512 => Box::new(sha3::Sha3_512::new()),
            HashAlgorithm::Blake3 => return Hasher::Blake3(Box::default()),
        };
        Hasher::RustCrypto(digest)
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::RustCrypto(digest) => digest.update(data),
            Hasher::Blake3(hasher) => {
                hasher.update(data);
            }
        }
    }

    fn finish(self) -> Vec<u8> {
        match self {
            Hasher::RustCrypto(digest) => digest.finalize().into_vec(),
            Hasher::Blake3(hasher) => hasher.finalize().as_bytes().to_vec(),
        }
    }
}

fn compute_hmac(algorithm: HashAlgorithm, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let bytes = match algorithm {
        HashAlgorithm::Md5 => hmac_with::<md5::Md5>(key, data),
        HashAlgorithm::Sha1 => hmac_with::<sha1::Sha1>(key, data),
        HashAlgorithm::Sha224 => hmac_with::<sha2::Sha224>(key, data),
        HashAlgorithm::Sha256 => hmac_with::<sha2::Sha256>(key, data),
        HashAlgorithm::Sha384 => hmac_with::<sha2::Sha384>(key, data),
        HashAlgorithm::Sha512 => hmac_with::<sha2::Sha512>(key, data),
        HashAlgorithm::Sha3_256 => hmac_with::<sha3::Sha3_256>(key, data),
        HashAlgorithm::Sha3_384 => hmac_with::<sha3::Sha3_384>(key, data),
        HashAlgorithm::Sha3_512 => hmac_with::<sha3::Sha3_512>(key, data),
        HashAlgorithm::Blake3 => return None,
    };
    Some(bytes)
}

fn hmac_with<D: Digest + digest::common::BlockSizeUser>(key: &[u8], data: &[u8]) -> Vec<u8> {
    // HMAC takes keys of any length: longer keys are hashed first, so this never fails.
    let Ok(mut mac) = <SimpleHmac<D> as KeyInit>::new_from_slice(key) else {
        return Vec::new();
    };
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect()
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    STANDARD
        .decode(text)
        .or_else(|_| URL_SAFE_NO_PAD.decode(text.trim_end_matches('=')))
        .ok()
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_of(digests: &[HashOutput], algorithm: HashAlgorithm) -> String {
        let digest = digests.iter().find(|d| d.algorithm == algorithm).unwrap();
        encode_digest(&digest.bytes, DigestEncoding::HexLower)
    }

    #[test]
    fn matches_known_vectors_for_abc() {
        let digests = hash_all(b"abc");
        let expected = [
            (HashAlgorithm::Md5, "900150983cd24fb0d6963f7d28e17f72"),
            (
                HashAlgorithm::Sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d",
            ),
            (
                HashAlgorithm::Sha224,
                "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
            ),
            (
                HashAlgorithm::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                HashAlgorithm::Sha384,
                "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
            ),
            (
                HashAlgorithm::Sha512,
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            ),
            (
                HashAlgorithm::Sha3_256,
                "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
            ),
            (
                HashAlgorithm::Sha3_384,
                "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25",
            ),
            (
                HashAlgorithm::Sha3_512,
                "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
            ),
            (
                HashAlgorithm::Blake3,
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
            ),
        ];
        assert_eq!(digests.len(), expected.len());
        for (algorithm, hex) in expected {
            assert_eq!(hex_of(&digests, algorithm), hex, "{}", algorithm.label());
        }
    }

    #[test]
    fn streaming_matches_hashing_at_once() {
        let data: Vec<u8> = (0..3 * READ_CHUNK_BYTES + 17)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let mut progress = Vec::new();
        let streamed = hash_reader(data.as_slice(), |read| progress.push(read)).unwrap();
        assert_eq!(streamed, hash_all(&data));
        assert_eq!(progress.last().copied(), Some(data.len() as u64));
        assert_eq!(progress.len(), 4);
    }

    #[test]
    fn hashes_files_and_reports_missing_ones() {
        let path = std::env::temp_dir().join(format!("oxbench-hash-{}.txt", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let digests = hash_file(&path, |_| {}).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(digests, hash_all(b"abc"));
        let error = hash_file(&path, |_| {}).unwrap_err();
        assert!(error.to_string().starts_with("Could not open"));
    }

    #[test]
    fn matches_rfc_hmac_vectors() {
        let macs = hmac_all(b"Jefe", b"what do ya want for nothing?");
        assert!(
            macs.iter()
                .all(|mac| mac.algorithm != HashAlgorithm::Blake3)
        );
        assert_eq!(
            hex_of(&macs, HashAlgorithm::Md5),
            "750c783e6ab0b503eaa86e310a5db738"
        );
        assert_eq!(
            hex_of(&macs, HashAlgorithm::Sha1),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
        assert_eq!(
            hex_of(&macs, HashAlgorithm::Sha256),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hex_of(&macs, HashAlgorithm::Sha512),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    fn decodes_keys_in_each_encoding() {
        assert_eq!(decode_key("Jefe", KeyEncoding::Text).unwrap(), b"Jefe");
        assert_eq!(decode_key(" 4a656665 ", KeyEncoding::Hex).unwrap(), b"Jefe");
        assert_eq!(
            decode_key("SmVmZQ==", KeyEncoding::Base64).unwrap(),
            b"Jefe"
        );
        assert!(decode_key("4a6", KeyEncoding::Hex).is_err());
        assert!(decode_key("zz", KeyEncoding::Hex).is_err());
        assert!(decode_key("not base64!", KeyEncoding::Base64).is_err());
    }

    #[test]
    fn encodes_digests() {
        let bytes = [0xab, 0x01, 0xff];
        assert_eq!(encode_digest(&bytes, DigestEncoding::HexLower), "ab01ff");
        assert_eq!(encode_digest(&bytes, DigestEncoding::HexUpper), "AB01FF");
        assert_eq!(encode_digest(&bytes, DigestEncoding::Base64), "qwH/");
    }

    #[test]
    fn compares_pasted_checksums() {
        let digest = hash_all(b"abc").remove(3).bytes;
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(matches_digest(hex, &digest));
        assert!(matches_digest(&hex.to_uppercase(), &digest));
        assert!(matches_digest(&format!("  sha256:{hex}\n"), &digest));
        assert!(matches_digest(&format!("{hex}  ubuntu.iso"), &digest));
        assert!(matches_digest(
            "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=",
            &digest
        ));
        assert!(matches_digest(
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0",
            &digest
        ));
        assert!(!matches_digest(&hex[1..], &digest));
        assert!(!matches_digest("", &digest));
    }
}
