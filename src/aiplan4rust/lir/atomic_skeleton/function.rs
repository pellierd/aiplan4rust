//! Function Signature Representation (`AtomicFunctionSkeleton`)
//!
//! This module defines the [`Function`] struct, which represents the signature of a PDDL function.
//! Functions have a name, typed parameters, and a return type_checker.
//!
//! This structure is re-exported as [`AtomicFunctionSkeleton`] from the parent module.
//!
//! # Example Use
//!
//! ```rust
//! use aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
//! use aiplan4rust::lang::{Ident, TypedList, Type};
//!
//! let func = AtomicFunctionSkeleton::new(
//!     Ident::new("distance"),
//!     TypedList::empty(),
//!     Type::Number
//! );
//! ```

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::{StringID, RemapIdents, RemapTypes, Type, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};

/// Represents the signature of an atomic function in a PDDL-like domain.
///
/// A `Function` is defined by:
/// - An identifier (its name),
/// - A list of typed parameters (its arguments),
/// - A return type_checker (e.g., `Number`, `Object`, etc.).
///
/// This structure is the functional counterpart to [`Formula`] (which represents predicates),
/// except that it carries a return type_checker instead of being implicitly Boolean.
///
/// Internally, it reuses [`NamedTypedList`] to encapsulate the name and arguments.
///
/// # Example
///
/// ```
/// use aiplan4rust::lang::{Ident, Type, TypedList};
/// use aiplan4rust::lir::atomic_skeleton::function::Function;
///
/// let func = Function::new(
///     Ident::new("distance"),
///     TypedList::from(vec![Type::Location, Type::Location]),
///     Type::Number,
/// );
///
/// assert_eq!(func.return_type(), &Type::Number);
/// assert_eq!(func.arity(), 2);
/// ```
///
/// # Notes
///
/// - Implements [`Deref`] and [`DerefMut`] to access the underlying [`NamedTypedList`] directly.
/// - Can be constructed from an AST syntax with [`FromAst`].
/// - Supports pretty-printing and interner-aware rendering.
///
/// # Display
///
/// Default formatting prints:
/// ```text
/// (distance ?from - location ?to - location) -> number
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Function {
    /// The internal signature: name and parameters.
    header: NamedTypedList,

    /// The return type_checker of the function.
    ty: Type<StringID>,
}

impl Function {
    /// Creates a new `Function` from name, parameters, and return type_checker.
    ///
    /// # Parameters
    /// - `name`: The function identifier.
    /// - `parameters`: A typed list of the function’s parameters.
    /// - `ty`: The return type_checker of the function.
    pub fn new(name: StringID, parameters: TypedList<StringID>, ty: Type<StringID>) -> Self {
        let signature = NamedTypedList::new(name, parameters);
        Self { header: signature, ty }
    }

    // Creates a new `Function` from an already constructed header and a return type.
    ///
    /// This constructor is intended for **internal use only** within the crate.
    /// It allows creating a `Function` skeleton by taking ownership of an
    /// existing [`NamedTypedList`], which is particularly useful when
    /// encoding domain functions where the signature and type are parsed separately.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed header containing the function name and parameters.
    /// * `ty` - The return type of the function (usually a numeric type).
    ///
    /// # Returns
    ///
    /// A new `Function` instance.
    pub(crate) fn from_header(header: NamedTypedList, ty: Type<StringID>) -> Self {
        Self { header, ty }
    }

    /// Returns a reference to the return type_checker.
    pub fn return_type(&self) -> &Type<StringID> {
        &self.ty
    }

    pub fn functor(&self) -> Symbol {
        Symbol::new( self.header.symbol(), SymbolKind::Function)
    }

}

impl RemapIdents for Function {
    /// Remaps all identifiers in this `Function`, including its name (header),
    /// parameters, and return type, according to the provided mapping table.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.header.remap_idents(map)?;
        self.ty.remap_idents(map)?;
        Ok(())
    }
}

impl RemapTypes for Function {
    /// Remaps union types (`Type::Either`) in the function's parameters and return type.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during remapping.
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        self.header.remap_types(map)?;
        self.ty.remap_types(map)?;
        Ok(())
    }
}
// Allow transparent access to the underlying NamedTypedList (e.g., name, parameters).
impl Deref for Function {
    type Target = NamedTypedList;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Function {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl fmt::Display for Function {
    /// Displays the function as: `(name params) -> return_type`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.header, self.ty)
    }
}

impl InternerDisplay for Function {
    /// Displays the function using interned identifiers.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        write!(
            f,
            "{} -> {}",
            self.header.to_string_with_interner(interner),
            self.ty.to_string_with_interner(interner)
        )
    }
}

impl SyntaxInternerDisplay for Function {
    /// Displays the function in a syntax-oriented form (e.g., PDDL-style).
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        // Write the header with indentation
        self.header.fmt_syntax_with_interner_and_indent(f, interner, indent)?;

        // Write the separator " - "
        write!(f, " - ")?;

        // Write the type_checker by converting it to string and then writing to formatter
        let ty_str = self.ty.to_syntax_string_with_interner(interner);
        write!(f, "{}", ty_str)?;

        Ok(())
    }
}
