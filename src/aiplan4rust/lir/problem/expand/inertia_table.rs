//! This module provides the [`InertiaTable`], which categorizes both predicates
//! and functions as constant or dynamic.

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{Ident, StringInterner};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::expand::inertia::Inertia;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

/// A lookup table for inertia, covering both Predicates and Numeric Functions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InertiaTable {
    /// Inertia of predicates, indexed by their LIR position (usize).
    predicates: HashMap<usize, Inertia>,
    /// Inertia of functions, indexed by their LIR position (usize).
    functions: HashMap<usize, Inertia>,
}

impl InertiaTable {
    pub fn new() -> Self {
        Self::default()
    }

    // --- Predicate Management ---

    pub fn get_predicate(&self, index: usize) -> Option<Inertia> {
        self.predicates.get(&index).copied()
    }

    pub fn insert_predicate(&mut self, index: usize, inertia: Inertia) {
        self.predicates.insert(index, inertia);
    }

    // --- Function Management ---

    pub fn get_function(&self, index: usize) -> Option<Inertia> {
        self.functions.get(&index).copied()
    }

    pub fn insert_function(&mut self, index: usize, inertia: Inertia) {
        self.functions.insert(index, inertia);
    }

    // --- Getters avec Result ---

    /// Récupère l'inertie d'un prédicat ou renvoie une erreur si l'index est inconnu.
    pub fn try_get_predicate(&self, index: usize) -> Result<Inertia, LirError> {
        self.predicates
            .get(&index)
            .copied()
            .ok_or_else(|| LirError::inertia_information_missing_predicate(index))
    }

    /// Récupère l'inertie d'une fonction ou renvoie une erreur si l'index est inconnu.
    pub fn try_get_function(&self, index: usize) -> Result<Inertia, LirError> {
        self.functions
            .get(&index)
            .copied()
            .ok_or_else(|| LirError::inertia_information_missing_function(index))
    }

    // --- Helpers de validation (Predicates) ---

    pub fn is_predicate_positive(&self, index: usize) -> Result<bool, LirError> {
        Ok(self.try_get_predicate(index)? == Inertia::Positive)
    }

    pub fn is_predicate_negative(&self, index: usize) -> Result<bool, LirError> {
        Ok(self.try_get_predicate(index)? == Inertia::Negative)
    }

    pub fn is_predicate_static(&self, index: usize) -> Result<bool, LirError> {
        Ok(!matches!(self.try_get_predicate(index)?, Inertia::Fluent))
    }

    // --- Helpers de validation (Functions) ---

    /// Pour une fonction, "Positive" signifie souvent qu'elle est constante avec une valeur
    /// (utile si tu veux plus tard stocker la valeur constante dans l'Inertia).
    pub fn is_function_positive(&self, index: usize) -> Result<bool, LirError> {
        Ok(self.try_get_function(index)? == Inertia::Positive)
    }

    /// Pour une fonction, "Negative" pourrait signifier qu'elle n'est jamais initialisée
    /// ou explicitement marquée comme nulle/statique négative.
    pub fn is_function_negative(&self, index: usize) -> Result<bool, LirError> {
        Ok(self.try_get_function(index)? == Inertia::Negative)
    }

    pub fn is_function_static(&self, index: usize) -> Result<bool, LirError> {
        Ok(!matches!(self.try_get_function(index)?, Inertia::Fluent))
    }
}

impl SyntaxInternerDisplay for InertiaTable {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize
    ) -> fmt::Result {
        let pad = " ".repeat(indent);

        // --- Section PRÉDICATS ---
        writeln!(f, "{}Predicate Inertia:", pad)?;
        writeln!(f, "{}+----------------------------+----------+", pad)?;
        writeln!(f, "{}| Index / Name               | Inertia  |", pad)?;
        writeln!(f, "{}+----------------------------+----------+", pad)?;

        for (idx, inertia) in &self.predicates {
            writeln!(f, "{}| {:<26} | {:<8} |", pad, idx, inertia.to_string())?;
        }

        writeln!(f, "\n{}Function Inertia:", pad)?;
        writeln!(f, "{}+----------------------------+----------+", pad)?;
        writeln!(f, "{}| Index / Name               | Inertia  |", pad)?;
        writeln!(f, "{}+----------------------------+----------+", pad)?;

        for (idx, inertia) in &self.functions {
            writeln!(f, "{}| {:<26} | {:<8} |", pad, idx, inertia.to_string())?;
        }

        Ok(())
    }
}
