//! Random tokens: Nano IDs, custom alphabets, and random bytes as hex or Base64.

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::CryptoRng;
use rand::seq::IndexedRandom;

use crate::hash::encode_hex;
use crate::{CoreError, CoreResult};

pub const MAX_LENGTH: usize = 4096;
pub const MAX_COUNT: usize = 1000;

const LETTERS_AND_DIGITS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const URL_SAFE_EXTRA: &str = "_-";
const DIGITS: &str = "0123456789";
/// Characters that are easy to misread in many fonts.
pub const LOOK_ALIKES: &str = "0Oo1lI";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RandomFormat {
    #[default]
    NanoId,
    Alphanumeric,
    Numeric,
    Custom,
    Hex,
    Base64Url,
    Base64,
}

impl RandomFormat {
    pub const ALL: [RandomFormat; 7] = [
        RandomFormat::NanoId,
        RandomFormat::Alphanumeric,
        RandomFormat::Numeric,
        RandomFormat::Custom,
        RandomFormat::Hex,
        RandomFormat::Base64Url,
        RandomFormat::Base64,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            RandomFormat::NanoId => "Nano ID",
            RandomFormat::Alphanumeric => "Letters and digits",
            RandomFormat::Numeric => "Digits only",
            RandomFormat::Custom => "Custom alphabet",
            RandomFormat::Hex => "Hex bytes",
            RandomFormat::Base64Url => "Base64URL bytes",
            RandomFormat::Base64 => "Base64 bytes",
        }
    }

    /// Byte formats encode `length` random bytes; the others pick `length` characters.
    pub const fn is_byte_based(self) -> bool {
        matches!(
            self,
            RandomFormat::Hex | RandomFormat::Base64Url | RandomFormat::Base64
        )
    }

    pub const fn default_length(self) -> usize {
        match self {
            RandomFormat::NanoId => 21,
            RandomFormat::Numeric => 6,
            RandomFormat::Custom => 16,
            RandomFormat::Alphanumeric
            | RandomFormat::Hex
            | RandomFormat::Base64Url
            | RandomFormat::Base64 => 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RandomSpec<'a> {
    pub format: RandomFormat,
    /// Characters, or bytes for byte formats.
    pub length: usize,
    /// Used when `format` is `Custom`.
    pub custom_alphabet: &'a str,
    pub is_excluding_look_alikes: bool,
}

pub fn generate_random_strings<R: CryptoRng + ?Sized>(
    spec: &RandomSpec,
    count: usize,
    rng: &mut R,
) -> CoreResult<Vec<String>> {
    if !(1..=MAX_COUNT).contains(&count) {
        return Err(invalid(&format!(
            "Choose between 1 and {MAX_COUNT} strings."
        )));
    }
    check_length(spec.length)?;
    if spec.format.is_byte_based() {
        return Ok((0..count)
            .map(|_| encode_random_bytes(spec.format, spec.length, rng))
            .collect());
    }
    let alphabet = resolve_alphabet(spec)?;
    Ok((0..count)
        .map(|_| pick_characters(&alphabet, spec.length, rng))
        .collect())
}

/// Bits of randomness in one string.
pub fn entropy_bits(spec: &RandomSpec) -> CoreResult<f64> {
    check_length(spec.length)?;
    let length = f64::from(u32::try_from(spec.length).unwrap_or(u32::MAX));
    if spec.format.is_byte_based() {
        return Ok(length * 8.0);
    }
    let alphabet_size = resolve_alphabet(spec)?.len();
    Ok(length * f64::from(u32::try_from(alphabet_size).unwrap_or(u32::MAX)).log2())
}

/// The distinct characters to pick from, in first-seen order.
pub fn resolve_alphabet(spec: &RandomSpec) -> CoreResult<Vec<char>> {
    let source = match spec.format {
        RandomFormat::NanoId => format!("{LETTERS_AND_DIGITS}{URL_SAFE_EXTRA}"),
        RandomFormat::Alphanumeric => LETTERS_AND_DIGITS.to_owned(),
        RandomFormat::Numeric => DIGITS.to_owned(),
        RandomFormat::Custom => spec.custom_alphabet.to_owned(),
        RandomFormat::Hex | RandomFormat::Base64Url | RandomFormat::Base64 => {
            return Err(invalid("Byte formats have no alphabet."));
        }
    };
    let mut alphabet: Vec<char> = Vec::new();
    for character in source.chars() {
        let is_skipped = spec.is_excluding_look_alikes && LOOK_ALIKES.contains(character);
        if !is_skipped && !alphabet.contains(&character) {
            alphabet.push(character);
        }
    }
    if alphabet.len() < 2 {
        return Err(invalid(
            "The alphabet needs at least two different characters.",
        ));
    }
    Ok(alphabet)
}

pub(crate) fn pick_characters<R: CryptoRng + ?Sized>(
    alphabet: &[char],
    length: usize,
    rng: &mut R,
) -> String {
    (0..length)
        .filter_map(|_| alphabet.choose(rng).copied())
        .collect()
}

fn encode_random_bytes<R: CryptoRng + ?Sized>(
    format: RandomFormat,
    length: usize,
    rng: &mut R,
) -> String {
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);
    match format {
        RandomFormat::Base64Url => URL_SAFE_NO_PAD.encode(&bytes),
        RandomFormat::Base64 => STANDARD.encode(&bytes),
        _ => encode_hex(&bytes),
    }
}

