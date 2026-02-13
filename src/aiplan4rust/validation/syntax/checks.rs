//! Validation functions for AST nodes.
//!
//! This module provides example validation functions for various AST node types,
//! illustrating how to check node content and structure according to expected rules.
//!
//! The provided functions serve as reference implementations for common AST node checks,
//! such as verifying node content kinds and child counts.
//!
//! These examples can be extended or adapted for other node types as needed.
//!
//! # Examples of node validations included:
//! - Symbol nodes: must have identifier content and no children.
//! - Number nodes: must contain a float value and no children.
//! - Requirement nodes: must contain a requirement value and no children.
//! - etc.
//!
//! # Usage
//!
//! Call these functions with AST nodes to ensure they conform to expected syntax and semantics.
//!
//! # Notes
//! These validations rely on common utilities from the `common::checks` module.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::validation::common;
use crate::aiplan4rust::validation::common::checks::{ContentKind, EXPRESSION};
use crate::WellFormedError;

/// Checks that a symbol node has identifier content and no children.
///
/// # Arguments
/// * `node` - The node to check.
///
pub fn check_symbol(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Ident)?;
    common::checks::check_children_count(node.arity(), 0, node)?;
    Ok(())
}

/// Checks that the node represents a well-formed number (float)
/// with no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the content is not a float or if the node has any children.
pub fn check_number(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Float)?;
    common::checks::check_children_count(node.arity(), 0, node)?;
    Ok(())
}

/// Checks that the node represents a well-formed requirement
/// with no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the content is not a requirement or if the node has any children.
pub fn check_requirement(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Requirement)?;
    common::checks::check_children_count(node.arity(), 0, node)?;
    Ok(())
}

/// Checks that the given node is a `RequireDef` with all children of kind `Requirement`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node's children are not all of kind `Requirement`.
pub fn check_require_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_all_children_kind(ast, node, &[AstKind::Requirement])
}

/// Checks that the given node of kind `Type` has at least one child,
/// and that all its children are of kind `PrimitiveType`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// or if any child is not of kind `PrimitiveType`.
pub fn check_type(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 1, node)?;
    common::checks::check_all_children_kind(ast, node, &[AstKind::PrimitiveType])
}

/// Checks that the given node of kind `TypesDef` has at least one child,
/// and that the first child is of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// or if the first child is not of kind `TypedList`.
pub fn check_types_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 1, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::PrimitiveType])
}

/// Checks that the given node of kind `TypedList` has all its children of kind `TypedItem`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if any child is not of kind `TypedItem`.
pub fn check_typed_list(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_all_children_kind(ast, node, &[AstKind::TypedItem])
}

/// Checks that the given node of kind `TypedItem` has either one or two children:
/// - If one child, it must be of kind `TypedItemElements`.
/// - If two children, the first must be `TypedItemElements`, the second must be `Type`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the number of children is not 1 or 2,
/// or if the children are not of the expected kinds.
pub fn check_typed_item(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => common::checks::check_child_kind(ast, node, 0, &[AstKind::TypedItemElements]),
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::TypedItemElements])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::Type])
        }
        _ => unreachable!(),
    }
}

/// Checks that the given node of kind `TypedItemElements` has at least one child,
/// and that all its children are of the specified allowed kinds.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// or if any child is not of one of the allowed kinds.
pub fn check_typed_item_elements(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 1, node)?;
    common::checks::check_all_children_kind(
        ast,
        node,
        &[
            AstKind::PrimitiveType,
            AstKind::Variable,
            AstKind::Constant,
            AstKind::FunctionTerm,
            AstKind::AtomicFunctionSkeleton,
        ],
    )
}

/// Checks that the given node of kind `ConstantsDef` or `ObjectsDef`
/// has exactly one child, which must be of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `TypedList`.
pub fn check_constants_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])
}

