//! This module provides functions to validate that an Abstract Syntax Tree (AST)
//! is *well normalized*. Well logic means the AST nodes conform to
//! additional structural and semantic rules beyond basic well-formedness.
//!
//! It performs recursive checks starting from the root node, traversing all descendants,
//! and delegates node-specific validations to appropriate submodules (`logic::checks`
//! or `syntax::checks`).
//!
//! # Overview of Provided Functions
//!
//! - [`is_well_normalized`]: Entry point to check if the entire AST is well normalized.
//! - [`check_well_normalized`]: Alias of `is_well_normalized`, provided for naming consistency.
//! - [`check_well_normalized_from`]: Recursively validates a subtree starting at a specified node.
//! - [`check_well_normalized_node`]: Validates a single node by dispatching to logic or syntax checks.
//!
//! # Error Handling
//!
//! Returns [`WellNormalizedError`] if any node violates logic criteria.
//!
//! # Usage
//!
//! Typically, call [`is_well_normalized`] on a fully constructed AST to verify logic
//! before further processing or compilation.
//!
//! [`is_well_normalized`]: fn.is_well_normalized.html
//! [`check_well_normalized`]: fn.check_well_normalized.html
//! [`check_well_normalized_from`]: fn.check_well_normalized_from.html
//! [`check_well_normalized_node`]: fn.check_well_normalized_node.html
//! [`WellNormalizedError`]: ../common/struct.WellNormalizedError.html

use crate::aiplan4rust::normalization::validation::checks::{
    check_parameters_def, check_quantified_expression, check_typed_item, check_types_def,
};
use crate::aiplan4rust::normalization::validation::WellNormalizedError;
use crate::aiplan4rust::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::syntax::validation;
use crate::aiplan4rust::syntax::validation::checks;

/// Checks if the entire AST is well normalized starting from the root node.
///
/// This function verifies that:
/// 1. The AST forms a valid tree (no cycles) using `ArenaTree::is_tree()`.
/// 2. The subtree satisfies logic rules defined in `check_well_normalized_from`.
///
/// # Arguments
/// * `ast` - The AST to validate.
///
/// # Returns
/// * `Ok(())` if the AST is well normalized.
/// * `Err(WellNormalizedError)` if the AST contains cycles or violates logic rules.
pub fn is_well_normalized(ast: &Ast) -> bool {
    check_well_normalized(ast).is_ok()
}

/// Checks that the AST is well normalized starting from its root node.
///
/// This function verifies that the AST forms a valid tree (no cycles)
/// and that the root node is a valid PDDL entry point (Domain or Problem).
/// It then recursively validates that all nodes satisfy normalization logic rules.
///
/// # Arguments
///
/// * `ast` - The AST to validate.
///
/// # Returns
///
/// * `Ok(())` if the AST is well normalized and contains a valid root.
/// * `Err(WellNormalizedError)` if the AST is empty, cyclic, or violates logic rules.
///
/// # Note
///
/// Unlike basic parsing, a normalized AST must have a root node to be considered valid
/// for the rest of the compilation pipeline.
pub fn check_well_normalized(ast: &Ast) -> Result<(), WellNormalizedError> {
    let arena = ast.syntax_tree();

    // 1. Basic structural integrity
    if !arena.is_tree() {
        return Err(WellNormalizedError::cycle_detected());
    }

    // 2. Root existence check
    // We no longer accept an empty AST as "well-normalized".
    let root = arena
        .root_node()
        .ok_or_else(WellNormalizedError::missing_root)?;

    // 3. Root Kind validation
    let root_kind = root.kind();
    if root_kind != AstKind::Domain && root_kind != AstKind::Problem {
        return Err(WellNormalizedError::invalid_root(root_kind));
    }

    // 4. Recursive logic validation
    check_well_normalized_from(root, ast)
}

/// Recursively checks that the subtree rooted at `node` is well normalized.
///
/// # Arguments
///
/// * `node` - The root node of the subtree to validate.
/// * `ast` - The full AST containing the node.
///
/// # Returns
///
/// * `Ok(())` if the subtree is well normalized.
/// * `Err(WellNormalizedError)` if any node violates logic rules.
pub fn check_well_normalized_from(node: &AstNode, ast: &Ast) -> Result<(), WellNormalizedError> {
    check_well_normalized_node(node, ast)?;
    for child_id in node.children() {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_well_normalized_from(child_node, ast)?;
    }
    Ok(())
}

/// Validates that the given AST node conforms to the logic rules.
///
/// This function dispatches to logic-specific checks for certain node kinds,
/// delegates to syntax checks for quantifier logic,
/// invalidates explicitly forbidden node kinds,
/// and falls back to standard syntax validation for all other nodes.
///
/// # Arguments
/// * `node` - The AST node to validate.
/// * `ast` - The whole AST context.
///
/// # Returns
/// * `Ok(())` if the node logic logic checks.
/// * `Err(WellNormalizedError)` if any check fails.
pub fn check_well_normalized_node(node: &AstNode, ast: &Ast) -> Result<(), WellNormalizedError> {
    match node.kind() {
        AstKind::TypesDef => check_types_def(ast, node),
        AstKind::TypedItem => check_typed_item(ast, node),
        AstKind::ParametersDef => check_parameters_def(ast, node),
        AstKind::Forall | AstKind::Exists => check_quantified_expression(ast, node),
        AstKind::TypedItemElements => validation::checks::throw_invalid(node),
        _ => validation::validator::check_well_formed_node(node, ast),
    }
}
