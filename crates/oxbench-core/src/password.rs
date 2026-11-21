//! Password and passphrase generation, and strength estimates.

use rand::seq::{IndexedRandom, SliceRandom};
use rand::{CryptoRng, RngExt};

use crate::random_string::{LOOK_ALIKES, pick_characters};
use crate::{CoreError, CoreResult};

pub const MIN_PASSWORD_LENGTH: usize = 4;
pub const MAX_PASSWORD_LENGTH: usize = 256;
pub const MIN_WORD_COUNT: usize = 3;
pub const MAX_WORD_COUNT: usize = 20;
/// zxcvbn's matching slows down sharply on long input, and length alone makes such
/// passwords strong.
const MAX_CHECKED_LENGTH: usize = 128;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
/// Leaves out quotes, backslash, backtick and space, which break shells and config files.
const SYMBOLS: &str = "!#$%&*+-=?@^_~.,:;()[]{}<>/";

#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent checkbox in the generator"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PasswordSpec {
    pub length: usize,
    pub has_lowercase: bool,
    pub has_uppercase: bool,
    pub has_digits: bool,
    pub has_symbols: bool,
    pub is_excluding_look_alikes: bool,
}

impl Default for PasswordSpec {
    fn default() -> Self {
        Self {
            length: 20,
            has_lowercase: true,
            has_uppercase: true,
            has_digits: true,
            has_symbols: true,
            is_excluding_look_alikes: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassphraseSpec<'a> {
    pub word_count: usize,
    pub separator: &'a str,
    pub is_capitalized: bool,
    /// Adds one digit to the end of one word.
    pub has_number: bool,
}

impl Default for PassphraseSpec<'_> {
    fn default() -> Self {
        Self {
            word_count: 5,
            separator: "-",
            is_capitalized: false,
            has_number: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Strength {
    /// 0 (guessable in under a thousand tries) to 4 (over ten billion).
    pub score: u8,
    pub label: &'static str,
    /// Estimated time to crack offline against a slow hash, such as bcrypt.
    pub crack_time: String,
    pub warning: Option<String>,
    pub suggestions: Vec<String>,
}

/// A password with at least one character from every chosen set, in random order.
pub fn generate_password<R: CryptoRng + ?Sized>(
    spec: &PasswordSpec,
    rng: &mut R,
) -> CoreResult<String> {
    let sets = chosen_sets(spec)?;
    check_password_length(spec.length, sets.len())?;
    let pool: Vec<char> = sets.concat();
    let mut characters: Vec<char> = sets
        .iter()
        .filter_map(|set| set.choose(rng).copied())
        .collect();
    let remaining = spec.length - characters.len();
    characters.extend(pick_characters(&pool, remaining, rng).chars());
    characters.shuffle(rng);
    Ok(characters.into_iter().collect())
}

/// Bits of randomness, counting every character as drawn from the whole pool.
pub fn password_entropy_bits(spec: &PasswordSpec) -> CoreResult<f64> {
    let sets = chosen_sets(spec)?;
    check_password_length(spec.length, sets.len())?;
    let pool_size: usize = sets.iter().map(Vec::len).sum();
    Ok(to_f64(spec.length) * to_f64(pool_size).log2())
}

/// Words from the EFF large word list (7,776 words, about 12.9 bits each).
pub fn generate_passphrase<R: CryptoRng + ?Sized>(
    spec: &PassphraseSpec,
    rng: &mut R,
) -> CoreResult<String> {
    check_word_count(spec.word_count)?;
    let mut words: Vec<String> = (0..spec.word_count)
        .filter_map(|_| eff_wordlist::large::LIST.choose(rng))
        .map(|(_, word)| {
            if spec.is_capitalized {
                capitalize(word)
            } else {
                (*word).to_owned()
            }
        })
        .collect();
    if spec.has_number {
        let index = rng.random_range(0..words.len());
        let digit = rng.random_range(0..10);
        words[index].push_str(&digit.to_string());
    }
    Ok(words.join(spec.separator))
}

pub fn passphrase_entropy_bits(spec: &PassphraseSpec) -> CoreResult<f64> {
    check_word_count(spec.word_count)?;
    let words = to_f64(spec.word_count);
    let word_bits = words * to_f64(eff_wordlist::large::LIST.len()).log2();
    let number_bits = if spec.has_number {
        10f64.log2() + words.log2()
    } else {
        0.0
    };
    Ok(word_bits + number_bits)
}

/// zxcvbn's estimate, which looks for words, names, dates, keyboard patterns and repeats.
pub fn check_strength(password: &str) -> Strength {
    let checked: String = password.chars().take(MAX_CHECKED_LENGTH).collect();
    let estimate = zxcvbn::zxcvbn(&checked, &[]);
    let score = u8::from(estimate.score());
    let (warning, suggestions) = estimate.feedback().map_or((None, Vec::new()), |feedback| {
        (
            feedback.warning().map(|warning| warning.to_string()),
            feedback
                .suggestions()
                .iter()
                .map(ToString::to_string)
                .collect(),
        )
    });
    Strength {
        score,
        label: describe_score(score),
        crack_time: estimate
            .crack_times()
            .offline_slow_hashing_1e4_per_second()
            .to_string(),
        warning,
        suggestions,
    }
}

fn describe_score(score: u8) -> &'static str {
    match score {
        0 => "Very weak",
        1 => "Weak",
        2 => "Fair",
        3 => "Strong",
        _ => "Very strong",
    }
}

fn chosen_sets(spec: &PasswordSpec) -> CoreResult<Vec<Vec<char>>> {
    let sets: Vec<Vec<char>> = [
        (spec.has_lowercase, LOWERCASE),
        (spec.has_uppercase, UPPERCASE),
        (spec.has_digits, DIGITS),
        (spec.has_symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(is_chosen, _)| *is_chosen)
    .map(|(_, set)| {
        set.chars()
            .filter(|c| !(spec.is_excluding_look_alikes && LOOK_ALIKES.contains(*c)))
            .collect()
    })
    .collect();
    if sets.is_empty() {
        return Err(invalid("Choose at least one kind of character."));
    }
    Ok(sets)
}

fn check_password_length(length: usize, set_count: usize) -> CoreResult<()> {
    let minimum = MIN_PASSWORD_LENGTH.max(set_count);
    if (minimum..=MAX_PASSWORD_LENGTH).contains(&length) {
        return Ok(());
    }
    Err(invalid(&format!(
        "Choose a length between {minimum} and {MAX_PASSWORD_LENGTH}."
    )))
}

fn check_word_count(word_count: usize) -> CoreResult<()> {
    if (MIN_WORD_COUNT..=MAX_WORD_COUNT).contains(&word_count) {
        return Ok(());
    }
    Err(invalid(&format!(
        "Choose between {MIN_WORD_COUNT} and {MAX_WORD_COUNT} words."
    )))
}

fn capitalize(word: &str) -> String {
    let mut characters = word.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

/// Counts here are small (lengths and list sizes), so the conversion is exact.
fn to_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn passwords_include_every_chosen_set() {
        let mut rng = StdRng::seed_from_u64(7);
        let spec = PasswordSpec {
            length: 4,
            ..PasswordSpec::default()
        };
        for _ in 0..200 {
            let password = generate_password(&spec, &mut rng).unwrap();
            assert_eq!(password.chars().count(), 4);
            assert!(password.chars().any(|c| c.is_ascii_lowercase()));
            assert!(password.chars().any(|c| c.is_ascii_uppercase()));
            assert!(password.chars().any(|c| c.is_ascii_digit()));
            assert!(password.chars().any(|c| SYMBOLS.contains(c)));
        }
    }

    #[test]
    fn respects_the_chosen_sets_and_look_alikes() {
        let mut rng = StdRng::seed_from_u64(8);
        let spec = PasswordSpec {
            length: 200,
            has_uppercase: false,
            has_symbols: false,
            is_excluding_look_alikes: true,
            ..PasswordSpec::default()
        };
        let password = generate_password(&spec, &mut rng).unwrap();
        assert!(
            password
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
        assert!(!password.contains(['0', 'o', '1', 'l']));
    }

    #[test]
    fn rejects_impossible_passwords() {
        let mut rng = StdRng::seed_from_u64(9);
        let none = PasswordSpec {
            has_lowercase: false,
            has_uppercase: false,
            has_digits: false,
            has_symbols: false,
            ..PasswordSpec::default()
        };
        assert!(
            generate_password(&none, &mut rng)
                .unwrap_err()
                .to_string()
                .contains("at least one")
        );
        let short = PasswordSpec {
            length: 3,
            ..PasswordSpec::default()
        };
        assert!(
            generate_password(&short, &mut rng)
                .unwrap_err()
                .to_string()
                .contains("between 4")
        );
    }

    #[test]
    fn computes_password_entropy() {
        let digits_only = PasswordSpec {
            length: 10,
            has_lowercase: false,
            has_uppercase: false,
            has_symbols: false,
            ..PasswordSpec::default()
        };
        let bits = password_entropy_bits(&digits_only).unwrap();
        assert!((bits - 33.22).abs() < 0.01);
    }

    #[test]
    fn passphrases_use_list_words() {
        let mut rng = StdRng::seed_from_u64(10);
        let spec = PassphraseSpec {
            word_count: 6,
            separator: " ",
            is_capitalized: true,
            has_number: true,
        };
        let phrase = generate_passphrase(&spec, &mut rng).unwrap();
        let words: Vec<&str> = phrase.split(' ').collect();
        assert_eq!(words.len(), 6);
        assert!(
            words
                .iter()
                .all(|w| w.starts_with(|c: char| c.is_ascii_uppercase()))
        );
        assert_eq!(phrase.chars().filter(char::is_ascii_digit).count(), 1);
        let plain = words[1]
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_lowercase();
        assert!(
            eff_wordlist::large::LIST
                .iter()
                .any(|(_, word)| *word == plain)
        );
    }

    #[test]
    fn computes_passphrase_entropy() {
        let bits = passphrase_entropy_bits(&PassphraseSpec::default()).unwrap();
        assert!((bits - 64.62).abs() < 0.01);
        assert!(
            passphrase_entropy_bits(&PassphraseSpec {
                word_count: 2,
                ..PassphraseSpec::default()
            })
            .is_err()
        );
    }

    #[test]
    fn rates_strength() {
        let weak = check_strength("password1");
        assert_eq!(weak.score, 0);
        assert_eq!(weak.label, "Very weak");
        assert!(weak.warning.is_some());
        let strong = check_strength("correct-horse-battery-staple-quantum-9");
        assert_eq!(strong.score, 4);
        assert_eq!(strong.label, "Very strong");
        assert_eq!(strong.crack_time, "centuries");
    }

    #[test]
    fn long_input_is_checked_quickly() {
        let mut rng = StdRng::seed_from_u64(11);
        let spec = PasswordSpec {
            length: MAX_PASSWORD_LENGTH,
            ..PasswordSpec::default()
        };
        let long = generate_password(&spec, &mut rng).unwrap().repeat(4);
        let started = std::time::Instant::now();
        assert_eq!(check_strength(&long).score, 4);
        assert!(started.elapsed().as_secs() < 2);
    }
}
