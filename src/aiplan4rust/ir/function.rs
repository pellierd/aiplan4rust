use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Serialize, Deserialize};

use crate::aiplan4rust::lang::{Ident, Type, TypedList};
use crate::aiplan4rust::ir::Signature;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Represents the signature of a PDDL function (name, typed parameters, return type).
///
/// This type encapsulates a [`Signature`] to factor out the name and parameters,
/// and explicitly adds the return type.
///
/// # Example
///
/// ```
/// let func = FunctionSkeleton::new(
///     Ident::new("distance"),
///     TypedList::from(vec![Type::Location, Type::Location]),
///     Type::Number,
/// );
/// assert_eq!(func.return_type(), &Type::Number);
/// assert_eq!(func.parameters().len(), 2);
/// ```
///
/// # Details
///
/// - Implements [`Deref`] and [`DerefMut`] to [`Signature`] so you can directly access
///   methods like `name()` or `parameters()`.
/// - Can be created from an AST using [`FromAst`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Function {
    /// The skeleton holding the name and parameters.
    signature: Signature,
    /// The return type of the function.
    ty: Type,
}

impl Function {
    /// Creates a new function signature with a name, a list of parameters, and a return type.
    ///
    /// # Parameters
    ///
    /// * `name` - The function identifier.
    /// * `parameters` - The typed list of parameters.
    /// * `ty` - The return type.
    pub fn new(name: Ident, parameters: TypedList, ty: Type) -> Self {
        let skeleton = Signature::new(name, parameters);
        Self { signature: skeleton, ty }
    }

    /// Returns a reference to the return type.
    pub fn return_type(&self) -> &Type {
        &self.ty
    }
}

// Enable treating FunctionSkeleton as a Skeleton directly.
impl Deref for Function {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        &self.signature
    }
}

impl DerefMut for Function {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.signature
    }
}

// Conversion from AST.
impl FromAst for Function {
    /// Builds a `FunctionSkeleton` from an AST node.
    ///
    /// Expects a tree whose children are:
    /// 0 - the function name (Ident)
    /// 1 - the typed parameter list
    /// 2 - the return type
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        let signature = Signature::from_ast(node, ast)?;

        let ty_id = node.try_child(2)?;
        let ty_node = ast.try_node(ty_id)?;
        let ty = Type::from_ast(ty_node, ast)?;

        Ok(Function { signature, ty })
    }
}

/// Simple display: `name(params) -> return_type`.
impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.signature, self.ty)
    }
}

/// Display with interner support to resolve identifiers to strings.
impl DisplayWithInterner for Function {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        write!(
            f,
            "{} -> {:?}",
            self.signature.to_string_with_interner(interner),
            self.ty.to_string_with_interner(interner)
        )
    }
}

/// Syntax display (currently identical to `fmt_with`).
impl DisplaySyntax for Function {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
