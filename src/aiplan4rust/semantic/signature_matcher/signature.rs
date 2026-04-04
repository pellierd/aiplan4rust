//! Symbol Signature Management
//!
//! This module defines the [`Signature`] abstraction, which provides a unified
//! view of a symbol's structure.
//!
//! ## Role of Signatures
//! In semantic analysis, we often need to compare a "usage" (how a symbol is called)
//! against its "declaration" (how it was defined).
//!
//! The [`Signature`] struct captures:
//! 1. **Identity**: The underlying [`SymbolId`].
//! 2. **Structure**: The arguments (parameters or call-site arguments).
//! 3. **Context**: The specific [`Scope`] (declaration scope vs. call-site scope)
//!    and the [`SymbolKind`] (which may vary between a task usage and an action declaration).
//!
//! ## Dual Nature
//! A `Signature` is versatile:
//! * Use [`Signature::from_declaration`] to represent the **Expected** contract.
//! * Use [`Signature::from_usage`] to represent the **Observed** call site.
//!
//! This duality allows the `SignatureMatcher` to perform structural and semantic
//! validation using a single, consistent interface.

use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::symbol::declaration::DeclarationError;
use crate::aiplan4rust::semantic::symbol::{Declaration, Filterable, Scope, SymbolKind, Usage};
use crate::aiplan4rust::tree::NodeId;

/// A unified representation of a symbol's signature, bridging the gap between
/// its static declaration and its specific usage sites.
///
/// The `Signature` struct acts as a context-aware view of a symbol. It carries
/// information about the underlying declaration while tracking usage-specific
/// data such as arguments and the local scope of a call.
#[derive(Debug, Clone, Copy)]
pub struct Signature<'a> {
    /// The reference declaration for the symbol.
    declaration: &'a Declaration,
    /// The list of arguments (Node IDs) associated with this signature.
    /// This is `Some` for predicates/actions and `None` for simple constants/variables.
    arguments: Option<&'a [NodeId]>,
    /// The specific symbol kind (e.g., Task, Action, Predicate).
    kind: SymbolKind,
    /// The semantic scope context.
    scope: &'a Scope,
}

impl<'a> Signature<'a> {
    /// Creates a new `Signature` from a static symbol declaration.
    ///
    /// This is typically used to represent the "Expected" side of a match,
    /// using the definition's own scope and parameter list.
    ///
    /// # Arguments
    /// * `decl` - The symbol declaration to wrap.
    pub fn from_declaration(decl: &'a Declaration) -> Self {
        Self {
            declaration: decl,
            arguments: decl.argument_sources(),
            kind: decl.kind(),
            scope: decl.scope(), // Uses the definition scope
        }
    }

    /// Creates a new `Signature` from a symbol usage (a call site).
    ///
    /// This is typically used to represent the "Observed" side of a match.
    /// Unlike a declaration-based signature, this captures the arguments passed
    /// at the call site and the scope from which the symbol is being called.
    ///
    /// # Arguments
    /// * `decl` - The declaration resolved for this usage.
    /// * `usage` - The specific usage site in the AST.
    pub fn from_usage(decl: &'a Declaration, usage: &'a Usage) -> Self {
        Self {
            declaration: decl,
            arguments: usage.argument_sources(),
            kind: usage.kind(),
            scope: usage.scope(), // Captures the CALLER'S scope context
        }
    }

    /// Returns the contextual scope of this signature.
    ///
    /// For a declaration, this is the scope where the symbol is defined.
    /// For a usage, this is the scope where the symbol is being invoked.
    pub fn scope(&self) -> &'a Scope {
        self.scope
    }

    /// Returns the unique identifier of the underlying symbol.
    pub fn id(&self) -> SymbolId {
        self.declaration.symbol().id()
    }

    /// Returns the symbol kind associated with this signature context.
    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// Returns the slice of argument Node IDs if the symbol supports arguments.
    pub fn arguments(&self) -> Option<&'a [NodeId]> {
        self.arguments
    }

    /// Attempts to retrieve the primary type of the symbol from the underlying declaration.
    ///
    /// # Errors
    /// Returns [`DeclarationError`] if the type information is missing or malformed.
    pub fn try_type(&self) -> Result<&'a Type<SymbolId>, DeclarationError> {
        self.declaration.try_type()
    }

    /// Attempts to retrieve the type of a specific argument by its positional index.
    ///
    /// # Arguments
    /// * `index` - The zero-based position of the parameter.
    ///
    /// # Errors
    /// Returns [`DeclarationError`] if the index is out of bounds or the type is undefined.
    pub fn try_get_arg_type(&self, index: usize) -> Result<&'a Type<SymbolId>, DeclarationError> {
        self.declaration.try_get_arg_type(index)
    }

    /// Returns a reference to the underlying declaration.
    pub fn declaration(&self) -> &Declaration {
        self.declaration
    }
}
