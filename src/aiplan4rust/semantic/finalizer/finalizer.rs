// src/semantic/finalizer.rs

/*/// Finalise un AST en mettant à jour le type de ses déclarations.
///
/// Pour chaque symbole de la table, si une déclaration existe dans cet AST,
/// son type est remplacé par le `resolved_type` (le plus spécifique).
pub fn finalize_ast_types(symbol_table: &SymbolTable, ast: &mut Tree<AstNode>) {
    for symbol in symbol_table.values() {
        for declaration in symbol.declarations() {
            let node_id = declaration.node_id();
            let ty = declaration.ty().unwrap();
            let node = ast.try_node_mut(node_id).unwrap();
        }
    }
}*/
