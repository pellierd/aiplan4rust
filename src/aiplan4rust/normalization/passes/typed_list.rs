use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::Ast;

/// Recursively normalizes all `TypedList` nodes in the given AST subtree.
///
/// This function performs a depth-first traversal of the AST starting from the provided root node.
/// For each node of kind `TypedList`, it restructures its children by:
/// - Validating that each child is a `TypedItem` node containing at least one child.
/// - Extracting the first child as the element (which must be a valid element kind).
/// - Optionally cloning the second child if it exists, which represents the type annotation.
/// - Rebuilding the `TypedItem` nodes with just the element and optional type.
///
/// After normalizing a `TypedList`, the function pushes its children onto the stack to
/// continue normalization recursively.
///
/// # Preconditions
///
/// This function assumes that:
/// - The AST is already syntactically valid.
/// - The `TypedList` nodes have not yet been normalized.
/// - This is the **first normalization pass** performed on the AST.
///
/// Running this function on an already normalized AST, or after other normalization passes,
/// may result in incorrect behavior or internal panics due to structural assumptions.
///
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - Any `TypedItem` node is missing or malformed.
/// - The element of a `TypedItem` node is not one of the expected kinds (`Constant`, `Variable`,
///   `PrimitiveType`, or `AtomicFunctionSkeleton`).
///
/// # Parameters
///
/// - `ast`: Mutable reference to the full AST to normalize.
///
/// # Returns
///
/// * `Ok(())` if the normalization completed successfully.
/// * `Err(ParserInternalError)` if structural inconsistencies are encountered.
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
/// This function uses an explicit stack to avoid deep recursion and potential stack overflow
/// with very large ASTs. It is typically the first normalization step before deeper
/// semantic transformations or type inference.
pub fn normalize_typed_list(ast: &mut Ast) -> Result<(), ParserInternalError> {
    normalize_typed_list_node(ast.root_mut())
}

/// Normalizes all `TypedList` nodes in the AST rooted at `root`.
///
/// This function performs a non-recursive depth-first traversal of the AST,
/// normalizing each `TypedList` node it encounters by:
/// 1. Validating each child `TypedItem` node and its element.
/// 2. Removing redundant child nodes and reconstructing each `TypedItem` with
///    exactly one element and optionally one type node.
/// 3. Replacing the children of the `TypedList` node with the normalized items.
///
/// # Arguments
///
/// * `root` - A mutable reference to the root `AstNode` of the AST to normalize.
///
/// # Returns
///
/// * `Ok(())` if the normalization completes successfully.
/// * `Err(ParserInternalError)` if the AST contains unexpected node kinds or structural errors.
///
/// # Panics
///
/// This function does not panic. All errors are returned as `ParserInternalError`.
///
/// # Example
///
/// ```ignore
/// let mut ast = parse(...);
/// normalize_typed_list_node(&mut ast)?;
/// ```
fn normalize_typed_list_node(root: &mut AstNode) -> Result<(), ParserInternalError> {
    // Use an explicit stack for non-recursive DFS traversal
    let mut stack = vec![root];

    // Continue until all nodes have been processed
    while let Some(node) = stack.pop() {
        // Check if the current node is a TypedList node to normalize
        if node.kind() == &AstKind::TypedList {
            // Take ownership of current children (TypedItem nodes)
            let old_typed_items = std::mem::take(node.children_mut());
            // Prepare a vector to hold the normalized TypedItem nodes
            let mut new_typed_items = Vec::new();

            // Iterate over each TypedItem node in the TypedList
            for typed_item in old_typed_items {
                // Access the children of the TypedItem
                let children = typed_item.children();

                // The first child must be TypedItemElements
                let typed_item_elements = &children[0];

                // The second child, if present, is the optional type annotation node
                let type_node_opt = children.get(1).cloned();

                // Iterate over each element inside TypedItemElements
                for element in typed_item_elements.children() {
                    // Construct new children vector containing the element and optional type node
                    let mut new_children = vec![element.clone()];
                    if let Some(type_node) = type_node_opt.clone() {
                        new_children.push(type_node);
                    }

                    // Create a new TypedItem node with the original span
                    let new_typed_item = Box::new(AstNode::new_with_span(
                        AstKind::TypedItem,
                        new_children,
                        typed_item.span().clone(),
                    ));

                    // Add the new TypedItem node to the normalized list
                    new_typed_items.push(new_typed_item);
                }
            }

            // Replace the TypedList's children with the normalized TypedItem nodes
            node.set_children(new_typed_items);
        }

        // Push the children of the current node onto the stack for further DFS traversal
        for child in node.children_mut().iter_mut().rev() {
            stack.push(child);
        }
    }

    Ok(())
}