/// Checks that the given node of kind `PredicatesDef`
/// has all its children of kind `AtomicFormulaSkeleton`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if any child is not of kind `AtomicFormulaSkeleton`.
pub fn check_predicates_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_all_children_kind(ast, node, &[AstKind::AtomicFormulaSkeleton])
}

/// Checks that the given node of kind `AtomicFormulaSkeleton`
/// has 1 or 2 children: the first must be a `Predicate`
/// and the optional second must be a `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not 1 or 2,
/// or if the first child is not `Predicate`,
/// or if the second child (if present) is not `TypedList`.
pub fn check_atomic_formula_skeleton(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::Predicate])
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::Predicate])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::TypedList])
        }
        _ => unreachable!("Children count outside validated range"),
    }
}

/// Checks that the given node of kind `FunctionsDef`
/// has exactly one child of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if that child is not of kind `TypedList`.
pub fn check_functions_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])
}

/// Checks that the given node of kind `AtomicFunctionSkeleton`
/// has 1 or 2 children:
/// - The first child must be of kind `FunctionSymbol`
/// - The second child, if present, must be of kind `TypedList`
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have 1 or 2 children,
/// or if the children are not of the expected kinds.
pub fn check_atomic_function_skeleton(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::TypedList])
        }
        _ => unreachable!("Children count already validated to be 1 or 2"),
    }
}

/// Checks that the given node of kind `ActionDef` has exactly three children:
/// - The first child must be an `ActionSymbol`
/// - The second child must be a `ParametersDef`
/// - The third child must be an `ActionDefBody`
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly three children,
/// or if any child is not of the expected kind.
pub fn check_action_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 3, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::ActionSymbol])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    common::checks::check_child_kind(ast, node, 2, &[AstKind::ActionDefBody])
}

/// Checks that the given node of kind `ActionDefBody` has between 0 and 2 children.
///
/// The allowed children configurations are:
/// - 0 children: valid (empty body)
/// - 1 child: must be either `PreconditionDef` or `EffectDef`
/// - 2 children: first must be `PreconditionDef`, second must be `EffectDef`
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is out of range or
/// if any child does not match the expected kinds.
pub fn check_action_def_body(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 0, 2, node)?;

    match children_len {
        0 => Ok(()),
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::PreconditionDef, AstKind::EffectDef])
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::PreconditionDef])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::EffectDef])
        }
        _ => unreachable!("children count already validated to be between 0 and 2"),
    }
}

/// Checks that the given node of kind `MethodDef` has exactly three children:
/// - The first child must be a `MethodSymbol`.
/// - The second child must be a `ParametersDef`.
/// - The third child must be a `MethodDefBody`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not exactly three,
/// or if any child is not of the expected kind.
pub fn check_method_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 3, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::MethodSymbol])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    common::checks::check_child_kind(ast, node, 2, &[AstKind::MethodDefBody])
}

/// Checks that the given node of kind `ParametersDef` has exactly one child,
/// and that this child is of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `TypedList`.
pub fn check_parameters_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 1, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::Variable])
}

/// Checks that the given node of kind `MethodDefBody` has between 2 and 3 children,
/// and that the children conform to expected kinds depending on the number of children.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not between 2 and 3,
/// or if the children are not of the expected kinds.
pub fn check_method_def_body(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 2, 3, node)?;

    match children_len {
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::Task])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
        }
        3 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::Task])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::MethodPreconditionDef])?;
            common::checks::check_child_kind(ast, node, 2, &[AstKind::TaskNetworkDef])
        }
        _ => unreachable!(),
    }
}

/// Checks that the given node of kind `Task` has at least one child,
/// the first child is of kind `TaskSymbol`,
/// and all subsequent children are either `Variable` or `Constant`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the minimum children count is not met,
/// if the first child is not `TaskSymbol`,
/// or if any child from index 1 onwards is not `Variable` or `Constant`.
pub fn check_task(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_min_children_count(children_len, 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
    common::checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])
}

