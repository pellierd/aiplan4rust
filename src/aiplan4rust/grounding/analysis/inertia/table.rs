//! Data structures for storing and querying symbol inertia.
//!
//! This module provides the [`InertiaTable`], which acts as a centralized repository
//! for the results of an inertia analysis. It maps every predicate and numeric
//! function in the problem to its respective [`Inertia`] category.

use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId};
use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;

/// A lookup table for inertia, covering both Predicates and Numeric Functions.
///
/// The `InertiaTable` is the primary output of the inertia analysis. It provides
/// constant-time lookup for the stability of any symbol ID. This information is
/// used by:
/// - **The Simplifier**: To prune unreachable logical branches.
/// - **The Grounder**: To decide which predicates can be treated as constants.
/// - **The Evaluator**: To optimize state transitions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InertiaTable {
    /// Inertia of predicates, indexed by their unique [`AtomSkeletonId`].
    predicates: HashMap<AtomSkeletonId, Inertia>,
    /// Inertia of functions, indexed by their unique [`FunctionSkeletonId`].
    functions: HashMap<FunctionSkeletonId, Inertia>,
}

impl InertiaTable {
    /// Initializes a new, empty `InertiaTable`.
    ///
    /// By default, all mappings are empty. Any lookup for a symbol not
    /// explicitly inserted will return `None`.
    pub fn new() -> Self {
        Self::default()
    }

    // --- Predicate Management ---

    /// Retrieves the inertia status of a specific predicate.
    ///
    /// # Arguments
    /// * `index` - The unique identifier of the predicate skeleton.
    ///
    /// # Returns
    /// Returns `Some(Inertia)` if analyzed, or `None` if the predicate
    /// index is unknown to this table.
    pub fn get_predicate(&self, index: AtomSkeletonId) -> Option<Inertia> {
        self.predicates.get(&index).copied()
    }

    /// Registers or updates the inertia status for a predicate.
    ///
    /// # Arguments
    /// * `index` - The identifier of the predicate skeleton.
    /// * `inertia` - The inertia category to assign.
    pub fn insert_predicate(&mut self, index: AtomSkeletonId, inertia: Inertia) {
        self.predicates.insert(index, inertia);
    }

    // --- Function Management ---

    /// Retrieves the inertia status of a specific numeric function.
    ///
    /// # Arguments
    /// * `index` - The unique identifier of the function skeleton.
    ///
    /// # Returns
    /// Returns `Some(Inertia)` if analyzed, or `None` if the function
    /// index is unknown.
    pub fn get_function(&self, index: FunctionSkeletonId) -> Option<Inertia> {
        self.functions.get(&index).copied()
    }

    /// Registers or updates the inertia status for a numeric function.
    ///
    /// # Arguments
    /// * `index` - The identifier of the function skeleton.
    /// * `inertia` - The inertia category to assign.
    pub fn insert_function(&mut self, index: FunctionSkeletonId, inertia: Inertia) {
        self.functions.insert(index, inertia);
    }

    // --- Getters avec Result ---

    // --- Secure Accessors ---

    /// Retrieves the inertia of a predicate or returns an error if the index is missing.
    ///
    /// This method is preferred when the caller expects the predicate to have been
    /// previously analyzed, ensuring data consistency throughout the pipeline.
    ///
    /// # Errors
    /// Returns a [`LirError::MissingPredicateInertia`] if the index is not in the table.
    pub fn try_get_predicate(&self, index: AtomSkeletonId) -> Result<Inertia, InertiaError> {
        self.predicates
            .get(&index)
            .copied()
            .ok_or_else(|| InertiaError::missing_predicate_inertia(index))
    }

    /// Retrieves the inertia of a function or returns an error if the index is missing.
    ///
    /// # Errors
    /// Returns a [`LirError::MissingFunctionInertia`] if the index is not in the table.
    pub fn try_get_function(&self, index: FunctionSkeletonId) -> Result<Inertia, InertiaError> {
        self.functions
            .get(&index)
            .copied()
            .ok_or_else(|| InertiaError::missing_function_inertia(index))
    }

    // --- Validation Helpers (Predicates) ---

