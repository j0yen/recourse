use serde_json::Value;
use std::collections::BTreeMap;

/// Canonicalize a JSON value: sort object keys, compact (no whitespace).
/// This is used to produce a stable byte sequence for blake3 hashing.
pub fn canonicalize(value: &Value) -> String {
    serde_json::to_string(&canonical_value(value))
        .expect("canonical serialization is infallible for valid JSON")
}

fn canonical_value(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            // Collect into BTreeMap to sort keys
            let sorted: BTreeMap<_, _> =
                map.iter().map(|(k, v)| (k.clone(), canonical_value(v))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

/// Hash canonical JSON and return "blake3:<hex>"
pub fn digest(canonical_json: &str) -> String {
    let hash = blake3::hash(canonical_json.as_bytes());
    format!("blake3:{}", hash.to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys() {
        let v = json!({"z": 1, "a": 2, "m": 3});
        let s = canonicalize(&v);
        assert_eq!(s, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn canonical_is_compact() {
        let v = json!({"key": "value"});
        let s = canonicalize(&v);
        assert!(!s.contains('\n'));
        assert!(!s.contains("  "));
    }

    #[test]
    fn digest_format() {
        let d = digest(r#"{"test":1}"#);
        assert!(d.starts_with("blake3:"), "digest must start with blake3:");
        assert_eq!(d.len(), 7 + 64); // "blake3:" + 64 hex chars
    }
}
