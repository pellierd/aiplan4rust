//! This module provides helper functions and constants to perform
//! structural validation of an Abstract Syntax Tree (AST) used in
//! the AIPlan4Rust project.
//!
//! The validation focuses on checking node kinds, children counts,
//! expected content kinds, and other well-formedness properties.
//!
//! Note: This validation is not exhaustive and does not guarantee
//! semantic correctness. It only ensures that the AST structure
//! meets basic well-formedness criteria.
//!
//! The constant `EXPRESSION` defines a set of `AstKind` variants
//! considered valid logic within the AST.
//!
//! # Usage
//! Use the provided functions to check specific properties of AST nodes,
//! including children kinds, counts, and content, returning detailed
//! `WellFormedError`s in case of validation failures.

use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::NodeId;
use crate::aiplan4rust::compiler::syntax::ast::{Ast, AstContent, AstKind, AstNode};
use crate::aiplan4rust::compiler::syntax::validation::WellFormedError;

/// Set of AST node kinds considered as valid logic.
pub const EXPRESSION: &[AstKind] = &[
    AstKind::Or,
    AstKind::And,
    AstKind::Not,
    AstKind::Imply,
    AstKind::Exists,
    AstKind::Forall,
    AstKind::AtomicFormula,
    AstKind::Preference,
    AstKind::When,
    AstKind::Comparison,
    AstKind::Assignment,
    AstKind::Arithmetic,
    AstKind::AtStart,
    AstKind::AtEnd,
    AstKind::Overall,
    AstKind::Always,
    AstKind::Sometime,
    AstKind::Within,
    AstKind::AtMostOnce,
    AstKind::SometimeAfter,
    AstKind::SometimeBefore,
    AstKind::AlwaysWithin,
    AstKind::HoldDuring,
    AstKind::HoldAfter,
    AstKind::TimedInitialLiteral,
    AstKind::LabeledTask,
    AstKind::Task,
    AstKind::TaskOrderingConstraint,
];

/// Kinds of nodes allowed within a PDDL metric optimization expression.
///
/// A metric expression (found under `(:metric minimize ...)` or `(:metric maximize ...)`)
/// specifically expects numeric terms or fluents. This list defines the
/// legal syntactic vocabulary for these expressions, distinct from logical goal
/// descriptions.
///
/// # Allowed Components
/// * **Arithmetic**: Complex numeric expressions (e.g., `(+ (total-cost) 5)`).
/// * **Function**: Direct references to numeric fluents or functions.
/// * **Number**: Literal numeric values.
/// * **Variable**: Variables that resolve to numeric values in the current context.
/// * **IsViolated**: Special PDDL construct for preferences/constraints.
pub const METRIC_EXPRESSION: &[AstKind] = &[
    AstKind::Arithmetic,
    AstKind::Function,
    AstKind::Number,
    AstKind::Variable,
    AstKind::IsViolated,
];

/// Enumeration representing the expected kind of content inside an AST node.
#[derive(Debug)]
pub enum ContentKind {
    /// Node with no content.
    None,
    /// Identifier content.
    Ident,
    /// Floating point number content.
    Float,
    /// Requirement content.
    Requirement,
    /// Binary comparison operator content.
    BinaryComp,
    /// Assignment operator content.
    AssignOp,
    /// Arithmetic operator content.
    ArithmeticOp,
    /// Optimization specification content.
    Optimization,
}

