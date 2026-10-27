//! Values of contexts: what manifests read as `{{ user.name }}`, compare in `where`, and
//! what variable includes and `Comtrya.yaml` provide

mod de;
mod json;
mod number;

pub use number::Number;

use serde::Serialize;
use std::{
    ffi::OsString,
    fmt::{Debug, Display},
    path::PathBuf,
};

/// A context value. Booleans are stored as the numbers 1 and 0
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    String(String),
    Number(Number),
    List(Vec<Value>),
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

/// How values end up in templates and commands. Lists are joined with commas
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::String(string) => f.write_str(string),
            Value::Number(number) => write!(f, "{number}"),
            Value::List(list) => {
                let items: Vec<String> = list.iter().map(|value| value.to_string()).collect();
                f.write_str(&items.join(","))
            }
        }
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
        Self::Number(value.into())
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Number(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value.into())
    }
}

/// `true` is 1 and `false` is 0
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Number((value as u64).into())
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

#[cfg(test)]
mod test {
    use std::{borrow::Cow, ffi::OsString, path::PathBuf};

    use crate::values::{Number, Value};
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
            format!("{:?}", Value::Number(Number::from(2u64))),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!("{:?}", Value::Number(Number::from(2i64))),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!("{:?}", Value::Number(Number::from(2.0))),
            "Number(2)".to_string()
        );

        Ok(())
    }
}
