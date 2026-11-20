use serde_json::Value;

use super::path::{PathSegment, format_json_path};

const PREVIEW_MAX_CHARS: usize = 80;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub preview: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    /// More matches exist beyond the limit.
    pub is_truncated: bool,
}

/// Finds keys and scalar values that contain `query`, ignoring case.
pub fn search_json(value: &Value, query: &str, limit: usize) -> SearchResult {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return SearchResult::default();
    }
    let mut searcher = Searcher {
        needle,
        limit,
        path: Vec::new(),
        result: SearchResult::default(),
    };
    searcher.visit(value);
    searcher.result
}

/// A one-line summary: `{3 keys}`, `[5 items]`, or the scalar shortened to fit.
pub fn preview_value(value: &Value) -> String {
    match value {
        Value::Object(map) => format!("{{{} {}}}", map.len(), plural(map.len(), "key", "keys")),
        Value::Array(items) => {
            format!("[{} {}]", items.len(), plural(items.len(), "item", "items"))
        }
        scalar => shorten(&scalar.to_string(), PREVIEW_MAX_CHARS),
    }
}

struct Searcher {
    needle: String,
    limit: usize,
    path: Vec<PathSegment>,
    result: SearchResult,
}

impl Searcher {
    fn visit(&mut self, value: &Value) {
        if self.result.is_truncated {
            return;
        }
        if self.is_key_match() || self.is_scalar_match(value) {
            self.record_hit(value);
        }
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    self.path.push(PathSegment::Key(key.clone()));
                    self.visit(child);
                    self.path.pop();
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    self.path.push(PathSegment::Index(index));
                    self.visit(child);
                    self.path.pop();
                }
            }
            _ => {}
        }
    }

    fn is_key_match(&self) -> bool {
        matches!(self.path.last(), Some(PathSegment::Key(key)) if key.to_lowercase().contains(&self.needle))
    }

    fn is_scalar_match(&self, value: &Value) -> bool {
        let text = match value {
            Value::String(text) => text.to_lowercase(),
            Value::Number(number) => number.to_string(),
            Value::Bool(flag) => flag.to_string(),
            Value::Null => "null".to_owned(),
            Value::Object(_) | Value::Array(_) => return false,
        };
        text.contains(&self.needle)
    }

    fn record_hit(&mut self, value: &Value) {
        if self.result.hits.len() >= self.limit {
            self.result.is_truncated = true;
            return;
        }
        self.result.hits.push(SearchHit {
            path: format_json_path(&self.path),
            preview: preview_value(value),
        });
    }
}

fn plural<'a>(count: usize, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}

fn shorten(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{kept}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hit_paths(result: &SearchResult) -> Vec<&str> {
        result.hits.iter().map(|hit| hit.path.as_str()).collect()
    }

    #[test]
    fn finds_keys_and_values_ignoring_case() {
        let document =
            json!({"userName": "Ada", "friends": [{"name": "Grace"}], "note": "ADA rocks"});
        let result = search_json(&document, "ada", 10);
        assert_eq!(hit_paths(&result), ["$.userName", "$.note"]);
        let result = search_json(&document, "name", 10);
        assert_eq!(hit_paths(&result), ["$.userName", "$.friends[0].name"]);
    }

    #[test]
    fn matches_numbers_booleans_and_null() {
        let document = json!({"a": 404, "b": true, "c": null});
        assert_eq!(hit_paths(&search_json(&document, "40", 10)), ["$.a"]);
        assert_eq!(hit_paths(&search_json(&document, "true", 10)), ["$.b"]);
        assert_eq!(hit_paths(&search_json(&document, "null", 10)), ["$.c"]);
    }

    #[test]
    fn blank_query_finds_nothing() {
        assert_eq!(
            search_json(&json!({"a": 1}), "  ", 10),
            SearchResult::default()
        );
    }

    #[test]
    fn stops_at_the_limit_and_says_so() {
        let document = json!(["x", "x", "x"]);
        let result = search_json(&document, "x", 2);
        assert_eq!(result.hits.len(), 2);
        assert!(result.is_truncated);
    }

    #[test]
    fn previews_containers_and_long_strings() {
        assert_eq!(preview_value(&json!({"a": 1})), "{1 key}");
        assert_eq!(preview_value(&json!([1, 2])), "[2 items]");
        let long = "a".repeat(200);
        let preview = preview_value(&json!(long));
        assert_eq!(preview.chars().count(), PREVIEW_MAX_CHARS);
        assert!(preview.ends_with("..."));
    }
}
