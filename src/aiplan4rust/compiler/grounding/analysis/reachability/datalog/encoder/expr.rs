use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::{AtomSkeletonId, CompareOp, VariableId};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::core::rule::RuleBody;
use crate::analysis::reachability::datalog::encoder::{aliasing, predicate};
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::state::DatalogState;

/// Encodes the preconditions of an action into Datalog atoms and rules.
///
/// This function serves as the public entry point for flattening action preconditions.
/// It delegates the iterative post-order traversal to `encode_expr`, starting from
/// the root of the provided expression tree.
///
/// Complex logical structures (AND/OR) are decomposed into auxiliary predicates
/// using a Tseitin-like transformation to maintain a flat Horn-clause structure.
///
/// # Arguments
///
/// * `expr` - The global expression handle wrapping the store and the root.
/// * `ctx` - The Datalog context containing immuable information like parameter lists and types.
/// * `state` - The mutable Datalog state accumulating rules, cache, and IDs.
/// * `store` - The expression store containing the precondition trees.
///
/// # Returns
///
/// * `Ok(Some(Atom))` - The head atom representing the unified precondition logic.
/// * `Ok(None)` - If the expression is empty or contains only ignored nodes (e.g., empty AND).
/// * `Err(DatalogError)` - If the expression tree is malformed or contains unsupported nodes.
pub fn encode_preconditions(
    expr: ExprId,
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    store: &mut ExprStore,
) -> Result<Option<Atom>, DatalogError> {
    // 1. Nettoyage et préparation des alias (pour gérer les ?x = ?y)
    // 🌟 Remplissage direct de l'état mutable pour que `encode_expr` puisse y accéder !
    *state.current_aliases = aliasing::extract_variable_aliases(expr, store)?;

    // 2. 🌟 Appel mis à jour avec le contexte et l'état complets (plus de déballage !)
    encode_condition(expr, ctx, state, store)
}

