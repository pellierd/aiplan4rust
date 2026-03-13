//! Module for normalizing `TypedList` nodes in the AST.
//!
//! This module provides functionality to recursively normalize all `TypedList` nodes
//! within an AST subtree by transforming them so that each `TypedItem` syntax contains
//! exactly one element along with its optional type_checker annotation.
//!
//! The expr process involves a depth-first traversal starting from the root
//! syntax of the AST. For each `TypedList` syntax encountered:
//! - It verifies that each child is a `TypedItem` syntax containing at least one child.
//! - Extracts the first child as the element (which must be a valid element kind).
//! - Optionally clones the second child if present, representing the type_checker annotation.
//! - Rebuilds the `TypedItem` nodes so that each contains exactly one element plus an optional type_checker.
//!
//! This transformation ensures that after expr, type_checker annotations are duplicated
//! per element, which simplifies subsequent semantic analysis and code generation.
//!
//! # Example of the transformation
//!
//! Before expr:
//! ```pddl
//! (:types
//!     e1 e2 - types
//!     e3
//! )
//! ```
//!
//! After expr:
//! ```pddl
//! (:types
//!    e1 - types
//!    e2 - types
//!    e3
//! )
//! ```
//!
//! # Preconditions
//!
//! - The AST must be syntactically valid.
//! - `TypedList` nodes must not be normalized yet.
//! - This should be the first expr pass on the AST.
//!
//! Running expr multiple times or on an already normalized AST may cause
//! incorrect behavior or internal errors due to assumptions about AST structure.
//!
//! # Errors
//!
//! Returns `ParserInternalError` if structural inconsistencies or unexpected syntax kinds
//! are encountered during expr.
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

use crate::aiplan4rust::syntax::ast::{Ast, AstNode, AstContent};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::normalization::passes::NormalizationPassError;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::tree::Tree;

/// Recursively normalizes all `TypedList` nodes in the given AST subtree.
///
/// This function performs a **depth-first traversal** starting from the root syntax of the AST.
/// For each `TypedList` syntax encountered, it:
/// - Validates that each child is a `TypedItem` syntax with at least one child.
/// - Extracts the first child as the element (which must be one of the valid element kinds).
/// - Optionally clones the second child if present, representing the type_checker annotation.
/// - Rebuilds each `TypedItem` syntax to contain exactly one element plus an optional type_checker.
///
/// After normalizing a `TypedList` syntax, its children are pushed onto the stack to continue
/// the expr recursively.
///
/// # Example of the transformation
///
/// Before expr:
/// ```pddl
/// (:types
///     e1 e2 - types
///     e3
/// )
/// ```
///
/// After expr:
/// ```pddl
/// (:types
///    e1 - types
///    e2 - types
///    e3
/// )
/// ```
///
/// # Preconditions
///
/// - The AST must already be syntactically valid.
/// - `TypedList` nodes are expected **not** to be normalized yet.
/// - This function should be the **first expr pass** on the AST.
///
/// Running this on an already normalized or partially normalized AST may cause
/// incorrect behavior or internal panics due to violated structural assumptions.
///
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - Any `TypedItem` syntax is missing, malformed, or structurally invalid.
/// - The element inside a `TypedItem` syntax is not one of the expected kinds:
///   `Constant`, `Variable`, `PrimitiveType`, or `AtomicFunctionSkeleton`.
///
/// # Parameters
///
/// - `ast`: Mutable reference to the AST arena to normalize.
///
/// # Returns
///
/// * `Ok(())` if expr completes successfully.
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
/// on very large ASTs. It is typically the first expr step before semantic analysis,
/// type_checker inference, or code generation.
pub fn normalize_typed_list(ast: &mut Ast) -> Result<(), NormalizationPassError> {
    if !ast.syntax_tree().is_empty() {
        normalize_typed_list_node(ast)?
    }
    Ok(())
}