fn check_length(length: usize) -> CoreResult<()> {
    if (1..=MAX_LENGTH).contains(&length) {
        return Ok(());
    }
    Err(invalid(&format!(
        "Choose a length between 1 and {MAX_LENGTH}."
    )))
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn spec(format: RandomFormat, length: usize) -> RandomSpec<'static> {
        RandomSpec {
            format,
            length,
            ..RandomSpec::default()
        }
    }

    #[test]
    fn nano_ids_use_the_url_safe_alphabet() {
        let mut rng = StdRng::seed_from_u64(1);
        let ids = generate_random_strings(&spec(RandomFormat::NanoId, 21), 200, &mut rng).unwrap();
        assert_eq!(ids.len(), 200);
        for id in &ids {
            assert_eq!(id.chars().count(), 21);
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            );
        }
        let all: String = ids.concat();
        assert!(all.contains('_') && all.contains('-'));
    }

    #[test]
    fn byte_formats_encode_the_requested_bytes() {
        let mut rng = StdRng::seed_from_u64(2);
        let hex = &generate_random_strings(&spec(RandomFormat::Hex, 16), 1, &mut rng).unwrap()[0];
        assert_eq!(hex.len(), 32);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        let url =
            &generate_random_strings(&spec(RandomFormat::Base64Url, 32), 1, &mut rng).unwrap()[0];
        assert_eq!(url.len(), 43);
        assert!(!url.contains(['+', '/', '=']));
        let standard =
            &generate_random_strings(&spec(RandomFormat::Base64, 32), 1, &mut rng).unwrap()[0];
        assert_eq!(standard.len(), 44);
        assert!(standard.ends_with('='));
    }

    #[test]
    fn custom_alphabets_drop_repeats_and_look_alikes() {
        let custom = RandomSpec {
            format: RandomFormat::Custom,
            length: 64,
            custom_alphabet: "abcabc01O",
            is_excluding_look_alikes: true,
        };
        assert_eq!(resolve_alphabet(&custom).unwrap(), ['a', 'b', 'c']);
        let mut rng = StdRng::seed_from_u64(3);
        let text = &generate_random_strings(&custom, 1, &mut rng).unwrap()[0];
        assert!(text.chars().all(|c| "abc".contains(c)));
        let too_small = RandomSpec {
            custom_alphabet: "aaa",
            ..custom
        };
        assert!(
            resolve_alphabet(&too_small)
                .unwrap_err()
                .to_string()
                .contains("at least two")
        );
    }

    #[test]
    fn reports_entropy() {
        let nano = entropy_bits(&spec(RandomFormat::NanoId, 21)).unwrap();
        assert!((nano - 126.0).abs() < 1e-9);
        assert!((entropy_bits(&spec(RandomFormat::Hex, 32)).unwrap() - 256.0).abs() < 1e-9);
        let digits = entropy_bits(&spec(RandomFormat::Numeric, 6)).unwrap();
        assert!((digits - 19.93).abs() < 0.01);
    }

    #[test]
    fn rejects_out_of_range_lengths_and_counts() {
        let mut rng = StdRng::seed_from_u64(4);
        assert!(generate_random_strings(&spec(RandomFormat::NanoId, 0), 1, &mut rng).is_err());
        assert!(
            generate_random_strings(&spec(RandomFormat::NanoId, MAX_LENGTH + 1), 1, &mut rng)
                .is_err()
        );
        assert!(generate_random_strings(&spec(RandomFormat::NanoId, 8), 0, &mut rng).is_err());
    }
}