/// Encodes action effects into Datalog rules by propagating causality from the action to its consequences.
///
/// This function performs an iterative top-down traversal of the effect expression tree.
/// It establishes a logical chain between the "cause" (the action atom) and the resulting
/// "facts" (atomic formulas).
///
/// # Conditional Effects (WHEN)
///
/// When encountering a `When` node, the function:
/// 1. Uses `encode_expr` to flatten the condition into an auxiliary atom.
/// 2. Creates a "pivot" auxiliary predicate representing the conjunction of the action
///    and the condition.
/// 3. Propagates this new auxiliary atom as the "cause" for all nested sub-effects.
///
/// # Relaxed Semantics
///
/// For reachability analysis, this encoder follows a positive-only Datalog model:
/// * **Delete-effects (`Not`)** are explicitly ignored.
/// * **Numerical effects** (Assign, Operation, etc.) are skipped as they do not contribute
///   to atomic fact reachability.
///
/// # Arguments
///
/// * `effect` - The expression tree representing the action's effects.
/// * `action_atom` - The atom representing the execution of the action (the initial cause).
/// * `action_index` - The unique offset index of the current action.
/// * `ctx` - The immuable context (parameters list, skeleton maps, negation offset).
/// * `state` - The mutable compiler state (rules sink, cache, tables, global alias map).
/// * `action_effects` - Accummulator vector for storing effect causes per action.
/// * `store` - The expression store containing the node contents.
///
/// # Errors
///
/// Returns a [`DatalogError`] if an unsupported node kind is encountered.
pub fn encode_effects(
    effect: ExprId,
    action_atom: &Atom,
    action_index: usize,
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    action_effects: &mut Vec<Vec<(Atom, Cause)>>,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // ÉTAPE 1 : Remplissage direct de la table d'alias dans l'état partagé
    *state.current_aliases = aliasing::extract_variable_aliases(effect, store)?;

    let mut root_cause = action_atom.clone();
    for term in root_cause.arguments_mut() {
        if let Term::Variable(v) = *term {
            *term = aliasing::resolve_var(v, state.current_aliases);
        }
    }

    let root_cause_id = root_cause.symbol();
    // On utilise directement l'id de départ pour la pile
    let mut work_stack = vec![(effect, root_cause)];
    let mut visited = std::collections::HashSet::new();

    while let Some((current_id, current_cause)) = work_stack.pop() {
        if !visited.insert((current_id, current_cause.symbol())) {
            continue;
        }

        // Appel direct au store : l'emprunt sur `store` s'arrête dès que `node` ou `kind` sort du scope
        let node = store.fetch(current_id)?;
        let kind = node.kind();

        match kind {
            ExprKind::AtomicFormula(_) => {
                let mut effect_atom = predicate::extract_atom(node, store)?;

                for term in effect_atom.arguments_mut() {
                    if let Term::Variable(v) = *term {
                        *term = aliasing::resolve_var(v, state.current_aliases);
                    }
                }

                let cause = if current_cause.symbol() == root_cause_id {
                    Cause::Action
                } else {
                    Cause::Pivot(current_cause.clone())
                };
                action_effects[action_index].push((effect_atom.clone(), cause));
                let mut rule_body = RuleBody::new();
                rule_body.push(current_cause.clone());

                state.rules.push(Rule::new(effect_atom, rule_body));
            }

            ExprKind::And => {
                for &child_id in node.children().iter().rev() {
                    work_stack.push((child_id, current_cause.clone()));
                }
            }

            ExprKind::When => {
                let children = node.children();
                let condition_id = children[0];
                let sub_effect_id = children[1];

                // 🌟 Appel propre à encode_expr avec nos structures unifiées
                if let Some(cond_atom) = encode_condition(condition_id, ctx, state, store)? {
                    let mut combined_body = vec![current_cause.clone(), cond_atom.clone()];
                    combined_body.sort_by_key(|a| a.symbol());

                    let aux_when_atom = if let Some(existing_head) = state.cache.get(&combined_body)
                    {
                        existing_head.clone()
                    } else {
                        let (head, secured_body) = predicate::allocate_auxiliary_predicate(
                            &combined_body,
                            ctx.param_list_id,
                            state.next_aux_id,
                            state.aux_defs,
                            ctx.type_to_skeleton,
                            state.current_aliases,
                            store,
                        )?;

                        state.rules.push(Rule::new(head.clone(), secured_body));
                        state.cache.insert(combined_body, head.clone());
                        head
                    };

                    work_stack.push((sub_effect_id, aux_when_atom));
                } else {
                    work_stack.push((sub_effect_id, current_cause));
                }
            }

            ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                if let Some(&child_id) = node.children().first() {
                    work_stack.push((child_id, current_cause));
                }
            }

            ExprKind::Assignment(_) | ExprKind::Arithmetic(_) => {
                continue;
            }

            ExprKind::Not => {
                let children = node.children();
                if let Some(&child_id) = children.first() {
                    let child_node = store.fetch(child_id)?;
                    let child_kind = child_node.kind();

                    if let ExprKind::AtomicFormula(_) = child_kind {
                        let mut del_atom = predicate::extract_atom(child_node, store)?;

                        for term in del_atom.arguments_mut() {
                            if let Term::Variable(v) = *term {
                                *term = aliasing::resolve_var(v, state.current_aliases);
                            }
                        }

                        del_atom.set_negated(true);

                        let pure_id = del_atom.symbol().as_usize();
                        let target_idx = pure_id + ctx.negation_offset;

                        let mut final_skeleton = AtomSkeletonId::from(target_idx);
                        final_skeleton.set_negated(true);
                        del_atom.set_symbol(final_skeleton);

                        let cause = if current_cause.symbol() == root_cause_id {
                            Cause::Action
                        } else {
                            Cause::Pivot(current_cause.clone())
                        };

                        action_effects[action_index].push((del_atom.clone(), cause));
                    }
                }
            }

            ExprKind::Forall(_) | ExprKind::Exists(_) | ExprKind::Imply => {
                return Err(DatalogError::feature_not_supported(
                    format!("ADL construct {:?} in effects", kind),
                    current_id,
                ));
            }

            _ => {
                return Err(DatalogError::incompatible_node(kind.clone(), current_id));
            }
        }
    }
    Ok(())
}

