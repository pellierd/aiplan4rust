use thiserror::Error;

/// Errors that can occur during the execution of a bindings iterator.
///
/// This enum covers edge cases in combinatorial logic, specifically when
/// domain sizes lead to mathematical overflows.
#[derive(Error, Debug)]
pub enum BindingsIteratorError {
    /// The Cartesian product of the domains exceeds the maximum capacity of a `usize`.
    ///
    /// This happens when the total number of possible combinations is too large
    /// to be represented or indexed on the current system (usually 64-bit).
    #[error("Combinatorial explosion (Arity {arity}): the number of combinations exceeds system limits.")]
    CombinatorialExplosion {
        /// The number of variables (arity) involved in the calculation.
        arity: usize,
    },
}

impl BindingsIteratorError {
    /// Creates a new `CombinatorialExplosion` error.
    ///
    /// This helper is used when a multiplication of domain sizes overflows.
    /// It uses `#[track_caller]` to help pinpoint where the overflow check failed.
    ///
    /// # Parameters
    /// - `arity`: The number of variables or domains that caused the overflow.
    ///
    /// # Returns
    /// - An instance of `BindingsIteratorError::CombinatorialExplosion`.
    #[track_caller]
    pub fn combinatorial_explosion(arity: usize) -> Self {
        Self::CombinatorialExplosion { arity }
    }
}
