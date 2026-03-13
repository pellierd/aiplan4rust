use thiserror::Error;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::grounding::problem::registry::value::error::ValueRegistryError;

/// Errors that can occur during the execution or initialization of a bindings iterator.
///
/// This enum handles failures related to the underlying value registry as well as
/// limits inherent to combinatorial grounding, such as arity-related overflows.
#[derive(Error, Debug)]
pub enum BindingsIteratorError {
    /// Transparent wrapper for errors originating from the [`ValueRegistry`].
    ///
    /// This typically occurs when a variable's type cannot be resolved to a
    /// valid domain (e.g., out-of-bounds type or cycle).
    #[error(transparent)]
    ValueRegistry(#[from] ValueRegistryError),

    /// The Cartesian product of the domains exceeds the maximum capacity of a `usize`.
    ///
    /// In grounded planning, this represents a "Combinatorial Explosion" where
    /// the number of possible ground actions/atoms for a given operator is
    /// mathematically too large to be indexed (exceeding $2^{64}-1$ on 64-bit systems).
    ///
    /// ### Parameters
    /// * `arity`: The number of parameters (variables) of the operator being grounded.
    #[error("Combinatorial explosion (Arity {arity}): the number of combinations exceeds system limits.")]
    CombinatorialExplosion {
        /// The number of variables (arity) involved in the calculation.
        arity: usize,
    },
}

impl BindingsIteratorError {
    /// Creates a new `CombinatorialExplosion` error.
    ///
    /// This helper is used when a multiplication of domain sizes during the
    /// pre-calculation of the iterator's size overflows.
    ///
    /// # Parameters
    /// * `arity`: The number of variables or domains that triggered the overflow.
    ///
    /// # Returns
    /// * An instance of [`BindingsIteratorError::CombinatorialExplosion`].
    ///
    /// # Example
    /// ```ignore
    /// let total_size = size_a.checked_mul(size_b)
    ///     .ok_or_else(|| BindingsIteratorError::combinatorial_explosion(2))?;
    /// ```
    #[track_caller]
    pub fn combinatorial_explosion(arity: usize) -> Self {
        Self::CombinatorialExplosion { arity }.trace()
    }
}

impl Traceable for BindingsIteratorError {}
