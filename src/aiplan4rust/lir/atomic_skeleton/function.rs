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

use crate::aiplan4rust::lang::{Type, TypedList, TypeId, VariableId, FunctionSymbolId};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::grounding::problem::SymbolRegistry;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    /// The internal signature: name and parameters.
    header: NamedTypedList<FunctionSymbolId>,

    /// The return type_checker of the function.
    ty: Type<TypeId>,

    variable_symbols: SymbolRegistry<VariableId>,
}

impl Function {
    /// Creates a new `Function` from name, parameters, and return type_checker.
    ///
    /// # Parameters
    /// - `name`: The function identifier.
    /// - `parameters`: A typed list of the function’s parameters.
    /// - `types`: The return type_checker of the function.
    pub fn new(functor: FunctionSymbolId, parameters: TypedList<VariableId, TypeId>, ty: Type<TypeId>) -> Self {
        let header = NamedTypedList::new(functor, parameters);
        Self {
            header,
            ty,
            variable_symbols: SymbolRegistry::new()
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    /// Usage : Action::new_simple(...).with_symbols(ma_table)
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    /// Returns a reference to the return type_checker.
    pub fn ty(&self) -> &Type<TypeId> {
        &self.ty
    }

    /// Returns a mutable reference to the return type.
    pub fn ty_mut(&mut self) -> &mut Type<TypeId> {
        &mut self.ty
    }

    pub fn functor(&self) -> FunctionSymbolId {
        self.header.symbol()
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> { &self.variable_symbols }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> { &mut self.variable_symbols }


}

// Allow transparent access to the underlying NamedTypedList (e.g., name, parameters).
impl Deref for Function {
    type Target = NamedTypedList<FunctionSymbolId>;

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
