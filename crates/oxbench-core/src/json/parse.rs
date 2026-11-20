use serde_json::Value;

use crate::{CoreError, CoreResult};

const BYTE_ORDER_MARK: char = '\u{feff}';

/// Parses JSON text. A raw HTTP response (status line and headers before a blank line)
/// is accepted too, so an API response can be pasted as is.
pub fn parse_json(input: &str) -> CoreResult<Value> {
    let input = input.trim_start_matches(BYTE_ORDER_MARK);
    let (body, header_lines) = split_http_body(input);
    if body.trim().is_empty() {
        return Err(CoreError::InvalidInput(
            "Paste or type some JSON.".to_owned(),
        ));
    }
    serde_json::from_str(body).map_err(|error| to_invalid_json(&error, header_lines))
}

/// Returns the body and how many lines came before it, so error positions match the input.
fn split_http_body(input: &str) -> (&str, usize) {
    if !input.starts_with("HTTP/") {
        return (input, 0);
    }
    let separator = ["\r\n\r\n", "\n\n"]
        .iter()
        .filter_map(|separator| input.find(separator).map(|index| (index, separator.len())))
        .min_by_key(|(index, _)| *index);
    let Some((index, separator_len)) = separator else {
        return (input, 0);
    };
    let body_start = index + separator_len;
    let header_lines = input[..body_start].matches('\n').count();
    (&input[body_start..], header_lines)
}

fn to_invalid_json(error: &serde_json::Error, header_lines: usize) -> CoreError {
    let full_message = error.to_string();
    let position_suffix = format!(" at line {} column {}", error.line(), error.column());
    let message = full_message
        .strip_suffix(&position_suffix)
        .unwrap_or(&full_message)
        .to_owned();
    CoreError::InvalidJson {
        message,
        line: error.line() + header_lines,
        column: error.column(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_plain_json() {
        assert_eq!(
            parse_json(r#"{"a": [1, true, null]}"#),
            Ok(json!({"a": [1, true, null]}))
        );
    }

    #[test]
    fn ignores_a_byte_order_mark() {
        assert_eq!(parse_json("\u{feff}[1]"), Ok(json!([1])));
    }

    #[test]
    fn reports_line_and_column_of_a_syntax_error() {
        let error = parse_json("{\n  \"a\": 1,\n  \"b\": \n}").unwrap_err();
        let CoreError::InvalidJson {
            line,
            column,
            message,
        } = error
        else {
            panic!("expected InvalidJson, got {error:?}");
        };
        assert_eq!((line, column), (4, 1));
        assert_eq!(message, "expected value");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(
            parse_json("  \n"),
            Err(CoreError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_trailing_characters() {
        assert!(matches!(
            parse_json("{} x"),
            Err(CoreError::InvalidJson { .. })
        ));
    }

    #[test]
    fn accepts_a_raw_http_response() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\": true}";
        assert_eq!(parse_json(response), Ok(json!({"ok": true})));
    }

    #[test]
    fn error_lines_in_an_http_response_count_the_headers() {
        let response = "HTTP/2 200\ncontent-type: application/json\n\n{\n  \"ok\": }";
        let Err(CoreError::InvalidJson { line, .. }) = parse_json(response) else {
            panic!("expected InvalidJson");
        };
        assert_eq!(line, 5);
    }

    #[test]
    fn keeps_big_numbers_exact() {
        let value = parse_json("12345678901234567890123").unwrap();
        assert_eq!(value.to_string(), "12345678901234567890123");
    }
}
