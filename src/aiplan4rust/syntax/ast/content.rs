use std::fmt;
use ordered_float::OrderedFloat;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;
use crate::aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Optimization, Requirement};
use crate::aiplan4rust::syntax::StringInterner;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    #[default]
    None,                             // Pas de données associées
    Ident(usize),                    // Index vers le string pool
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),       // Pour les float
    Requirement(Requirement),        // Enum définie ailleurs
    Comparison(BinaryComp),          // Enum définie ailleurs
    Assign(AssignOp),              // Enum définie ailleurs
    Operation(ArithmeticOp),      // Enum définie ailleurs
    Optimization(Optimization),      // Enum définie ailleurs
}

impl Content {
    pub fn display_with_context(&self, ctx: &StringInterner) -> String {
        match self {
            Content::None => "".to_string(),
            Content::Ident(idx) => {
                ctx.get_str(*idx).unwrap_or("(unknown)").to_string()
            }
            Content::Float(val) => format!("{}", val),
            Content::Requirement(req) => format!("{:?}", req),
            Content::Comparison(comp) => format!("{:?}", comp),
            Content::Assign(assign) => format!("{:?}", assign),
            Content::Operation(op) => format!("{:?}", op),
            Content::Optimization(opt) => format!("{:?}", opt),
        }
    }
}
impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => {
                // Attention ici, il te faut accès au context pour récupérer la chaîne
                // Soit tu passes le contexte différemment, soit tu fais une méthode dédiée (voir ci-dessous)
                write!(f, "Ident({})", idx) // Placeholder, à améliorer
            }
            Content::Float(val) => write!(f, "{}", val),
            Content::Requirement(req) => write!(f, "{:?}", req),  // À améliorer avec Display si possible
            Content::Comparison(comp) => write!(f, "{:?}", comp),
            Content::Assign(assign) => write!(f, "{:?}", assign),
            Content::Operation(op) => write!(f, "{:?}", op),
            Content::Optimization(opt) => write!(f, "{:?}", opt),
        }
    }
}

/// Serialization implementation for `OrderedFloat<f64>`.
///
/// This function implements custom serialization for the `OrderedFloat<f64>` type, which
/// wraps a `f64` value while preserving the order of floating-point numbers, handling edge cases
/// like NaN values.
///
/// # Parameters
/// - `x`: A reference to the `OrderedFloat<f64>` value that needs to be serialized.
/// - `serializer`: The serializer that will be used to convert the `OrderedFloat<f64>` to a
///   suitable format (e.g., JSON).
///
/// # Type Parameters
/// - `S`: The type of the serializer that implements the `Serializer` trait.
///
/// # Return Value
/// - This function returns the result of calling the `serialize_f64` method on the serializer,
///   which will serialize the inner `f64` value of the `OrderedFloat`.
fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(x.into_inner())
}

/// Deserialization implementation for `OrderedFloat<f64>`.
///
/// This function implements custom deserialization for the `OrderedFloat<f64>` type, which wraps a
/// `f64` value. It allows deserializing a floating-point number and wrapping it into an
/// `OrderedFloat<f64>`.
///
/// # Parameters
/// - `deserializer`: The deserializer that will be used to convert the serialized data back into an
///   `OrderedFloat<f64>`.
///
/// # Type Parameters
/// - `'de`: The lifetime of the deserialization data.
/// - `D`: The type of the deserializer, which implements the `Deserializer` trait.
///
/// # Return Value
/// - This function returns the result of deserializing the floating-point number into an
///   `OrderedFloat<f64>` wrapped value.
fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
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
