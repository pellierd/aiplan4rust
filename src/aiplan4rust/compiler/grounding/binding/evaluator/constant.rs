use crate::aiplan4rust::support::lang::ObjectId;
use ordered_float::OrderedFloat;

/// Represents a fully resolved constant value extracted during compile-time or static analysis.
///
/// This enumeration serves as the unified domain exchange language between the Logical Intermediate
/// Representation (LIR), the grounding binder, and external semantic modules (such as
/// rigid relation analyzers or functional inertia detectors). By isolating these primitives,
/// the pipeline can cleanly separate dynamic state tracking from static evaluation.
///
/// # Mathematical & Floating-Point Safety
///
/// Numerical values are internally encapsulated inside an `OrderedFloat<f64>`. This guarantees
/// that the enum satisfies the strict total order property required by standard Rust containers
/// (e.g., `Eq`, `Hash`) without risking undefined behavior or panics from `NaN` comparisons during
/// lookups or hash-consing passes.
///
/// # Examples
///
/// Creating constants and using transparent type conversions:
///
/// ```rust
/// use crate::aiplan4rust::grounding::binding::evaluator::ExprConstant;
/// use crate::aiplan4rust::lang::ObjectId;
///
/// // Direct instantiation
/// let is_static_true = ExprConstant::Boolean(true);
/// let location_obj = ExprConstant::Object(ObjectId::new(100));
///
/// // Transparent conversion from primitives using standard `.into()`
/// let pi_constant: ExprConstant = 3.14159.into();
/// let integer_constant: ExprConstant = 42.into();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExprConstant {
    /// A boolean invariant, typically representing a grounded static PDDL predicate
    /// whose truth value cannot be altered by any action effect.
    Boolean(bool),

    /// A total-ordered numerical constant representing an invariant metric fluent
    /// or a static function output.
    Number(OrderedFloat<f64>),

    /// A reference to a constant PDDL object instance, commonly returned by
    /// functional object fluents that remain rigid throughout execution.
    Object(ObjectId),
}

// --- TRANSPARENT CONVERSIONS (FROM TRAIT IMPLEMENTATIONS) ---

impl From<f64> for ExprConstant {
    /// Seamlessly converts a standard 64-bit floating-point primitive into an `ExprConstant::Number`.
    #[inline]
    fn from(value: f64) -> Self {
        ExprConstant::Number(OrderedFloat(value))
    }
}

impl From<f32> for ExprConstant {
    /// Automatically promotes and converts a 32-bit floating-point primitive into an `ExprConstant::Number`.
    #[inline]
    fn from(value: f32) -> Self {
        ExprConstant::Number(OrderedFloat(value as f64))
    }
}

impl From<i32> for ExprConstant {
    /// Automatically casts and wraps a standard signed 32-bit integer into a floating-point `ExprConstant::Number`.
    #[inline]
    fn from(value: i32) -> Self {
        ExprConstant::Number(OrderedFloat(value as f64))
    }
}
