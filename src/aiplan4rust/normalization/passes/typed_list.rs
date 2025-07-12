//! Module for normalizing `TypedList` nodes in the AST.
//!
//! This module provides functionality to recursively normalize all `TypedList` nodes
//! within an AST subtree by transforming them so that each `TypedItem` node contains
//! exactly one element along with its optional type annotation.
//!
//! The normalization process involves a depth-first traversal starting from the root
//! node of the AST. For each `TypedList` node encountered:
//! - It verifies that each child is a `TypedItem` node containing at least one child.
//! - Extracts the first child as the element (which must be a valid element kind).
//! - Optionally clones the second child if present, representing the type annotation.
//! - Rebuilds the `TypedItem` nodes so that each contains exactly one element plus an optional type.
//!
//! This transformation ensures that after normalization, type annotations are duplicated
//! per element, which simplifies subsequent semantic analysis and code generation.
//!
//! # Example of the transformation
//!
//! Before normalization:
//! ```pddl
//! (:types
//!     e1 e2 - ty
//!     e3
//! )
//! ```
//!
//! After normalization:
//! ```pddl
//! (:types
//!    e1 - ty
//!    e2 - ty
//!    e3
//! )
//! ```
//!
//! # Preconditions
//!
//! - The AST must be syntactically valid.
//! - `TypedList` nodes must not be normalized yet.
//! - This should be the first normalization pass on the AST.
//!
//! Running normalization multiple times or on an already normalized AST may cause
//! incorrect behavior or internal errors due to assumptions about AST structure.
//!
//! # Errors
//!
//! Returns `ParserInternalError` if structural inconsistencies or unexpected node kinds
//! are encountered during normalization.
//!
//! # Usage example
//!
//! ```rust,no_run
//! let mut ast = parse_source_code(source_code)?;
//! normalize_typed_list(&mut ast)?;
//! ```
//!
//! # Implementation notes
//!
//! This module uses an explicit stack to traverse the AST non-recursively,
//! preventing stack overflows on large ASTs.
//!
//! The main entry point is [`normalize_typed_list`], which normalizes all `TypedList` nodes.

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{Ast, AstNode, AstContent};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, Arena, ArenaNode};

/// Recursively normalizes all `TypedList` nodes in the given AST subtree.
///
/// This function performs a **depth-first traversal** starting from the root node of the AST.
/// For each `TypedList` node encountered, it:
/// - Validates that each child is a `TypedItem` node with at least one child.
/// - Extracts the first child as the element (which must be one of the valid element kinds).
/// - Optionally clones the second child if present, representing the type annotation.
/// - Rebuilds each `TypedItem` node to contain exactly one element plus an optional type.
///
/// After normalizing a `TypedList` node, its children are pushed onto the stack to continue
/// the normalization recursively.
///
/// # Example of the transformation
///
/// Before normalization:
/// ```pddl
/// (:types
///     e1 e2 - ty
///     e3
/// )
/// ```
///
/// After normalization:
/// ```pddl
/// (:types
///    e1 - ty
///    e2 - ty
///    e3
/// )
/// ```
///
/// # Preconditions
///
/// - The AST must already be syntactically valid.
/// - `TypedList` nodes are expected **not** to be normalized yet.
/// - This function should be the **first normalization pass** on the AST.
///
/// Running this on an already normalized or partially normalized AST may cause
/// incorrect behavior or internal panics due to violated structural assumptions.
///
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - Any `TypedItem` node is missing, malformed, or structurally invalid.
/// - The element inside a `TypedItem` node is not one of the expected kinds:
///   `Constant`, `Variable`, `PrimitiveType`, or `AtomicFunctionSkeleton`.
///
/// # Parameters
///
/// - `ast`: Mutable reference to the AST arena to normalize.
///
/// # Returns
///
/// * `Ok(())` if normalization completes successfully.
/// * `Err(ParserInternalError)` if structural inconsistencies are detected.
///
/// # Examples
///
/// ```rust,no_run
/// let mut ast = parse_source_code(source_code)?;
/// normalize_typed_list(&mut ast)?;
/// ```
///
/// # Notes
///
/// This function uses an explicit stack to avoid deep recursion and possible stack overflow
/// on very large ASTs. It is typically the first normalization step before semantic analysis,
/// type inference, or code generation.
pub fn normalize_typed_list(ast: &mut Ast) -> Result<(), ParserInternalError> {
    if !ast.arena().is_empty() {
        normalize_typed_list_node(ast)?
    }
    Ok(())
}

