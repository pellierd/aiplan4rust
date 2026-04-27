use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lir::store::encoding::error::EncodingError;
use crate::aiplan4rust::lir::store::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::store::encoding::typed_list;
use crate::aiplan4rust::lir::store::expr::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::ExprId;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{Node, NodeId, SyntaxSubtree};
use std::collections::HashMap;

/// Encode un AST en LIR en utilisant un itérateur post-ordre (Bottom-Up).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder,
) -> Result<ExprId, EncodingError> {
    let mut resolved: HashMap<NodeId, ExprId> = HashMap::new();

    for (ast_id, _depth, ast_node) in subtree.tree().postorder_from(subtree.node_id()) {
        let kind = ast_node.kind();

        // --- FILTRAGE STRUCTUREL ---
        if kind == AstKind::TypedList {
            continue;
        }

        // --- RÉCUPÉRATION DES ENFANTS ---
        let children_ids: Vec<ExprId> = ast_node
            .children()
            .iter()
            .filter_map(|id| resolved.get(id).copied())
            .collect();

        // --- MATCH STRICT (Miroir de encode_content) ---
        let expr_id = match kind {
            // 1. Complex Terms (Skeletons)
            AstKind::AtomicFormula => {
                // 1. On récupère l'ID du nœud qui porte le nom (ex: "at")
                let predicate_usage_id = ast_node.children()[0];

                // 2. On trouve sa déclaration d'origine (vissage O(1))
                let decl = registry.symbol_table().resolve_usage(predicate_usage_id)?;
                let effective_id = decl.alias().unwrap_or(decl.source());

                // 3. On demande au registre : "Quel est l'ID LIR pour cette déclaration ?"
                let predicate_id = registry.try_resolve_predicate(effective_id)?;

                // 4. On récupère aussi le squelette (la signature complète)
                let skeleton_id = registry.try_resolve_atom_skeleton(effective_id)?;

                // 5. On passe les arguments réels (on saute l'index 0 qui est le symbole)
                let args = &children_ids[1..];

                builder.atomic_formula(predicate_id, args, skeleton_id)
            }

            AstKind::Function => {
                let function_id = ast_node.children()[0];
                let function_node = subtree.tree().try_node(function_id)?;
                let symbol = function_node.try_ident()?;

                // 1. On identifie la déclaration (NodeId)
                let effective_id = match symbol {
                    SymbolInterner::TOTAL_TIME_SYMBOL_ID => EncodingRegistry::TOTAL_TIME_NODE_ID,
                    SymbolInterner::TOTAL_COST_SYMBOL_ID => EncodingRegistry::TOTAL_COST_NODE_ID,
                    _ => {
                        let decl = registry.symbol_table().resolve_usage(function_id)?;
                        decl.alias().unwrap_or(decl.source())
                    }
                };

                // 2. On résout les IDs logiques du LIR
                let skeleton_id = registry.try_resolve_function_skeleton(effective_id)?;
                let function_symbol_id = registry.try_resolve_functor(effective_id)?;

                // 3. IMPORTANT : Ton builder recréant le symbole en interne,
                // on ne lui passe que les ARGUMENTS (on saute l'index 0).
                let args = &children_ids[1..];

                builder.function_term(function_symbol_id, args, skeleton_id)
            }

            AstKind::Task => {
                // 1. On récupère le NodeId de l'usage (le premier enfant de l'appel de tâche)
                let task_usage_id = ast_node.children()[0];

                // 2. On trouve la déclaration originale via la symbol table
                let decl = registry.symbol_table().resolve_usage(task_usage_id)?;
                let effective_id = decl.alias().unwrap_or(decl.source());

                // 3. On résout les identifiants LIR (Squelette et Symbole)
                let skeleton_id = registry.try_resolve_task_skeleton(effective_id)?;
                let task_symbol_id = registry.try_resolve_task_symbol(effective_id)?;

                // 4. CORRECTION : On ne passe que les arguments réels au builder
                // On ignore children_ids[0] car le builder va gérer le symbole lui-même.
                let args = &children_ids[1..];

                builder.task_with_skeleton(task_symbol_id, args, skeleton_id)
            }

            AstKind::Forall | AstKind::Exists => {
                let tl_id = ast_node.children()[0];
                let tl_node = subtree.tree().try_node(tl_id)?;

                // 1. Enregistrement des variables via ta fonction extraite
                register_local_variables(tl_node, subtree, registry)?;

                // 2. Encodage de la liste des variables pour le LIR
                let tl_subtree = SyntaxSubtree::new(tl_node, tl_id, subtree.tree());
                let vars = typed_list::encode_variable_list(&tl_subtree, registry)?;

                // 3. Construction du nœud de quantification
                let body = *children_ids.last().expect("Quantifier body missing");
                if kind == AstKind::Forall {
                    builder.forall(vars, body)?
                } else {
                    builder.exists(vars, body)?
                }
            }

            // 3. Atomic Symbols
            AstKind::PredicateSymbol => {
                let decl = registry.symbol_table().resolve_usage(ast_id)?;
                let effective_id = decl.alias().unwrap_or(decl.source());
                let pred_id = registry.try_resolve_predicate(effective_id)?;
                builder.predicate(pred_id)
            }

            AstKind::FunctionSymbol => {
                let symbol = ast_node.try_ident()?;
                let effective_id = match symbol {
                    SymbolInterner::TOTAL_TIME_SYMBOL_ID => EncodingRegistry::TOTAL_TIME_NODE_ID,
                    SymbolInterner::TOTAL_COST_SYMBOL_ID => EncodingRegistry::TOTAL_COST_NODE_ID,
                    _ => {
                        let decl = registry.symbol_table().resolve_usage(ast_id)?;
                        decl.alias().unwrap_or(decl.source())
                    }
                };
                builder.function_symbol(registry.try_resolve_functor(effective_id)?)
            }

            AstKind::Object => {
                let decl = registry.symbol_table().resolve_usage(ast_id)?;
                let effective_id = decl.alias().unwrap_or(decl.source());
                let obj_id = registry.try_resolve_object(effective_id)?;
                builder.object(obj_id)
            }

            AstKind::Variable => {
                let symbol_id = ast_node.try_ident()?;
                let effective_id = if symbol_id == SymbolInterner::DURATION_VARIABLE_SYMBOL_ID {
                    EncodingRegistry::DURATION_VARIABLE_NODE_ID
                } else {
                    registry.symbol_table().resolve_usage(ast_id)?.source()
                };
                builder.variable(registry.try_resolve_variable(effective_id)?)
            }

            AstKind::TaskSymbol => {
                let decl = registry.symbol_table().resolve_usage(ast_id)?;
                let effective_id = decl.alias().unwrap_or(decl.source());
                let task_sym_id = registry.try_resolve_task_symbol(effective_id)?;
                builder.task_symbol(task_sym_id)
            }

            AstKind::TaskLabel => {
                let label = ast_node.try_ident()?;
                builder.task_label(registry.try_resolve_task_label(label)?)
            }

            AstKind::PrefName => {
                let symbol = ast_node.try_ident()?;
                builder.pref_name(registry.register_preference_symbol(symbol))
            }

            AstKind::Number => builder.number(ast_node.try_number()?),

            AstKind::Comparison => {
                let op = ast_node.try_compare_op()?;
                builder.comparison(op, children_ids[0], children_ids[1])
            }

            AstKind::Assignment => {
                let op = ast_node.try_assign_op()?;
                builder.assignment(op, children_ids[0], children_ids[1])
            }

            AstKind::Arithmetic => {
                let op = ast_node.try_arithmetic_op()?;
                builder.arithmetic(op, &children_ids)
            }

            AstKind::Metric => {
                // On assume ici que la passe préalable a validé la présence de OptimizationOp
                let op = ast_node.try_optimization_op()?;
                builder.metric_exp(op, children_ids[0])
            }

            // --- Catch-all final ---
            _ => return Err(EncodingError::unsupported_ast_node_kind(kind)),
        };

        resolved.insert(ast_id, expr_id);
    }

    // Récupération sécurisée du résultat final
    resolved
        .get(&subtree.node_id())
        .copied()
        .ok_or_else(|| EncodingError::unsupported_ast_node_kind(subtree.node().kind()))
}

fn register_local_variables(
    node: &AstNode,
    subtree: &SyntaxSubtree<AstNode>,
    reg: &mut EncodingRegistry,
) -> Result<(), EncodingError> {
    for &child_id in node.children() {
        let typed_var = subtree.tree().try_node(child_id)?;
        let var_id = typed_var.try_child(0)?;
        let var_node = subtree.tree().try_node(var_id)?;
        reg.register_variable(var_id, var_node.try_ident()?);
    }
    Ok(())
}
