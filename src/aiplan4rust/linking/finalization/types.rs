use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::linking::finalization::error::SemanticFinalizationError;
use crate::aiplan4rust::linking::finalization::FinalizationContext;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::syntax::ast::{AstContent, AstNode};
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// La nouvelle version "systématique"
pub fn finalize(
    _context: &FinalizationContext,
    symbol_table: &SymbolTable,
    ast: &mut Tree<AstNode>,
) -> Result<(), SemanticFinalizationError> {
    for symbol in symbol_table {
        for declaration in symbol.declarations() {
            // 1. Garde : Si pas de type défini, on ne touche à rien (cas STRIPS ou non-typé)
            if declaration.ty().is_none() {
                continue;
            }

            // 2. Extraction sécurisée des données de type et des nœuds sources
            let (resolved_ty, original_node_ids) = match try_get_type_and_nodes(declaration) {
                Ok((ty, ids)) if !ids.is_empty() => (ty, ids),
                _ => continue,
            };

            // 3. Localisation du conteneur "Type"
            let first_pt_id = original_node_ids[0];
            let type_node_id = ast.try_node(first_pt_id)?.try_parent()?;
            let members_to_keep = resolved_ty.members();

            // --- ÉTAPE A : Collecte des IDs à supprimer (Emprunt Immuable) ---
            let mut to_remove = Vec::new();
            if let Ok(type_node) = ast.try_node(type_node_id) {
                for &child_id in type_node.children() {
                    if let Ok(child_node) = ast.try_node(child_id) {
                        if let AstContent::Ident(id) = child_node.content() {
                            // Si le symbole n'est plus dans la table, on marque pour suppression
                            if !members_to_keep.contains(id) {
                                to_remove.push(child_id);
                            }
                        }
                    }
                }
            }

            // --- ÉTAPE B : Nettoyage et Orphelinage (Emprunt Mutable) ---
            // On débranche les nœuds supprimés de leur parent pour la sécurité
            for &id in &to_remove {
                if let Ok(node) = ast.try_node_mut(id) {
                    node.set_parent(None);
                }
            }

            // Mise à jour de la liste des enfants du nœud Type
            let type_node_mut = ast.try_node_mut(type_node_id)?;
            let children = type_node_mut.children_mut();

            // On ne garde que ceux qui ne sont pas dans la liste de suppression
            children.retain(|id| !to_remove.contains(id));
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

/*/// Attempts to retrieve the semantic type and corresponding AST node IDs from a declaration.
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
}*/
