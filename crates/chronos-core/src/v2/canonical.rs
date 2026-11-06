//! Canonical JSON bytes (§4.2). The choice of projection and the order of
//! lists belong to the calling contract; this function does not guess them.
use super::model::{ErrorCode, HistoryError};
use serde::Serialize;
use serde_json::Value;

/// Fact of a list sorted by ID (§4.2). `normalize` orders its own sorted
/// lists; its episodes and registers, which are ordered lists, are left alone.
pub trait CanonicalRecord: Clone + Serialize {
    type Key: Ord;
    fn canonical_key(&self) -> Self::Key;
    fn normalize(&mut self) {}
}
/// Canonical bytes of a list of facts, independent of the order received.
pub fn canonical_list_bytes<T: CanonicalRecord>(items: &[T]) -> Result<Vec<u8>, HistoryError> {
    let mut items = items.to_vec();
    items.iter_mut().for_each(T::normalize);
    items.sort_by_key(T::canonical_key);
    canonical_value_bytes(&items)
}
/// Serializes then canonicalizes an already normalized projection.
pub(crate) fn canonical_value_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, HistoryError> {
    let value = serde_json::to_value(value)
        .map_err(|e| HistoryError::new(ErrorCode::InvalidCanonicalValue, e.to_string()))?;
    canonical_bytes(&value)
}

/// Compact UTF-8, objects sorted by UTF-8 bytes, integers only.
/// The input is a JSON value: no implicit conversion of NaN to null.
pub fn canonical_bytes(value: &Value) -> Result<Vec<u8>, HistoryError> {
    fn write(value: &Value, out: &mut Vec<u8>) -> Result<(), HistoryError> {
        match value {
            Value::Null => out.extend_from_slice(b"null"),
            Value::Bool(v) => out.extend_from_slice(if *v { b"true" } else { b"false" }),
            Value::Number(n) => {
                if n.is_f64() {
                    return Err(HistoryError::new(
                        ErrorCode::InvalidCanonicalValue,
                        "canonical.float_forbidden",
                    ));
                }
                out.extend_from_slice(n.to_string().as_bytes());
            }
            Value::String(s) => {
                out.extend_from_slice(serde_json::to_string(s).expect("JSON string").as_bytes())
            }
            Value::Array(values) => {
                out.push(b'[');
                for (i, v) in values.iter().enumerate() {
                    if i != 0 {
                        out.push(b',');
                    }
                    write(v, out)?;
                }
                out.push(b']');
            }
            Value::Object(values) => {
                out.push(b'{');
                let mut entries: Vec<_> = values.iter().collect();
                entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                for (i, (k, v)) in entries.into_iter().enumerate() {
                    if i != 0 {
                        out.push(b',');
                    }
                    write(&Value::String(k.clone()), out)?;
                    out.push(b':');
                    write(v, out)?;
                }
                out.push(b'}');
            }
        }
        Ok(())
    }
    let mut bytes = Vec::new();
    write(value, &mut bytes)?;
    Ok(bytes)
}