/// Checks that the given node, either `PreconditionDef` or `MethodPreconditionDef`,
/// has exactly one child and that this child is of kind `Expression`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if that child is not of kind `Expression`.
pub fn check_precondition_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, EXPRESSION)
}

/// Checks that the given node of kind `EffectDef` has exactly one child,
/// and that this child is of kind `Expression`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if that child is not of kind `Expression`.
pub fn check_effect_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, EXPRESSION)
}

/// Checks that the given node of kind `FunctionTerm` has at least one child,
/// the first child is a `FunctionSymbol`, and all subsequent children are
/// either `Variable` or `Constant`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// if the first child is not `FunctionSymbol`,
/// or if any other child is not `Variable` or `Constant`.
pub fn check_function_term(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
    common::checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])
}

/// Checks that the given node of kind `AtomicFormula` has at least one child,
/// the first child is a `Predicate`, and all subsequent children are
/// either `Variable` or `Constant`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// if the first child is not `Predicate`,
/// or if any other child is not `Variable` or `Constant`.
pub fn check_atomic_formula(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Predicate])?;
    common::checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])
}

/// Checks that all children of the given node are of kind `Expression`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if any child is not of kind `Expression`.
pub fn check_all_children_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_all_children_kind(ast, node, EXPRESSION)
}
/// Checks that the given node has exactly one child,
/// and that this child is of kind `Expression`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `Expression`.
pub fn check_unary_child_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, EXPRESSION)
}

/// Checks that the given node has at least two children,
/// and that the first two children are of kind `Expression`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than two children,
/// or if either of the first two children is not of kind `Expression`.
pub fn check_binary_child_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, EXPRESSION)?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that the given node is a quantifier (`Forall` or `Exists`) with:
/// - at least two children,
/// - the first child being a `TypedList`,
/// - the second child being an expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have at least two children,
/// or if the children do not have the expected kinds.
pub fn check_quantified_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_min_children_count(children_len, 2, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::Variable])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that the given node is a `Preference` expression node, with:
/// - at least two children,
/// - the first child being a `PrefName`,
/// - the second child being an expression (`EXPRESSION`).
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if:
/// - the node has fewer than two children,
/// - the first child is not a `PrefName`,
/// - the second child is not an expression.
pub fn check_preference_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_min_children_count(children_len, 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::PrefName])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that the given node of kind `FComp` has 1 or 2 children,
/// and that the children are of allowed kinds:
/// - If 1 child: Number, FComp, FunctionTerm, Variable, or Constant.
/// - If 2 children:
///     - First child: Number, FComp, FunctionTerm, Variable, or Constant.
///     - Second child: FComp, FunctionTerm, Variable, or Constant.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node structure does not conform.
pub fn check_fcomp_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 1, 2, node)?;
    match children_len {
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[
                    AstKind::Number,
                    AstKind::FComp,
                    AstKind::FunctionTerm,
                    AstKind::Variable,
                    AstKind::Constant,
                ],
            )
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[
                    AstKind::Number,
                    AstKind::FComp,
                    AstKind::FunctionTerm,
                    AstKind::Variable,
                    AstKind::Constant,
                ],
            )?;
            common::checks::check_child_kind(ast, node, 1, &[
                    AstKind::Number,
                    AstKind::FComp,
                    AstKind::FunctionTerm,
                    AstKind::Variable,
                    AstKind::Constant,
                ],
            )
        }
        _ => unreachable!(),
    }
}

/// Checks that an `Assign` node has exactly two children:
/// the first must be a `FunctionTerm`, the second must be a numeric expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children do not match the expected kinds.
pub fn check_assign_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::FunctionTerm])?;
    common::checks::check_child_kind(ast, node, 1, &[
            AstKind::Number,
            AstKind::Variable,
            AstKind::Constant,
            AstKind::FunctionTerm,
        ],
    )
    // Todo: Adding undefined
}

