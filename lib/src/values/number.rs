//! Numbers in contexts. They keep the type they were read with, but compare by value, so
//! `2 == 2.0` and `2 < 3.0`

use serde::Serialize;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};

/// A number from a context, a manifest or a variable include
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Number {
    /// The number, in the type it was read with
    inner: NumberVariant,
}

/// The type a number was read with
#[derive(Clone, Copy)]
enum NumberVariant {
    /// A positive integer
    Unsigned(u64),
    /// An integer read or given as `i64`, usually a negative one
    Signed(i64),
    /// A number with a fraction
    Float(f64),
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        Number {
            inner: NumberVariant::Unsigned(value),
        }
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Number {
            inner: NumberVariant::Signed(value),
        }
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Number {
            inner: NumberVariant::Float(value),
        }
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.inner {
            NumberVariant::Unsigned(u) => serializer.serialize_u64(u),
            NumberVariant::Signed(s) => serializer.serialize_i64(s),
            NumberVariant::Float(f) => serializer.serialize_f64(f),
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
    /// Integers compare as `i64`. As soon as a float is involved, both compare as `f64`
    fn total_cmp(&self, other: &Self) -> Ordering {
        use NumberVariant::*;

        match (*self, *other) {
            (Unsigned(a), Unsigned(b)) => a.cmp(&b),
            (Signed(a), Signed(b)) => a.cmp(&b),
            (Unsigned(a), Signed(b)) => (a as i64).cmp(&b),
            (Signed(a), Unsigned(b)) => a.cmp(&(b as i64)),
            (Float(a), Float(b)) => cmp_floats(a, b),
            (Signed(a), Float(b)) => cmp_floats(a as f64, b),
            (Unsigned(a), Float(b)) => cmp_floats(a as f64, b),
            (Float(a), Signed(b)) => cmp_floats(a, b as f64),
            (Float(a), Unsigned(b)) => cmp_floats(a, b as f64),
        }
    }
}

/// Like `partial_cmp`, but sorts NaN last. Not `f64::total_cmp`, which also orders `-0.0`
/// before `0.0` and puts negative NaN first
fn cmp_floats(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or_else(|| {
        if !a.is_nan() {
            Ordering::Less
        } else if !b.is_nan() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    })
}

impl PartialEq for NumberVariant {
    fn eq(&self, other: &Self) -> bool {
        use NumberVariant::*;

        match (*self, *other) {
            (Unsigned(a), Unsigned(b)) => a == b,
            (Signed(a), Signed(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Unsigned(a), Signed(b)) => (a as i64) == b,
            (Signed(a), Unsigned(b)) => a == (b as i64),
            (Unsigned(a), Float(b)) => (a as f64) == b,
            (Signed(a), Float(b)) => (a as f64) == b,
            (Float(a), Unsigned(b)) => a == (b as f64),
            (Float(a), Signed(b)) => a == (b as f64),
        }
    }
}

impl PartialOrd for NumberVariant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.total_cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Number;
    use pretty_assertions::assert_eq;

    #[test]
    fn numbers_compare_by_value_whatever_their_type() {
        let twos = [Number::from(2u64), Number::from(2i64), Number::from(2.0)];
        let threes = [Number::from(3u64), Number::from(3i64), Number::from(3.0)];

        for two in &twos {
            for other_two in &twos {
                assert_eq!(two, other_two);
            }
            for three in &threes {
                assert!(two < three, "{two} < {three}");
                assert!(three > two, "{three} > {two}");
            }
        }
    }

    #[test]
    fn nan_sorts_last() {
        let nan = Number::from(f64::NAN);

        assert!(Number::from(1.0) < nan);
        assert!(Number::from(1u64) < nan);
        assert!(nan > Number::from(-1i64));
        assert_eq!(
            Some(std::cmp::Ordering::Equal),
            nan.partial_cmp(&Number::from(f64::NAN))
        );
    }

    #[test]
    fn numbers_print_without_their_type() {
        assert_eq!("2", Number::from(2u64).to_string());
        assert_eq!("-2", Number::from(-2i64).to_string());
        assert_eq!("2", Number::from(2.0).to_string());
        assert_eq!("2.5", Number::from(2.5).to_string());
        assert_eq!("Number(2)", format!("{:?}", Number::from(2i64)));
    }
}
