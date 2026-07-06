use crate::aiplan4rust::compiler::lir::encoding::error::EncodingError;
use crate::aiplan4rust::compiler::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::compiler::lir::encoding::typed_list;
use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::{Node, NodeId, SyntaxSubtree};
use crate::aiplan4rust::compiler::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::support::interner::SymbolInterner;

/// Encode un AST en LIR en utilisant un itérateur post-ordre (Bottom-Up).
pub enum Step {
    Enter(NodeId),
    Exit(NodeId, usize),
}

pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder,
) -> Result<ExprId, EncodingError> {
    // On récupère les buffers du registry pour éviter toute allocation
    let mut stack = std::mem::take(&mut registry.stack_buffer);
    let mut results = std::mem::take(&mut registry.results_buffer);

    stack.clear();
    results.clear();

    stack.push(Step::Enter(subtree.node_id()));

    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(ast_id) => {
                let ast_node = subtree.tree().try_node(ast_id)?;
                let kind = ast_node.kind();

                // --- PHASE PRE-ORDER : Scopes ---
                if matches!(kind, AstKind::Forall | AstKind::Exists) {
                    let tl_id = ast_node.children()[0];
                    let tl_node = subtree.tree().try_node(tl_id)?;
                    register_local_variables(tl_node, subtree, registry)?;
                }

                // 1. ON EMPLE LE EXIT EN PREMIER
                // Il sera tout en bas par rapport à ses enfants, donc il sortira en dernier.

                // On doit calculer children_to_process AVANT ou stocker les IDs
                let mut children_to_process = 0;
                let mut children_stack = Vec::new(); // Temporaire pour l'ordre

                for &child_id in ast_node.children().iter().rev() {
                    let child_node = subtree.tree().try_node(child_id)?;
                    if child_node.kind() != AstKind::TypedList {
                        children_stack.push(child_id);
                        children_to_process += 1;
                    }
                }

                // L'ordre critique pour une pile LIFO :
                // [BAS] EXIT -> ENFANT_1 -> ENFANT_2 -> ENFANT_N [HAUT]

                stack.push(Step::Exit(ast_id, children_to_process));

                for child_id in children_stack {
                    stack.push(Step::Enter(child_id));
                }
            }

            Step::Exit(ast_id, children_count) => {
                let ast_node = subtree.tree().try_node(ast_id)?;
                let kind = ast_node.kind();

                // --- PHASE POST-ORDER : Construction ---
                // On pointe directement dans le buffer sans allouer
                let results_len = results.len();
                if results_len < children_count {
                    // C'EST ICI QUE LE BUG SERA DÉMASQUÉ
                    panic!(
                        "DÉCALAGE PILE : Le nœud {:?} (ID #{}) attend {} enfants, mais results n'en a que {}. \
            Contenu AST du nœud : {:?}",
                        kind, ast_id, children_count, results_len, ast_node.content()
                    );
                }

                let start_idx = results.len() - children_count;
                let children_ids = &results[start_idx..];

                let expr_id = match kind {
                    // 1. Termes Complexes
                    AstKind::AtomicFormula => {
                        let predicate_usage_id = ast_node.children()[0];
                        let decl = registry.symbol_table().resolve_usage(predicate_usage_id)?;
                        let effective_id = decl.alias().unwrap_or(decl.source());
                        let pred_id = registry.try_resolve_predicate(effective_id)?;
                        let skel_id = registry.try_resolve_atom_skeleton(effective_id)?;
                        builder.atomic_formula(pred_id, &children_ids[1..], skel_id)
                    }

                    AstKind::Function => {
                        let function_id = ast_node.children()[0];
                        let symbol = subtree.tree().try_node(function_id)?.try_ident()?;
                        let effective_id = match symbol {
                            SymbolInterner::TOTAL_TIME_SYMBOL_ID => {
                                EncodingRegistry::TOTAL_TIME_NODE_ID
                            }
                            SymbolInterner::TOTAL_COST_SYMBOL_ID => {
                                EncodingRegistry::TOTAL_COST_NODE_ID
                            }
                            _ => {
                                let decl = registry.symbol_table().resolve_usage(function_id)?;
                                decl.alias().unwrap_or(decl.source())
                            }
                        };
                        let skel_id = registry.try_resolve_function_skeleton(effective_id)?;
                        let func_id = registry.try_resolve_functor(effective_id)?;
                        builder.function_term(func_id, &children_ids[1..], skel_id)
                    }

                    AstKind::Task => {
                        let task_usage_id = ast_node.children()[0];
                        let decl = registry.symbol_table().resolve_usage(task_usage_id)?;
                        let effective_id = decl.alias().unwrap_or(decl.source());
                        let skel_id = registry.try_resolve_task_skeleton(effective_id)?;
                        let task_id = registry.try_resolve_task_symbol(effective_id)?;
                        builder.task_with_skeleton(task_id, &children_ids[1..], skel_id)
                    }

                    // 2. Logique
                    AstKind::And => builder.and(children_ids),
                    AstKind::Or => builder.or(children_ids),
                    AstKind::Not => builder.not(children_ids[0]),
                    AstKind::Imply => builder.imply(children_ids[0], children_ids[1]),

                    AstKind::Forall | AstKind::Exists => {
                        let tl_id = ast_node.children()[0];
                        let tl_subtree = SyntaxSubtree::new(
                            subtree.tree().try_node(tl_id)?,
                            tl_id,
                            subtree.tree(),
                        );
                        let vars = typed_list::encode_variable_list(
                            &tl_subtree,
                            registry,
                            builder.store_mut(),
                        )?;
                        let body = *children_ids
                            .last()
                            .ok_or_else(|| EncodingError::unsupported_ast_node_kind(kind))?;
                        if kind == AstKind::Forall {
                            builder.forall(vars, body)?
                        } else {
                            builder.exists(vars, body)?
                        }
                    }

                    // 3. Symboles Atomiques
                    AstKind::PredicateSymbol => {
                        let decl = registry.symbol_table().resolve_usage(ast_id)?;
                        builder.predicate(
                            registry
                                .try_resolve_predicate(decl.alias().unwrap_or(decl.source()))?,
                        )
                    }

                    // 3. Symboles de function
                    AstKind::FunctionSymbol => {
                        let symbol = ast_node.try_ident()?;

                        // 1. On identifie l'ID de nœud effectif (en gérant les réservés)
                        let effective_node_id = match symbol {
                            SymbolInterner::TOTAL_TIME_SYMBOL_ID => {
                                EncodingRegistry::TOTAL_TIME_NODE_ID
                            }
                            SymbolInterner::TOTAL_COST_SYMBOL_ID => {
                                EncodingRegistry::TOTAL_COST_NODE_ID
                            }
                            _ => {
                                // Pour les fonctions normales, on passe par la table des symboles
                                let decl = registry.symbol_table().resolve_usage(ast_id)?;
                                decl.alias().unwrap_or(decl.source())
                            }
                        };

                        // 2. On résout le functor LIR
                        let func_id = registry.try_resolve_functor(effective_node_id)?;
                        builder.function_symbol(func_id)
                    }

                    AstKind::TaskSymbol => {
                        let decl = registry.symbol_table().resolve_usage(ast_id)?;
                        builder.task_symbol(
                            registry
                                .try_resolve_task_symbol(decl.alias().unwrap_or(decl.source()))?,
                        )
                    }

                    AstKind::Variable => {
                        let symbol_id = ast_node.try_ident()?;
                        let effective_id =
                            if symbol_id == SymbolInterner::DURATION_VARIABLE_SYMBOL_ID {
                                EncodingRegistry::DURATION_VARIABLE_NODE_ID
                            } else {
                                registry.symbol_table().resolve_usage(ast_id)?.source()
                            };
                        builder.variable(registry.try_resolve_variable(effective_id)?)
                    }
                    AstKind::Object => {
                        let decl = registry.symbol_table().resolve_usage(ast_id)?;
                        builder.object(
                            registry.try_resolve_object(decl.alias().unwrap_or(decl.source()))?,
                        )
                    }

                    // 4. Numériques & Opérateurs
                    AstKind::Number => builder.number(ast_node.try_number()?),
                    AstKind::Comparison => builder.comparison(
                        ast_node.try_compare_op()?,
                        children_ids[0],
                        children_ids[1],
                    ),
                    AstKind::Assignment => builder.assignment(
                        ast_node.try_assign_op()?,
                        children_ids[0],
                        children_ids[1],
                    ),
                    AstKind::Arithmetic => {
                        builder.arithmetic(ast_node.try_arithmetic_op()?, children_ids)
                    }
                    AstKind::Metric => {
                        builder.metric_exp(ast_node.try_optimization_op()?, children_ids[0])
                    }

                    // 5. Temporel & Modal
                    AstKind::AtStart => builder.at_start(children_ids[0])?,
                    AstKind::AtEnd => builder.at_end(children_ids[0])?,
                    AstKind::Overall => builder.overall(children_ids[0])?,
                    AstKind::TimedInitialLiteral => {
                        // L'enfant 0 est le temps (Number), l'enfant 1 est l'expression (AtomicFormula/Assignment)
                        // Selon ta logique d'empilement, children_ids[0] est le temps, children_ids[1] est le fait.
                        let time = subtree
                            .tree()
                            .try_node(ast_node.children()[0])?
                            .try_number()?;

                        let effect = children_ids[1];

                        // On appelle le builder pour créer l'expression temporelle initiale
                        builder.timed_initial_literal(time, effect)?
                    }
                    AstKind::When => builder.when(children_ids[0], children_ids[1]),
                    AstKind::Preference => {
                        let pref_name_id = ast_node.children()[0];
                        let symbol = subtree.tree().try_node(pref_name_id)?.try_ident()?;
                        let pref_symbol_id = registry.register_preference_symbol(symbol);
                        builder.preference(pref_symbol_id, children_ids[1])
                    }

                    // 6. Contraintes (PDDL 3.0)
                    AstKind::Always => builder.always(children_ids[0]),
                    AstKind::Sometime => builder.sometime(children_ids[0]),
                    AstKind::Within => {
                        let deadline = subtree
                            .tree()
                            .try_node(ast_node.children()[0])?
                            .try_number()?;
                        builder.within(deadline, children_ids[1])
                    }
                    AstKind::AlwaysWithin => {
                        let duration = subtree
                            .tree()
                            .try_node(ast_node.children()[0])?
                            .try_number()?;
                        builder.always_within(duration, children_ids[1], children_ids[2])
                    }

                    AstKind::SometimeBefore => {
                        // children_ids[0] : la condition (ex: a atteint le but)
                        // children_ids[1] : ce qui doit s'être passé avant (ex: a ouvert la porte)
                        builder.sometime_before(children_ids[0], children_ids[1])
                    }
                    AstKind::SometimeAfter => {
                        // (sometime-after A B)
                        builder.sometime_after(children_ids[0], children_ids[1])
                    }

                    AstKind::AtMostOnce => {
                        // (at-most-once A)
                        builder.at_most_once(children_ids[0])
                    }

                    AstKind::HoldDuring => {
                        let start = subtree
                            .tree()
                            .try_node(ast_node.children()[0])?
                            .try_number()?;
                        let end = subtree
                            .tree()
                            .try_node(ast_node.children()[1])?
                            .try_number()?;
                        // Les expressions commencent à l'index 2 dans children_ids
                        builder.hold_during(start, end, children_ids[2])
                    }

                    AstKind::HoldAfter => {
                        let time = subtree
                            .tree()
                            .try_node(ast_node.children()[0])?
                            .try_number()?;
                        builder.hold_after(time, children_ids[1])
                    }

                    // 7. Divers
                    AstKind::TaskLabel => {
                        builder.task_label(registry.try_resolve_task_label(ast_node.try_ident()?)?)
                    }
                    AstKind::LabeledTask => {
                        // 1. Récupérer le label (le nom 't1', 't2', etc.)
                        let label_node_id = ast_node.children()[0];
                        let label_ident = subtree.tree().try_node(label_node_id)?.try_ident()?;

                        // 2. Enregistrer ou résoudre le label dans le registre
                        // On utilise register pour s'assurer qu'il existe un ID pour ce label dans la méthode
                        let label_symbol_id = registry.register_task_label(label_ident);

                        // 3. Récupérer l'ID de la tâche (déjà encodée par Step::Enter)
                        // children_ids[0] car le label n'a pas été poussé sur la pile results
                        let task_expr_id = children_ids[0];

                        // 4. Construire le nœud LIR
                        builder.labeled_task(label_symbol_id, task_expr_id)
                    }
                    AstKind::TaskOrderingConstraint => {
                        // Dans l'AST, les enfants sont les étiquettes (TaskLabel)
                        // children_ids contient les ExprId de ces étiquettes déjà encodées
                        let predecessor = children_ids[0];
                        let successor = children_ids[1];

                        // On utilise la méthode de ton ExprBuilder
                        builder.task_ordering_constraint(predecessor, successor)
                    }

                    AstKind::PrefName => builder
                        .pref_name(registry.register_preference_symbol(ast_node.try_ident()?)),

                    AstKind::IsViolated => {
                        // L'enfant unique est le nom de la préférence (PrefName ou PredicateSymbol servant de nom)
                        let pref_name_id = ast_node.children()[0];
                        let symbol = subtree.tree().try_node(pref_name_id)?.try_ident()?;

                        // On enregistre/récupère l'ID de la préférence dans le registry
                        let pref_symbol_id = registry.register_preference_symbol(symbol);

                        // On construit le nœud "is-violated"
                        builder.is_violated(pref_symbol_id)
                    }
                    AstKind::TotalTime => builder.total_time(),

                    _ => return Err(EncodingError::unsupported_ast_node_kind(kind)),
                };

                // On nettoie la section des enfants et on pousse le résultat parent
                results.truncate(start_idx);
                results.push(expr_id);
            }
        }
    }

    results
        .pop()
        .ok_or_else(|| EncodingError::unsupported_ast_node_kind(AstKind::Number))
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