/// Checks that an `Operation` node has one or two children,
/// and that each child is of kind `FComp`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the number of children is not 1 or 2,
/// or if any child does not have kind `FComp`.
pub fn check_arithmetic_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => common::checks::check_child_kind(ast, node, 0, &[AstKind::FComp]),
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::FComp])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::FComp])
        }
        _ => unreachable!(),
    }
}

/// Checks that a `Within` or `HoldAfter` node has exactly two children:
/// the first must be a `Number`, the second an expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children do not match the expected kinds.
pub fn check_within_hold_after_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that an `AlwaysWithin` node has exactly three children:
/// the first must be a `Number`, the second and third must be expr.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children do not match the expected kinds.
pub fn check_always_within_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 3, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)?;
    common::checks::check_child_kind(ast, node, 2, EXPRESSION)
}

/// Checks that a `HoldDuring` node has exactly three children:
/// the first two must be `Number`s, the third must be an expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children do not match the expected kinds.
pub fn check_hold_during_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count(children_len, 3, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::Number])?;
    common::checks::check_child_kind(ast, node, 2, EXPRESSION)
}

/// Checks that an `Init` node has exactly one child of kind `And`,
/// and that all children of this `And` node are one of the allowed kinds.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The `Init` AST node to validate.
///
/// # Errors
/// Returns an error if the node's children do not match the expected structure
/// or if the `And` node's children are not among the allowed kinds.
pub fn check_init_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::And])?;

    let init_elements = common::checks::get_child_node(ast, node, 0)?;

    common::checks::check_all_children_kind(
        ast,
        init_elements,
        &[
            AstKind::TimedInitialLiteral,
            AstKind::FComp,
            AstKind::AtomicFormula,
            AstKind::Not,
        ],
    )?;

    // TODO: Add check that `Not` nodes contain only atomic formula

    Ok(())
}

/// Checks that a `TimedInitialLiteral` node has exactly two children:
/// the first child must be a `Number`,
/// the second child must be either an `FComp` or a `Not`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not exactly two,
/// or if the children do not match the expected kinds.
pub fn check_timed_initial_literal(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::FComp, AstKind::Not])

    // TODO: Add check that `Not` nodes contain only atomic formula
}

/// Checks that a `DerivedDef` node has exactly two children:
/// the first child must be an `AtomicFormulaSkeleton`,
/// the second child must be an expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not exactly two,
/// or if the children do not match the expected kinds.
pub fn check_derived_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::AtomicFormulaSkeleton])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that an `OrderedSubtaskDef` or `PartiallyOrderedSubtaskDef` node
/// has exactly one child which is an `And` node,
/// and that all children of this `And` node are either `TaggedTask` or `Task`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `And`,
/// or if any of the `And` node's children are not `TaggedTask` or `Task`.
pub fn check_ordered_subtask_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::And])?;
    let tasks = common::checks::get_child_node(ast, node, 0)?;
    common::checks::check_all_children_kind(ast, tasks, &[AstKind::TaggedTask, AstKind::Task])
}

/// Checks that a `TaggedTask` node has exactly two children:
/// the first must be a `TaskID`, the second must be a `Task`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly two children,
/// or if the children are not of the expected kinds.
pub fn check_tagged_task(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TaskID])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::Task])
}

/// Checks that a `TaskOrderingConstraintDef` node has exactly one child,
/// which must be an `And` node, and all children of this `And` node
/// must be of kind `TaskOrderingConstraint`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `And`,
/// or if any child of the `And` node is not of kind `TaskOrderingConstraint`.
pub fn check_task_ordering_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::And])?;
    let ordering = common::checks::get_child_node(ast, node, 0)?;
    common::checks::check_all_children_kind(ast, ordering, &[AstKind::TaskOrderingConstraint])
}

/// Checks that a `TaskOrderingConstraint` node has exactly two children,
/// both of which must be of kind `TaskID`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly two children,
/// or if any child is not of kind `TaskID`.
pub fn check_task_ordering_constraint(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TaskID])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::TaskID])
}

