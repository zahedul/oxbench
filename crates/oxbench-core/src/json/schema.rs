use serde_json::Value;

use crate::{CoreError, CoreResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaViolation {
    /// JSON Pointer to the failing value; `(root)` for the whole document.
    pub path: String,
    pub message: String,
}

/// Validates offline: remote `$ref`s are not fetched, so a schema that needs one is rejected.
pub fn validate_against_schema(
    document: &Value,
    schema: &Value,
) -> CoreResult<Vec<SchemaViolation>> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| CoreError::InvalidInput(format!("Invalid schema: {error}")))?;
    let violations = validator
        .iter_errors(document)
        .map(|error| SchemaViolation {
            path: display_pointer(&error.instance_path().to_string()),
            message: error.to_string(),
        })
        .collect();
    Ok(violations)
}

fn display_pointer(pointer: &str) -> String {
    if pointer.is_empty() {
        return "(root)".to_owned();
    }
    pointer.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user_schema() -> Value {
        json!({
            "type": "object",
            "required": ["id", "email"],
            "properties": {
                "id": {"type": "integer", "minimum": 1},
                "email": {"type": "string", "format": "email"},
                "tags": {"type": "array", "items": {"type": "string"}}
            }
        })
    }

    #[test]
    fn a_valid_document_has_no_violations() {
        let document = json!({"id": 1, "email": "ada@example.com", "tags": ["x"]});
        assert_eq!(
            validate_against_schema(&document, &user_schema()).unwrap(),
            []
        );
    }

    #[test]
    fn reports_each_failing_value_by_pointer() {
        let document = json!({"id": 0, "tags": ["ok", 7]});
        let violations = validate_against_schema(&document, &user_schema()).unwrap();
        let paths: Vec<&str> = violations.iter().map(|v| v.path.as_str()).collect();
        assert!(paths.contains(&"(root)"), "missing email: {paths:?}");
        assert!(paths.contains(&"/id"), "minimum: {paths:?}");
        assert!(paths.contains(&"/tags/1"), "item type: {paths:?}");
    }

    #[test]
    fn rejects_an_invalid_schema() {
        let error = validate_against_schema(&json!(1), &json!({"type": "nonsense"})).unwrap_err();
        assert!(error.to_string().starts_with("Invalid schema"));
    }

    #[test]
    fn does_not_fetch_remote_references() {
        let schema = json!({"$ref": "https://example.com/schema.json"});
        assert!(validate_against_schema(&json!(1), &schema).is_err());
    }
}