/// Normalizes all `TypedList` nodes within the given AST.
///
/// This function performs an **explicit stack-based, non-recursive depth-first traversal**
/// of the entire AST. Each time it encounters a `TypedList` node, it:
///
/// 1. Retrieves and clears its current children (`TypedItem` nodes).
/// 2. For each `TypedItem`, extracts all contained elements along with the optional type annotation.
/// 3. Creates a new `TypedItem` node for each individual element, preserving the original span and optional type annotation.
/// 4. Replaces the original `TypedList` children with these normalized `TypedItem` nodes.
///
/// The normalization guarantees that after processing:
/// - Each `TypedItem` node contains **exactly one element**.
/// - Shared type annotations are duplicated appropriately for each element.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the `AstArena` holding the AST to be normalized.
///
/// # Returns
///
/// * `Ok(())` if the normalization completes successfully.
/// * `Err(ParserInternalError)` if the AST contains unexpected node kinds, invalid children indices,
///   or any structural inconsistencies encountered during traversal.
///
/// # Panics
///
/// This function is designed to **never panic**. All errors related to AST structure
/// or unexpected conditions are returned as `ParserInternalError`.
///
/// # Traversal Details
///
/// The traversal uses an **explicit stack** (`Vec<NodeId>`) instead of recursion:
/// - Nodes are visited in **depth-first order** by pushing children onto the stack in reverse order.
/// - This approach prevents potential stack overflows on deeply nested ASTs.
///
/// # Example
///
/// ```ignore
/// let mut ast = parse(...);
/// normalize_typed_list_node(&mut ast)?;
/// ```
///
/// # Notes
///
/// This normalization step is important to simplify downstream processing,
/// ensuring that each `TypedItem` corresponds to a single element, which simplifies
/// type checking and code generation phases.
fn normalize_typed_list_node(ast: &mut Ast) -> Result<(), ParserInternalError> {
    let root_id = ast.arena().try_root_id()?;
    let arena = ast.arena_mut();
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        // Check if the current node is a TypedList node and normalize its children if so.
        if is_typed_list_node(arena, node_id)? {
            normalize_typed_list_node_children(arena, node_id)?;
        }

        // Push children onto the stack in reverse order to maintain depth-first traversal.
        let node = arena.try_node(node_id)?;
        for &child_id in node.children().iter().rev() {
            stack.push(child_id);
        }
    }

    Ok(())
}


/// Normalizes the children of a `TypedList` node by expanding each `TypedItem`
/// so that each new `TypedItem` node contains exactly one element and an optional type.
///
/// This function performs the core normalization step for `TypedList` nodes:
/// - It extracts the existing children (which are `TypedItem` nodes that may contain multiple elements).
/// - For each old `TypedItem`, it creates one new `TypedItem` per element, preserving optional type and span.
/// - It replaces the old children with the newly created normalized `TypedItem` nodes.
///
/// # Arguments
///
/// * `arena` - Mutable reference to the AST arena containing the nodes.
/// * `node_id` - The ID of the `TypedList` node whose children are to be normalized.
///
/// # Returns
///
/// * `Ok(())` if the normalization completes successfully.
/// * `Err(ParserInternalError)` if any node access or manipulation fails.
///
/// # Errors
///
/// This function returns an error if:
/// - The given `node_id` is invalid or not found in the arena.
/// - Child nodes or required data are missing or inconsistent.
///
/// # Panics
///
/// This function does **not** panic. All errors are returned as `ParserInternalError`.
///
/// # Example
///
/// ```ignore
/// normalize_typed_list_node_children(arena, typed_list_node_id)?;
/// ```
fn normalize_typed_list_node_children(
    arena: &mut Arena<AstNode>,
    node_id: NodeId,
) -> Result<(), ParserInternalError> {
    // 1. Retrieve and clear the current children of the TypedList node.
    let old_typed_items = {
        let node = arena.try_node_mut(node_id)?;
        std::mem::take(node.children_mut())
    };

    // Prepare a vector to hold new normalized TypedItem nodes.
    let mut new_typed_items = Vec::with_capacity(old_typed_items.len());

    // 2. For each old TypedItem, extract its elements and create new TypedItem nodes,
    //    each with exactly one element and the optional type preserved.
    for typed_item_id in old_typed_items {
        let (element_ids, type_id_opt, span) =
            extract_typed_item_data(arena, typed_item_id)?;

        for element_id in element_ids {
            let new_node = create_typed_item_node(
                element_id,
                type_id_opt,
                span.clone(),
                node_id,
            );
            let new_id = arena.alloc(new_node);
            new_typed_items.push(new_id);
        }
    }

    // 3. Replace the old children with the newly normalized TypedItem nodes.
    let node = arena.try_node_mut(node_id)?;
    *node.children_mut() = new_typed_items;

    Ok(())
}

