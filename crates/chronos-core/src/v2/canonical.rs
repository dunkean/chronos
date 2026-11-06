//! Octets JSON canoniques (§4.2). Le choix de la projection et l'ordre des
//! listes appartiennent au contrat appelant ; cette fonction ne les devine pas.
use super::model::{ErrorCode, HistoryError};
use serde_json::Value;

/// UTF-8 compact, objets triés par octets UTF-8, entiers uniquement.
/// L'entrée est une valeur JSON : aucune conversion implicite de NaN en null.
pub fn canonical_bytes(value: &Value) -> Result<Vec<u8>, HistoryError> {
    fn write(value: &Value, out: &mut Vec<u8>) -> Result<(), HistoryError> {
        match value {
            Value::Null => out.extend_from_slice(b"null"),
            Value::Bool(v) => out.extend_from_slice(if *v { b"true" } else { b"false" }),
            Value::Number(n) => {
                if n.is_f64() {
                    return Err(HistoryError::new(
                        ErrorCode::InvalidCanonicalValue,
                        "flottant interdit",
                    ));
                }
                out.extend_from_slice(n.to_string().as_bytes());
            }
            Value::String(s) => {
                out.extend_from_slice(serde_json::to_string(s).expect("chaîne JSON").as_bytes())
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
