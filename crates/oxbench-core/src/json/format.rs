use serde::Serialize;
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

use crate::{CoreError, CoreResult};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Indent {
    #[default]
    TwoSpaces,
    FourSpaces,
    Tab,
}

impl Indent {
    pub const ALL: [Indent; 3] = [Indent::TwoSpaces, Indent::FourSpaces, Indent::Tab];

    pub const fn label(self) -> &'static str {
        match self {
            Indent::TwoSpaces => "2 spaces",
            Indent::FourSpaces => "4 spaces",
            Indent::Tab => "Tab",
        }
    }

    const fn as_bytes(self) -> &'static [u8] {
        match self {
            Indent::TwoSpaces => b"  ",
            Indent::FourSpaces => b"    ",
            Indent::Tab => b"\t",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent: Indent,
    pub should_sort_keys: bool,
}

pub fn format_pretty(value: &Value, options: FormatOptions) -> CoreResult<String> {
    let sorted;
    let value = if options.should_sort_keys {
        sorted = sort_keys(value);
        &sorted
    } else {
        value
    };
    let mut output = Vec::new();
    let formatter = PrettyFormatter::with_indent(options.indent.as_bytes());
    let mut serializer = Serializer::with_formatter(&mut output, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|error| to_format_error(&error))?;
    String::from_utf8(output).map_err(|error| CoreError::InvalidInput(error.to_string()))
}

pub fn format_minified(value: &Value, should_sort_keys: bool) -> CoreResult<String> {
    if should_sort_keys {
        return serde_json::to_string(&sort_keys(value)).map_err(|error| to_format_error(&error));
    }
    serde_json::to_string(value).map_err(|error| to_format_error(&error))
}

/// Sorts object keys at every depth. Array order is meaningful, so arrays keep theirs.
pub fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, child)| (key.clone(), sort_keys(child)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        scalar => scalar.clone(),
    }
}

fn to_format_error(error: &serde_json::Error) -> CoreError {
    CoreError::InvalidInput(format!("Could not format JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse_json;

    fn options(indent: Indent, should_sort_keys: bool) -> FormatOptions {
        FormatOptions {
            indent,
            should_sort_keys,
        }
    }

    #[test]
    fn pretty_prints_with_two_spaces_by_default() {
        let value = parse_json(r#"{"a":[1,2]}"#).unwrap();
        let output = format_pretty(&value, FormatOptions::default()).unwrap();
        assert_eq!(output, "{\n  \"a\": [\n    1,\n    2\n  ]\n}");
    }

    #[test]
    fn pretty_prints_with_tabs() {
        let value = parse_json(r#"{"a":1}"#).unwrap();
        let output = format_pretty(&value, options(Indent::Tab, false)).unwrap();
        assert_eq!(output, "{\n\t\"a\": 1\n}");
    }

    #[test]
    fn keeps_key_order_unless_sorting() {
        let value = parse_json(r#"{"b":1,"a":2}"#).unwrap();
        assert_eq!(format_minified(&value, false).unwrap(), r#"{"b":1,"a":2}"#);
        assert_eq!(format_minified(&value, true).unwrap(), r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn sorts_nested_keys_but_not_arrays() {
        let value = parse_json(r#"{"z":[{"y":1,"x":2},3,1],"a":{"d":1,"c":2}}"#).unwrap();
        let output = format_minified(&value, true).unwrap();
        assert_eq!(output, r#"{"a":{"c":2,"d":1},"z":[{"x":2,"y":1},3,1]}"#);
    }

    #[test]
    fn keeps_number_digits_exact() {
        // Exact-number mode keeps the digits; serde_json only spells the exponent sign out.
        let value = parse_json(r"[1.10, 1e3, -0, 98765432109876543210]").unwrap();
        assert_eq!(
            format_minified(&value, false).unwrap(),
            "[1.10,1e+3,-0,98765432109876543210]"
        );
    }

    #[test]
    fn keeps_unicode_text() {
        let value = parse_json(r#"{"name":"Zürich 東京"}"#).unwrap();
        assert_eq!(
            format_minified(&value, false).unwrap(),
            r#"{"name":"Zürich 東京"}"#
        );
    }
}
