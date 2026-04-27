use crate::aiplan4rust::lang::{ObjectId, TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::store::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::store::encoding::{typed_symbol, EncodingError};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

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

        // Délégation à l'encodage des variables (avec enregistrement dans le registre)
        let symbol = typed_symbol::encode_typed_variable(&child_subtree, registry)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}

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

        // Délégation à l'encodage des objets/constantes
        let symbol = typed_symbol::encode_typed_object(&child_subtree, registry)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}
