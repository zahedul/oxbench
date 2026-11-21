use chrono::{DateTime, Utc};
use serde_json::Value;

/// Claims holding a `NumericDate`: seconds since 1970, possibly with a fraction.
const TIME_CLAIMS: [&str; 4] = ["exp", "nbf", "iat", "auth_time"];

/// One payload claim, ready to show.
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimRow {
    pub name: String,
    /// What a registered or common claim means, such as "Expires" for `exp`.
    pub meaning: Option<&'static str>,
    /// Strings as they are; anything else as compact JSON.
    pub value: String,
    /// Set for time claims with a numeric value.
    pub time: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenTiming {
    /// No `exp` claim, so the token never expires on its own.
    NoExpiry,
    Active {
        expires_at: DateTime<Utc>,
    },
    Expired {
        expired_at: DateTime<Utc>,
    },
    NotYetValid {
        valid_from: DateTime<Utc>,
    },
}

/// Every top-level claim in the payload, in the order the token lists them.
pub fn describe_claims(payload: &Value) -> Vec<ClaimRow> {
    let Value::Object(fields) = payload else {
        return Vec::new();
    };
    fields
        .iter()
        .map(|(name, value)| ClaimRow {
            name: name.clone(),
            meaning: describe_claim_name(name),
            value: match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            },
            time: TIME_CLAIMS
                .contains(&name.as_str())
                .then(|| read_numeric_date(value))
                .flatten(),
        })
        .collect()
}

/// Compares `nbf` and `exp` with `now`. A token is expired from the `exp` second onwards.
pub fn check_timing(payload: &Value, now: DateTime<Utc>) -> TokenTiming {
    let read = |name: &str| payload.get(name).and_then(read_numeric_date);
    if let Some(valid_from) = read("nbf").filter(|valid_from| now < *valid_from) {
        return TokenTiming::NotYetValid { valid_from };
    }
    match read("exp") {
        Some(expired_at) if now >= expired_at => TokenTiming::Expired { expired_at },
        Some(expires_at) => TokenTiming::Active { expires_at },
        None => TokenTiming::NoExpiry,
    }
}

fn read_numeric_date(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(seconds) = value.as_i64() {
        return DateTime::from_timestamp(seconds, 0);
    }
    let seconds = value.as_f64()?;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the value is floored first, and from_timestamp rejects anything out of range"
    )]
    let whole = seconds.floor() as i64;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fraction is in [0, 1), so the nanoseconds fit in u32"
    )]
    let nanos = ((seconds - seconds.floor()) * 1e9) as u32;
    DateTime::from_timestamp(whole, nanos)
}

fn describe_claim_name(name: &str) -> Option<&'static str> {
    let meaning = match name {
        "iss" => "Issuer",
        "sub" => "Subject",
        "aud" => "Audience",
        "exp" => "Expires",
        "nbf" => "Not before",
        "iat" => "Issued at",
        "jti" => "Token ID",
        "auth_time" => "Signed in at",
        "azp" => "Authorized party",
        "nonce" => "Nonce",
        "scope" | "scp" => "Scopes",
        "sid" => "Session ID",
        "name" => "Name",
        "email" => "Email",
        "roles" => "Roles",
        _ => return None,
    };
    Some(meaning)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    #[test]
    fn describes_claims_in_token_order() {
        let rows = describe_claims(&json!({
            "sub": "42", "aud": ["a", "b"], "exp": 1_700_000_000, "custom": true
        }));
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["sub", "aud", "exp", "custom"]);
        assert_eq!(rows[0].value, "42");
        assert_eq!(rows[1].value, r#"["a","b"]"#);
        assert_eq!(rows[2].meaning, Some("Expires"));
        assert_eq!(rows[2].time, Some(at(1_700_000_000)));
        assert_eq!(rows[3].meaning, None);
        assert_eq!(rows[3].time, None);
    }

    #[test]
    fn reads_fractional_dates_and_ignores_text_dates() {
        let rows = describe_claims(&json!({"iat": 1_700_000_000.5, "exp": "tomorrow"}));
        assert_eq!(rows[0].time.unwrap().timestamp_subsec_millis(), 500);
        assert_eq!(rows[1].time, None);
        assert_eq!(describe_claims(&json!([1, 2])), []);
    }

    #[test]
    fn checks_expiry_and_not_before() {
        let payload = json!({"nbf": 100, "exp": 200});
        assert_eq!(
            check_timing(&payload, at(50)),
            TokenTiming::NotYetValid {
                valid_from: at(100)
            }
        );
        assert_eq!(
            check_timing(&payload, at(150)),
            TokenTiming::Active {
                expires_at: at(200)
            }
        );
        assert_eq!(
            check_timing(&payload, at(200)),
            TokenTiming::Expired {
                expired_at: at(200)
            }
        );
        assert_eq!(
            check_timing(&json!({"sub": "x"}), at(0)),
            TokenTiming::NoExpiry
        );
    }
}
