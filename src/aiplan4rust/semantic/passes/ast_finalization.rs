use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::passes::error::SemanticPassError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// Point d'entrée unique pour la finalisation.
/// C'est la seule fonction que tu appelles depuis ton Match.
pub fn finalize(context: &mut SemanticContext) -> Result<(), SemanticPassError> {
    // Étape A : On déstructure le tuple renvoyé par split_all_mut.
    // Contrairement à des appels de méthodes séparés, ici Rust voit
    // que 'ast', 'symbol_table' et 'interner' proviennent de champs disjoints.
    let (symbol_table, ast, interner) = context.split_all_mut();

    // Étape B : On appelle la fonction de détail.
    // Note : On passe 'symbol_table' en tant que référence immuable
    // (Rust transformera automatiquement &mut SymbolTable en &SymbolTable si nécessaire)
    finalize_ast_types(ast, symbol_table, interner)?;

    Ok(())
}

/// Finalizes an AST by updating the types of its declarations based on the SymbolTable.
///
/// This pass uses the precise NodeIds stored during semantic analysis to "patch"
/// the AST without needing a full tree traversal.
fn finalize_ast_types(
    ast: &mut Tree<AstNode>,
    symbol_table: &SymbolTable,
    _interner: &SymbolInterner,
) -> Result<(), SemanticPassError> {
    for symbol in symbol_table.values() {
        for declaration in symbol.declarations().values() {
            if declaration.ty().is_none() {
                continue;
            }

            // 1. Grâce à apply_type_simplifications, resolved_ty et original_node_ids sont synchros
            let (resolved_ty, original_node_ids) = try_get_type_and_nodes(declaration)?;
            let element_id = declaration.node_id();
            let parent_id = ast.try_node(element_id)?.try_parent()?;

            // 2. On récupère les spans.
            // Si le type est devenu 'object' (0 membres), original_spans sera vide.
            let original_spans: Vec<_> = original_node_ids
                .iter()
                .map(|&id| ast.try_node(id).map(|n| n.span().clone()))
                .collect::<Result<Vec<_>, _>>()?;

            // 3. Création des nœuds PrimitiveType
            let mut pt_ids = Vec::new();
            for (i, &pt_symbol_id) in resolved_ty.members().iter().enumerate() {
                // Mapping direct 1-pour-1
                let span = original_spans[i].clone();

                let primitive_node = AstNode::new(
                    AstKind::PrimitiveType,
                    AstContent::Ident(pt_symbol_id),
                    vec![],
                    span,
                    None,
                );
                pt_ids.push(ast.alloc(primitive_node));
            }

            // 4. Calcul du span du container Type
            // Si pas de membres (type object pur), on peut utiliser le span de l'élément (fallback)
            let type_span = if let Some(&first) = original_spans.first() {
                // .first() renvoie &Span, on déréférence avec &first pour obtenir une copie
                original_spans
                    .iter()
                    .skip(1)
                    .fold(first, |acc, &s| acc.merge(s)) // On passe les valeurs directement
            } else {
                ast.try_node(element_id)?.span()
            };

            let new_type_node_id = ast.alloc(AstNode::new(
                AstKind::Type,
                AstContent::None,
                pt_ids.clone(),
                type_span,
                Some(parent_id),
            ));

            // 5. Wiring des enfants
            for &child_id in &pt_ids {
                ast.try_node_mut(child_id)?
                    .set_parent(Some(new_type_node_id));
            }

            // 6. Mise à jour du parent (TypedItem)
            let old_children = ast.try_node(parent_id)?.children().to_vec();
            let mut new_children: Vec<_> = old_children
                .into_iter()
                .filter(|&id| {
                    ast.try_node(id)
                        .map(|n| n.kind() != AstKind::Type)
                        .unwrap_or(true)
                })
                .collect();

            new_children.push(new_type_node_id);

            let parent_node = ast.try_node_mut(parent_id)?;
            parent_node.set_children(new_children);
        }
    }
    Ok(())
}

/// Attempts to retrieve the semantic type and corresponding AST node IDs from a declaration.
///
/// This function replaces previous panics/asserts with a recoverable Result.
///
/// # Errors
/// * [`SemanticPassError::IncompleteDeclaration`] - If type data or node IDs are missing.
/// * [`SemanticPassError::TypeInconsistency`] - If there is a count mismatch between types and nodes.
pub fn try_get_type_and_nodes(
    declaration: &Declaration,
) -> Result<(&Type<SymbolId>, &[NodeId]), SemanticPassError> {
    let ty_opt = declaration.ty();
    let ids_opt = declaration.ty_node_ids();
    let symbol_id = declaration.symbol().id();

    // 1. Check if both data sets are present
    if ty_opt.is_none() || ids_opt.is_none() {
        return Err(SemanticPassError::incomplete_declaration(
            symbol_id,
            ty_opt.is_some(),
            ids_opt.is_some(),
        ));
    }

    let ty = ty_opt.unwrap();
    let ids = ids_opt.unwrap();

    // 2. Check for structural length consistency
    if ty.len() != ids.len() {
        return Err(SemanticPassError::type_inconsistency(
            symbol_id,
            ty.len(),
            ids.len(),
        ));
    }

    Ok((ty, ids))
}
