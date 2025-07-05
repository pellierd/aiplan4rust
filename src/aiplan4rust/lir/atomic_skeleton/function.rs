//! Function Signature Representation (`AtomicFunctionSkeleton`)
//!
//! This module defines the [`Function`] struct, which represents the signature of a PDDL function.
//! Functions have a name, typed parameters, and a return type.
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

use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Serialize, Deserialize};

use crate::aiplan4rust::lang::{Ident, Type, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::PlanningDisplay;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Represents the signature of an atomic function in a PDDL-like domain.
///
/// A `Function` is defined by:
/// - An identifier (its name),
/// - A list of typed parameters (its arguments),
/// - A return type (e.g., `Number`, `Object`, etc.).
///
/// This structure is the functional counterpart to [`Formula`] (which represents predicates),
/// except that it carries a return type instead of being implicitly Boolean.
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
/// - Can be constructed from an AST node with [`FromAst`].
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

    /// The return type of the function.
    ty: Type,
}

impl Function {
    /// Creates a new `Function` from name, parameters, and return type.
    ///
    /// # Parameters
    /// - `name`: The function identifier.
    /// - `parameters`: A typed list of the function’s parameters.
    /// - `ty`: The return type of the function.
    pub fn new(name: Ident, parameters: TypedList, ty: Type) -> Self {
        let signature = NamedTypedList::new(name, parameters);
        Self { header: signature, ty }
    }

    /// Returns a reference to the return type.
    pub fn return_type(&self) -> &Type {
        &self.ty
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

impl FromAst for Function {
    /// Builds a [`Function`] from an AST node.
    ///
    /// The AST node is expected to have the following children:
    /// - Child 0: Function identifier (`Ident`)
    /// - Child 1: Typed parameter list
    /// - Child 2: Return type
    ///
    /// # Errors
    ///
    /// Returns a [`ParserInternalError`] if any required child is missing
    /// or if type parsing fails.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let signature = NamedTypedList::from_ast(node, ast)?;

        let ty_id = node.try_child(2)?;
        let ty_node = ast.try_node(ty_id)?;
        let ty = Type::from_ast(ty_node, ast)?;

        Ok(Function { header: signature, ty })
    }
}

impl fmt::Display for Function {
    /// Displays the function as: `(name params) -> return_type`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.header, self.ty)
    }
}

impl DisplayWithInterner for Function {
    /// Displays the function using interned identifiers.
    fn fmt_with(
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

impl PlanningDisplay for Function {
    /// Displays the function in a syntax-oriented form (e.g., PDDL-style).
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
