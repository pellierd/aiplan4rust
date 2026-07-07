use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, CompareOp, PredicateSymbolId, TypedList, TypedListId, TypedSymbol, VariableId,
};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::state::DatalogState;
use std::collections::HashMap;

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
    *state.current_aliases = extract_variable_aliases(expr, store)?;

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
    *state.current_aliases = extract_variable_aliases(effect, store)?;

    let mut root_cause = action_atom.clone();
    for term in root_cause.terms_mut() {
        if let Term::Variable(v) = *term {
            *term = resolve_var(v, state.current_aliases);
        }
    }

    let root_cause_id = root_cause.skeleton_id();
    // On utilise directement l'id de départ pour la pile
    let mut work_stack = vec![(effect, root_cause)];
    let mut visited = std::collections::HashSet::new();

    while let Some((current_id, current_cause)) = work_stack.pop() {
        if !visited.insert((current_id, current_cause.skeleton_id())) {
            continue;
        }

        // Appel direct au store : l'emprunt sur `store` s'arrête dès que `node` ou `kind` sort du scope
        let node = store.fetch(current_id)?;
        let kind = node.kind();

        match kind {
            ExprKind::AtomicFormula(_) => {
                let mut effect_atom = extract_atom(node, store)?;

                for term in effect_atom.terms_mut() {
                    if let Term::Variable(v) = *term {
                        *term = resolve_var(v, state.current_aliases);
                    }
                }

                let cause = if current_cause.skeleton_id() == root_cause_id {
                    Cause::Action
                } else {
                    Cause::Pivot(current_cause.clone())
                };
                action_effects[action_index].push((effect_atom.clone(), cause));
                state
                    .rules
                    .push(Rule::new(effect_atom, vec![current_cause.clone()]));
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
                    combined_body.sort_by_key(|a| a.skeleton_id());

                    let aux_when_atom = if let Some(existing_head) = state.cache.get(&combined_body)
                    {
                        existing_head.clone()
                    } else {
                        let (head, secured_body) = encode_new_aux_predicate(
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
                        let mut del_atom = extract_atom(child_node, store)?;

                        for term in del_atom.terms_mut() {
                            if let Term::Variable(v) = *term {
                                *term = resolve_var(v, state.current_aliases);
                            }
                        }

                        del_atom.set_negated(true);

                        let pure_id = del_atom.skeleton_id().as_usize();
                        let target_idx = pure_id + ctx.negation_offset;

                        let mut final_skeleton = AtomSkeletonId::from(target_idx);
                        final_skeleton.set_negated(true);
                        del_atom.set_skeleton_id(final_skeleton);

                        let cause = if current_cause.skeleton_id() == root_cause_id {
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
                    let mut atom = extract_atom(node, store)?;
                    for term in atom.terms_mut() {
                        if let Term::Variable(v) = *term {
                            // 🌟 Utilisation de la table partagée de l'état mutable
                            *term = resolve_var(v, state.current_aliases);
                        }
                    }
                    if skeleton_id.is_negated() {
                        let mut sk = atom.skeleton_id();
                        sk.set_negated(true);
                        atom.set_skeleton_id(sk);
                    }
                    Some(atom)
                }

                ExprKind::Not => match results_stack.pop().flatten() {
                    Some(mut atom) => {
                        let terms = atom.terms();
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
                                let terms = a.terms();
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
                            atoms.sort_by_key(|a| a.skeleton_id());
                            if let Some(existing_head) = state.cache.get(&atoms) {
                                Some(existing_head.clone())
                            } else {
                                // 🌟 Passage des sous-champs unifiés
                                let (head, secured_body) = encode_new_aux_predicate(
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
                        atoms.sort_by_key(|a| a.skeleton_id());
                        atoms.dedup();

                        if let Some(existing_head) = state.cache.get(&atoms) {
                            Some(existing_head.clone())
                        } else {
                            // 🌟 Passage des sous-champs unifiés
                            let (head, _) = encode_new_aux_predicate(
                                &atoms,
                                ctx.param_list_id,
                                state.next_aux_id,
                                state.aux_defs,
                                ctx.type_to_skeleton,
                                state.current_aliases,
                                store,
                            )?;

                            for atom in &atoms {
                                let mut branch_body = vec![atom.clone()];
                                let mut covered_vars = std::collections::HashSet::new();
                                if !atom.is_negated() {
                                    for term in atom.terms() {
                                        if let Term::Variable(v) = term {
                                            covered_vars.insert(*v);
                                        }
                                    }
                                }

                                for term in head.terms() {
                                    if let Term::Variable(v) = term {
                                        if !covered_vars.contains(v) {
                                            let parameters =
                                                store.fetch_typed_list(ctx.param_list_id)?;
                                            let type_id = parameters[v.as_usize()].ty().members()
                                                [0]
                                            .as_usize();
                                            let type_sk = ctx.type_to_skeleton[type_id];

                                            branch_body
                                                .push(Atom::new(type_sk, vec![Term::Variable(*v)]));
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
                        let mut atom = extract_atom(node, store)?;

                        for term in atom.terms_mut() {
                            if let Term::Variable(v) = *term {
                                // 🌟 Résolution via l'état partagé des alias
                                *term = resolve_var(v, state.current_aliases);
                            }
                        }

                        let terms = atom.terms();
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

/// Version locale (associée) pour extraire la table des alias d'un groupe d'égalité
/// Version locale (associée) pour extraire la table des alias d'un groupe d'égalité
fn extract_variable_aliases(
    effect_id: ExprId,
    store: &ExprStore,
) -> Result<std::collections::HashMap<VariableId, Term>, DatalogError> {
    let mut aliases = std::collections::HashMap::new();

    // Gardien du DAG (Hash-Consing) - Allocation unique de la taille du store
    let mut visited = vec![false; store.len()];
    let mut stack = vec![effect_id];

    // 🌟 Version optimisée : Zéro allocation, parcours direct et sécurisé par le tri des IDs
    let find_rep = |map: &std::collections::HashMap<VariableId, Term>, v: VariableId| -> Term {
        let mut curr = Term::Variable(v);
        while let Term::Variable(var) = curr {
            if let Some(next) = map.get(&var) {
                curr = next.clone();
            } else {
                break;
            }
        }
        curr
    };

    while let Some(node_id) = stack.pop() {
        let idx = node_id.as_usize();
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        // 🌟 Appel direct au store au lieu de expr
        let node = store.fetch(node_id)?;
        let kind = node.kind();

        match kind {
            ExprKind::Not => {
                continue; // Les égalités dans un NOT sont des inégalités, on ignore.
            }

            ExprKind::Comparison(op) => {
                if *op == CompareOp::Equal {
                    let children = node.children();
                    if children.len() == 2 {
                        // 💡 Appels mis à jour pour passer l'ID et le store
                        let t1 = node_to_term(children[0], store)?;
                        let t2 = node_to_term(children[1], store)?;

                        match (t1, t2) {
                            (Some(Term::Variable(v1)), Some(Term::Variable(v2))) => {
                                let r1 = find_rep(&aliases, v1);
                                let r2 = find_rep(&aliases, v2);

                                if r1 != r2 {
                                    match (r1, r2) {
                                        (Term::Variable(var1), Term::Variable(var2)) => {
                                            aliases.insert(
                                                var1.max(var2),
                                                Term::Variable(var1.min(var2)),
                                            );
                                        }
                                        (Term::Variable(var), Term::Constant(c))
                                        | (Term::Constant(c), Term::Variable(var)) => {
                                            aliases.insert(var, Term::Constant(c));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            (Some(Term::Variable(v)), Some(Term::Constant(c)))
                            | (Some(Term::Constant(c)), Some(Term::Variable(v))) => {
                                let r = find_rep(&aliases, v);
                                if let Term::Variable(var) = r {
                                    aliases.insert(var, Term::Constant(c));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ => {
                for &child_id in node.children().iter().rev() {
                    stack.push(child_id);
                }
            }
        }
    }

    // Aplatissement final unique (Path Compression)
    compute_transitive_closure(&mut aliases);
    Ok(aliases)
}

/// Version locale (associée) pour convertir un identifiant de nœud en Term
fn node_to_term(node_id: ExprId, store: &ExprStore) -> Result<Option<Term>, DatalogError> {
    // 🌟 Appel direct au store pour récupérer le nœud
    let n = store.fetch(node_id)?;

    Ok(match n.kind() {
        ExprKind::Variable(v_id) => Some(Term::Variable(*v_id)),
        ExprKind::Object(obj_id) => Some(Term::Constant(*obj_id)),
        _ => None,
    })
}

/// Encodes a new auxiliary predicate based on a collection of atoms.
///
/// This is a helper function used during the transformation of complex formulas
/// (like AND/OR) into Horn clauses. It performs two main steps:
/// 1. It collects all unique variables from the provided `atoms` to determine
///    the signature (arity and types) of the new predicate.
/// 2. It allocates a new unique predicate ID and returns the corresponding [`Atom`].
///
/// # Arguments
/// * `atoms` - The list of atoms that will form the body (for AND) or the options (for OR)
///             of the rules associated with this auxiliary predicate.
/// * `parameters` - The typed list of parameters from the current scope (e.g., action parameters).
///
/// # Errors
/// Returns a [`DatalogError`] if variable collection fails or if there is an
/// inconsistency in the typed list.
///
fn encode_new_aux_predicate(
    atoms: &[Atom],
    parameters_id: TypedListId, // 🌟 Mis à jour : passage par ID
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    type_to_skeleton: &[AtomSkeletonId],
    current_aliases: &std::collections::HashMap<VariableId, Term>,
    store: &mut ExprStore,
) -> Result<(Atom, Vec<Atom>), DatalogError> {
    // 1. Collecte et résolution des variables
    let used_vars = collect_variables(atoms, current_aliases)?;

    let mut resolved_terms: Vec<Term> = used_vars
        .into_iter()
        .map(|v_id| resolve_var(v_id, current_aliases))
        .collect();

    resolved_terms.sort();
    resolved_terms.dedup();

    let final_vars: Vec<VariableId> = resolved_terms
        .iter()
        .filter_map(|t| {
            if let Term::Variable(v) = t {
                Some(*v)
            } else {
                None
            }
        })
        .collect();

    // --- 2. LA RÉPARATION (Type Guard Injection) ---
    let mut covered_vars = std::collections::HashSet::new();
    for atom in atoms {
        if !atom.is_negated() {
            for term in atom.terms() {
                if let Term::Variable(v) = term {
                    covered_vars.insert(*v);
                }
            }
        }
    }

    let mut secured_body = atoms.to_vec();

    // 🌟 Récupération locale et temporaire des paramètres pour injecter les Type Guards
    let parameters = store.fetch_typed_list(parameters_id)?;

    for v_id in &final_vars {
        if !covered_vars.contains(v_id) {
            let type_id = parameters[v_id.as_usize()].ty().members()[0].as_usize();
            let type_sk = type_to_skeleton[type_id];

            secured_body.push(Atom::new(type_sk, vec![Term::Variable(*v_id)]));
            covered_vars.insert(*v_id);
        }
    }

    // 3. Création de l'atome de tête
    let head = create_aux_atom(
        final_vars,
        resolved_terms,
        parameters_id, // 🌟 On relaie l'ID ici aussi
        next_aux_id,
        aux_defs,
        store,
    );

    Ok((head, secured_body))
}

/// Extracts a logical [`Atom`] from a specific expression node.
///
/// This function serves as the bridge between the high-level expression tree ([`Expr`][`ExprNode`][`Atom`]).
/// It resolves the predicate identity and maps each child argument to its concrete Datalog representation.
///
/// # Arguments
///
/// * `expr` - The global expression tree used to resolve the nature of child nodes.
/// * `node` - A reference to the current [`ExprNode`], which must represent an `AtomicFormula` or a `Comparison`.
///
/// # Returns
///
/// A [`Result`] containing the grounded or lifted [`Atom`], or a [`DatalogError`] if resolution fails.

fn extract_atom(node: ExprNode<'_>, store: &ExprStore) -> Result<Atom, DatalogError> {
    let kind = node.kind();
    let children = node.children();

    // 1. Fast determination of the Skeleton ID and the child skip offset.
    let (skeleton_id, skip_count) = match kind {
        ExprKind::Comparison(_) => (AtomSkeletonId::from(Atom::EQUALITY_ID), 0),
        ExprKind::AtomicFormula(sk_id) => (*sk_id, 1),
        _ => {
            return Err(DatalogError::incompatible_node(
                kind.clone(),
                node.id(), // 🌟 Plus précis : donne l'ID du nœud fautif directement
            ));
        }
    };

    // 2. Exact allocation to prevent vector resizing during the loop.
    let capacity = children.len().saturating_sub(skip_count);
    let mut terms = Vec::with_capacity(capacity);

    // 3. Optimized term collection.
    for &arg_id in children.iter().skip(skip_count) {
        // 🌟 Appel direct au store pour récupérer le nœud de l'argument
        let arg_node = store.fetch(arg_id)?;

        let term = match arg_node.kind() {
            ExprKind::Variable(v_id) => Term::Variable(*v_id),
            ExprKind::Object(obj_id) => Term::Constant(*obj_id),
            _ => {
                return Err(DatalogError::invalid_atom_argument(arg_id));
            }
        };
        terms.push(term);
    }

    Ok(Atom::new(skeleton_id, terms))
}

/// Collects all unique variables from a slice of atoms and returns them as a sorted vector.
///
/// This function identifies every `VariableId` present in the terms of the provided atoms
/// and produces a compact, deduplicated list.
///
/// # Logic and Implementation
/// This function uses a **Bitset** (a `u64` mask) to perform a "Union" operation of all
/// variables in a single pass.
/// 1. It iterates through all terms of all atoms.
/// 2. For each variable, it sets the corresponding bit in a 64-bit integer.
/// 3. It then extracts the set bits to reconstruct the `VariableId` list.
///
/// # Performance
/// - **Time Complexity**: $O(T + V)$, where $T$ is the total number of terms across all atoms
///   and $V$ is the number of unique variables. The bitwise extraction is extremely fast
///   thanks to CPU-level instructions.
/// - **Space Complexity**: $O(V)$ for the resulting `Vec`. The internal bitset resides
///   entirely on the stack (`u64`).
/// - **Instruction Level Optimization**:
///     - Uses `count_ones()` (POP_CNT) to pre-allocate the exact capacity of the `Vec`,
///       preventing reallocations.
///     - Uses `trailing_zeros()` (TZ_CNT) to jump directly to the next set bit, avoiding
///       a full 64-iteration loop.
///
/// # Constraints & Safety
/// - **Variable Limit**: This implementation is strictly limited to **64 variables**
///   (indexed 0 to 63). This is a design trade-off to ensure the bitset fits into
///   a single CPU register.
/// - **Debug Assertions**: In debug builds, the function will panic if a `VariableId`
///   exceeds `MAX_VARS`. In release builds, it uses a modulo wrap-around to prevent
///   undefined behavior, though this would indicate a logic error in the caller.
///
/// # Returns
/// A `Vec<VariableId>` sorted by ID in ascending order (due to the nature of bit-scanning).
fn collect_variables(
    atoms: &[Atom],
    current_aliases: &HashMap<VariableId, Term>, // 💡 Ajouté ici pour propager
) -> Result<Vec<VariableId>, DatalogError> {
    // 💡 Transmis ici à local_collect_mask
    let mask = collect_mask(atoms, current_aliases);
    Ok(mask_to_vars(mask))
}

/// Version locale (associée) pour collecter le masque binaire des variables utilisées
fn collect_mask(
    atoms: &[Atom],
    current_aliases: &std::collections::HashMap<VariableId, Term>, // 💡 Type aligné sur ta fonction !
) -> u64 {
    let mut mask: u64 = 0;
    for atom in atoms {
        for term in atom.terms() {
            // AJOUT : Résolution systématique via ta fonction locale à 2 paramètres
            let resolved_term = match term {
                Term::Variable(v) => resolve_var(*v, current_aliases),
                Term::Constant(_) => term.clone(),
            };

            if let Term::Variable(v) = resolved_term {
                mask |= 1 << v.as_usize();
            }
        }
    }
    mask
}

/// Résout une variable vers son représentant canonique (le plus petit ID du groupe d'égalité)
#[inline(always)]
fn resolve_var(
    v: VariableId,
    current_aliases: &std::collections::HashMap<VariableId, Term>, // 💡 Injecté à la place de self
) -> Term {
    // Si la fermeture a bien aplati la map, un seul get suffit.
    current_aliases
        .get(&v)
        .cloned()
        .unwrap_or(Term::Variable(v))
}

fn mask_to_vars(mut mask: u64) -> Vec<VariableId> {
    let mut vars = Vec::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let bit = mask.trailing_zeros();
        vars.push(VariableId::from(bit as usize));
        mask &= mask - 1; // 💡 Efface le bit de poids faible mis à 1
    }
    vars
}

/// Creates a new auxiliary atom and registers its skeleton locally.
///
/// This method is a support part of the **Skolemization** process during flattening.
/// It generates a unique predicate ID for a sub-formula and maps the provided
/// variables to their respective types based on the action's parameter list.
///
/// # Arguments
/// * `skeleton_vars` - The subset of variables that will become the terms of this auxiliary atom.
/// * `resolved_terms` - The final evaluated terms to pack into the resulting atom.
/// * `parameters` - The master list of typed variables from the current action/context
///   used to resolve the types of `skeleton_vars`.
/// * `store` - The global unique expressions arena used to intern the newly created signature.
///
/// # Returns
/// A new [`Atom`] configured with an auxiliary [`AtomSkeletonId`] and variable terms.
///
/// # Performance
/// - **ID Management**: Increments an internal counter in $O(1)$.
/// - **Type Resolution**: Direct $O(1)` lookup per variable using the `parameters` list.
/// - **Hash-Consing Allocation**: Interns `aux_params` into the `ExprStore`. If the signature
fn create_aux_atom(
    skeleton_vars: Vec<VariableId>,
    resolved_terms: Vec<Term>,
    parameters_id: TypedListId, // 🌟 Mis à jour : passage par ID
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    store: &mut ExprStore,
) -> Atom {
    let id = *next_aux_id;
    *next_aux_id += 1;

    // 🌟 Récupération temporaire de la liste originale pour lire les types
    let parameters = store
        .fetch_typed_list(parameters_id)
        .expect("Valid TypedListId");

    let mut aux_params = TypedList::new();
    for &v_id in &skeleton_vars {
        let ty = parameters[v_id.as_usize()].ty();
        aux_params.push(TypedSymbol::new(v_id, ty.clone()));
    }

    // --- On interne la liste brute dans l'arène globale ---
    let list_id = store.intern_typed_list(aux_params);

    // Enregistrement avec le TypedListId conforme au nouveau modèle
    aux_defs.push(AtomicFormulaSkeleton::new(
        PredicateSymbolId::from(id),
        list_id,
    ));

    Atom::new(AtomSkeletonId::from(id), resolved_terms)
}

/// Version locale (associée) pour calculer la fermeture transitive (Path Compression)
fn compute_transitive_closure(aliases: &mut std::collections::HashMap<VariableId, Term>) {
    let keys: Vec<VariableId> = aliases.keys().cloned().collect();

    for start_var in keys {
        // On récupère le terme cible initial
        let mut current_term = aliases.get(&start_var).unwrap().clone();
        let mut visited = std::collections::HashSet::new();
        visited.insert(start_var);

        // On suit la chaîne des variables aliasées
        while let Term::Variable(v) = current_term {
            if let Some(next_term) = aliases.get(&v) {
                // Sécurité anti-cycle (ex: v1 = v2 et v2 = v1)
                if !visited.insert(v) {
                    break;
                }
                current_term = next_term.clone();
            } else {
                break;
            }
        }

        // On "aplatit" la structure (Path Compression)
        if let Some(alias) = aliases.get_mut(&start_var) {
            *alias = current_term;
        }
    }
}
