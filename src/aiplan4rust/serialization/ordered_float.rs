use ordered_float::OrderedFloat;
use serde::{Deserializer, Serializer, de::{self, Visitor}};
use std::fmt;

pub fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(x.into_inner())
}

pub fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OrderedFloatVisitor;

    impl<'de> Visitor<'de> for OrderedFloatVisitor {
        type Value = OrderedFloat<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a floating point number")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(OrderedFloat(value))
        }
    }

    deserializer.deserialize_f64(OrderedFloatVisitor)
}
