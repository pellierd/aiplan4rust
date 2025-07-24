//! Module defining abstract skeletons shared by predicates and functions in PDDL.
//!
//! This module provides the `NamedTypedList` struct, which encapsulates the core
//! structure of predicates and functions, including their name and parameter signature.
//!
//! These skeletons serve as a base for more specific constructs by factorizing
//! shared fields and behaviors, allowing for code reuse and clearer abstractions.
//!
//! # Overview
//! - `NamedTypedList`: Represents a named typed list, used as a base for predicates and functions.
//!
//! # Example
//! ```
//! use crate::aiplan4rust::lang::Ident;
//! use crate::aiplan4rust::lang::TypedList;
//! use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
//!
//! let pred = NamedTypedList::new(Ident::new("at"), TypedList::new(vec![]));
//! println!("Predicate name: {}", pred.name());
//! ```

use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};

/// Abstract skeleton core to both predicates and functions in PDDL.
///
/// Encapsulates the shared parts like the name and the signature.
/// This allows factorizing core behavior and fields.
///
/// # Examples
///
/// ```
/// let pred = Skeleton::new(Ident::new("at"), Signature::new(vec![Type::Object]));
/// println!("Name: {}", pred.name());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NamedTypedList {
    /// Name of the predicate or function.
    name: Ident,
    /// Signature describing parameter types and optional return type.
    parameters: TypedList,
}

impl NamedTypedList {
    /// Creates a new `NamedTypedList` with the given name and parameters.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier representing the name of the predicate or function.
    /// - `parameters`: The list of typed parameters (signature).
    ///
    /// # Returns
    ///
    /// A new instance of `NamedTypedList`.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        Self { name, parameters }
    }

    /// Returns the name of the predicate or function.
    ///
    /// # Returns
    ///
    /// The `Ident` representing the name.
    pub fn name(&self) -> Ident {
        self.name
    }

    /// Sets the name of the predicate or function.
    ///
    /// # Parameters
    ///
    /// - `name`: The new name to set.
    pub fn set_name(&mut self, name: Ident) {
        self.name = name;
    }

    /// Returns a reference to the parameters (signature).
    ///
    /// # Returns
    ///
    /// A reference to the `TypedList` representing the parameters.
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    ///
    /// This allows modifying the parameter list directly.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `TypedList`.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }

    /// Sets the parameters (signature) of the predicate or function.
    ///
    /// # Parameters
    ///
    /// - `parameters`: The new `TypedList` to set as the parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.parameters = parameters;
    }
}

/// Attempts to construct a [`NamedTypedList`] from a [`SyntaxSubtree`] referencing an [`AstNode`]
/// and its corresponding [`SyntaxTree`].
///
/// # Parameters
/// - `subtree`: A reference to the `SyntaxSubtree` representing the named typed list structure.
///
/// # Behavior
/// - The first child of the referenced node is expected to be an identifier representing the name.
/// - The second child may either be:
///   - A direct [`TypedList`] node, or
///   - A wrapper node of kind `ParametersDef` whose first child is the actual [`TypedList`].
///
/// # Returns
/// Returns `Ok(NamedTypedList)` on success, or an `AiplanError` if any step of the conversion fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let named = NamedTypedList::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for NamedTypedList {
    type Error = AiplanError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        let name_id = node.try_child(0)?;
        let name_node = ast.try_node(name_id)?;
        let name = name_node.try_ident()?;

        let second_child_id = node.try_child(1)?;
        let second_child_node = ast.try_node(second_child_id)?;

        let parameters = match second_child_node.kind() {
            AstKind::ParametersDef => {
                let parameters_id = second_child_node.try_child(0)?;
                let parameters_node = ast.try_node(parameters_id)?;
                let parameters_subtree = SyntaxSubtree::new(parameters_node, ast);
                TypedList::try_from(&parameters_subtree)?
            }
            _ => {
                let parameters_subtree = SyntaxSubtree::new(second_child_node, ast);
                TypedList::try_from(&parameters_subtree)?
            }
        };

        Ok(NamedTypedList::new(name, parameters))
    }
}

impl Display for NamedTypedList {
    /// Formats the `NamedTypedList` as a string for display purposes.
    ///
    /// Output format: `[name: <name>, parameters: <parameters>]`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt(f)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt(f)?;
        write!(f, "]")
    }
}

impl InternerDisplay for NamedTypedList {
    /// Formats the `NamedTypedList` with a string interner, used for pretty printing.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt_with_interner(f, interner)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt_with_interner(f, interner)?;
        write!(f, "]")
    }
}

impl SyntaxDisplay for NamedTypedList {
    /// Formats the syntax representation of the `NamedTypedList` using an interner.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
