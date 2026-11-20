use serde_json::Value;
use serde_json_path::JsonPath;

use crate::{CoreError, CoreResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryMatch {
    /// Normalized path of the match, such as `$['items'][0]`.
    pub path: String,
    pub value: Value,
}

/// Runs an RFC 9535 `JSONPath` expression.
pub fn query_json_path(document: &Value, expression: &str) -> CoreResult<Vec<QueryMatch>> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(CoreError::InvalidInput(
            "Type a JSONPath expression, for example $.items[*].id".to_owned(),
        ));
    }
    let path = JsonPath::parse(expression)
        .map_err(|error| CoreError::InvalidInput(format!("Invalid JSONPath: {error}")))?;
    let matches = path
        .query_located(document)
        .all()
        .into_iter()
        .map(|node| QueryMatch {
            path: node.location().to_string(),
            value: node.node().clone(),
        })
        .collect();
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{PathSegment, format_json_path, parse_json};
    use serde_json::json;

    fn document() -> Value {
        json!({"items": [{"id": 1, "price": 5}, {"id": 2, "price": 15}], "odd key": true})
    }

    #[test]
    fn selects_values_with_their_paths() {
        let matches = query_json_path(&document(), "$.items[*].id").unwrap();
        let found: Vec<(&str, &Value)> = matches
            .iter()
            .map(|m| (m.path.as_str(), &m.value))
            .collect();
        assert_eq!(
            found,
            [
                ("$['items'][0]['id']", &json!(1)),
                ("$['items'][1]['id']", &json!(2))
            ]
        );
    }

    #[test]
    fn supports_filters() {
        let matches = query_json_path(&document(), "$.items[?@.price > 10].id").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, json!(2));
    }

    #[test]
    fn filters_compare_exact_numbers() {
        let document = parse_json(r#"[{"n": 1.10}, {"n": 2}]"#).unwrap();
        let matches = query_json_path(&document, "$[?@.n < 1.5].n").unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn no_match_is_an_empty_list() {
        assert_eq!(query_json_path(&document(), "$.missing").unwrap(), []);
    }

    #[test]
    fn rejects_an_invalid_expression() {
        let error = query_json_path(&document(), "$.items[").unwrap_err();
        assert!(error.to_string().starts_with("Invalid JSONPath"));
    }

    #[test]
    fn rejects_an_empty_expression() {
        assert!(query_json_path(&document(), " ").is_err());
    }

    #[test]
    fn paths_copied_from_the_tree_work_as_queries() {
        let path = format_json_path(&[PathSegment::Key("odd key".into())]);
        let matches = query_json_path(&document(), &path).unwrap();
        assert_eq!(matches[0].value, json!(true));
    }
}
