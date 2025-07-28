//! Serialization and deserialization helpers for `OrderedFloat<f64>` using Serde.
//!
//! The `OrderedFloat` wrapper from the `ordered_float` crate provides total ordering for floating-point
//! numbers, which is not possible with plain `f64` due to IEEE NaN semantics.
//!
//! However, `OrderedFloat<f64>` does not implement Serde's `Serialize` and `Deserialize` traits by default.
//! This module provides custom Serde serializer and deserializer functions that convert
//! between `OrderedFloat<f64>` and a regular `f64` during serialization and deserialization.
//!
//! This is necessary to allow types containing `OrderedFloat<f64>` to be serialized and
//! deserialized seamlessly with Serde-compatible formats such as JSON, YAML, or BSON.
//!
//! # Provided Functions
//!
//! - `serialize_ordered_float`: Serializes an `OrderedFloat<f64>` as a plain `f64`.
//! - `deserialize_ordered_float`: Deserializes an `f64` value into an `OrderedFloat<f64>`.

use ordered_float::OrderedFloat;
use serde::{
    Deserializer, Serializer,
    de::{self, Visitor},
};
use std::fmt;

/// Serializes an `OrderedFloat<f64>` as a regular `f64` value.
///
/// This function extracts the inner `f64` from the `OrderedFloat` wrapper and
/// serializes it directly. This allows integration of `OrderedFloat<f64>` into
/// Serde serialization workflows without extra boilerplate.
///
/// # Arguments
///
/// * `x` - The `OrderedFloat<f64>` instance to serialize.
/// * `serializer` - The Serde serializer to write to.
///
/// # Returns
///
/// A `Result` with the serialized output or a serialization error.
///
/// # Example
///
/// ```rust
/// use ordered_float::OrderedFloat;
/// use serde_json;
///
/// let val = OrderedFloat(1.23);
/// let json = serde_json::to_string(&val).unwrap();
/// assert_eq!(json, "1.23");
/// ```
pub fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(x.into_inner())
}

/// Deserializes an `f64` value into an `OrderedFloat<f64>`.
///
/// This function expects the input data to be a floating-point number and
/// wraps it into an `OrderedFloat<f64>`. It provides a Serde-compatible
/// way to parse floating-point numbers directly into `OrderedFloat`.
///
/// # Arguments
///
/// * `deserializer` - The Serde deserializer providing the data.
///
/// # Returns
///
/// A `Result` containing the deserialized `OrderedFloat<f64>` or an error if
/// the input is not a valid floating-point number.
///
/// # Example
///
/// ```rust
/// use ordered_float::OrderedFloat;
/// use serde_json;
///
/// let json = "1.23";
/// let val: OrderedFloat<f64> = serde_json::from_str(json).unwrap();
/// assert_eq!(val, OrderedFloat(1.23));
/// ```
pub fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OrderedFloatVisitor;

    impl<'de> Visitor<'de> for OrderedFloatVisitor {
        type Value = OrderedFloat<f64>;

        /// Specifies what the deserializer expects: here, a floating point number.
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a floating point number")
        }

        /// Handles the deserialized floating point value by wrapping it
        /// into an `OrderedFloat` for ordered comparisons.
        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(OrderedFloat(value))
        }
    }

    deserializer.deserialize_f64(OrderedFloatVisitor)
}
