use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::encoding::{ty, EncodingError};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

pub fn encode_typed_type(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedSymbol<TypeId, TypeId>, EncodingError> {
    let ast = subtree.tree();
    let children = subtree.node().children();

    // 1. On récupère le StringID du type à gauche du tiret
    let symbol_node_id = children[0];
    let symbol_id = ast.try_node(symbol_node_id)?.try_ident()?;

    // 2. Résolution du TypeId. On utilise resolve_type_symbol_by_name car
    // c'est la définition même du type.
    let type_id = registry.try_resolve_type_symbol_by_name(symbol_id)?;

    // 3. Encodage du parent (à droite du tiret)
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_subtree = SyntaxSubtree::new(ast.try_node(ty_node_id)?, ty_node_id, ast);
        ty::encode(&ty_subtree, registry)?
    } else {
        Type::default()
    };

    Ok(TypedSymbol::new(type_id, ty))
}

pub fn encode_typed_object(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedSymbol<ObjectId, TypeId>, EncodingError> {
    let ast = subtree.tree();
    let children = subtree.node().children();

    // 1. On récupère le StringID de l'objet (ex: 'robot1')
    let symbol_node_id = children[0];
    let symbol_id = ast.try_node(symbol_node_id)?.try_ident()?;

    // 2. Résolution de l'ObjectId via le nom
    let object_id = registry.try_resolve_object_symbol_by_name(symbol_id)?;

    // 3. Encodage du typing (à droite du tiret)
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_subtree = SyntaxSubtree::new(ast.try_node(ty_node_id)?, ty_node_id, ast);
        ty::encode(&ty_subtree, registry)?
    } else {
        Type::default()
    };

    Ok(TypedSymbol::new(object_id, ty))
}

pub fn encode_typed_variable(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TypedSymbol<VariableId, TypeId>, EncodingError> {
    let ast = subtree.tree();
    let children = subtree.node().children();

    // 1. On récupère les infos de la variable (ex: '?x')
    let variable_node_id = children[0];
    let variable_symbol = ast.try_node(variable_node_id)?.try_ident()?;

    // 2. Enregistrement de la variable dans le registre
    // On lie le NodeId de l'AST à un nouveau VariableId LIR
    let variable_id = registry.register_variable(variable_node_id, variable_symbol);

    // 3. Encodage du typing (à droite du tiret)
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_subtree = SyntaxSubtree::new(ast.try_node(ty_node_id)?, ty_node_id, ast);
        ty::encode(&ty_subtree, registry)?
    } else {
        Type::default()
    };

    Ok(TypedSymbol::new(variable_id, ty))
}
