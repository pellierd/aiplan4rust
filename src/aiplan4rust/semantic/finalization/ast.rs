use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::finalization::error::SemanticPassError;
use crate::aiplan4rust::semantic::finalization::type_simplification::TypeSimplification;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// Point d'entrée unique pour la finalisation.
/// C'est la seule fonction que tu appelles depuis ton Match.
pub fn finalize(
    context: &mut SemanticContext,
    type_changes: &[TypeSimplification], // Utilise une slice pour accepter Vec ou Box
) -> Result<(), SemanticPassError> {
    let (symbol_table, ast, interner) = context.split_all_mut();

    // On passe la slice de changements
    finalize_types(ast, symbol_table, type_changes, interner)?;

    Ok(())
}

/// Finalise l'AST en mettant à jour les types des déclarations basés sur les changements.
///
/// Cette version est chirurgicale : elle ne boucle que sur les symboles modifiés.
fn finalize_types(
    ast: &mut Tree<AstNode>,
    symbol_table: &SymbolTable,
    changes: &[TypeSimplification],
    _interner: &SymbolInterner,
) -> Result<(), SemanticPassError> {
    for change in changes {
        // 1. On récupère la déclaration directement depuis le changement
        let entry = symbol_table.try_get_symbol(change.symbol_id())?;
        let declaration = entry.try_get_declaration(change.node_id())?;

        // 2. Ta fonction 'try' qui garantit que resolved_ty et original_node_ids sont synchros
        let (resolved_ty, original_node_ids) = try_get_type_and_nodes(declaration)?;

        let element_id = declaration.source();
        let parent_id = ast.try_node(element_id)?.try_parent()?;

        // 3. Récupération sécurisée des spans via les NodeIds synchronisés
        let original_spans: Vec<_> = original_node_ids
            .iter()
            .map(|&id| ast.try_node(id).map(|n| n.span().clone()))
            .collect::<Result<Vec<_>, _>>()?;

        // 4. Création des nouveaux nœuds PrimitiveType
        let mut pt_ids = Vec::new();
        for (i, &pt_symbol_id) in resolved_ty.members().iter().enumerate() {
            // Le mapping est 1-pour-1 entre membres du type et IDs de l'AST
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

        // 5. Calcul du span du container Type
        let type_span = if let Some(&first) = original_spans.first() {
            original_spans
                .iter()
                .skip(1)
                .fold(first, |acc, &s| acc.merge(s))
        } else {
            // Fallback si le type est vide (cas du type 'object')
            ast.try_node(element_id)?.span()
        };

        let new_type_node_id = ast.alloc(AstNode::new(
            AstKind::Type,
            AstContent::None,
            pt_ids.clone(),
            type_span,
            Some(parent_id),
        ));

        // 6. Wiring des nouveaux enfants vers leur parent Type
        for &child_id in &pt_ids {
            ast.try_node_mut(child_id)?
                .set_parent(Some(new_type_node_id));
        }

        // 7. Mise à jour chirurgicale du parent (TypedItem)
        // On récupère les enfants, on filtre l'ancien Type, et on ajoute le nouveau.
        let mut new_children: Vec<_> = ast
            .try_node(parent_id)?
            .children()
            .iter()
            .filter(|&&id| {
                ast.try_node(id)
                    .map(|n| n.kind() != AstKind::Type)
                    .unwrap_or(true)
            })
            .copied()
            .collect();

        new_children.push(new_type_node_id);

        let parent_node = ast.try_node_mut(parent_id)?;
        parent_node.set_children(new_children);
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
    let ids_opt = declaration.type_sources();
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
