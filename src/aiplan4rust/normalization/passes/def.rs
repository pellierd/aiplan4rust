

use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::normalization::passes::NormalizationPassError;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::syntax::Span;

pub fn normalize_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,      // Ex: AstKind::ObjectsDef ou AstKind::TypesDef
) -> Result<bool, NormalizationPassError> {
    // 1. Trouver le nœud correspondant au genre demandé
    let def_id = match ast.find_node_id_of_kind(kind) {
        Some(id) => id,
        None => return Ok(false),
    };

    // 2. Rapporter les warnings (version générique)
    report_implicit_either_declaration_warning(def_id, ast, diagnostic_manager, kind)?;

    // 3. Fusionner les doublons (version générique)
    let modified = merge_duplicate_declarations(def_id, ast)?;

    Ok(modified)
}

fn report_implicit_either_warning(
    def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,
) -> Result<(), NormalizationPassError> {
    // Utilise la version générique de collecte que nous avons définie précédemment
    let seen = collect_implicit_either_declarations(def_id, ast)?;

    // Passe le nom de l'entité pour que le diagnostic soit contextuel
    emit_implicit_either_warnings(seen, ast, diagnostic_manager, kind)
}


fn report_implicit_either_declaration_warning(
    def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,
) -> Result<(), NormalizationPassError> {
    // On appelle la fonction de collecte générique que tu as déjà
    let seen = collect_implicit_either_declarations(def_id, ast)?;

    // On appelle la fonction d'émission générique que tu as déjà
    emit_implicit_either_warnings(seen, ast, diagnostic_manager, kind)
}

fn collect_implicit_either_declarations(
    def_id: NodeId,
    ast: &Ast,
) -> Result<HashMap<SymbolId, (HashSet<SymbolId>, Span, Vec<(HashSet<SymbolId>, Span)>)>, NormalizationPassError> {
    let syntax_tree = ast.syntax_tree();
    let def_node = syntax_tree.try_node(def_id)?;

    // Accès à la TypedList (enfant 0 du bloc de définition)
    let typed_list_id = def_node.try_child(0)?;
    let typed_list = syntax_tree.try_node(typed_list_id)?;

    let mut seen: HashMap<SymbolId, (HashSet<SymbolId>, Span, Vec<(HashSet<SymbolId>, Span)>)> = HashMap::new();

    for typed_item_id in typed_list.children() {
        let item = syntax_tree.try_node(*typed_item_id)?;

        // 1. Extraction de l'identifiant (ex: l'objet 'p3' ou le type 'truck')
        let ident_node_id = item.try_child(0)?;
        let ident_node = syntax_tree.try_node(ident_node_id)?;
        let ident = ident_node.try_ident()?;
        let span = ident_node.span();

        // 2. Extraction des types parents / super-types
        let super_idents = match item.get_child(1) {
            Some(ty_id) => {
                let ty_node = syntax_tree.try_node(ty_id)?;
                let mut set = HashSet::new();
                for super_node_id in ty_node.children() {
                    let super_node = syntax_tree.try_node(*super_node_id)?;
                    let super_ident = super_node.try_ident()?;
                    set.insert(super_ident);
                }
                set
            }
            None => HashSet::new(),
        };

        // 3. Stockage et détection de collision
        if let Some((_, _, duplicates)) = seen.get_mut(&ident) {
            duplicates.push((super_idents, span.clone()));
        } else {
            seen.insert(
                ident,
                (super_idents, span.clone(), Vec::new()),
            );
        }
    }

    Ok(seen)
}

fn emit_implicit_either_warnings(
    seen: HashMap<SymbolId, (HashSet<SymbolId>, Span, Vec<(HashSet<SymbolId>, Span)>)>,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,
) -> Result<(), NormalizationPassError> {
    for (ident, (_, first_span, duplicates)) in seen {
        if !duplicates.is_empty() {
            let mut duplicate_types = Vec::new();
            let mut duplicate_spans = Vec::new();

            for (supertypes, span) in duplicates {
                for st in supertypes {
                    duplicate_types.push(st);
                    duplicate_spans.push(span.clone());
                }
            }

            // On utilise une version générique du constructeur de diagnostic
            let warning = Diagnostic::warning_implicit_either_type_declaration(
                ident,
                //kind,      // "type", "object", etc.
                duplicate_types,
                duplicate_spans,
                Provider::Normalizer,
                ast.source_id(),
                first_span.clone(),
            );

            diagnostic_manager.add_diagnostic(warning);
        }
    }

    Ok(())
}


pub fn merge_duplicate_declarations(
    def_id: NodeId,
    ast: &mut Ast,
) -> Result<bool, NormalizationPassError> {
    let syntax_tree = ast.syntax_tree_mut();

    // Accès au nœud parent (ex: ObjectsDef)
    let def_node = syntax_tree.try_node(def_id)?;

    // On récupère la TypedList (enfant 0)
    let typed_list_id = def_node.try_child(0)?;
    let typed_list = syntax_tree.try_node_mut(typed_list_id)?;

    let mut modified = false;
    let mut seen: HashMap<SymbolId, NodeId> = HashMap::new();
    let mut duplicates_to_remove: HashSet<NodeId> = HashSet::new();

    // Snapshot pour itérer sans conflit avec le borrow checker sur l'arena
    let children_ids = typed_list.children().to_vec();

    for &item_id in &children_ids {
        if duplicates_to_remove.contains(&item_id) {
            continue;
        }

        let item_node = syntax_tree.try_node(item_id)?;

        // 1. Extraction de l'identifiant (p3, truck, etc.)
        let ident_node_id = item_node.try_child(0)?;
        let ident_node = syntax_tree.try_node(ident_node_id)?;
        let ident = ident_node.try_ident()?;

        if let Some(&existing_item_id) = seen.get(&ident) {
            // Doublon trouvé : on fusionne les enfants du nœud de type/annotation

            // Nœuds de type du premier élément rencontré
            let existing_item = syntax_tree.try_node(existing_item_id)?;
            let existing_super_id = existing_item.try_child(1)?;

            // Nœuds de type de l'élément actuel (doublon)
            let current_super_id = item_node.try_child(1)?;

            let existing_super = syntax_tree.try_node(existing_super_id)?;
            let current_super = syntax_tree.try_node(current_super_id)?;

            // Fusion des listes d'enfants (NodeIds)
            let mut merged_children = existing_super.children().to_vec();
            let mut seen_children: HashSet<NodeId> = merged_children.iter().copied().collect();

            for &child_id in current_super.children() {
                if !seen_children.contains(&child_id) {
                    merged_children.push(child_id);
                    seen_children.insert(child_id);
                }
            }

            // Mise à jour de l'élément original dans l'arena
            let existing_super_mut = syntax_tree.try_node_mut(existing_super_id)?;
            existing_super_mut.set_children(merged_children);

            // Marquage du doublon pour suppression
            duplicates_to_remove.insert(item_id);
            modified = true;
        } else {
            // Premier enregistrement de l'identifiant
            seen.insert(ident, item_id);
        }
    }

    // Nettoyage de la liste des enfants du nœud parent
    if modified {
        let list_mut = syntax_tree.try_node_mut(typed_list_id)?;
        list_mut.set_children(
            list_mut
                .children()
                .iter()
                .copied()
                .filter(|id| !duplicates_to_remove.contains(id))
                .collect(),
        );
    }

    Ok(modified)
}