/// Normalizes all `TypedList` nodes within the given AST.
///
/// This function performs an **explicit stack-based, non-recursive depth-first traversal**
/// of the entire AST. Each time it encounters a `TypedList` syntax, it:
///
/// 1. Retrieves and clears its current children (`TypedItem` nodes).
/// 2. For each `TypedItem`, extracts all contained elements along with the optional type_checker annotation.
/// 3. Creates a new `TypedItem` syntax for each individual element, preserving the original span
///   and optional type_checker annotation.
/// 4. Replaces the original `TypedList` children with these normalized `TypedItem` nodes.
///
/// The expr guarantees that after processing:
/// - Each `TypedItem` syntax contains **exactly one element**.
/// - Shared type_checker annotations are duplicated appropriately for each element.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the `AstArena` holding the AST to be normalized.
///
/// # Returns
///
/// * `Ok(())` if the expr completes successfully.
/// * `Err(NormalizationPassError)` if the AST contains unexpected syntax kinds, invalid children
///   indices, or any structural inconsistencies encountered during traversal.
///
/// # Panics
///
/// This function is designed to **never panic**. All errors related to AST structure
/// or unexpected conditions are returned as `NormalizationPassError`.
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
/// This expr step is important to simplification downstream processing,
/// ensuring that each `TypedItem` corresponds to a single element, which simplifies
/// type_checker checking and code generation phases.
fn normalize_typed_list_node(ast: &mut Ast) -> Result<(), NormalizationPassError> {
    let root_id = ast.syntax_tree().try_root_id()?;
    let syntax_tree = ast.syntax_tree_mut();
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        // Check if the current syntax is a TypedList syntax and normalize its children if so.
        if is_typed_list_node(syntax_tree, node_id)? {
            normalize_typed_list_node_children(syntax_tree, node_id)?;
        }

        // Push children onto the stack in reverse order to maintain depth-first traversal.
        let node = syntax_tree.try_node(node_id)?;
        for &child_id in node.children().iter().rev() {
            stack.push(child_id);
        }
    }

    Ok(())
}


/// Normalizes the children of a `TypedList` syntax by expanding each `TypedItem`
/// so that each new `TypedItem` syntax contains exactly one element and an optional type_checker.
///
/// This function performs the tree expr step for `TypedList` nodes:
/// - It extracts the existing children (which are `TypedItem` nodes that may contain multiple
///   elements).
/// - For each old `TypedItem`, it creates one new `TypedItem` per element, preserving optional type_checker
///   and span.
/// - It replaces the old children with the newly created normalized `TypedItem` nodes.
///
/// # Arguments
///
/// * `syntax_tree` - Mutable reference to the syntax tree containing the nodes.
/// * `node_id` - The ID of the `TypedList` syntax whose children are to be normalized.
///
/// # Returns
///
/// * `Ok(())` if the expr completes successfully.
/// * `Err(NormalizationPassError)` if any syntax access or manipulation fails.
///
/// # Errors
///
/// This function returns an error if:
/// - The given `node_id` is invalid or not found in the syntax tree.
/// - Child nodes or required data are missing or inconsistent.
///
/// # Panics
///
/// This function does **not** panic. All errors are returned as `NormalizationPassError`.
///
/// # Example
///
/// ```ignore
/// normalize_typed_list_node_children(syntax_tree, typed_list_node_id)?;
/// ```
fn normalize_typed_list_node_children(
    syntax_tree: &mut Tree<AstNode>,
    node_id: NodeId,
) -> Result<(), NormalizationPassError> {
    // 1. Retrieve and clear the current children of the TypedList syntax.
    let old_typed_items = {
        let node = syntax_tree.try_node_mut(node_id)?;
        std::mem::take(node.children_mut())
    };

    // Prepare a vector to hold new normalized TypedItem nodes.
    let mut new_typed_items = Vec::with_capacity(old_typed_items.len());

    // 2. For each old TypedItem, extract its elements and create new TypedItem nodes,
    //    each with exactly one element and the optional type_checker preserved.
    for typed_item_id in old_typed_items {
        let (element_ids, type_id_opt, span) =
            extract_typed_item_data(syntax_tree, typed_item_id)?;

        for element_id in element_ids {

            let new_node = create_typed_item_node(
                element_id,
                type_id_opt,
                span.clone(),
                node_id,
                syntax_tree,
            )?;
            let new_id = syntax_tree.alloc(new_node);
            new_typed_items.push(new_id);
        }
    }

    // 3. Replace the old children with the newly normalized TypedItem nodes.
    let node = syntax_tree.try_node_mut(node_id)?;
    *node.children_mut() = new_typed_items;

    Ok(())
}

/// Checks whether a given syntax is a `TypedList` syntax.
///
/// This function is used during AST expr to identify nodes
/// that represent a `TypedList`, which require special processing.
///
/// # Arguments
///
/// * `syntax_tree` - Mutable reference to the syntax tree containing the nodes.
/// * `node_id` - The ID of the syntax to check.
///
/// # Returns
///
/// * `Ok(true)` if the syntax kind is `TypedList`.
/// * `Ok(false)` otherwise.
///
/// # Errors
///
/// Returns `NormalizationError` if the syntax ID is invalid or cannot be found in the syntax tree.
///
/// # Example
///
/// ```ignore
/// if is_typed_list_node(syntax_tree, some_node_id)? {
///     // handle TypedList syntax
/// }
/// ```
fn is_typed_list_node(
    syntax_tree: &mut Tree<AstNode>,
    node_id: NodeId,
) -> Result<bool, NormalizationPassError> {
    let node = syntax_tree.try_node(node_id)?;
    Ok(node.kind() == AstKind::TypedList)
}