/// Checks that a `TaskNetworkDef` node has between 0 and 3 children,
/// and verifies that each child is of an expected kind depending on
/// the number of children.
///
/// The valid child kinds are:
/// - If 0 children: valid (empty).
/// - If 1 child: must be one of `OrderedSubtaskDef`, `PartiallyOrderedSubtaskDef`, or `TaskLogicalConstraintDef`.
/// - If 2 children: first must be `OrderedSubtaskDef` or `PartiallyOrderedSubtaskDef`,
///   second must be `TaskOrderingConstraintDef` or `TaskLogicalConstraintDef`.
/// - If 3 children: first must be `OrderedSubtaskDef` or `PartiallyOrderedSubtaskDef`,
///   second must be `TaskOrderingConstraintDef`,
///   third must be `TaskLogicalConstraintDef`.
///
/// # Arguments
///
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
///
/// Returns an error if the children count is not between 0 and 3,
/// or if the children do not match the expected kinds.
pub fn check_task_network_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    common::checks::check_children_count_range(children_len, 0, 3, node)?;
    match children_len {
        0 => Ok(()),
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[
                    AstKind::OrderedSubtaskDef,
                    AstKind::PartiallyOrderedSubtaskDef,
                    AstKind::TaskLogicalConstraintDef,
                ],
            )
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[
                    AstKind::OrderedSubtaskDef,
                    AstKind::PartiallyOrderedSubtaskDef,
                ],
            )?;
            common::checks::check_child_kind(ast, node, 1, &[
                    AstKind::TaskOrderingConstraintDef,
                    AstKind::TaskLogicalConstraintDef,
                ],
            )
        }
        3 => {
            common::checks::check_child_kind(ast, node, 0, &[
                    AstKind::OrderedSubtaskDef,
                    AstKind::PartiallyOrderedSubtaskDef,
                ],
            )?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::TaskOrderingConstraintDef])?;
            common::checks::check_child_kind(ast, node, 2, &[AstKind::TaskLogicalConstraintDef])
        }
        _ => unreachable!(),
    }
}

/// Checks that a `TaskDef` node has exactly two children:
/// the first must be a `TaskSymbol`,
/// the second must be a `ParametersDef`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not 2,
/// or if the children do not match the expected kinds.
pub fn check_task_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])
}

/// Checks that a `TotalTime` node has no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has any children.
pub fn check_total_time(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 0, node)
}

/// Checks that an `IsViolated` node has exactly one child,
/// which must be of kind `PrefName`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the child count is not 1 or the child kind is not `PrefName`.
pub fn check_is_violated(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::PrefName])
}

/// Checks that a `Length` node has between 0 and 2 children,
/// all of which must be either `Serial` or `Parallel`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the number of children is outside the allowed range
/// or if any child is not `Serial` or `Parallel`.
pub fn check_length_spec(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count_range(node.arity(), 0, 2, node)?;
    common::checks::check_all_children_kind(ast, node, &[AstKind::Serial, AstKind::Parallel])
}

/// Checks that a `Serial` or `Parallel` node has exactly one child,
/// which must be of kind `Number`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the child count is not exactly one or
/// if the child is not a `Number`.
pub fn check_serial_parallel_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 1, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::Number])
}

/// Checks that a `DurativeActionDef` node has exactly three children:
/// 1. A `DASymbol`
/// 2. A `ParametersDef`
/// 3. A `DADefBody`
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count is not exactly three or
/// if the children are not of the expected kinds.
pub fn check_durative_action_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 3, node)?;
    common::checks::check_child_kind(ast, node, 0, &[AstKind::DASymbol])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    common::checks::check_child_kind(ast, node, 2, &[AstKind::DADefBody])
}

/// Checks that a `DADefBody` node has exactly three children,
/// all of which must be valid expr.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the number of children is not three
/// or if any child is not a valid expression.
pub fn check_duartive_action_def_body(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count(node.arity(), 3, node)?;
    common::checks::check_child_kind(ast, node, 0, EXPRESSION)?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)?;
    common::checks::check_child_kind(ast, node, 2, EXPRESSION)
}

