use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

/// Formats a location as RFC 9535 `JSONPath` (`$.user.tags[0]`, `$['odd key']`),
/// so a copied path can be pasted straight into the Query tab.
pub fn format_json_path(segments: &[PathSegment]) -> String {
    let mut path = String::from("$");
    for segment in segments {
        match segment {
            PathSegment::Key(key) if is_plain_identifier(key) => {
                path.push('.');
                path.push_str(key);
            }
            PathSegment::Key(key) => {
                path.push_str("['");
                path.push_str(&escape_single_quoted(key));
                path.push_str("']");
            }
            PathSegment::Index(index) => {
                // Writing to a String cannot fail.
                let _ = write!(path, "[{index}]");
            }
        }
    }
    path
}

fn is_plain_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn escape_single_quoted(key: &str) -> String {
    key.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use PathSegment::{Index, Key};

    #[test]
    fn root_is_a_dollar_sign() {
        assert_eq!(format_json_path(&[]), "$");
    }

    #[test]
    fn uses_dot_notation_for_identifiers() {
        let path = [Key("user".into()), Key("tags".into()), Index(2)];
        assert_eq!(format_json_path(&path), "$.user.tags[2]");
    }

    #[test]
    fn quotes_other_keys() {
        let path = [
            Key("first name".into()),
            Key("it's".into()),
            Key("9lives".into()),
            Key(String::new()),
        ];
        assert_eq!(
            format_json_path(&path),
            r"$['first name']['it\'s']['9lives']['']"
        );
    }
}
