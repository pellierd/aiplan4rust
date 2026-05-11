use crate::aiplan4rust::grounding::binding::iter::BindingsIteratorError;
use crate::aiplan4rust::grounding::binding::Bindings;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{ObjectId, TypeId, TypedList, VariableId};
use std::fmt;

/// A lazy combination iterator designed for exploring variable value domains.
///
/// `BindingsIterator` traverses the Cartesian product of multiple [`ValueDomain`]s.
/// It is optimized for grounding by allowing dynamic "pruning" of specific branches
/// in the search space, avoiding unnecessary computations of invalid combinations.
pub struct BindingsIterator<'a> {
    /// The variable identifiers associated with each domain in the iterator.
    variables: &'a TypedList<VariableId, TypeId>,

    /// The source value domains for each parameter, ordered by variable.
    domains: Vec<&'a [ObjectId]>,

    /// Current cursor positions within each domain.
    indices: Vec<usize>,

    /// Internal buffer used to expose the current combination without re-allocating.
    current_bindings: Bindings,

    /// A flag indicating whether the entire search space has been traversed.
    exhausted: bool,

    /// The total number of possible combinations in the Cartesian product.
    total_count: usize,

    /// Internal state to handle the first iteration correctly.
    is_first: bool,
}

impl<'a> BindingsIterator<'a> {
    /// Initializes the iterator. Returns an exhausted iterator if the variables list
    /// or any of the domains are empty.
    ///
    /// # Parameters
    /// - `variables`: The list of typed variables to instantiate.
    /// - `value_registry`: The evaluator used to retrieve domains for each variable.
    ///
    /// # Returns
    /// - A new `BindingsIterator` instance or a `BindingsIteratorError` if an overflow occurs.
    pub fn new(
        variables: &'a TypedList<VariableId, TypeId>,
        value_registry: &'a ValueRegistry,
    ) -> Result<Self, BindingsIteratorError> {
        let domains = value_registry.get_variable_domains(variables)?;
        let arity = domains.len();

        // 1. Calculate the total number of combinations.
        // If arity is 0 (no variables), total_count is forced to 0 to trigger immediate exhaustion.
        let total_count = match arity {
            0 => 0,
            _ => domains
                .iter()
                .map(|d| d.len())
                .try_fold(1usize, |acc, x| acc.checked_mul(x))
                .ok_or_else(|| BindingsIteratorError::combinatorial_explosion(arity))?,
        };

        // 2. Vacuity check (Grounding-specific logic).
        // The iterator is considered exhausted if:
        // - There are no variables to instantiate (arity == 0)
        // - The total combination count is 0
        // - Any of the required domains is empty
        let is_domain_empty = arity > 0 && domains.iter().any(|d| d.is_empty());
        let exhausted = arity == 0 || total_count == 0 || is_domain_empty;

        // 3. Buffer initialization
        let indices = vec![0; arity];

        // Pre-fill current_bindings with debug ObjectIds to avoid allocations during iteration.
        let mut current_bindings = Bindings::with_capacity(arity);
        for typed_var in variables {
            current_bindings.insert(typed_var.symbol(), ObjectId::default());
        }

        Ok(Self {
            domains,
            variables,
            indices,
            current_bindings,
            exhausted,
            total_count,
            is_first: true,
        })
    }

    /// Returns the current combination and advances the iterator to the next state.
    ///
    /// This implementation is optimized to minimize bounds checks and avoids
    /// re-allocating the result by updating an internal buffer.
    ///
    /// # Returns
    /// - `Some(&Bindings)`: A reference to the current set of variable-to-object mappings.
    /// - `None`: If all combinations have been exhausted.
    pub fn next(&mut self) -> Option<&Bindings> {
        if self.exhausted {
            return None;
        }

        let arity = self.variables.len();

        // Case: First iteration
        if self.is_first {
            self.is_first = false;

            // Fill the buffer only if variables exist.
            // For arity 0, we return the empty Bindings buffer already initialized.
            if arity > 0 {
                for i in 0..arity {
                    let val = self.domains[i][self.indices[i]];
                    self.current_bindings
                        .insert(self.variables[i].symbol(), val);
                }
            } else {
                // For arity 0, mark as exhausted immediately after this first yield.
                self.exhausted = true;
            }
        } else {
            // Subsequent calls (only valid if arity > 0)
            if arity == 0 {
                self.exhausted = true;
                return None;
            }

            // Advance indices and find the first level that changed.
            let changed_idx = self.prepare_next()?;

            // Update the buffer from the changed index onwards to synchronize with new indices.
            for i in changed_idx..arity {
                let val = self.domains[i][self.indices[i]];
                self.current_bindings
                    .insert(self.variables[i].symbol(), val);
            }
        }

        Some(&self.current_bindings)
    }

