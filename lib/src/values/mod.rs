use std::{
    cmp::Ordering,
    ffi::OsString,
    fmt::{Debug, Display},
    path::PathBuf,
};

use serde_json::Value as JsonValue;

use serde::{
    de::{Error as SError, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    String(String),
    Number(Number),
    List(Vec<Value>),
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Number {
    inner: NumberVariant,
}

#[derive(Clone, Copy)]
enum NumberVariant {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => serializer.serialize_str(s),
            Value::List(seq) => seq.serialize(serializer),
        }
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        {
            match self.inner {
                NumberVariant::Unsigned(u) => serializer.serialize_u64(u),
                NumberVariant::Signed(s) => serializer.serialize_i64(s),
                NumberVariant::Float(f) => serializer.serialize_f64(f),
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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
                Ok(Value::Number(Number {
                    inner: NumberVariant::Signed(i),
                }))
            }

            fn visit_u64<E>(self, u: u64) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Number(Number {
                    inner: NumberVariant::Unsigned(u),
                }))
            }

            fn visit_f64<E>(self, f: f64) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Number(Number {
                    inner: NumberVariant::Float(f),
                }))
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

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Debug for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => formatter.write_str("Null"),
            Value::String(string) => write!(formatter, "String({string:?})"),
            Value::Number(number) => write!(formatter, "Number({number})"),
            Value::List(list) => {
                formatter.write_str("List ")?;
                formatter.debug_list().entries(list).finish()
            }
        }
    }
}

impl Debug for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Number({self})")
    }
}

impl Display for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            NumberVariant::Unsigned(number) => Display::fmt(&number, formatter),
            NumberVariant::Signed(number) => Display::fmt(&number, formatter),
            NumberVariant::Float(number) => Display::fmt(&number, formatter),
        }
    }
}

impl NumberVariant {
    fn total_cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (NumberVariant::Unsigned(a), NumberVariant::Unsigned(b)) => a.cmp(&b),
            (NumberVariant::Signed(a), NumberVariant::Signed(b)) => a.cmp(&b),
            (NumberVariant::Unsigned(a), NumberVariant::Signed(b)) => (a as i64).cmp(&b),
            (NumberVariant::Signed(a), NumberVariant::Unsigned(b)) => a.cmp(&(b as i64)),
            (NumberVariant::Float(a), NumberVariant::Float(b)) => {
                // FIXME: change to total_cmp for Rust >= 1.62.0
                a.partial_cmp(&b).unwrap_or_else(|| {
                    // arbitrarily sort the NaN last
                    if !a.is_nan() {
                        Ordering::Less
                    } else if !b.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
            }
            (NumberVariant::Signed(a), NumberVariant::Float(b)) => {
                // FIXME: change to total_cmp for Rust >= 1.62.0
                (a as f64).partial_cmp(&b).unwrap_or_else(|| {
                    // arbitrarily sort the NaN last
                    if !(a as f64).is_nan() {
                        Ordering::Less
                    } else if !b.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
            }
            (NumberVariant::Unsigned(a), NumberVariant::Float(b)) => {
                // FIXME: change to total_cmp for Rust >= 1.62.0
                (a as f64).partial_cmp(&b).unwrap_or_else(|| {
                    // arbitrarily sort the NaN last
                    if !(a as f64).is_nan() {
                        Ordering::Less
                    } else if !b.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
            }
            (NumberVariant::Float(a), NumberVariant::Signed(b)) => {
                // FIXME: change to total_cmp for Rust >= 1.62.0
                a.partial_cmp(&(b as f64)).unwrap_or_else(|| {
                    // arbitrarily sort the NaN last
                    if !a.is_nan() {
                        Ordering::Less
                    } else if !(b as f64).is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
            }
            (NumberVariant::Float(a), NumberVariant::Unsigned(b)) => {
                // FIXME: change to total_cmp for Rust >= 1.62.0
                a.partial_cmp(&(b as f64)).unwrap_or_else(|| {
                    // arbitrarily sort the NaN last
                    if !a.is_nan() {
                        Ordering::Less
                    } else if !(b as f64).is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
            }
        }
    }
}

impl PartialEq for NumberVariant {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (NumberVariant::Unsigned(a), NumberVariant::Unsigned(b)) => a == b,
            (NumberVariant::Signed(a), NumberVariant::Signed(b)) => a == b,
            (NumberVariant::Float(a), NumberVariant::Float(b)) => a == b,
            (NumberVariant::Unsigned(a), NumberVariant::Signed(b)) => (a as i64) == b,
            (NumberVariant::Signed(a), NumberVariant::Unsigned(b)) => a == (b as i64),
            (NumberVariant::Unsigned(a), NumberVariant::Float(b)) => (a as f64) == b,
            (NumberVariant::Signed(a), NumberVariant::Float(b)) => (a as f64) == b,
            (NumberVariant::Float(a), NumberVariant::Unsigned(b)) => a == (b as f64),
            (NumberVariant::Float(a), NumberVariant::Signed(b)) => a == (b as f64),
        }
    }
}

impl PartialOrd for NumberVariant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.total_cmp(other))
    }
}

impl From<String> for Value {
    fn from(from: String) -> Self {
        Value::String(from)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(from: &'a str) -> Self {
        Value::String(from.to_string())
    }
}

impl<'a> From<std::borrow::Cow<'a, str>> for Value {
    fn from(from: std::borrow::Cow<'a, str>) -> Self {
        Value::String(from.to_string())
    }
}

impl From<OsString> for Value {
    fn from(from: OsString) -> Self {
        Value::String(from.to_str().unwrap_or("unknown").to_string())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Signed(value),
        })
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Unsigned(value),
        })
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Float(value),
        })
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Unsigned(value as u64),
        })
    }
}

