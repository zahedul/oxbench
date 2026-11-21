//! Bcrypt password hashes. Hashing is slow on purpose, so callers should run it off the UI thread.

use crate::{CoreError, CoreResult};

pub const MIN_COST: u32 = 4;
/// Bcrypt allows up to 31, but each step doubles the time; 16 already takes seconds.
pub const MAX_COST: u32 = 16;
pub const DEFAULT_COST: u32 = 12;
/// Bcrypt ignores everything after this many bytes of the password.
pub const MAX_PASSWORD_BYTES: usize = 72;

const SALT_LENGTH: usize = 22;
const CHECKSUM_LENGTH: usize = 31;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BcryptParts {
    /// `2a`, `2b`, `2x` or `2y`.
    pub version: String,
    pub cost: u32,
    pub salt: String,
    pub checksum: String,
}

/// A `$2b$` hash with a fresh random salt. Like other bcrypt libraries, it uses only the
/// first 72 bytes; check `is_truncated` to warn about that.
pub fn hash_password(password: &str, cost: u32) -> CoreResult<String> {
    if !(MIN_COST..=MAX_COST).contains(&cost) {
        return Err(invalid(&format!(
            "Choose a cost between {MIN_COST} and {MAX_COST}."
        )));
    }
    ::bcrypt::hash(password, cost)
        .map_err(|error| invalid(&format!("Could not hash the password: {error}.")))
}

pub fn verify_password(password: &str, hash: &str) -> CoreResult<bool> {
    let hash = hash.trim();
    parse_bcrypt_hash(hash)?;
    ::bcrypt::verify(password, hash)
        .map_err(|error| invalid(&format!("Could not check the hash: {error}.")))
}

pub fn parse_bcrypt_hash(hash: &str) -> CoreResult<BcryptParts> {
    let hash = hash.trim();
    let format_hint = "A bcrypt hash looks like $2b$12$ followed by 53 characters.";
    let Some(("", rest)) = hash.split_once('$') else {
        return Err(invalid(format_hint));
    };
    let Some((version, rest)) = rest.split_once('$') else {
        return Err(invalid(format_hint));
    };
    let Some((cost_text, salt_and_checksum)) = rest.split_once('$') else {
        return Err(invalid(format_hint));
    };
    if !matches!(version, "2a" | "2b" | "2x" | "2y") {
        return Err(invalid(&format!(
            "${version}$ is not a bcrypt version; expected $2a$, $2b$ or $2y$."
        )));
    }
    let Ok(cost) = cost_text.parse::<u32>() else {
        return Err(invalid(format_hint));
    };
    if !(MIN_COST..=31).contains(&cost) {
        return Err(invalid(&format!(
            "A cost of {cost} is outside bcrypt's 4 to 31."
        )));
    }
    if salt_and_checksum.len() != SALT_LENGTH + CHECKSUM_LENGTH || !salt_and_checksum.is_ascii() {
        return Err(invalid(format_hint));
    }
    let (salt, checksum) = salt_and_checksum.split_at(SALT_LENGTH);
    Ok(BcryptParts {
        version: version.to_owned(),
        cost,
        salt: salt.to_owned(),
        checksum: checksum.to_owned(),
    })
}

pub fn is_truncated(password: &str) -> bool {
    password.len() > MAX_PASSWORD_BYTES
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From the `OpenWall` `crypt_blowfish` test vectors.
    const OPENWALL_HASH: &str = "$2a$05$CCCCCCCCCCCCCCCCCCCCC.E5YPO9kmyuRGyh0XouQYb4YMJKvyOeW";

    #[test]
    fn hashes_and_verifies() {
        let hash = hash_password("hunter2", MIN_COST).unwrap();
        let parts = parse_bcrypt_hash(&hash).unwrap();
        assert_eq!(parts.version, "2b");
        assert_eq!(parts.cost, MIN_COST);
        assert!(verify_password("hunter2", &hash).unwrap());
        assert!(!verify_password("hunter3", &hash).unwrap());
        assert_ne!(hash, hash_password("hunter2", MIN_COST).unwrap());
    }

    #[test]
    fn verifies_known_vectors() {
        assert!(verify_password("U*U", OPENWALL_HASH).unwrap());
        assert!(verify_password("U*U", &format!("  {OPENWALL_HASH}\n")).unwrap());
        assert!(!verify_password("U*V", OPENWALL_HASH).unwrap());
    }

    #[test]
    fn splits_a_hash_into_parts() {
        let parts = parse_bcrypt_hash(OPENWALL_HASH).unwrap();
        assert_eq!(
            parts,
            BcryptParts {
                version: "2a".to_owned(),
                cost: 5,
                salt: "CCCCCCCCCCCCCCCCCCCCC.".to_owned(),
                checksum: "E5YPO9kmyuRGyh0XouQYb4YMJKvyOeW".to_owned(),
            }
        );
    }

    #[test]
    fn explains_malformed_hashes() {
        let message = |hash: &str| parse_bcrypt_hash(hash).unwrap_err().to_string();
        assert!(message("password").contains("looks like $2b$12$"));
        assert!(message("$1$05$abc").contains("not a bcrypt version"));
        assert!(
            message("$2b$99$CCCCCCCCCCCCCCCCCCCCC.E5YPO9kmyuRGyh0XouQYb4YMJKvyOeW")
                .contains("outside")
        );
        assert!(message("$2b$05$tooshort").contains("53 characters"));
        assert!(verify_password("x", "nope").is_err());
    }

    #[test]
    fn limits_cost_and_flags_long_passwords() {
        assert!(hash_password("x", 3).is_err());
        assert!(hash_password("x", MAX_COST + 1).is_err());
        assert!(!is_truncated(&"a".repeat(72)));
        assert!(is_truncated(&"a".repeat(73)));
        // Bytes, not characters: 25 three-byte characters are 75 bytes.
        assert!(is_truncated(&"€".repeat(25)));
    }
}