    /// Checks if a predicate is static and always true (Positive inertia).
    ///
    /// Useful for simplifying conjunctions: if a positive static predicate is
    /// found, it can be treated as a constant `True`.
    ///
    /// # Errors
    /// Returns an error if the predicate has not been analyzed.
    pub fn is_predicate_positive(&self, index: AtomSkeletonId) -> Result<bool, InertiaError> {
        Ok(self.try_get_predicate(index)? == Inertia::Positive)
    }

    /// Checks if a predicate is static and always false (Negative inertia).
    ///
    /// Useful for pruning: if a negative static predicate is part of a
    /// conjunction, the entire branch can be discarded.
    ///
    /// # Errors
    /// Returns an error if the predicate has not been analyzed.
    pub fn is_predicate_negative(&self, index: AtomSkeletonId) -> Result<bool, InertiaError> {
        Ok(self.try_get_predicate(index)? == Inertia::Negative)
    }

    /// Checks if a predicate is static (either Positive or Negative).
    ///
    /// A static predicate's truth value is determined solely by the initial state
    /// and will never change during plan execution.
    ///
    /// # Errors
    /// Returns an error if the predicate has not been analyzed.
    pub fn is_predicate_static(&self, index: AtomSkeletonId) -> Result<bool, InertiaError> {
        Ok(!matches!(self.try_get_predicate(index)?, Inertia::Fluent))
    }

    // --- Validation Helpers (Functions) ---

    /// Checks if a numeric function is static and initialized (Positive inertia).
    ///
    /// In the context of numeric planning, a positive static function often represents
    /// a constant resource capacity or a fixed cost that remains unchanged.
    ///
    /// # Errors
    /// Returns a [`LirError`] if the function index has not been analyzed.
    pub fn is_function_positive(&self, index: FunctionSkeletonId) -> Result<bool, InertiaError> {
        Ok(self.try_get_function(index)? == Inertia::Positive)
    }

    /// Checks if a numeric function is uninitialized or explicitly marked as negative.
    ///
    /// A negative static function typically indicates a symbol that is never
    /// assigned a value in the initial state and never modified by effects.
    ///
    /// # Errors
    /// Returns a [`LirError`] if the function index is unknown.
    pub fn is_function_negative(&self, index: FunctionSkeletonId) -> Result<bool, InertiaError> {
        Ok(self.try_get_function(index)? == Inertia::Negative)
    }

    /// Checks if a numeric function is static (either Positive or Negative).
    ///
    /// If a function is static, the grounder or evaluator can replace its
    /// expr with constant values, significantly reducing the overhead
    /// of state evaluations.
    ///
    /// # Errors
    /// Returns a [`LirError`] if the function has not been categorized.
    pub fn is_function_static(&self, index: FunctionSkeletonId) -> Result<bool, InertiaError> {
        Ok(!matches!(self.try_get_function(index)?, Inertia::Fluent))
    }
}

impl fmt::Display for InertiaTable {
    /// Formats the `InertiaTable` as a human-readable summary.
    ///
    /// The output is structured into sections (Predicates and Functions),
    /// with IDs sorted alphabetically and their inertia status aligned for
    /// terminal readability. If a section is empty, a specific placeholder
    /// message is displayed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Inertia Table ===")?;

        // --- PREDICATES ---
        writeln!(f, "\n[ Predicates ]")?;
        if self.predicates.is_empty() {
            writeln!(f, "  (no predicates analyzed)")?;
        } else {
            let mut pred_ids: Vec<_> = self.predicates.keys().collect();
            pred_ids.sort();

            for id in pred_ids {
                let inertia = &self.predicates[id];
                writeln!(f, "  {:<15} : {}", format!("{:?}", id), inertia)?;
            }
        }

        // --- FUNCTIONS ---
        writeln!(f, "\n[ Functions ]")?;
        if self.functions.is_empty() {
            writeln!(f, "  (no functions analyzed)")?;
        } else {
            let mut func_ids: Vec<_> = self.functions.keys().collect();
            func_ids.sort();

            for id in func_ids {
                let inertia = &self.functions[id];
                writeln!(f, "  {:<15} : {}", format!("{:?}", id), inertia)?;
            }
        }

        writeln!(f, "\n=====================")?;
        Ok(())
    }
}
