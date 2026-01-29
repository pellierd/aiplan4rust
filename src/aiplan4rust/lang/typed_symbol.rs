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

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lang::{StringID, RemapIdents};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents a typed symbol identified by an [`StringID`],
/// with one or more associated types.
///
/// This struct models a semantic symbol (such as a variable or function)
/// along with its associated type_checker(s).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol {
    symbol: StringID,
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
    pub fn new(symbol: StringID, types: Type) -> Self {
        TypedSymbol { symbol, ty: types }
    }

    /// Returns the main symbol identifier.
    pub fn symbol(&self) -> StringID {
        self.symbol
    }

    /// Sets the main symbol identifier.
    pub fn set_symbol(&mut self, symbol: StringID) {
        self.symbol = symbol;
    }

    /// Returns a reference to the associated type_checker(s).
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// Returns a mutable reference to the associated type(s),
    /// allowing in-place modification.
    pub fn ty_mut(&mut self) -> &mut Type {
        &mut self.ty
    }

    /// Sets the associated type.
    pub fn set_ty(&mut self, ty: Type) {
        self.ty = ty;
    }

}

impl RemapIdents for TypedSymbol {
    /// Remaps the main symbol and all atomic type identifiers associated with this `TypedSymbol`
    /// according to the provided mapping table.
    ///
    /// Both the symbol itself and each identifier in `self.ty` are replaced if a corresponding
    /// entry exists in `map`.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap` mapping old `Ident`s to new `Ident`s.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.ty.remap_idents(map)?;
        Ok(())
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
