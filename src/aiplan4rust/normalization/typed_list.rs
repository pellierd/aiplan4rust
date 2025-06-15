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
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - Any `TypedItem` node is missing or malformed.
/// - The element of a `TypedItem` node is not one of the expected kinds (`Constant`, `Variable`,
///   `PrimitiveType`, or `AtomicFunctionSkeleton`).
///
/// # Parameters
///
/// - `root`: Mutable reference to the root node of the AST subtree to normalize.
///
/// # Examples
///
/// ```rust,no_run
/// let mut ast = parse_source_code(source_code)?;
/// normalize_typed_list_node(&mut ast.root_mut())?;
/// ```
///
/// # Notes
///
/// This function uses an explicit stack to avoid deep recursion and potential stack overflow
/// with very large ASTs.
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
pub fn normalize_typed_list_node(root: &mut AstNode) -> Result<(), ParserInternalError> {
    // On utilise une pile pour un parcours DFS explicite
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == &AstKind::TypedList {
            // Prendre les enfants actuels du TypedList
            let old_typed_items = std::mem::take(node.children_mut());
            let mut new_typed_items = Vec::new();

            for typed_item in old_typed_items {
                // Valider la structure du TypedItem
                validate_typed_item(&typed_item)?;

                let children = typed_item.children();

                if children.is_empty() {
                    return Err(ParserInternalError::new(
                        "TypedItem must have at least one child (TypedItemElements)".to_string(),
                    ));
                }

                // Premier enfant = TypedItemElements obligatoire
                let typed_item_elements = &children[0];
                if typed_item_elements.kind() != &AstKind::TypedItemElements {
                    return Err(ParserInternalError::new(
                        "First child of TypedItem must be TypedItemElements".to_string(),
                    ));
                }

                // Second enfant optionnel = le type
                let type_node_opt = children.get(1).cloned();

                // Parcourir chaque élément individuel dans TypedItemElements
                for element in typed_item_elements.children() {
                    // Valider que c’est bien un élément attendu
                    validate_typed_element(element)?;

                    // Construire les enfants du nouveau TypedItem (élément + type optionnel)
                    let mut new_children = vec![element.clone()];
                    if let Some(type_node) = type_node_opt.clone() {
                        new_children.push(type_node);
                    }

                    // Créer un nouveau TypedItem avec span original
                    let new_typed_item = Box::new(AstNode::new_with_span(
                        AstKind::TypedItem,
                        new_children,
                        typed_item.span().clone(),
                    ));

                    new_typed_items.push(new_typed_item);
                }
            }

            // Remplacer les enfants du TypedList par la liste "explosée"
            node.set_children(new_typed_items);
        }

        // Empiler les enfants du noeud actuel pour continuer le DFS
        for child in node.children_mut().iter_mut().rev() {
            stack.push(child);
        }
    }

    Ok(())
}



/// Validates that the given node is a `TypedItem` and has at least one child.
///
/// # Errors
///
/// Returns a `ParserInternalError` if the node is not of kind `TypedItem`
/// or if it has no children.
fn validate_typed_item(node: &AstNode) -> Result<(), ParserInternalError> {
    if node.kind() != &AstKind::TypedItem {
        return Err(ParserInternalError::new("Expected TypedItem inside TypedList".into()));
    }
    if node.children().is_empty() {
        return Err(ParserInternalError::new("TypedItem must have at least one child".into()));
    }
    Ok(())
}

/// Validates that the given node is a valid element kind for a `TypedItem`.
///
/// Acceptable kinds are:
/// - `Constant`
/// - `Variable`
/// - `PrimitiveType`
/// - `AtomicFunctionSkeleton`
///
/// # Errors
///
/// Returns a `ParserInternalError` if the node kind is not one of the accepted kinds.
fn validate_typed_element(node: &AstNode) -> Result<(), ParserInternalError> {
    match node.kind() {
        AstKind::Constant(_)
        | AstKind::Variable(_)
        | AstKind::PrimitiveType(_)
        | AstKind::AtomicFunctionSkeleton => Ok(()),

        other => Err(ParserInternalError::new(format!(
            "Unexpected TypedItem element kind: {:?}",
            other
        ))),
    }
}