/// Checks that an `InitialTaskNetwork` node has 1 or 2 children:
/// - If 1 child: it must be a `TaskNetworkDef`.
/// - If 2 children: first must be `ParametersDef`, second `TaskNetworkDef`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children count or kinds do not match the expected structure.
pub fn check_initial_task_network(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_children_count_range(node.arity(), 1, 2, node)?;
    match node.arity() {
        1 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::TaskNetworkDef])
        }
        2 => {
            common::checks::check_child_kind(ast, node, 0, &[AstKind::ParametersDef])?;
            common::checks::check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
        }
        _ => unreachable!(),
    }
}

/// Recursively validates a subtree of the AST representing typed lists and their components.
///
/// This function checks that the given `node` and its descendants conform to the expected
/// structure and kinds for typed lists, typed items, typed item elements, and types.
///
/// # Parameters
/// - `ast`: Reference to the AST structure containing all nodes.
/// - `node`: The current AST node to validate.
/// - `expected`: A slice of `AstKind` representing the expected kinds for children of `TypedItemElements`.
///
/// # Behavior
/// - If `node` is a `TypedList`, it verifies that all children are `TypedItem` nodes.
/// - If `node` is a `TypedItem`, it verifies it has two children:
///   a `TypedItemElements` node and a `Type` node, which are checked accordingly.
/// - If `node` is `TypedItemElements`, it ensures it has at least one child, and all children
///   are among the `expected` kinds provided.
/// - If `node` is a `Type`, it validates the type_checker node (leaf node).
/// - For any unexpected node kinds, it returns an error.
///
/// # Recursion
/// The function recursively validates all children of `TypedList` and `TypedItem` nodes,
/// but stops recursion for leaf nodes (`TypedItemElements` and `Type`).
///
/// # Errors
/// Returns `WellFormedError::InvalidNodeKind` if an unexpected node kind is encountered.
/// Propagates errors from internal checks performed on nodes.
///
/// # Example
/// ```ignore
/// check_typed_list_of(&ast, root_node, &[AstKind::PrimitiveType])?;
/// ```
pub fn check_typed_list_of(
    ast: &Ast,
    node: &AstNode,
    expected: &[AstKind],
) -> Result<(), WellFormedError> {
    // Get the list of child node IDs of the current node
    let children_ids = node.children();

    // Match on the kind of the current node to apply the appropriate checks
    match node.kind() {
        AstKind::TypedList => {
            // If it's a TypedList, check that all its children are TypedItem nodes
            check_typed_list(ast, node)?;
        }
        AstKind::TypedItem => {
            // TypedItem should have exactly two children: TypedItemElements and Type
            // Check these constraints accordingly
            check_typed_item(ast, node)?;
        }
        AstKind::TypedItemElements => {
            // TypedItemElements must have at least one child
            common::checks::check_min_children_count(node.arity(), 1, node)?;
            // All children must be of the expected kinds passed in the `expected` slice
            common::checks::check_all_children_kind(ast, node, expected)?;
            // Return early because TypedItemElements are leaf nodes for this validation
            // No need to recurse further down this branch
            return Ok(())
        }
        AstKind::Type => {
            // Type nodes are also leaf nodes: validate and stop recursion here
            check_type(ast, node)?;
            return Ok(())
        }
        _ => {
            // If the node kind is unexpected here, return an error indicating invalid node kind
            return Err(WellFormedError::InvalidNodeKind { found: node.clone() }.into());
        }
    }

    // For TypedList and TypedItem nodes, recursively check all children
    for child_id in children_ids {
        let child_node = common::checks::get_node(ast, node, child_id.as_usize())?;
        // Recursive call with the same expected kinds
        check_typed_list_of(ast, child_node, expected)?;
    }

    Ok(())
}