/// Encodes a sub-expression starting from a specific node into Datalog atoms and rules.
///
/// This is the internal engine used by both `encode_preconditions` and `encode_effects`.
/// It performs an iterative post-order traversal starting at `node_id` to flatten
/// complex logical structures (AND/OR) into auxiliary predicates.
///
/// # Arguments
///
/// * `expr` - The starting point for the encoding (root of the sub-tree).
/// * `ctx` - The Datalog context containing immuable information like parameter lists and types.
/// * `state` - The mutable Datalog state accumulating rules, cache, and the active alias map.
/// * `store` - The expression store containing the tree nodes.
///
/// # Returns
///
/// * `Ok(Some(Atom))` - The head atom representing the encoded sub-expression.
/// * `Ok(None)` - If the branch contains no logical content (e.g., empty AND, ignored nodes).
/// * `Err(DatalogError)` - If the stack is inconsistent or an unsupported node is encountered.
fn encode_condition(
    expr: ExprId,
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    store: &mut ExprStore,
) -> Result<Option<Atom>, DatalogError> {
    let mut work_stack = vec![(expr, false)];
    let mut results_stack: Vec<Option<Atom>> = Vec::with_capacity(32);

    // =========================================================================
    // TABLEAU DE MÉMOÏSATION INDEXÉ
    // =========================================================================
    let mut memo: Vec<Option<Option<Atom>>> = vec![None; store.len()];

    while let Some((current_id, visited)) = work_stack.pop() {
        let idx = current_id.as_usize();

        if !visited {
            if let Some(cached_result) = &memo[idx] {
                results_stack.push(cached_result.clone());
                continue;
            }
        }

        // Interrogation directe du store
        let node = store.fetch(current_id)?;
        let kind = node.kind();

        if !visited {
            match kind {
                // FILTRAGE EN DESCENTE
                ExprKind::Not => {
                    let children = node.children();

                    if children.len() != 1 {
                        return Err(StorerError::invalid_node(current_id).into());
                    }

                    let child_id = children[0];
                    let child_node = store.fetch(child_id)?;
                    let child_kind = child_node.kind();

                    let is_valid_comparison = match child_kind {
                        ExprKind::Comparison(op) => *op == CompareOp::Equal,
                        _ => false,
                    };

                    if !is_valid_comparison {
                        let feature_desc = format!("Negation of {:?}", child_kind);
                        return Err(DatalogError::feature_not_supported(feature_desc, child_id));
                    }

                    work_stack.push((current_id, true));
                    work_stack.push((child_id, false));
                }

                ExprKind::And
                | ExprKind::Or
                | ExprKind::AtStart
                | ExprKind::AtEnd
                | ExprKind::Overall => {
                    work_stack.push((current_id, true));
                    for &child_id in node.children().iter().rev() {
                        work_stack.push((child_id, false));
                    }
                }

                ExprKind::AtomicFormula(_) | ExprKind::Comparison(_) => {
                    work_stack.push((current_id, true));
                }

                ExprKind::Arithmetic(_) => {
                    results_stack.push(None);
                }

                _ => return Err(DatalogError::incompatible_node(kind.clone(), current_id)),
            }
        } else {
            // --- PHASE 2 : Synthèse ---
            let num_children = node.children().len();
            let result = match kind {
                ExprKind::AtomicFormula(skeleton_id) => {
                    let mut atom = predicate::extract_atom(node, store)?;
                    for term in atom.arguments_mut() {
                        if let Term::Variable(v) = *term {
                            // 🌟 Utilisation de la table partagée de l'état mutable
                            *term = aliasing::resolve_var(v, state.current_aliases);
                        }
                    }
                    if skeleton_id.is_negated() {
                        let mut sk = atom.symbol();
                        sk.set_negated(true);
                        atom.set_symbol(sk);
                    }
                    Some(atom)
                }

                ExprKind::Not => match results_stack.pop().flatten() {
                    Some(mut atom) => {
                        let terms = atom.arguments();
                        if terms[0] == terms[1] {
                            None
                        } else {
                            atom.set_negated(true);
                            Some(atom)
                        }
                    }
                    None => {
                        let v = VariableId::from(0);
                        Some(Atom::equality(Term::Variable(v), Term::Variable(v)))
                    }
                },

                ExprKind::And => {
                    let start_idx = results_stack.len() - num_children;
                    let child_results: Vec<Option<Atom>> =
                        results_stack.drain(start_idx..).collect();

                    if child_results.iter().any(|r| r.is_none()) {
                        None
                    } else {
                        let mut atoms: Vec<Atom> = Vec::new();
                        for a in child_results.into_iter().flatten() {
                            if a.is_equality() {
                                let terms = a.arguments();
                                if a.is_negated() {
                                    if terms[0] == terms[1] {
                                        return Ok(None);
                                    }
                                    atoms.push(a);
                                } else {
                                    if terms[0] != terms[1] {
                                        atoms.push(a);
                                    }
                                }
                            } else {
                                atoms.push(a);
                            }
                        }

                        if atoms.is_empty() {
                            Some(Atom::equality(
                                Term::Variable(VariableId::from(0)),
                                Term::Variable(VariableId::from(0)),
                            ))
                        } else if atoms.len() == 1 {
                            Some(atoms[0].clone())
                        } else {
                            atoms.sort_by_key(|a| a.symbol());
                            if let Some(existing_head) = state.cache.get(&atoms) {
                                Some(existing_head.clone())
                            } else {
                                // 🌟 Passage des sous-champs unifiés
                                let (head, secured_body) = predicate::allocate_auxiliary_predicate(
                                    &atoms,
                                    ctx.param_list_id,
                                    state.next_aux_id,
                                    state.aux_defs,
                                    ctx.type_to_skeleton,
                                    state.current_aliases,
                                    store,
                                )?;

                                state.rules.push(Rule::new(head.clone(), secured_body));
                                state.cache.insert(atoms, head.clone());
                                Some(head)
                            }
                        }
                    }
                }

                ExprKind::Or => {
                    let start_idx = results_stack.len() - num_children;
                    let mut atoms: Vec<Atom> = results_stack.drain(start_idx..).flatten().collect();

                    if atoms.is_empty() {
                        None
                    } else if atoms.len() == 1 {
                        Some(atoms[0].clone())
                    } else {
                        atoms.sort_by_key(|a| a.symbol());
                        atoms.dedup();

                        if let Some(existing_head) = state.cache.get(&atoms) {
                            Some(existing_head.clone())
                        } else {
                            // 🌟 Passage des sous-champs unifiés
                            let (head, _) = predicate::allocate_auxiliary_predicate(
                                &atoms,
                                ctx.param_list_id,
                                state.next_aux_id,
                                state.aux_defs,
                                ctx.type_to_skeleton,
                                state.current_aliases,
                                store,
                            )?;

                            for atom in &atoms {
                                let mut branch_body = RuleBody::new();
                                branch_body.push(atom.clone());
                                let mut covered_vars = std::collections::HashSet::new();
                                if !atom.is_negated() {
                                    for term in atom.arguments() {
                                        if let Term::Variable(v) = term {
                                            covered_vars.insert(*v);
                                        }
                                    }
                                }

                                for term in head.arguments() {
                                    if let Term::Variable(v) = term {
                                        if !covered_vars.contains(v) {
                                            let parameters =
                                                store.fetch_typed_list(ctx.param_list_id)?;
                                            let type_id = parameters[v.as_usize()].ty().members()
                                                [0]
                                            .as_usize();
                                            let type_sk = ctx.type_to_skeleton[type_id];

                                            branch_body
                                                .push(Atom::unary(type_sk, Term::Variable(*v)));

                                            covered_vars.insert(*v);
                                        }
                                    }
                                }

                                state.rules.push(Rule::new(head.clone(), branch_body));
                            }

                            state.cache.insert(atoms, head.clone());
                            Some(head)
                        }
                    }
                }

                ExprKind::Comparison(op) => {
                    if *op == CompareOp::Equal {
                        let mut atom = predicate::extract_atom(node, store)?;

                        for term in atom.arguments_mut() {
                            if let Term::Variable(v) = *term {
                                // 🌟 Résolution via l'état partagé des alias
                                *term = aliasing::resolve_var(v, state.current_aliases);
                            }
                        }

                        let terms = atom.arguments();
                        if terms[0] == terms[1] {
                            Some(atom)
                        } else if let (Term::Constant(_), Term::Constant(_)) =
                            (&terms[0], &terms[1])
                        {
                            None
                        } else {
                            Some(atom)
                        }
                    } else {
                        None
                    }
                }

                ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                    if num_children > 0 {
                        results_stack.pop().flatten()
                    } else {
                        None
                    }
                }
                _ => None,
            };

            memo[idx] = Some(result.clone());
            results_stack.push(result);
        }
    }
    Ok(results_stack.pop().flatten())
}
