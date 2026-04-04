use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::linking::finalization::error::SemanticFinalizationError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// Point d'entrée unique pour la finalisation.
/// C'est la seule fonction que tu appelles depuis ton Match.
pub fn finalize(context: &mut SemanticContext) -> Result<(), SemanticFinalizationError> {
    let (symbol_table, ast, interner) = context.split_all_mut();

    // On passe la slice de changements
    finalize_all_types(ast, symbol_table)?;

    Ok(())
}

/// La nouvelle version "systématique"
fn finalize_all_types(
    ast: &mut Tree<AstNode>,
    symbol_table: &SymbolTable,
) -> Result<(), SemanticFinalizationError> {
    for entry in symbol_table.values() {
        for declaration in entry.declarations().values() {
            // 1. On récupère le type s'il existe
            let Ok((resolved_ty, original_node_ids)) = try_get_type_and_nodes(declaration) else {
                continue;
            };

            // 2. Localisation du parent dans l'AST
            let element_id = declaration.source();
            let parent_id = ast.try_node(element_id)?.try_parent()?;

            // 3. Collecte des spans originaux (pour garder le formattage)
            let original_spans: Vec<_> = original_node_ids
                .iter()
                .map(|&id| ast.try_node(id).map(|n| n.span().clone()))
                .collect::<Result<Vec<_>, _>>()?;

            // 4. Création des nouveaux nœuds PrimitiveType
            let mut pt_ids = Vec::with_capacity(resolved_ty.len());
            for (i, &pt_symbol_id) in resolved_ty.members().iter().enumerate() {
                let span = original_spans
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| ast.try_node(element_id).map(|n| n.span().clone()).unwrap());

                let primitive_node = AstNode::new(
                    AstKind::PrimitiveType,
                    AstContent::Ident(pt_symbol_id),
                    vec![],
                    span,
                    None,
                );
                pt_ids.push(ast.alloc(primitive_node));
            }

            // 5. Calcul du span global du conteneur 'Type'
            let type_span = if let Some(first) = original_spans.first() {
                original_spans
                    .iter()
                    .skip(1)
                    .fold(first.clone(), |acc, s| acc.merge(s.clone()))
            } else {
                ast.try_node(element_id)?.span().clone()
            };

            // 6. Création du nœud Type parent
            let new_type_node_id = ast.alloc(AstNode::new(
                AstKind::Type,
                AstContent::None,
                pt_ids.clone(),
                type_span,
                Some(parent_id),
            ));

            // Wiring des enfants
            for &child_id in &pt_ids {
                ast.try_node_mut(child_id)?
                    .set_parent(Some(new_type_node_id));
            }

            // 7. Remplacement CHIRURGICAL dans le parent (TypedItem)
            let mut updated_children = Vec::new();
            let current_children = ast.try_node(parent_id)?.children().to_vec();

            let mut type_replaced = false;
            for &child_id in &current_children {
                // Si on tombe sur l'ancien nœud de type, on met le nouveau à la place
                if ast.try_node(child_id)?.kind() == AstKind::Type {
                    if !type_replaced {
                        updated_children.push(new_type_node_id);
                        type_replaced = true;
                    }
                } else {
                    updated_children.push(child_id);
                }
            }

            // Si le parent n'avait pas de type du tout (type implicite), on l'ajoute
            if !type_replaced {
                updated_children.push(new_type_node_id);
            }

            ast.try_node_mut(parent_id)?.set_children(updated_children);
        }
    }
    Ok(())
}

/// Attempts to retrieve the semantic type and corresponding AST node IDs from a declaration.
///
/// This function replaces previous panics/asserts with a recoverable Result.
///
/// # Errors
/// * [`SemanticFinalizationError::IncompleteDeclaration`] - If type data or node IDs are missing.
/// * [`SemanticFinalizationError::TypeInconsistency`] - If there is a count mismatch between types and nodes.
pub fn try_get_type_and_nodes(
    declaration: &Declaration,
) -> Result<(&Type<SymbolId>, &[NodeId]), SemanticFinalizationError> {
    let ty_opt = declaration.ty();
    let ids_opt = declaration.type_sources();
    let symbol_id = declaration.symbol().id();

    // 1. Check if both data sets are present
    if ty_opt.is_none() || ids_opt.is_none() {
        return Err(SemanticFinalizationError::incomplete_declaration(
            symbol_id,
            ty_opt.is_some(),
            ids_opt.is_some(),
        ));
    }

    let ty = ty_opt.unwrap();
    let ids = ids_opt.unwrap();

    // 2. Check for structural length consistency
    if ty.len() != ids.len() {
        return Err(SemanticFinalizationError::type_inconsistency(
            symbol_id,
            ty.len(),
            ids.len(),
        ));
    }

    Ok((ty, ids))
}

/*/// Finalise l'AST en mettant à jour les types des déclarations basés sur les changements.
///
/// Cette version est chirurgicale : elle ne boucle que sur les symboles modifiés.
fn finalize_types(
    ast: &mut Tree<AstNode>,
    symbol_table: &SymbolTable,
    changes: &[TypeSimplification],
    _interner: &SymbolInterner,
) -> Result<(), SemanticFinalizationError> {
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
}*/
