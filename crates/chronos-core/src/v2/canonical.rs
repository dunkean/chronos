//! Octets JSON canoniques (§4.2). Le choix de la projection et l'ordre des
//! listes appartiennent au contrat appelant ; cette fonction ne les devine pas.
use super::model::{ErrorCode, HistoryError};
use serde::Serialize;
use serde_json::Value;

/// Fait d'une liste triée par ID (§4.2). `normalize` ordonne ses propres listes
/// triées ; ses épisodes et registres, listes ordonnées, n'y sont pas touchés.
pub trait CanonicalRecord: Clone + Serialize {
    type Key: Ord;
    fn canonical_key(&self) -> Self::Key;
    fn normalize(&mut self) {}
}
/// Octets canoniques d'une liste de faits, indépendants de l'ordre reçu.
pub fn canonical_list_bytes<T: CanonicalRecord>(items: &[T]) -> Result<Vec<u8>, HistoryError> {
    let mut items = items.to_vec();
    items.iter_mut().for_each(T::normalize);
    items.sort_by_key(T::canonical_key);
    canonical_value_bytes(&items)
}
/// Sérialise puis canonise une projection déjà normalisée.
pub(crate) fn canonical_value_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, HistoryError> {
    let value = serde_json::to_value(value)
        .map_err(|e| HistoryError::new(ErrorCode::InvalidCanonicalValue, e.to_string()))?;
    canonical_bytes(&value)
}

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