    /// Increments the iteration indices to the next valid combination.
    ///
    /// This logic performs a lexicographical increment (similar to an odometer).
    /// When an index reaches the maximum cardinality of its domain, it wraps
    /// around to zero and carries over the increment to the next position to the left.
    ///
    /// # Returns
    /// - `Some(usize)`: The leftmost index that was incremented. This indicates that
    ///   all values to the left of this index remain unchanged, allowing for optimized buffer updates.
    /// - `None`: If the indices wrap around completely, indicating that all combinations
    ///   have been traversed.
    fn prepare_next(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            self.exhausted = true;
            return None;
        }

        // Iterate backwards from the rightmost domain (least significant position)
        for i in (0..self.indices.len()).rev() {
            if self.indices[i] + 1 < self.domains[i].len() {
                self.indices[i] += 1;
                return Some(i);
            }
            self.indices[i] = 0;
        }

        // Wrap around occurred for all domains
        self.exhausted = true;
        None
    }

    /// PRUNING: Skips all future combinations for positions to the right of `pos`.
    ///
    /// Calling `skip_at(1)` tells the iterator: "Move to the next value for index 1,
    /// ignoring all remaining possibilities for indices 2, 3, etc."
    ///
    /// # Parameters
    /// - `pos`: The index at which the pruning occurs. All domains to the right of this
    ///   position will be fast-forwarded to their last element.
    pub fn skip_at(&mut self, pos: usize) {
        let arity = self.indices.len();
        if pos < arity {
            // Optimization: We only iterate over the suffix to the right of pos.
            // This avoids re-evaluating the entire index set during the next increment.
            let suffix_start = pos + 1;
            if suffix_start < arity {
                for i in suffix_start..arity {
                    // By setting the indices to their maximum cardinality minus one,
                    // we force a "carry-over" effect during the next call to `prepare_next()`.
                    self.indices[i] = self.domains[i].len().saturating_sub(1);
                }
            }
        }
    }

    /// Indicates whether the iterator has more combinations to yield.
    ///
    /// # Returns
    /// - `true` if there are remaining combinations, `false` if the iterator is exhausted.
    pub fn has_next(&self) -> bool {
        !self.exhausted
    }

    /// Resets the iterator to its initial state.
    ///
    /// This allows the same iterator to be reused for a fresh traversal of the
    /// Cartesian product without re-allocating the internal buffers or domains.
    pub fn reset(&mut self) {
        // 1. Reset all indices to the first element of each domain.
        self.indices.fill(0);

        // 2. Set state back to "first iteration" so the next call to next()
        // properly initializes the buffer.
        self.is_first = true;

        // 3. Re-evaluate the exhaustion state (identical to constructor logic).
        let arity = self.indices.len();
        let is_really_empty = arity > 0 && self.domains.iter().any(|d| d.is_empty());

        // Note: total_count is cached, so we don't need to re-check for overflows.
        self.exhausted = self.total_count == 0 || is_really_empty;
    }

    /// Returns the total number of possible combinations calculated at initialization.
    ///
    /// # Returns
    /// - The total count of combinations in the Cartesian product as a `usize`.
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Returns the number of combinations remaining to be traversed.
    ///
    /// The calculation has a complexity of `O(N)`, where `N` is the arity.
    /// This remains highly efficient (nearly instantaneous) even when the
    /// total number of combinations reaches millions.
    ///
    /// # Returns
    /// - The estimated count of remaining combinations as a `usize`.
    /// - Returns `0` if the iterator is already exhausted.
    pub fn remaining_count(&self) -> usize {
        if self.exhausted {
            return 0;
        }

        let mut remaining = 0usize;
        let mut multiplier = 1usize;

        // We traverse backwards (from least significant to most significant domain)
        // to calculate the remaining distance in the Cartesian product space.
        for i in (0..self.indices.len()).rev() {
            let card = self.domains[i].len();
            let current_idx = self.indices[i];

            // Calculate how many elements are left in the current domain
            let diff = card.saturating_sub(1).saturating_sub(current_idx);

            // Accumulate the weighted remaining count
            remaining = remaining.saturating_add(diff.saturating_mul(multiplier));

            // Update the multiplier for the next (more significant) dimension.
            // Saturating multiplication prevents panics on theoretical overflows.
            multiplier = multiplier.saturating_mul(card);
        }

        // Add 1 to include the current combination in the count
        remaining.saturating_add(1)
    }
}

impl<'a> fmt::Display for BindingsIterator<'a> {
    /// Formats the iterator for display purposes, showing grounding progress.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of the formatting operation.
    ///
    /// The output provides a human-readable status including:
    /// - The number of combinations processed vs. the total count.
    /// - A percentage-based progress indicator.
    /// - The arity (number of variables) of the iterator.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.total_count().saturating_sub(self.remaining_count());
        let total = self.total_count();

        // Calculate progress percentage
        let progress = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        write!(
            f,
            "BindingsIterator [Progress: {}/{} ({:.2}%) | Arity: {}]",
            current,
            total,
            progress,
            self.indices.len()
        )
    }
}