/// Checks that the child at a given index of a parent AST node has one of the expected kinds.
///
/// This validation ensures that the child node at the specified `index` within the `parent` node
/// matches one of the `expected_kinds`, which is important for enforcing structural constraints
/// on the AST.
///
/// # Arguments
/// * `ast` - The full AST, used to resolve child node references from the parent.
/// * `parent` - The parent node whose child is being validated.
/// * `index` - The index of the child in the parent's children list to check.
/// * `expected_kinds` - A slice of acceptable [`AstKind`]s that the child is expected to match.
///
/// # Errors
/// Returns a [`WellFormedError::UnexpectedChildKind`] if the actual kind of the child node
/// does not match any of the kinds listed in `expected_kinds`.
///
/// Also propagates any error returned by [`get_node`] if the child cannot be resolved from the AST.
pub fn check_child_kind(
    ast: &Ast,
    parent: &AstNode,
    index: usize,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    let children = parent.children();

    let child = get_node(ast, parent, children[index].as_usize())?;
    let found = child.kind();

    if expected_kinds.contains(&found) {
        Ok(())
    } else {
        Err(WellFormedError::unexpected_child_kind(
            index,
            expected_kinds.to_vec(),
            found,
            parent.clone(),
        )
        .into())
    }
}

/// Checks that the number of children of a node is exactly equal to `expected`.
///
/// This validation is useful to enforce the arity of AST nodes, ensuring that
/// nodes have the exact number of children expected for their kind.
///
/// # Arguments
/// * `children_len` - The actual number of children the node has.
/// * `expected` - The required number of children for this node.
/// * `parent` - The AST node whose children are being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityMismatch`] if `children_len` does not equal `expected`.
pub fn check_children_count(
    children_len: usize,
    expected: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len != expected {
        Err(WellFormedError::children_arity_mismatch(
            expected,
            children_len,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the number of children of a node is at least `min`.
///
/// This ensures that a node meets the minimum required arity for correct structure.
///
/// # Arguments
/// * `children_len` - The actual number of children the node has.
/// * `min` - The minimum required number of children for this node.
/// * `parent` - The AST node whose children are being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityOutOfRange`] if `children_len` is less than `min`.
pub fn check_min_children_count(
    children_len: usize,
    min: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len < min {
        Err(WellFormedError::children_arity_out_of_range(
            min,
            usize::MAX,
            children_len,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the number of children lies within the inclusive range [`min`, `max`].
///
/// Ensures the node's arity falls within the accepted bounds for structural validation.
///
/// # Arguments
/// * `children_count` - The actual number of children the node has.
/// * `min` - The minimum allowed number of children (inclusive).
/// * `max` - The maximum allowed number of children (inclusive).
/// * `parent` - The AST node being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityOutOfRange`] if `children_count` is less than `min` or greater than `max`.
pub fn check_children_count_range(
    children_count: usize,
    min: usize,
    max: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_count < min || children_count > max {
        Err(WellFormedError::children_arity_out_of_range(
            min,
            max,
            children_count,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the content of the `node` matches the expected `ContentKind`.
///
/// Validates that the node's content variant matches the expected kind,
/// ensuring semantic correctness of the AST node.
///
/// # Arguments
/// * `node` - The AST node to validate.
/// * `expected` - The expected content kind for this node.
///
/// # Errors
/// Returns a `WellFormedError::UnexpectedNodeContent` if the node's content does not match the expected kind.
pub fn check_content(node: &AstNode, expected: ContentKind) -> Result<(), WellFormedError> {
    match (node.content(), expected) {
        (AstContent::None, ContentKind::None) => Ok(()),
        (AstContent::Ident(_), ContentKind::Ident) => Ok(()),
        (AstContent::Number(_), ContentKind::Float) => Ok(()),
        (AstContent::Requirement(_), ContentKind::Requirement) => Ok(()),
        (AstContent::CompareOp(_), ContentKind::BinaryComp) => Ok(()),
        (AstContent::AssignOp(_), ContentKind::AssignOp) => Ok(()),
        (AstContent::ArithmeticOp(_), ContentKind::ArithmeticOp) => Ok(()),
        (AstContent::OptimizationOp(_), ContentKind::Optimization) => Ok(()),

        (found, _) => Err(WellFormedError::unexpected_node_content(
            found.clone(),
            node.clone(),
        )),
    }
}

/// Immediately returns an `InvalidNodeKind` error for the given `found` node.
///
/// # Arguments
/// * `found` - The AST node that has an invalid kind.
///
/// # Errors
/// Always returns a `WellFormedError::InvalidNodeKind` containing the given node.
pub fn throw_invalid(found: &AstNode) -> Result<(), WellFormedError> {
    Err(WellFormedError::invalid_node_kind(found.clone()))
}

/// Attempts to retrieve a child node by `node_id` from the `ast` arena.
///
/// # Arguments
/// * `ast` - The AST containing the nodes.
/// * `parent` - The parent node from which we expect the child.
/// * `node_id` - The ID of the child node to retrieve.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the node with `node_id` is not found in the AST.
pub fn get_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    node_id: usize,
) -> Result<&'a AstNode, WellFormedError> {
    match ast.syntax_tree().get_node(NodeId::new(node_id)) {
        Some(node) => Ok(node),
        None => Err(WellFormedError::missing_child_node(
            NodeId::new(node_id),
            parent.clone(),
        )),
    }
}
/// Attempts to retrieve the child node at `child_index` from `parent` node in the `ast` arena.
///
/// # Arguments
/// * `ast` - The AST containing the nodes.
/// * `parent` - The parent node whose child is to be retrieved.
/// * `child_index` - The index of the child node to retrieve.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the child node at `child_index` does not exist.
pub fn get_child_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    child_index: usize,
) -> Result<&'a AstNode, WellFormedError> {
    // Retrieve the child node ID at the given index
    let child_id = parent.children()[child_index].as_usize();
    get_node(ast, parent, child_id)
}

/// Checks that *all* children of `parent` have kinds included in `expected_kinds`.
///
/// This is equivalent to calling [`check_children_kind_from`] with `start_index` set to 0.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the nodes.
/// * `parent` - The parent node whose children will be checked.
/// * `expected_kinds` - A slice of allowed kinds for the children.
///
/// # Errors
/// Returns the first encountered `WellFormedError`, typically
/// a `WellFormedError::UnexpectedChildKind` if any child’s kind
/// is not among `expected_kinds`.
pub fn check_all_children_kind(
    ast: &Ast,
    parent: &AstNode,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    check_children_kind_from(ast, parent, 0, expected_kinds)
}

/// Checks that all children of `parent` starting from `start_index` have kinds included in `expected_kinds`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the nodes.
/// * `parent` - The parent node whose children will be checked.
/// * `start_index` - The index from which to start checking children (inclusive).
/// * `expected_kinds` - A slice of allowed kinds for the children.
///
/// # Errors
/// Returns the first encountered `WellFormedError`, typically
/// a `WellFormedError::UnexpectedChildKind` if any child’s kind
/// is not among `expected_kinds`.
pub fn check_children_kind_from(
    ast: &Ast,
    parent: &AstNode,
    start_index: usize,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    let children = parent.children();

    for i in start_index..children.len() {
        check_child_kind(ast, parent, i, expected_kinds)?;
    }

    Ok(())
}

/// Checks that a symbol node has identifier content and no children.
///
/// # Arguments
/// * `node` - The node to check.
///
pub fn check_symbol(node: &AstNode) -> Result<(), WellFormedError> {
    check_content(node, ContentKind::Ident)?;
    check_children_count(node.arity(), 0, node)?;
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
    check_content(node, ContentKind::Float)?;
    check_children_count(node.arity(), 0, node)?;
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
    check_content(node, ContentKind::Requirement)?;
    check_children_count(node.arity(), 0, node)?;
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
    check_all_children_kind(ast, node, &[AstKind::Requirement])
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
    check_min_children_count(node.arity(), 1, node)?;
    check_all_children_kind(ast, node, &[AstKind::PrimitiveType])
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
    check_min_children_count(node.arity(), 1, node)?;
    let typed_list = get_child_node(ast, node, 0)?;
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
    check_all_children_kind(ast, node, &[AstKind::TypedItem])
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
    check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => check_child_kind(ast, node, 0, &[AstKind::TypedItemElements]),
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::TypedItemElements])?;
            check_child_kind(ast, node, 1, &[AstKind::Type])
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
    check_min_children_count(node.arity(), 1, node)?;
    check_all_children_kind(
        ast,
        node,
        &[
            AstKind::PrimitiveType,
            AstKind::Variable,
            AstKind::Object,
            AstKind::Function,
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TypedList])
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
    check_all_children_kind(ast, node, &[AstKind::AtomicFormulaSkeleton])
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
    check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => check_child_kind(ast, node, 0, &[AstKind::PredicateSymbol]),
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::PredicateSymbol])?;
            check_child_kind(ast, node, 1, &[AstKind::TypedList])
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TypedList])
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
    check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol]),
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
            check_child_kind(ast, node, 1, &[AstKind::TypedList])
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
    check_children_count(children_len, 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::ActionSymbol])?;
    check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    check_child_kind(ast, node, 2, &[AstKind::ActionDefBody])
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
    check_children_count_range(children_len, 0, 2, node)?;

    match children_len {
        0 => Ok(()),
        1 => check_child_kind(
            ast,
            node,
            0,
            &[AstKind::PreconditionDef, AstKind::EffectDef],
        ),
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::PreconditionDef])?;
            check_child_kind(ast, node, 1, &[AstKind::EffectDef])
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
    check_children_count(children_len, 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::MethodSymbol])?;
    check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    check_child_kind(ast, node, 2, &[AstKind::MethodDefBody])
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
    check_children_count(children_len, 1, node)?;
    let typed_list = get_child_node(ast, node, 0)?;
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
    check_children_count_range(children_len, 2, 3, node)?;

    match children_len {
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::Task])?;
            check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
        }
        3 => {
            check_child_kind(ast, node, 0, &[AstKind::Task])?;
            check_child_kind(ast, node, 1, &[AstKind::MethodPreconditionDef])?;
            check_child_kind(ast, node, 2, &[AstKind::TaskNetworkDef])
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
    check_min_children_count(children_len, 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
    check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Object])
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, EXPRESSION)
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, EXPRESSION)
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
    check_min_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
    check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Object])
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
    check_min_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::PredicateSymbol])?;
    check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Object])
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
    check_all_children_kind(ast, node, EXPRESSION)
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, EXPRESSION)
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
    check_min_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, EXPRESSION)?;
    check_child_kind(ast, node, 1, EXPRESSION)
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
    check_min_children_count(children_len, 2, node)?;
    let typed_list = get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::Variable])?;
    check_child_kind(ast, node, 1, EXPRESSION)
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
    check_min_children_count(children_len, 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::PrefName])?;
    check_child_kind(ast, node, 1, EXPRESSION)
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
    check_children_count_range(children_len, 1, 2, node)?;
    match children_len {
        1 => check_child_kind(
            ast,
            node,
            0,
            &[
                AstKind::Number,
                AstKind::Comparison,
                AstKind::Function,
                AstKind::Variable,
                AstKind::Object,
                AstKind::Arithmetic,
            ],
        ),
        2 => {
            check_child_kind(
                ast,
                node,
                0,
                &[
                    AstKind::Number,
                    AstKind::Comparison,
                    AstKind::Function,
                    AstKind::Variable,
                    AstKind::Object,
                    AstKind::Arithmetic,
                ],
            )?;
            check_child_kind(
                ast,
                node,
                1,
                &[
                    AstKind::Number,
                    AstKind::Comparison,
                    AstKind::Function,
                    AstKind::Variable,
                    AstKind::Object,
                    AstKind::Arithmetic,
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
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Function])?;
    check_child_kind(
        ast,
        node,
        1,
        &[
            AstKind::Number,
            AstKind::Variable,
            AstKind::Object,
            AstKind::Function,
            AstKind::Arithmetic,
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
/// or if any child does not have kind `Number` or 'FunctionTerm'.
pub fn check_arithmetic_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    check_min_children_count(node.arity(), 1, node)?;
    for index in 0..node.arity() {
        check_child_kind(
            ast,
            node,
            index,
            &[
                AstKind::Number,
                AstKind::Variable,
                AstKind::Function,
                AstKind::Arithmetic,
                AstKind::IsViolated,
            ],
        )?;
    }
    Ok(())
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
pub fn check_within_hold_after_expression(
    ast: &Ast,
    node: &AstNode,
) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    check_children_count(children_len, 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Number])?;
    check_child_kind(ast, node, 1, EXPRESSION)
}

/// Checks that an `AlwaysWithin` node has exactly three children:
/// the first must be a `Number`, the second and third must be logic.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the children do not match the expected kinds.
pub fn check_always_within_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    check_children_count(children_len, 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Number])?;
    check_child_kind(ast, node, 1, EXPRESSION)?;
    check_child_kind(ast, node, 2, EXPRESSION)
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
    check_children_count(children_len, 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Number])?;
    check_child_kind(ast, node, 1, &[AstKind::Number])?;
    check_child_kind(ast, node, 2, EXPRESSION)
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::And])?;

    let init_elements = get_child_node(ast, node, 0)?;

    check_all_children_kind(
        ast,
        init_elements,
        &[
            AstKind::TimedInitialLiteral,
            AstKind::Comparison,
            AstKind::AtomicFormula,
            AstKind::Not,
        ],
    )?;

    // TODO: Add check that `Not` nodes contain only atomic formula

    Ok(())
}

