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

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolOrigin};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::NodeId;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
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
/// use crate::aiplan::semantic::symbol::{Usage, SymbolRef, Scope, SymbolOrigin, Span, NodeId};
///
/// let usage = Usage::new(
///     Symbol::new(...),
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
    symbol: Symbol,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The origin or source (e.g., domain, problem) where the usage comes from.
    origin: SymbolOrigin,

    /// The span in the source code corresponding to this usage.
    span: Span,

    /// The AST node identifier where the symbol usage occurs.
    node_id: NodeId,

    resolved_declaration: Option<NodeId>,
}

impl Usage {
    /// Creates a new `Usage` instance.
    ///
    /// # Parameters
    ///
    /// - `symbol`: A reference to the symbol being used.
    /// - `scope`: The lexical or logical scope in which the usage occurs.
    /// - `source`: The origin of the usage (e.g., domain or problem file).
    /// - `span`: The span in source code where the symbol is used.
    /// - `ast`: The AST node identifier (`NodeId`) corresponding to this usage.
    ///
    /// # Returns
    ///
    /// A new `Usage` struct.
    pub fn new(
        symbol: Symbol,
        scope: Scope,
        source: SymbolOrigin,
        span: Span,
        ast: NodeId,
    ) -> Self {
        Usage {
            symbol,
            scope,
            origin: source,
            span,
            node_id: ast,
            resolved_declaration: None,
        }
    }

    /// Returns a reference to the underlying `Symbol`.
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    /// Returns the identifier of the referenced symbol.
    pub fn symbol_id(&self) -> SymbolId {
        self.symbol.id()
    }

    /// Returns the kind of the referenced symbol.
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol.kind()
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
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns the AST node identifier (`NodeId`) where the symbol is used.
    pub fn source(&self) -> NodeId {
        self.node_id
    }

    pub fn set_resolved_declaration(&mut self, node_id: NodeId) {
        self.resolved_declaration = Some(node_id);
    }

    pub fn resolved_declaration(&self) -> Option<NodeId> {
        self.resolved_declaration
    }
}

impl RemapSymbol for Usage {
    /// Remaps the identifier of this usage according to the provided mapping.
    ///
    /// If the usage's symbol identifier exists in `map`, it is replaced with the
    /// corresponding new identifier. Other fields remain unchanged.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if remapping cannot be applied (propagated from nested remaps, if any).
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        if let Some(new_ident) = map.get(&self.symbol_id()) {
            self.symbol.set_ident(new_ident.clone());
        }
        Ok(())
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[node: {}, kind: {}, ident: {}, scope: {}, origin: {}",
            self.node_id.as_usize(),
            self.symbol_kind(),
            self.symbol_id(),
            self.scope,
            self.origin
        )?;

        if let Some(decl_node_id) = self.resolved_declaration {
            write!(f, ", resolved_decl: {}", decl_node_id.as_usize())?;
        } else {
            write!(f, ", resolved_decl: None")?;
        }

        write!(f, "]")
    }
}

impl InternerDisplay for Usage {
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        let symbol_str = interner
            .resolve_symbol(self.symbol_id())
            .unwrap_or("<uninterned>");

        write!(
            f,
            "[node: {}, kind: {}, ident: {}, scope: {}, origin: {}",
            self.node_id.as_usize(),
            self.symbol_kind(),
            symbol_str,
            self.scope,
            self.origin
        )?;

        // Affichage du lien vers la déclaration (le vissage)
        if let Some(decl_node_id) = self.resolved_declaration {
            write!(f, ", resolved_decl: {}", decl_node_id.as_usize())?;
        } else {
            write!(f, ", resolved_decl: None")?;
        }

        write!(f, "]")
    }
}
