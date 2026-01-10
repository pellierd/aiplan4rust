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

use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Serialize, Deserialize};

use crate::aiplan4rust::lang::{FlattenTypes, Ident, RemapIdents, Type, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::remap_idents::RemapIdentError;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

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
    ty: Type,
}

impl Function {
    /// Creates a new `Function` from name, parameters, and return type_checker.
    ///
    /// # Parameters
    /// - `name`: The function identifier.
    /// - `parameters`: A typed list of the function’s parameters.
    /// - `ty`: The return type_checker of the function.
    pub fn new(name: Ident, parameters: TypedList, ty: Type) -> Self {
        let signature = NamedTypedList::new(name, parameters);
        Self { header: signature, ty }
    }

    /// Returns a reference to the return type_checker.
    pub fn return_type(&self) -> &Type {
        &self.ty
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
    /// Returns [`RemapIdentError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) -> Result<(), RemapIdentError> {
        self.header.remap_idents(map)?;
        self.ty.remap_idents(map)?;
        Ok(())
    }
}

impl FlattenTypes for Function {
    /// Replaces union types (`Type::Either`) in the function’s parameters
    /// and return type with their corresponding primitive identifiers
    /// based on the provided mapping.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their new primitive `Ident`s.
    fn flatten_types(&mut self, map: &HashMap<Type, Ident>) {
        self.header.flatten_types(map);
        self.ty.flatten_types(map);
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

/// Attempts to construct a [`Function`] from a [`SyntaxSubtree`] referencing an [`AstNode`]
/// and its associated [`SyntaxTree`].
///
/// # Expectations
///
/// The AST node must follow this structure:
/// - **Child 0**: Function identifier (`Ident`)
/// - **Child 1**: Typed parameter list
/// - **Child 2**: Return type_checker
///
/// # Returns
///
/// - `Ok(Function)` on success.
/// - `Err(AiplanError)` if any required child is missing or if type_checker resolution fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let function = Function::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for Function {
    type Error = LirError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let signature = NamedTypedList::try_from(subtree)?;

        let ty_id = subtree.node().try_child(2)?;
        let ty_node = subtree.tree().try_node(ty_id)?;
        let ty = Type::try_from(&SyntaxSubtree::new(ty_node, subtree.tree()))?;

        Ok(Function { header: signature, ty })
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
