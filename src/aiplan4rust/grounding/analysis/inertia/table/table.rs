//! Data structures for storing and querying symbol inertia.
//!
//! This module provides the [`InertiaTable`], which acts as a centralized repository
//! for the results of an inertia analysis. It maps every predicate and numeric
//! function in the problem to its respective [`Inertia`] category.

use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
use crate::aiplan4rust::grounding::analysis::inertia::table::builder::build;
use crate::aiplan4rust::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId};
use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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
    /// Builds the inertia table by analyzing the provided problem.
    ///
    /// This is the standard entry point for determining which predicates
    /// and functions are constant (Inert) versus fluent.
    pub fn build(problem: &LiftedProblem) -> Result<Self, InertiaTableError> {
        Ok(build(problem)?)
    }

    /// Returns an empty table for testing or edge cases.
    pub fn empty() -> Self {
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

    // --- Secure Accessors ---

    /// Retrieves the inertia of a predicate or returns an error if the index is missing.
    ///
    /// This method is preferred when the caller expects the predicate to have been
    /// previously analyzed, ensuring data consistency throughout the pipeline.
    ///
    /// # Errors
    /// Returns a [`InertiaTableError::MissingPredicateInertia`] if the index is not in the table.
    pub fn try_get_predicate(&self, index: AtomSkeletonId) -> Result<Inertia, InertiaTableError> {
        self.predicates
            .get(&index)
            .copied()
            .ok_or_else(|| InertiaTableError::missing_predicate_inertia(index))
    }

    /// Retrieves the inertia of a function or returns an error if the index is missing.
    ///
    /// # Errors
    /// Returns a [`InertiaTableError::MissingFunctionInertia`] if the index is not in the table.
    pub fn try_get_function(
        &self,
        index: FunctionSkeletonId,
    ) -> Result<Inertia, InertiaTableError> {
        self.functions
            .get(&index)
            .copied()
            .ok_or_else(|| InertiaTableError::missing_function_inertia(index))
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

    // --- Validation Helpers (Predicates) ---

    /// Checks if a predicate is **positive-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`AtomSkeletonId`) of the predicate to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the predicate is never present in any action's `add` effect.
    /// * `Ok(false)` if the predicate can be added by at least one action.
    /// * `Err(InertiaTableError)` if the index is out of bounds or invalid.
    pub fn is_predicate_positive_inertia(
        &self,
        index: AtomSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_predicate(index)?.is_positive())
    }

    /// Checks if a predicate is **negative-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`AtomSkeletonId`) of the predicate to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the predicate is never present in any action's `delete` effect.
    /// * `Ok(false)` if the predicate can be removed by at least one action.
    /// * `Err(InertiaTableError)` if the index is out of bounds or invalid.
    pub fn is_predicate_negative_inertia(
        &self,
        index: AtomSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_predicate(index)?.is_negative())
    }

    /// Checks if a predicate is **positive-and-negative-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`AtomSkeletonId`) of the predicate to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the predicate is both positive-inert and negative-inert (it remains constant).
    /// * `Ok(false)` if the predicate is a fluent (can be added or deleted).
    /// * `Err(InertiaTableError)` if the index is out of bounds or invalid.
    pub fn is_predicate_positive_negative_inertia(
        &self,
        index: AtomSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_predicate(index)?.is_positive_negative())
    }

    // --- Validation Helpers (Functions) ---

    /// Checks if a numeric function is **positive-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`FunctionSkeletonId`) of the function to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the function's value is never increased or assigned by an action.
    /// * `Err(InertiaTableError)` if the index is invalid.
    pub fn is_function_positive_inertia(
        &self,
        index: FunctionSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_function(index)?.is_positive())
    }

    /// Checks if a numeric function is **negative-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`FunctionSkeletonId`) of the function to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the function's value is never decreased or assigned by an action.
    /// * `Err(InertiaTableError)` if the index is invalid.
    pub fn is_function_negative_inertia(
        &self,
        index: FunctionSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_function(index)?.is_negative())
    }

    /// Checks if a numeric function is **positive-and-negative-inert**.
    ///
    /// # Parameters
    /// * `index` - The unique identifier (`FunctionSkeletonId`) of the function to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the function value remains constant throughout the entire plan execution.
    /// * `Err(InertiaTableError)` if the index is invalid.
    pub fn is_function_positive_negative_inertia(
        &self,
        index: FunctionSkeletonId,
    ) -> Result<bool, InertiaTableError> {
        Ok(self.try_get_function(index)?.is_positive_negative())
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
