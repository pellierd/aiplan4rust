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

use crate::aiplan4rust::compiler::lir::problem::skeleton::NamedTypedList;
use crate::aiplan4rust::compiler::lir::problem::SymbolRegistry;
use crate::aiplan4rust::compiler::lir::renderers;
use crate::aiplan4rust::compiler::lir::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, LirRenderContext,
};
use crate::aiplan4rust::support::lang::{FunctionSymbolId, Type, TypeId, TypedListId, VariableId};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};

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
    pub fn new(functor: FunctionSymbolId, parameters: TypedListId, ty: Type<TypeId>) -> Self {
        let header = NamedTypedList::new(functor, parameters);
        Self {
            header,
            ty,
            variable_symbols: SymbolRegistry::new(),
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

    #[inline]
    pub fn set_type(&mut self, ty: Type<TypeId>) {
        self.ty = ty;
    }

    pub fn functor(&self) -> FunctionSymbolId {
        self.header.symbol()
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

    #[inline]
    pub fn set_parameters(&mut self, parameters: TypedListId) {
        self.header.set_parameters(parameters);
    }
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

impl LiftedSyntaxDisplay for Function {
    /// Rendu syntaxique PDDL : (distance ?l1 ?l2 - location) - number
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result {
        // On enrichit le contexte avec les variables locales avant d'appeler le renderer
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        // Appel direct au renderer que tu viens de définir
        renderers::syntax::function::render(f, self, &local_ctx)
    }
}

impl LiftedDebugDisplay for Function {
    /// Rendu technique détaillé pour le debugging.
    /// Utilise le renderer spécialisé pour un affichage hybride (PDDL + IDs internes).
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result {
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        renderers::debug::function::render(f, self, &local_ctx)
    }
}