impl From<PathBuf> for Value {
    fn from(from: PathBuf) -> Self {
        Value::String(from.display().to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(from: Vec<T>) -> Self {
        Value::List(from.into_iter().map(Into::into).collect())
    }
}

impl TryFrom<JsonValue> for Value {
    type Error = anyhow::Error;

    fn try_from(from: JsonValue) -> Result<Self, Self::Error> {
        let value = match from {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => b.into(),
            JsonValue::Number(number) => {
                if number.is_u64() {
                    match number.as_u64() {
                        Some(n) => n.into(),
                        None => {
                            return Err(anyhow::anyhow!(
                                "Failed converting number {:?} to json.",
                                number
                            ))
                        }
                    }
                } else if number.is_i64() {
                    match number.as_i64() {
                        Some(n) => n.into(),
                        None => {
                            return Err(anyhow::anyhow!(
                                "Failed converting number {:?} to json.",
                                number
                            ))
                        }
                    }
                } else {
                    match number.as_f64() {
                        Some(n) => n.into(),
                        None => {
                            return Err(anyhow::anyhow!(
                                "Failed converting number {:?} to json.",
                                number
                            ))
                        }
                    }
                }
            }
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(a) => Self::List(
                a.into_iter()
                    .map(TryInto::try_into)
                    .filter_map(Result::ok)
                    .collect(),
            ),
            JsonValue::Object(o) => Self::List(
                o.values()
                    .cloned()
                    .map(TryInto::try_into)
                    .filter_map(Result::ok)
                    .collect(),
            ),
        };

        Ok(value)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::Null => "null".to_string(),
                Value::String(string) => string.to_owned(),
                Value::Number(number) => number.to_string(),
                Value::List(list) => list
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            }
        )
    }
}

#[cfg(test)]
mod test {
    use std::{borrow::Cow, ffi::OsString, path::PathBuf};

    use crate::values::{Number, NumberVariant, Value};
    use anyhow::Ok;
    use pretty_assertions::assert_eq;

    #[test]
    fn from_string_tests() -> anyhow::Result<()> {
        assert_eq!(
            Value::from("John Sheppard"),
            Value::String("John Sheppard".to_string())
        );

        assert_eq!(
            Value::from("Elizabeth Weir".to_string()),
            Value::String("Elizabeth Weir".to_string())
        );

        assert_eq!(Value::from(PathBuf::new()), Value::String("".to_string()));

        assert_eq!(
            Value::from(Cow::from("Samantha Carter")),
            Value::String("Samantha Carter".to_string())
        );

        assert_eq!(
            Value::from(OsString::from("Jennifer Keller")),
            Value::String("Jennifer Keller".to_string())
        );

        Ok(())
    }

    #[test]
    fn from_vec_test() -> anyhow::Result<()> {
        assert_eq!(
            Value::from(vec!["Aiden Ford", "Rodney McKay", "Ronon Dex"]),
            Value::List(vec![
                Value::String("Aiden Ford".to_string()),
                Value::String("Rodney McKay".to_string()),
                Value::String("Ronon Dex".to_string())
            ])
        );

        Ok(())
    }

    #[test]
    fn number_compare_test() -> anyhow::Result<()> {
        // unsigned
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            })
        );

        // signed
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Signed(3)
            })
        );

        // float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        // unsigned with signed
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Signed(3)
            })
        );

        // signed with unsigned
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            })
        );

        // signed with float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        // unsigned with float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        Ok(())
    }

    #[test]
    fn debug_tests() -> anyhow::Result<()> {
        assert_eq!(format!("{:?}", Value::Null), "Null".to_string());

        assert_eq!(
            format!("{:?}", Value::String("Richard Woolsey".to_string())),
            "String(\"Richard Woolsey\")".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::List(vec![
                    Value::String("Aiden Ford".to_string()),
                    Value::String("Rodney McKay".to_string()),
                    Value::String("Ronon Dex".to_string())
                ])
            ),
            "List [String(\"Aiden Ford\"), String(\"Rodney McKay\"), String(\"Ronon Dex\")]"
                .to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Unsigned(2)
                })
            ),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Signed(2)
                })
            ),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Float(2.0)
                })
            ),
            "Number(2)".to_string()
        );

        Ok(())
    }
}
