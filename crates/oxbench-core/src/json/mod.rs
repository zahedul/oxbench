//! JSON Viewer logic: parse, format, inspect, search, query (`JSONPath`) and validate (JSON Schema).

mod format;
mod parse;
mod path;
mod query;
mod schema;
mod search;
mod stats;

pub use format::{FormatOptions, Indent, format_minified, format_pretty, sort_keys};
pub use parse::parse_json;
pub use path::{PathSegment, format_json_path};
pub use query::{QueryMatch, query_json_path};
pub use schema::{SchemaViolation, validate_against_schema};
pub use search::{SearchHit, SearchResult, preview_value, search_json};
pub use serde_json::Value;
pub use stats::{JsonStats, measure_json};
