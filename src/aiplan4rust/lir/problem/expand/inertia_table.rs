//! This module provides the [`InertiaTable`], which categorizes both predicates
//! and functions as constant or dynamic.

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::{FunctorID, PredicateID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::expand::inertia::Inertia;

/// A lookup table for inertia, covering both Predicates and Numeric Functions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InertiaTable {
    /// Inertia of predicates, indexed by their LIR position (usize).
    predicates: HashMap<PredicateID, Inertia>,
    /// Inertia of functions, indexed by their LIR position (usize).
    functions: HashMap<FunctorID, Inertia>,
}

impl InertiaTable {
    pub fn new() -> Self {
        Self::default()
    }

    // --- Predicate Management ---

    pub fn get_predicate(&self, index: PredicateID) -> Option<Inertia> {
        self.predicates.get(&index).copied()
    }

    pub fn insert_predicate(&mut self, index: PredicateID, inertia: Inertia) {
        self.predicates.insert(index, inertia);
    }

    // --- Function Management ---

    pub fn get_function(&self, index: FunctorID) -> Option<Inertia> {
        self.functions.get(&index).copied()
    }

    pub fn insert_function(&mut self, index: FunctorID, inertia: Inertia) {
        self.functions.insert(index, inertia);
    }

    // --- Getters avec Result ---

    /// Récupère l'inertie d'un prédicat ou renvoie une erreur si l'index est inconnu.
    pub fn try_get_predicate(&self, index: PredicateID) -> Result<Inertia, LirError> {
        self.predicates
            .get(&index)
            .copied()
            .ok_or_else(|| LirError::missing_predicate_inertia(index))
    }

    /// Récupère l'inertie d'une fonction ou renvoie une erreur si l'index est inconnu.
    pub fn try_get_function(&self, index: FunctorID) -> Result<Inertia, LirError> {
        self.functions
            .get(&index)
            .copied()
            .ok_or_else(|| LirError::missing_function_inertia(index))
    }

    // --- Helpers de validation (Predicates) ---

    pub fn is_predicate_positive(&self, index: PredicateID) -> Result<bool, LirError> {
        Ok(self.try_get_predicate(index)? == Inertia::Positive)
    }

    pub fn is_predicate_negative(&self, index: PredicateID) -> Result<bool, LirError> {
        Ok(self.try_get_predicate(index)? == Inertia::Negative)
    }

    pub fn is_predicate_static(&self, index: PredicateID) -> Result<bool, LirError> {
        Ok(!matches!(self.try_get_predicate(index)?, Inertia::Fluent))
    }

    // --- Helpers de validation (Functions) ---

    /// Pour une fonction, "Positive" signifie souvent qu'elle est constante avec une valeur
    /// (utile si tu veux plus tard stocker la valeur constante dans l'Inertia).
    pub fn is_function_positive(&self, index: FunctorID) -> Result<bool, LirError> {
        Ok(self.try_get_function(index)? == Inertia::Positive)
    }

    /// Pour une fonction, "Negative" pourrait signifier qu'elle n'est jamais initialisée
    /// ou explicitement marquée comme nulle/statique négative.
    pub fn is_function_negative(&self, index: FunctorID) -> Result<bool, LirError> {
        Ok(self.try_get_function(index)? == Inertia::Negative)
    }

    pub fn is_function_static(&self, index: FunctorID) -> Result<bool, LirError> {
        Ok(!matches!(self.try_get_function(index)?, Inertia::Fluent))
    }
}

impl fmt::Display for InertiaTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // --- Section PRÉDICATS ---
        writeln!(f, "Predicate Inertia:")?;
        let mut pred_ids: Vec<_> = self.predicates.keys().collect();
        pred_ids.sort();

        for id in pred_ids {
            let inertia = &self.predicates[id];
            writeln!(f, "{:<10} : {:?}", id, inertia)?;
        }

        // --- Section FONCTIONS ---
        writeln!(f, "\nFunction Inertia:")?;
        let mut func_ids: Vec<_> = self.functions.keys().collect();
        func_ids.sort();

        for id in func_ids {
            let inertia = &self.functions[id];
            writeln!(f, "{:<10} : {:?}", id, inertia)?;
        }

        Ok(())
    }
}
