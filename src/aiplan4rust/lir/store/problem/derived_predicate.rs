//! Module `derived_predicate`
//!
//! This module defines `DerivedPredicate`s used in PDDL problems.
//! A `DerivedPredicate` is a logical fact derived from other facts,
//! consisting of a head (name and parameters) and a body (logical expression).

use crate::aiplan4rust::lang::{AtomSkeletonId, VariableId};
use crate::aiplan4rust::lir::store::expr::ExprId;
use crate::aiplan4rust::lir::store::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::store::problem::SymbolRegistry;
use crate::aiplan4rust::lir::store::renderers;
use crate::aiplan4rust::lir::store::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext,
};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};

/// Represents a derived predicate in a PDDL problem.
///
/// A `DerivedPredicate` consists of:
/// - `head`: the predicate's name and parameters (`AtomicFormulaSkeleton`).
/// - `body`: a logical expression (`Expr`) defining when the predicate holds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DerivedPredicate {
    /// Unique identifier of the skeleton associated with this predicate.
    /// Essential for inertia analysis and efficient grounding.
    head_id: AtomSkeletonId,

    /// The predicate's head, containing name and parameter details.
    /// While the `skeleton_id` is technically sufficient to retrieve this
    /// information from the problem's predicate definitions, keeping the
    /// `head` here avoids frequent and costly lookups in the predicate table
    /// during rendering and grounding.
    head: AtomicFormulaSkeleton,

    /// The logical expression defining the derived predicate.
    body: ExprId,

    variable_symbols: SymbolRegistry<VariableId>,
}

impl DerivedPredicate {
    /// Creates a new `DerivedPredicate` with the given skeleton ID, head, and body expression.
    ///
    /// # Arguments
    ///
    /// * `header_id` - The unique identifier for this predicate's definition.
    /// * `head` - The atomic formula skeleton representing the predicate's name and parameters.
    /// * `body` - The logical expression defining when the derived predicate is true.
    ///
    /// # Returns
    ///
    /// A new `DerivedPredicate` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::lang::AtomSkeletonID;
    /// # use aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
    /// # use aiplan4rust::lir::expr::Expr;
    /// # use aiplan4rust::lir::problem::DerivedPredicate;
    /// let id = AtomSkeletonID::from(0);
    /// let head = AtomicFormulaSkeleton::new("reachable", vec!["?x", "?y"]);
    /// let body = Expr::empty_or();
    /// let dp = DerivedPredicate::new(id, head, body);
    /// ```
    pub fn new(header_id: AtomSkeletonId, head: AtomicFormulaSkeleton, body: ExprId) -> Self {
        Self {
            head_id: header_id,
            head,
            body,
            variable_symbols: SymbolRegistry::new(),
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    /// Returns the unique skeleton identifier for this derived predicate.
    ///
    /// This ID corresponds to the predicate's index in the problem's global definitions.
    pub fn header_id(&self) -> AtomSkeletonId {
        self.head_id
    }

    /// Returns a reference to the predicate's head.
    ///
    /// # Returns
    /// Immutable reference to the `AtomicFormulaSkeleton` representing the head.
    pub fn head(&self) -> &AtomicFormulaSkeleton {
        &self.head
    }

    /// Returns a mutable reference to the predicate's head.
    ///
    /// # Returns
    /// Mutable reference to the `AtomicFormulaSkeleton` representing the head.
    pub fn head_mut(&mut self) -> &mut AtomicFormulaSkeleton {
        &mut self.head
    }

    /// Sets the predicate's head.
    ///
    /// # Parameters
    /// - `head`: The new `AtomicFormulaSkeleton` to replace the current head.
    ///
    /// # Returns
    /// Nothing.
    pub fn set_head(&mut self, head: AtomicFormulaSkeleton) {
        self.head = head;
    }

    /// Returns a reference to the predicate's body expression.
    ///
    /// # Returns
    /// Immutable reference to the `Expr` defining the derived predicate.
    pub fn body(&self) -> ExprId {
        self.body
    }

    /// Sets the predicate's body expression.
    ///
    /// # Parameters
    /// - `body`: The new `Expr` to replace the current body.
    ///
    /// # Returns
    /// Nothing.
    pub fn set_body(&mut self, body: ExprId) {
        self.body = body;
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> {
        &self.variable_symbols
    }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> {
        &mut self.variable_symbols
    }
}

impl LiftedSyntaxDisplay for DerivedPredicate {
    /// Rendu PDDL propre (ex: (:derived (p ?x) (and ...)))
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::derived_predicate::render(f, self, ctx)
    }
}

impl LiftedDebugDisplay for DerivedPredicate {
    /// Rendu structurel pour le debug (Head ID, Body Expr Tree, Local Symbols)
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::derived_predicate::render(f, self, ctx)
    }
}
