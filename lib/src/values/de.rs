//! Reads values with serde: null, numbers, strings and lists. Maps are not values

use super::{Number, Value};
use serde::de::{Error as SError, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Builds a value from what serde reads
struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any comtrya context value")
    }

    fn visit_i64<E>(self, i: i64) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::Number(Number::from(i)))
    }

    fn visit_u64<E>(self, u: u64) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::Number(Number::from(u)))
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::Number(Number::from(f)))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::String(s.to_owned()))
    }

    fn visit_string<E>(self, s: String) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::String(s))
    }

    fn visit_unit<E>(self) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E>
    where
        E: SError,
    {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::new();

        while let Some(element) = visitor.next_element()? {
            vec.push(element);
        }

        Ok(Value::List(vec))
    }
}
