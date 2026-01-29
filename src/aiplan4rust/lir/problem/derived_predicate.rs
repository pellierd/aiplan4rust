//! Module `derived_predicate`
//!
//! This module defines `DerivedPredicate`s used in PDDL problems.
//! A `DerivedPredicate` is a logical fact derived from other facts,
//! consisting of a head (name and parameters) and a body (logical expression).

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{RemapTypes, StringID, Type};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::{normalize, renderers};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// Represents a derived predicate in a PDDL problem.
///
/// A `DerivedPredicate` consists of:
/// - `head`: the predicate's name and parameters (`AtomicFormulaSkeleton`).
/// - `body`: a logical expression (`Expr`) defining when the predicate holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DerivedPredicate {
    /// The predicate's head: its name and parameters.
    head: AtomicFormulaSkeleton,

    /// The logical expression defining the derived predicate.
    body: Expr,
}

impl DerivedPredicate {
    /// Creates a new `DerivedPredicate` with the given head and body expression.
    ///
    /// # Arguments
    ///
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
    /// # use aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
    /// # use aiplan4rust::lir::expr::Expr;
    /// # use aiplan4rust::lir::problem::DerivedPredicate;
    /// let head = AtomicFormulaSkeleton::new("reachable", vec!["?x", "?y"]);
    /// let body = Expr::empty_or(); // placeholder for the actual logic
    /// let dp = DerivedPredicate::new(head, body);
    /// ```
    pub fn new(head: AtomicFormulaSkeleton, body: Expr) -> Self {
        Self { head, body }
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
    pub fn body(&self) -> &Expr {
        &self.body
    }

    /// Returns a mutable reference to the predicate's body expression.
    ///
    /// # Returns
    /// Mutable reference to the `Expr` defining the derived predicate.
    pub fn body_mut(&mut self) -> &mut Expr {
        &mut self.body
    }

    /// Sets the predicate's body expression.
    ///
    /// # Parameters
    /// - `body`: The new `Expr` to replace the current body.
    ///
    /// # Returns
    /// Nothing.
    pub fn set_body(&mut self, body: Expr) {
        self.body = body;
    }

    /// Normalizes the expressions of this `DerivedPredicate`.
    ///
    /// This method applies normalization to the predicate's `body` expression.
    /// The `head` (name and parameters) is not modified. It is a thin wrapper
    /// around the internal `normalize_derived_predicate` function.
    ///
    /// # Returns
    ///
    /// Returns [`Ok(())`] if the normalization succeeds, or a [`LirError`] if
    /// the body expression cannot be normalized.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use aiplan4rust::lir::DerivedPredicate;
    /// # use aiplan4rust::lir::error::LirError;
    /// # fn example(derived: &mut DerivedPredicate) -> Result<(), LirError> {
    /// derived.normalize()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_derived_predicate(self)?)
    }
}

/// Implements [`RemapTypes`] for [`LiftedDerivedPredicate`].
///
/// This allows remapping all union types (`Type::Either`) in a derived predicate
/// to concrete types according to a provided mapping.
impl RemapTypes for DerivedPredicate {
    /// Remaps union types in the derived predicate's head and body.
    ///
    /// # Parameters
    /// - `map`: A [`HashMap<Type, StringID>`] mapping union types to primitive identifiers.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if any type cannot be remapped.
    fn remap_types(&mut self, map: &HashMap<Type, StringID>) -> Result<(), LirError> {
        self.body.remap_types(map)?;
        Ok(())
    }
}

/// Implements [`fmt::Display`] for `DerivedPredicate`.
///
/// This allows printing the derived predicate in a human-readable format using
/// the default renderer.
impl fmt::Display for DerivedPredicate {
    /// Formats the derived predicate into the given formatter.
    ///
    /// # Parameters
    /// - `f`: The [`fmt::Formatter`] to write the output into.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating whether writing was successful.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_derived_predicate(f, self)
    }
}

/// Implements [`InternerDisplay`] for `DerivedPredicate`.
///
/// This allows printing the derived predicate using a [`StringInterner`] to
/// resolve interned identifiers.
impl InternerDisplay for DerivedPredicate {
    /// Formats the derived predicate using the given interner.
    ///
    /// # Parameters
    /// - `f`: The [`Formatter`] to write the output into.
    /// - `interner`: The [`StringInterner`] to resolve identifiers.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating whether writing was successful.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderers::interner::render_derived_predicate(f, self, interner)
    }
}

/// Implements [`SyntaxInternerDisplay`] for `DerivedPredicate`.
///
/// This allows printing the derived predicate in a PDDL-like syntax format,
/// with indentation and identifier resolution via a [`StringInterner`].
impl SyntaxInternerDisplay for DerivedPredicate {
    /// Formats the derived predicate as PDDL syntax with the given interner and indentation.
    ///
    /// # Parameters
    /// - `f`: The [`Formatter`] to write the output into.
    /// - `interner`: The [`StringInterner`] to resolve identifiers.
    /// - `indent`: Indentation level for pretty-printing.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating whether writing was successful.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        renderers::syntax::render_derived_predicate(f, self, interner, indent)
    }
}
