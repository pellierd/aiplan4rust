use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Abstract skeleton common to both predicates and functions in PDDL.
///
/// Encapsulates the shared parts like the name and the signature.
/// This allows factorizing common behavior and fields.
///
/// # Examples
///
/// ```
/// let pred = Skeleton::new(Ident::new("at"), Signature::new(vec![Type::Object]));
/// println!("Name: {}", pred.name);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Signature {
    /// Name of the predicate or function.
    name: Ident,
    /// Signature describing parameter types and optional return type.
    parameters: TypedList,
}

impl Signature {
    /// Creates a new skeleton with a name and a list of parameters.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        Self { name, parameters }
    }

    /// Returns the name.
    pub fn name(&self) -> Ident {
        self.name
    }

    pub fn set_name(&mut self, name: Ident) {
        self.name = name;
    }

    /// Returns the signature.
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Returns a mutable reference to the signature.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.parameters = parameters;
    }
}

impl FromAst for Signature {
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        let name_id = node.try_child(0)?;
        let name_node = ast.try_node(name_id)?;
        let name = name_node.try_ident()?;

        let params_id = node.try_child(1)?;
        let params_node = ast.try_node(params_id)?;
        let parameters = TypedList::from_ast(params_node, ast)?;

        Ok(Signature::new(name, parameters))
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt(f)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt(f)?;
        write!(f, "]")
    }
}

impl DisplayWithInterner for Signature {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt_with(f, interner)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt_with(f, interner)?;
        write!(f, "]")
    }
}

impl DisplaySyntax for Signature {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
