use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JsonStats {
    pub objects: usize,
    pub arrays: usize,
    pub scalars: usize,
    pub max_depth: usize,
}

pub fn measure_json(value: &Value) -> JsonStats {
    let mut stats = JsonStats::default();
    // serde_json stops parsing at 128 levels, so this recursion stays shallow.
    visit(value, 1, &mut stats);
    stats
}

fn visit(value: &Value, depth: usize, stats: &mut JsonStats) {
    stats.max_depth = stats.max_depth.max(depth);
    match value {
        Value::Object(map) => {
            stats.objects += 1;
            for child in map.values() {
                visit(child, depth + 1, stats);
            }
        }
        Value::Array(items) => {
            stats.arrays += 1;
            for child in items {
                visit(child, depth + 1, stats);
            }
        }
        _ => stats.scalars += 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counts_containers_scalars_and_depth() {
        let stats = measure_json(&json!({"a": [1, {"b": null}], "c": "x"}));
        assert_eq!(
            stats,
            JsonStats {
                objects: 2,
                arrays: 1,
                scalars: 3,
                max_depth: 4
            }
        );
    }

    #[test]
    fn a_scalar_document_has_depth_one() {
        assert_eq!(measure_json(&json!(true)).max_depth, 1);
    }
}