/// Checks that a `TimedInitialLiteral` node is well-formed according to PDDL/HDDL standards.
///
/// A `TimedInitialLiteral` must have exactly two children:
/// 1. The first child must be a `Number` (representing the time point).
/// 2. The second child must be one of: `Comparison`, `Not`, or `AtomicFormula`.
///
/// If the second child is a `Not` node, this function further validates that it
/// contains exactly one child, which must be an `AtomicFormula`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The `TimedInitialLiteral` node to validate.
///
/// # Errors
/// Returns a `WellFormedError` if:
/// * The children count of the literal is not exactly two.
/// * The child kinds do not match the requirements.
/// * A negated literal (`Not`) contains something other than a single atomic formula.
/// Checks that a `TimedInitialLiteral` node is well-formed according to PDDL/HDDL standards.
///
/// A `TimedInitialLiteral` must have exactly two children:
/// 1. The first child must be a `Number` (representing the time point).
/// 2. The second child must be one of: `Comparison`, `Not`, or `AtomicFormula`.
///
/// If the second child is a `Not` node, this function further validates that it
/// contains exactly one child, which must be an `AtomicFormula`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The `TimedInitialLiteral` node to validate.
///
/// # Errors
/// Returns a `WellFormedError` if:
/// * The children count of the literal is not exactly two.
/// * The child kinds do not match the requirements.
/// * A negated literal (`Not`) contains something other than a single atomic formula.
pub fn check_timed_initial_literal(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    // 1. Basic structure check: (at <time> <literal>)
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Number])?;
    check_child_kind(
        ast,
        node,
        1,
        &[AstKind::Comparison, AstKind::Not, AstKind::AtomicFormula],
    )?;

    // 2. Verification of the (not <atomic_formula>)
    // Using node.children()[1] since the arity check passed.
    let child_id = node.children()[1];

    // Check your Ast implementation, it might be ast.tree.get_node or ast.syntax_tree().get_node
    // Here I use the method that your Trace log suggested was available:
    let child_node = ast.syntax_tree().try_node(child_id)?;

    if child_node.kind() == AstKind::Not {
        check_children_count(child_node.arity(), 1, child_node)?;
        check_child_kind(ast, child_node, 0, &[AstKind::AtomicFormula])?;
    }

    Ok(())
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
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::AtomicFormulaSkeleton])?;
    check_child_kind(ast, node, 1, EXPRESSION)
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::And])?;
    let tasks = get_child_node(ast, node, 0)?;
    check_all_children_kind(ast, tasks, &[AstKind::LabeledTask, AstKind::Task])
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
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TaskLabel])?;
    check_child_kind(ast, node, 1, &[AstKind::Task])
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::And])?;
    let ordering = get_child_node(ast, node, 0)?;
    check_all_children_kind(ast, ordering, &[AstKind::TaskOrderingConstraint])
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
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TaskLabel])?;
    check_child_kind(ast, node, 1, &[AstKind::TaskLabel])
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
    check_children_count_range(children_len, 0, 3, node)?;
    match children_len {
        0 => Ok(()),
        1 => check_child_kind(
            ast,
            node,
            0,
            &[
                AstKind::OrderedSubtaskDef,
                AstKind::PartiallyOrderedSubtaskDef,
                AstKind::TaskLogicalConstraintDef,
            ],
        ),
        2 => {
            check_child_kind(
                ast,
                node,
                0,
                &[
                    AstKind::OrderedSubtaskDef,
                    AstKind::PartiallyOrderedSubtaskDef,
                ],
            )?;
            check_child_kind(
                ast,
                node,
                1,
                &[
                    AstKind::TaskOrderingConstraintDef,
                    AstKind::TaskLogicalConstraintDef,
                ],
            )
        }
        3 => {
            check_child_kind(
                ast,
                node,
                0,
                &[
                    AstKind::OrderedSubtaskDef,
                    AstKind::PartiallyOrderedSubtaskDef,
                ],
            )?;
            check_child_kind(ast, node, 1, &[AstKind::TaskOrderingConstraintDef])?;
            check_child_kind(ast, node, 2, &[AstKind::TaskLogicalConstraintDef])
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
    check_children_count(node.arity(), 2, node)?;
    check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
    check_child_kind(ast, node, 1, &[AstKind::ParametersDef])
}

/// Checks that a `TotalTime` node has no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has any children.
pub fn check_total_time(node: &AstNode) -> Result<(), WellFormedError> {
    check_children_count(node.arity(), 0, node)
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::PrefName])
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
    check_children_count_range(node.arity(), 0, 2, node)?;
    check_all_children_kind(ast, node, &[AstKind::Serial, AstKind::Parallel])
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
    check_children_count(node.arity(), 1, node)?;
    check_child_kind(ast, node, 0, &[AstKind::Number])
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
    check_children_count(node.arity(), 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::DASymbol])?;
    check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
    check_child_kind(ast, node, 2, &[AstKind::DADefBody])
}