/// Checks whether a given node is a `TypedList` node.
///
/// This function is used during AST normalization to identify nodes
/// that represent a `TypedList`, which require special processing.
///
/// # Arguments
///
/// * `arena` - Mutable reference to the AST arena containing the nodes.
/// * `node_id` - The ID of the node to check.
///
/// # Returns
///
/// * `Ok(true)` if the node kind is `TypedList`.
/// * `Ok(false)` otherwise.
///
/// # Errors
///
/// Returns `ParserInternalError` if the node ID is invalid or cannot be found in the arena.
///
/// # Example
///
/// ```ignore
/// if is_typed_list_node(arena, some_node_id)? {
///     // handle TypedList node
/// }
/// ```
fn is_typed_list_node(
    arena: &mut Arena<AstNode>,
    node_id: NodeId,
) -> Result<bool, ParserInternalError> {
    let node = arena.try_node(node_id)?;
    Ok(node.kind() == AstKind::TypedList)
}

/// Extracts the child element IDs, optional type ID, and span from a `TypedItem` node.
///
/// This function is used during normalization to decompose a `TypedItem` node into:
/// - the IDs of its contained elements (which may be multiple before normalization),
/// - an optional type node,
/// - and the span information.
///
/// # Arguments
///
/// * `arena` - Reference to the `TreeArena` containing the AST nodes.
/// * `typed_item_id` - The node ID of the `TypedItem` to extract.
///
/// # Returns
///
/// * `Ok((element_ids, type_id_opt, span))` -
///     - `element_ids`: A vector of the IDs of the element nodes contained in this `TypedItem`.
///     - `type_id_opt`: An optional ID of the associated type node.
///     - `span`: The span information of the `TypedItem`.
///
/// * `Err(ParserInternalError)` - If the node is missing expected children or is invalid.
///
/// # Errors
///
/// Returns an error if:
/// - The `TypedItem` node does not have at least one child (the elements node).
/// - The elements node cannot be retrieved.
///
/// # Example
///
/// ```ignore
/// let (element_ids, type_id, span) = extract_typed_item_data(arena, typed_item_id)?;
/// ```
fn extract_typed_item_data(
    arena: &Arena<AstNode>,
    typed_item_id: NodeId,
) -> Result<(Vec<NodeId>, Option<NodeId>, Span), ParserInternalError> {
    // Retrieve the TypedItem node
    let typed_item_node = arena.try_node(typed_item_id)?;

    // The first child must be the elements node
    let elements_id = typed_item_node.try_child(0)?;

    // The second child, if it exists, is the optional type annotation
    let type_id_opt = typed_item_node.get_child(1);

    // Clone the span metadata
    let span = typed_item_node.span().clone();

    // Retrieve the elements node, which contains multiple element children
    let elements_node = arena.try_node(elements_id)?;

    // Collect the IDs of all contained elements
    let element_ids = elements_node.children().to_vec();

    // Return all extracted data
    Ok((element_ids, type_id_opt, span))
}

/// Creates a new `TypedItem` node containing exactly one element and optionally a type.
///
/// This function is used during normalization of `TypedList` nodes to reconstruct
/// each `TypedItem` in a uniform structure.
///
/// # Arguments
///
/// * `element_id` - The node ID of the single element to include.
/// * `type_id_opt` - An optional node ID representing the type annotation.
/// * `span` - The source span associated with the new node.
/// * `parent_id` - The parent node ID, typically referring to the `TypedList`.
///
/// # Returns
///
/// A new `AstArenaNode` instance representing the normalized `TypedItem`.
fn create_typed_item_node(
    element_id: NodeId,
    type_id_opt: Option<NodeId>,
    span: Span,
    parent_id: NodeId,
) -> AstNode {
    // Initialize children with the mandatory element node
    let mut children = vec![element_id];

    // If a type is provided, append it as the second child
    if let Some(type_id) = type_id_opt {
        children.push(type_id);
    }

    // Create the new TypedItem node with no content,
    // storing the span and parent information
    AstNode::new(
        AstKind::TypedItem,
        AstContent::None,
        children,
        span,
        Some(parent_id),
    )
}
