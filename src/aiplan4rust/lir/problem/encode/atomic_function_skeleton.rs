//! Numeric Function Encoding
//!
//! This module handles the transformation of PDDL function declarations
//! (numeric fluents) into the LIR. It maps the function's signature and
//! its return type to the internal representation.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::atomic_skeleton::function::Function;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{named_typed_list, ty, EncodingRegistry};

/// Encodes a PDDL numeric function (fluent) from the syntax tree into the LIR.
///
/// This function expects a specific AST structure where the function signature
/// and the return type are children of the current node.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the function declaration.
///
/// # Returns
///
/// * `Ok(Function)` - The encoded function skeleton with its return type.
/// * `Err(LirError)` - If the signature, the parameter list, or the return type is invalid.
///
/// # Errors
///
/// This function returns an error if:
/// * The first two children (name and parameters) cannot be parsed as a `NamedTypedList`.
/// * The third child (index 2) is missing or cannot be parsed as a valid `Type`.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<Function, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Encode the signature (name + parameters)
    // This handles Child 0 (Ident) and Child 1 (TypedList)
    let header = named_typed_list::encode(subtree, registry)?;

    // 2. Extract and encode the return type (Child 2)
    let ty_id = node.try_child(2)?;
    let ty_node = ast.try_node(ty_id)?;
    let return_type = ty::encode(&SyntaxSubtree::new(ty_node, ty_id, ast), registry)?;

    // 3. Construct the Function using the internal from_header constructor
    Ok(Function::from_header(header, return_type))
}