/// Checks that a `DADefBody` node has exactly three children,
/// all of which must be valid logic.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the number of children is not three
/// or if any child is not a valid expression.
pub fn check_duartive_action_def_body(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    check_children_count(node.arity(), 3, node)?;
    check_child_kind(ast, node, 0, &[AstKind::DurationConstraint])?;
    check_child_kind(ast, node, 1, EXPRESSION)?;
    check_child_kind(ast, node, 2, EXPRESSION)
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
    check_children_count_range(node.arity(), 1, 2, node)?;
    match node.arity() {
        1 => check_child_kind(ast, node, 0, &[AstKind::TaskNetworkDef]),
        2 => {
            check_child_kind(ast, node, 0, &[AstKind::ParametersDef])?;
            check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
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
            check_min_children_count(node.arity(), 1, node)?;
            // All children must be of the expected kinds passed in the `expected` slice
            check_all_children_kind(ast, node, expected)?;
            // Return early because TypedItemElements are leaf nodes for this validation
            // No need to recurse further down this branch
            return Ok(());
        }
        AstKind::Type => {
            // Type nodes are also leaf nodes: validate and stop recursion here
            check_type(ast, node)?;
            return Ok(());
        }
        _ => {
            // If the node kind is unexpected here, return an error indicating invalid node kind
            return Err(WellFormedError::InvalidNodeKind {
                found: node.clone(),
            }
            .into());
        }
    }

    // For TypedList and TypedItem nodes, recursively check all children
    for child_id in children_ids {
        let child_node = get_node(ast, node, child_id.as_usize())?;
        // Recursive call with the same expected kinds
        check_typed_list_of(ast, child_node, expected)?;
    }

    Ok(())
}
