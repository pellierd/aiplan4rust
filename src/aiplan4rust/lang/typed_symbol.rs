//! Module defining `TypedSymbol`, a semantic symbol with associated type_checker information.
//!
//! This module provides the `TypedSymbol` struct, which represents an identified symbol
//! (such as a variable or function name) together with one or more associated types.
//!
//! The main purpose of this struct is to model typed entities in a syntax domain,
//! where each symbol carries semantic meaning and type_checker constraints.
//!
//! Features:
//! - Construction from an identifier and associated types.
//! - Accessor methods for symbol and types.
//! - Ability to remap identifiers via a provided mapping, useful for renaming or
//!   interning operations during processing.
//!
//! This is commonly used in parsing and semantic analysis stages of a PDDL-like
//! domain-specific language processor.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

use crate::aiplan4rust::lang::error::LangError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents a typed symbol identified by an [`Ident`],
/// with one or more associated types.
///
/// This struct models a semantic symbol (such as a variable or function)
/// along with its associated type_checker(s).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol {
    symbol: Ident,
    ty: Type,
}

impl TypedSymbol {
    /// Creates a new `TypedSymbol` from a symbol and its associated types.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The main symbol identifier.
    /// * `types` - The associated type_checker(s) for the symbol.
    ///
    /// # Returns
    ///
    /// A new instance of `TypedSymbol`.
    pub fn new(symbol: Ident, types: Type) -> Self {
        TypedSymbol { symbol, ty: types }
    }

    /// Returns the main symbol identifier.
    pub fn symbol(&self) -> Ident {
        self.symbol
    }

    /// Returns a reference to the associated type_checker(s).
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// Remaps identifiers in the typed symbol according to the given mapping.
    ///
    /// This method updates the main symbol identifier as well as all associated
    /// types if a mapping is found in `map`.
    ///
    /// # Arguments
    ///
    /// * `map` - A mapping from old identifiers to new identifiers used for remapping.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Remap the main symbol
        if let Some(new_symbol) = map.get(&self.symbol) {
            self.symbol = new_symbol.clone();
        }

        // Remap all associated types
        for ty in self.ty.iter_mut() {
            if let Some(new_ty) = map.get(ty) {
                *ty = new_ty.clone();
            }
        }
    }
}

impl fmt::Display for TypedSymbol {
    /// Displays the symbol and types by printing their raw `usize` identifiers.
    ///
    /// This does **not** resolve the identifiers via interner; use
    /// [`to_string_with_interner`] for human-readable output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;

        if !self.ty.is_empty() {
            write!(f, " - ")?;
            for (i, ty) in self.ty.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", ty)?;
            }
        }

        Ok(())
    }
}

impl InternerDisplay for TypedSymbol {
    /// Formats the symbol and its associated type_checker using the string interner.
    ///
    /// # Arguments
    /// * `w` - The formatter to write to.
    /// * `interner` - The string interner used to resolve identifiers.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(
        &self,
        w: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        // Format the symbol name
        match interner.resolve_ident(self.symbol) {
            Some(name) => write!(w, "{}", name)?,
            None => write!(w, "<uninterned:{}>", self.symbol)?,
        }

        // If type_checker is not empty, format it after a separator
        if !self.ty.is_empty() {
            write!(w, " - ")?;
            self.ty.fmt_with_interner(w, interner)?;
        }

        Ok(())
    }
}

/// Displays a `TypedSymbol` in PDDL syntax.
///
/// A `TypedSymbol` represents a named variable or constant optionally associated with a type_checker.
/// The output follows this format:
/// - If the type_checker is empty: just the symbol name.
/// - If the type_checker is present: `symbol - type_checker`.
/// - If the type_checker has multiple members: `symbol - (either t1 t2 ...)`.
///
/// # Example
/// ```text
/// x
/// y - location
/// z - (either robot vehicle)
/// ```
impl SyntaxInternerDisplay for TypedSymbol {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        // Print the name of the symbol
        match interner.resolve_ident(self.symbol) {
            Some(name) => write!(f, "{}", name)?,
            None => write!(f, "<uninterned:{}>", self.symbol)?,
        }

        // If the type_checker is not empty, print " - " followed by the type_checker
        if !self.ty.is_empty() {
            write!(f, " - ")?;
            self.ty.fmt_syntax_with_interner(f, interner)?;
        }

        Ok(())
    }
}

/// Attempts to construct a [`TypedSymbol`] from a [`SyntaxSubtree`] referencing an [`AstNode`]
/// and its corresponding [`SyntaxTree`].
///
/// # Expectations
///
/// - The referenced node **must** be of kind `TypedItem`.
/// - The node is expected to have **at most two children**:
///   - The **first child** is the symbol (mandatory).
///   - The **second child** is the type_checker (optional).
/// - If the second child is absent, a `TypedSymbol` with an empty [`Type`] is returned.
///
/// # Errors
///
/// Returns an [`AiplanError`] if accessing the children or parsing the symbol/type_checker fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let symbol = TypedSymbol::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for TypedSymbol {
    type Error = LangError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        let children = node.children();
        let symbol_node = ast.try_node(children[0])?;

        let ty = if children.len() > 1 {
            let ty_node = ast.try_node(children[1])?;
            Type::try_from(&SyntaxSubtree::new(ty_node, ast))?
        } else {
            Type::new()
        };

        Ok(TypedSymbol::new(symbol_node.try_ident()?, ty))
    }
}