/// Extracts the child element IDs, optional type_checker ID, and span from a `TypedItem` syntax.
///
/// This function is used during expr to decompose a `TypedItem` syntax into:
/// - the IDs of its contained elements (which may be multiple before expr),
/// - an optional type_checker syntax,
/// - and the span information.
///
/// # Arguments
///
/// * `syntax_tree` - Reference to the syntax tree containing the AST nodes.
/// * `typed_item_id` - The syntax ID of the `TypedItem` to extract.
///
/// # Returns
///
/// * `Ok((element_ids, type_id_opt, span))` -
///     - `element_ids`: A vector of the IDs of the element nodes contained in this `TypedItem`.
///     - `type_id_opt`: An optional ID of the associated type_checker syntax.
///     - `span`: The span information of the `TypedItem`.
///
/// * `Err(NormalizationPassError)` - If the syntax is missing expected children or is invalid.
///
/// # Errors
///
/// Returns an error if:
/// - The `TypedItem` syntax does not have at least one child (the elements syntax).
/// - The elements syntax cannot be retrieved.
///
/// # Example
///
/// ```ignore
/// let (element_ids, type_id, span) = extract_typed_item_data(syntax_tree, typed_item_id)?;
/// ```
fn extract_typed_item_data(
    syntax_tree: &Tree<AstNode>,
    typed_item_id: NodeId,
) -> Result<(Vec<NodeId>, Option<NodeId>, Span), NormalizationPassError> {
    // Retrieve the TypedItem syntax
    let typed_item_node = syntax_tree.try_node(typed_item_id)?;

    // The first child must be the elements syntax
    let elements_id = typed_item_node.try_child(0)?;

    // The second child, if it exists, is the optional type_checker annotation
    let type_id_opt = typed_item_node.get_child(1);

    // Clone the span metadata
    let span = typed_item_node.span().clone();

    // Retrieve the elements syntax, which contains multiple element children
    let elements_node = syntax_tree.try_node(elements_id)?;

    // Collect the IDs of all contained elements
    let element_ids = elements_node.children().to_vec();

    // Return all extracted data
    Ok((element_ids, type_id_opt, span))
}

/// Creates a new `TypedItem` syntax containing exactly one element and optionally a cloned either_type.
///
/// This function is used during expr of `TypedList` nodes to reconstruct
/// each `TypedItem` in a uniform structure.
/// The element is included as-is, but if a either_type node is provided, it is **cloned**
/// to ensure each `TypedItem` has its own independent either_type subtree.
///
/// # Arguments
///
/// * `element_id` - The syntax ID of the single element to include. This is **not cloned**.
/// * `type_id_opt` - An optional syntax ID representing the either_type. If present, it is **cloned**.
/// * `span` - The source span associated with the new syntax.
/// * `parent_id` - The parent syntax ID, typically referring to the `TypedList`.
/// * `syntax_tree` - A mutable reference to the `SyntaxTree`, required for cloning the either_type.
///
/// # Returns
///
/// * `Ok(AstNode)` - A new `AstNode` instance representing the normalized `TypedItem`.
/// * `Err(NormalizationPassError)` - If cloning the either_type subtree fails.
///
/// # Notes
///
/// This function guarantees that each normalized `TypedItem` has an independent
/// either_type node, preventing accidental sharing of AST subtrees that could
/// lead to incorrect analysis or mutations.
fn create_typed_item_node(
    element_id: NodeId,
    type_id_opt: Option<NodeId>,
    span: Span,
    parent_id: NodeId,
    syntax_tree: &mut Tree<AstNode>,
) -> Result<AstNode, NormalizationPassError> {
    // Initialize children with the mandatory element syntax
    let mut children = vec![element_id];

    // Clone the either_type subtree if present
    if let Some(type_id) = type_id_opt {
        let clone_type = syntax_tree.clone_subtree(type_id)?;
        children.push(clone_type);
    }

    // Create the new TypedItem syntax with no content,
    // storing the span and parent information
    Ok(AstNode::new(
        AstKind::TypedItem,
        AstContent::None,
        children,
        span,
        Some(parent_id),
    ))
}
