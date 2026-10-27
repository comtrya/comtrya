//! Converts JSON into values. Objects become lists of their values, and array items that
//! can't be converted are left out

use super::Value;
use anyhow::anyhow;
use serde_json::Value as JsonValue;

impl TryFrom<JsonValue> for Value {
    type Error = anyhow::Error;

    fn try_from(from: JsonValue) -> Result<Self, Self::Error> {
        let value = match from {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => b.into(),
            JsonValue::Number(number) => from_number(&number)?,
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(a) => from_items(a),
            JsonValue::Object(o) => from_items(o.into_iter().map(|(_, value)| value)),
        };

        Ok(value)
    }
}

/// Keeps integers as integers, everything else becomes a float
fn from_number(number: &serde_json::Number) -> anyhow::Result<Value> {
    if let Some(n) = number.as_u64() {
        Ok(n.into())
    } else if let Some(n) = number.as_i64() {
        Ok(n.into())
    } else if let Some(n) = number.as_f64() {
        Ok(n.into())
    } else {
        Err(anyhow!("Failed converting number {number:?} to json."))
    }
}

/// A list of the items that can be converted
fn from_items(items: impl IntoIterator<Item = JsonValue>) -> Value {
    Value::List(
        items
            .into_iter()
            .filter_map(|item| Value::try_from(item).ok())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn json_numbers_keep_integers() {
        assert_eq!("3", Value::try_from(json!(3)).unwrap().to_string());
        assert_eq!("-3", Value::try_from(json!(-3)).unwrap().to_string());
        assert_eq!("3.5", Value::try_from(json!(3.5)).unwrap().to_string());
    }

    #[test]
    fn json_objects_become_lists_of_their_values() {
        let value = Value::try_from(json!({"a": "x", "b": [1, null], "c": true})).unwrap();

        assert_eq!(
            Value::List(vec![
                Value::from("x"),
                Value::List(vec![Value::from(1u64), Value::Null]),
                Value::from(true),
            ]),
            value
        );
    }
}
