use crate::aiplan4rust::compiler::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::compiler::lir::encoding::{typed_symbol, EncodingError};
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;
use crate::aiplan4rust::support::lang::{ObjectId, TypeId, TypedList, VariableId};

/// Encodes a syntax subtree representing a list of variables into a strongly-typed LIR list.
///
/// This function iterates over the children of the provided syntax node, assuming each child
/// represents a typed variable declaration (e.g., parameters in a function or action definition).
/// Each variable is mapped to a unique local identifier and registered within the mutable
/// [`EncodingRegistry`] to manage scope and binding during the LIR encoding pass.
///
/// # Arguments
///
/// * `subtree` - A reference to the [`SyntaxSubtree`] encapsulating the root node of the variable list.
/// * `registry` - A mutable reference to the [`EncodingRegistry`] used to register new variables and track scopes.
///
/// # Returns
///
/// * `Ok(TypedList<VariableId, TypeId>)` - A structured list containing the successfully encoded typed variables.
/// * `Err(EncodingError)` - If AST traversal fails or if a child node cannot be correctly decoded as a typed variable.
///
/// # Errors
///
/// This function returns an error if:
/// * A child node ID within the subtree cannot be resolved in the underlying syntax tree.
/// * The inner [`typed_symbol::encode_typed_variable`] fails due to structural or semantic invalidity.
pub fn encode_variable_list(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TypedList<VariableId, TypeId>, EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();
    let mut typed_list = TypedList::new();

    for &id in node.children() {
        let child_node = ast.try_node(id)?;
        let child_subtree = SyntaxSubtree::new(child_node, id, ast);

        // Delegation to variable encoding (with registry state mutations)
        let symbol = typed_symbol::encode_typed_variable(&child_subtree, registry)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}

/// Encodes a syntax subtree representing a list of domain objects or constants into a strongly-typed LIR list.
///
/// This function processes a syntax node representing a collection of objects or constants
/// (e.g., the `:constants` or `:objects` sections in PDDL). Unlike variables, objects are evaluated
/// against a read-only context, meaning the [`EncodingRegistry`] is kept immutable as identifiers
/// should already be globally declared and resolved.
///
/// # Arguments
///
/// * `subtree` - A reference to the [`SyntaxSubtree`] encapsulating the root node of the object list.
/// * `registry` - A shared reference to the [`EncodingRegistry`] used to resolve existing symbols and types.
///
/// # Returns
///
/// * `Ok(TypedList<ObjectId, TypeId>)` - A structured list containing the successfully encoded typed objects.
/// * `Err(EncodingError)` - If AST traversal fails or if a child node cannot be mapped to a known object symbol.
///
/// # Errors
///
/// This function returns an error if:
/// * A child node ID within the subtree cannot be resolved in the underlying syntax tree.
/// * The inner [`typed_symbol::encode_typed_object`] fails, typically because the object name or its type is missing from the registry.
#[allow(dead_code)]
pub fn encode_object_list(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedList<ObjectId, TypeId>, EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();
    let mut typed_list = TypedList::new();

    for &id in node.children() {
        let child_node = ast.try_node(id)?;
        let child_subtree = SyntaxSubtree::new(child_node, id, ast);

        // Delegation to object/constant encoding
        let symbol = typed_symbol::encode_typed_object(&child_subtree, registry)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}
