//! # Symbol Usage Tracking Module
//!
//! This module defines the [`Usage`] struct, which represents a concrete use of a symbol
//! in the abstract syntax tree (AST). Each usage includes metadata such as the symbol's
//! identifier, the context of its use (scope, origin), and its precise location in source code.
//!
//! [`Usage`]s are collected during semantic analysis and used for:
//! - Reference resolution
//! - Error reporting
//! - Refactoring tools
//! - Scope checking
//!
//! The module also provides formatting capabilities with and without interners
//! (see [`InternerDisplay`]) and supports identifier remapping, which is useful
//! for name rewriting or alpha-renaming in transformations.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::semantic::symbol::{SymbolRef, SymbolOrigin};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::Span;

use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a concrete use of a symbol within the Abstract Syntax Tree (AST).
///
/// `Usage` stores metadata about a specific occurrence of a symbol reference in source code.
/// This includes the symbol being used, the lexical scope of its use, its origin (e.g., domain or problem),
/// the location in the source (`Span`), and the corresponding AST node (`NodeId`).
///
/// This structure is used during semantic analysis and symbol resolution to trace how and where
/// symbols are used throughout the program.
///
/// # Examples
///
/// ```
/// use your_crate::{Usage, SymbolRef, Scope, SymbolOrigin, Span, NodeId};
///
/// let usage = Usage::new(
///     SymbolRef::new(...),
///     Scope::Global,
///     SymbolOrigin::Domain,
///     Span::new(5, 10),
///     NodeId(42),
/// );
/// println!("{}", usage);
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Usage {
    /// Reference to the symbol being used.
    symbol_ref: SymbolRef,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The origin or source (e.g., domain, problem) where the usage comes from.
    origin: SymbolOrigin,

    /// The span in the source code corresponding to this usage.
    span: Span,

    /// The AST node identifier where the symbol usage occurs.
    node_id: NodeId,
}

impl Usage {
    /// Creates a new `Usage` instance.
    ///
    /// # Parameters
    ///
    /// - `symbol_ref`: A reference to the symbol being used.
    /// - `scope`: The lexical or logical scope in which the usage occurs.
    /// - `source`: The origin of the usage (e.g., domain or problem file).
    /// - `span`: The span in source code where the symbol is used.
    /// - `ast`: The AST node identifier (`NodeId`) corresponding to this usage.
    ///
    /// # Returns
    ///
    /// A new `Usage` struct.
    pub fn new(
        symbol_ref: SymbolRef,
        scope: Scope,
        source: SymbolOrigin,
        span: Span,
        ast: NodeId,
    ) -> Self {
        Usage {
            symbol_ref,
            scope,
            origin: source,
            span,
            node_id: ast,
        }
    }

    /// Returns a reference to the underlying `SymbolRef`.
    pub fn symbol_ref(&self) -> &SymbolRef {
        &self.symbol_ref
    }

    /// Returns the identifier of the referenced symbol.
    pub fn symbol_ident(&self) -> Ident {
        self.symbol_ref.ident()
    }

    /// Returns the kind of the referenced symbol.
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol_ref.kind()
    }

    /// Returns a reference to the lexical scope of the usage.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns the origin of the symbol usage (e.g., domain, problem).
    pub fn origin(&self) -> SymbolOrigin {
        self.origin
    }

    /// Returns a reference to the source code span for this usage.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the AST node identifier (`NodeId`) where the symbol is used.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Remaps the identifier of the usage according to the provided mapping.
    ///
    /// If the symbol's identifier exists in the map, it is replaced with the mapped one.
    ///
    /// # Parameters
    ///
    /// - `map`: A mapping from old identifiers to new ones (`HashMap<Ident, Ident>`).
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Some(new_ident) = map.get(&self.symbol_ident()) {
            self.symbol_ref.set_ident(new_ident.clone());
        }
    }
}

impl fmt::Display for Usage {
    /// Formats the usage using standard formatting.
    ///
    /// The output includes the AST index, symbol kind, identifier, scope, and origin.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.node_id.as_usize(),
            self.symbol_kind(),
            self.symbol_ident(),
            self.scope,
            self.origin
        )
    }
}

impl InternerDisplay for Usage {
    /// Formats the usage using the given interner to resolve interned identifiers.
    ///
    /// # Parameters
    ///
    /// - `f`: Formatter used to produce the output.
    /// - `interner`: A `StringInterner` for resolving interned `Ident` values to strings.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        let symbol_str = interner
            .resolve_ident(self.symbol_ident())
            .unwrap_or("<uninterned>");
        write!(
            f,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.node_id.as_usize(),
            self.symbol_kind(),
            symbol_str,
            self.scope,
            self.origin
        )
    }
}
