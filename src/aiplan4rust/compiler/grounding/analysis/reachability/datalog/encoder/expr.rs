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
use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
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
    scratchpad: &mut DatalogScratchpad,
    store: &mut ExprStore,
) -> Result<Option<Atom>, DatalogError> {
    // 1. Nettoyage et préparation des alias (pour gérer les ?x = ?y)
    // 🌟 Remplissage direct de l'état mutable pour que `encode_expr` puisse y accéder !
    *state.current_aliases = aliasing::compute_variable_aliasing(expr, scratchpad, store)?;

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
    scratchpad: &mut DatalogScratchpad,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // ÉTAPE 1 : Remplissage direct de la table d'alias dans l'état partagé
    *state.current_aliases = aliasing::compute_variable_aliasing(effect, scratchpad, store)?;

    let mut root_cause = action_atom.clone();
    for term in root_cause.arguments_mut() {
        if let Term::Variable(v) = *term {
            *term = aliasing::find(v, state.current_aliases);
        }
    }

    let root_cause_id = root_cause.symbol();
    // On utilise directement l'id de départ pour la pile
    let mut work_stack = vec![(effect, root_cause)];
    scratchpad.prepare_effects_visited();

    while let Some((current_id, current_cause)) = work_stack.pop() {
        if !scratchpad
            .visited_effects
            .insert((current_id, current_cause.symbol()))
        {
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
                        *term = aliasing::find(v, state.current_aliases);
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

                if let Some(cond_atom) = encode_condition(condition_id, ctx, state, store)? {
                    let mut combined_body = [current_cause.clone(), cond_atom.clone()];
                    combined_body.sort_by_key(|a| a.symbol());

                    // 🌟 Utilisation de when_cache ici
                    let aux_when_atom =
                        if let Some(existing_head) = state.when_cache.get(&combined_body) {
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
                            state.when_cache.insert(combined_body, head.clone());
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
                                *term = aliasing::find(v, state.current_aliases);
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
/// This function serves as the internal core engine used by both `encode_preconditions`
/// and `encode_effects`. It performs an iterative post-order traversal starting at `expr`
/// to flatten complex logical structures (such as AND/OR combinations) into auxiliary predicates
/// using a Tseitin-like transformation.
///
/// # Key Execution Steps
/// 1. **Traversal & Memoization Check**: Utilizes a work stack and an indexed memoization table
///    to evaluate expressions efficiently and avoid redundant computations.
/// 2. **Descent & Validation (Phase 1)**: Traverses down the expression tree, validating node kinds
///    and pushing children onto the stack. Unsupported nodes or invalid negations immediately
///    trigger an error.
/// 3. **Synthesis & Fact Extraction (Phase 2)**: Reconstructs results bottom-up, processing
///    atomic formulas, logical operators, comparisons, and temporal wrappers while integrating
///    variable aliasing and auxiliary predicate allocation.
///
/// # Arguments
///
/// * `expr` - The starting expression identifier representing the root of the sub-tree.
/// * `ctx` - The Datalog context containing immutable configuration such as parameter lists and type mappings.
/// * `state` - The mutable compiler state accumulating generated rules, the cache, and active variable aliases.
/// * `store` - The expression store containing the global tree nodes.
///
/// # Returns
///
/// * `Ok(Some(Atom))` - The head atom representing the encoded sub-expression.
/// * `Ok(None)` - If the branch evaluates to empty logical content (e.g., empty AND or ignored nodes).
/// * `Err(DatalogError)` - If an invalid node, stack inconsistency, or unsupported feature is encountered.
pub fn encode_condition(
    expr: ExprId,
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    store: &mut ExprStore,
) -> Result<Option<Atom>, DatalogError> {
    // Initialize the iterative work stack with the root expression and its unvisited state.
    let mut work_stack = vec![(expr, false)];
    // Stack to accumulate the resulting atoms during bottom-up synthesis.
    let mut results_stack: Vec<Option<Atom>> = Vec::with_capacity(32);

    // Fast O(1) vector-based memoization table mapping expression IDs to cached results.
    let mut memo: Vec<Option<Option<Atom>>> = vec![None; store.len()];

    while let Some((current_id, visited)) = work_stack.pop() {
        let idx = current_id.as_usize();

        // Phase 1 check: if not yet marked as visited, verify the memoization cache first.
        if !visited {
            if let Some(cached_result) = &memo[idx] {
                results_stack.push(cached_result.clone());
                continue;
            }
        }

        // Direct store fetch for the current expression node.
        let node = store.fetch(current_id)?;
        let kind = node.kind();

        if !visited {
            // --- PHASE 1: DESCENT & VALIDATION ---
            match kind {
                // Validate and push negation nodes (ensuring they wrap equality comparisons).
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

                // Traverse logical connectors and temporal scopes by pushing children in reverse order.
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

                // Leaf nodes that require no child traversal.
                ExprKind::AtomicFormula(_)
                | ExprKind::Comparison(_)
                | ExprKind::Arithmetic(_)
                | ExprKind::Number(_) => {
                    work_stack.push((current_id, true));
                }

                _ => return Err(DatalogError::incompatible_node(kind.clone(), current_id)),
            }
        } else {
            // --- PHASE 2: SYNTHESIS ---
            let num_children = node.children().len();
            let result = match kind {
                // Extract and normalize atomic formulas with variable alias resolution.
                ExprKind::AtomicFormula(skeleton_id) => {
                    let mut atom = predicate::extract_atom(node, store)?;
                    for term in atom.arguments_mut() {
                        if let Term::Variable(v) = *term {
                            // Resolve via shared mutable state aliases.
                            *term = aliasing::find(v, state.current_aliases);
                        }
                    }
                    if skeleton_id.is_negated() {
                        let mut sk = atom.symbol();
                        sk.set_negated(true);
                        atom.set_symbol(sk);
                    }
                    Some(atom)
                }

                // Evaluate negation on the synthesized child result.
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

                // Synthesize conjunctions, handling equality fusion and auxiliary predicate allocation.
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
                                // Allocate auxiliary predicate with unified fields for complex conjunctions.
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

                // Synthesize disjunctions, generating Horn clauses and auxiliary predicates via Tseitin transformation.
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
                            let (head, _) = predicate::allocate_auxiliary_predicate(
                                &atoms,
                                ctx.param_list_id,
                                state.next_aux_id,
                                state.aux_defs,
                                ctx.type_to_skeleton,
                                state.current_aliases,
                                store,
                            )?;

                            // Retrieve type list once outside the loop for optimal performance.
                            let parameters = store.fetch_typed_list(ctx.param_list_id)?;

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

                // Process comparison expressions (focusing on equality handling).
                ExprKind::Comparison(op) => {
                    if *op == CompareOp::Equal {
                        let mut atom = predicate::extract_atom(node, store)?;

                        for term in atom.arguments_mut() {
                            if let Term::Variable(v) = *term {
                                // Resolve via shared mutable state aliases.
                                *term = aliasing::find(v, state.current_aliases);
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

                // Unwrap temporal scope wrappers.
                ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                    if num_children > 0 {
                        results_stack.pop().flatten()
                    } else {
                        None
                    }
                }
                // Ignore arithmetic and numeric literals for logical reachability constraints.
                ExprKind::Arithmetic(_) | ExprKind::Number(_) => None,
                _ => None,
            };

            // Store the computed result in the memoization table and push onto the results stack.
            memo[idx] = Some(result.clone());
            results_stack.push(result);
        }
    }
    Ok(results_stack.pop().flatten())
}

#[cfg(test)]
mod encoding_tests {
    use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{
        AtomSkeletonId, CompareOp, Type, TypedListId, TypedSymbol, VariableId,
    };
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::context::DatalogContext;
    use crate::analysis::reachability::datalog::core::{Atom, Database, Term};
    use crate::analysis::reachability::datalog::encoder::expr::{encode_condition, encode_effects};
    use crate::analysis::reachability::datalog::error::DatalogError;
    use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
    use crate::analysis::reachability::datalog::state::DatalogState;

    /// # Objective
    /// Verify the behavior of conjunction (`And`) builder methods with empty, single, and multiple elements,
    /// ensuring correct neutral fallback, direct element unwrapping, and interned `And` node creation.
    ///
    /// # Input
    /// - An expression store and builder initializing variables for various conjunction cases.
    ///
    /// # Expected Output
    /// - Assertions pass, verifying that empty conjunctions return the neutral true node, single-element conjunctions return the element directly, and multi-operand conjunctions produce an `And` expression node.
    /// Test: Conjunction (And) behavior with empty, single, and multiple elements,
    /// along with structural flattening and identity handling.
    #[test]
    fn test_encode_condition_conjunction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let q = builder.variable(VariableId::from(2));

        // 1. Empty conjunction -> Neutral True (empty_and)
        let empty_conj = builder.and(&[]);
        assert_eq!(
            empty_conj,
            builder.empty_and(),
            "An empty conjunction must return the neutral True node."
        );

        // 2. Single element conjunction -> Returns the element itself
        let single_conj = builder.and(&[p]);
        assert_eq!(
            single_conj, p,
            "A conjunction with a single element must return that element directly."
        );

        // 3. Multiple elements conjunction -> Interned And node
        let multi_conj = builder.and(&[p, q]);
        let node = builder.get(multi_conj).unwrap();
        assert!(
            matches!(node.kind(), ExprKind::And),
            "A multi-operand conjunction must produce an And expression node."
        );
    }

    /// # Objective
    /// Verify the behavior of disjunction (`Or`) builder methods with empty, single, and multiple elements,
    /// ensuring correct neutral fallback, direct element unwrapping, and interned `Or` node creation.
    ///
    /// # Input
    /// - An expression store and builder initializing variables and comparison expressions for various disjunction cases.
    ///
    /// # Expected Output
    /// - Assertions pass, verifying that empty disjunctions return the neutral false node, single-element disjunctions return the element directly, and multi-operand disjunctions produce an `Or` expression node.
    /// Test: Disjunction (Or) behavior with empty, single, and multiple elements,
    /// validating auxiliary creation paths and basic branching.
    #[test]
    fn test_encode_condition_disjunction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let p = builder.comparison(CompareOp::Equal, v1, v2);

        // 1. Empty disjunction -> Neutral False (empty_or)
        let empty_disj = builder.or(&[]);
        assert_eq!(
            empty_disj,
            builder.empty_or(),
            "An empty disjunction must return the neutral False node."
        );

        // 2. Single element disjunction -> Returns the element itself
        let single_disj = builder.or(&[p]);
        assert_eq!(
            single_disj, p,
            "A disjunction with a single element must return that element directly."
        );

        // 3. Multiple elements disjunction -> Interned Or node
        let v3 = builder.variable(VariableId::from(3));
        let q = builder.comparison(CompareOp::Equal, v1, v3);
        let multi_disj = builder.or(&[p, q]);
        let node = builder.get(multi_disj).unwrap();
        assert!(
            matches!(node.kind(), ExprKind::Or),
            "A multi-operand disjunction must produce an Or expression node."
        );
    }

    /// # Objective
    /// Verify that negating an equality comparison correctly generates a `Not` expression node,
    /// and that applying a double negation simplifies back to the original expression via identity reduction.
    ///
    /// # Input
    /// - An expression store containing an equality comparison, its single negation, and its double negation.
    ///
    /// # Expected Output
    /// - Assertions pass, verifying the `Not` node kind and the successful cancellation of double negation.
    /// Test: Negation (Not) handling, exclusive term management,
    /// and double negation cancellation (identity reduction).
    #[test]
    fn test_encode_condition_negation_and_simplification() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let eq = builder.comparison(CompareOp::Equal, v1, v2);

        // 1. Standard negation creation
        let not_eq = builder.not(eq);
        let not_node = builder.get(not_eq).unwrap();
        assert!(
            matches!(not_node.kind(), ExprKind::Not),
            "Applying not to a comparison must generate a Not expression node."
        );

        // 2. Double negation elimination (NOT NOT EQ -> EQ)
        let double_not = builder.not(not_eq);
        assert_eq!(
            double_not, eq,
            "A double negation must simplify back to the original expression via rule reduction."
        );
    }

    /// # Objective
    /// Verify that encoding a conjunction (`And`) condition expression involving equality constraints
    /// correctly fuses or filters them and succeeds without error.
    ///
    /// # Input
    /// - An expression store containing a conjunction of two comparison equality expressions used as a condition.
    /// - State, context, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Ok(...)` indicating successful conjunction encoding and equality constraint fusion.
    /// Test: Conjunction equality constraints fusion and filtering.
    #[test]
    fn test_encode_condition_conjunction_equality_fusion() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));

        let eq1 = builder.comparison(CompareOp::Equal, v1, v1);
        let eq2 = builder.comparison(CompareOp::Equal, v1, v2);

        let conj = builder.and(&[eq1, eq2]);

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let dummy_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: dummy_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(conj, ctx, &mut state, &mut store);
        assert!(
            result.is_ok(),
            "Conjunction with equality fusion should encode successfully."
        );
    }

    /// # Objective
    /// Verify that encoding a disjunction (`Or`) condition expression correctly creates auxiliary
    /// predicate definitions and propagates type constraints for multi-branch disjunctions.
    ///
    /// # Input
    /// - An expression store containing a disjunction of two comparison expressions used as a condition.
    /// - State, context, typed variable list, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Ok(Some(...))` and a non-empty auxiliary definitions vector (`state.aux_defs`).
    /// Test: Disjunction auxiliary predicate creation and type constraint propagation.
    #[test]
    fn test_encode_condition_disjunction_auxiliary_and_type_constraints() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let v3 = builder.variable(VariableId::from(3));

        let p = builder.comparison(CompareOp::Equal, v1, v2);
        let q = builder.comparison(CompareOp::Equal, v1, v3);

        let disj = builder.or(&[p, q]);

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let next_aux_id = 0;
        let mut next_aux_cell = next_aux_id;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_cell,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(10),
            AtomSkeletonId::from(20),
            AtomSkeletonId::from(30),
        ];

        // Use TypedSymbol::new and Type::default() constructor
        // to match the actual TypedSymbol<SID, TID> definition
        let typed_symbols = vec![
            TypedSymbol::new(VariableId::from(0), Type::default()),
            TypedSymbol::new(VariableId::from(1), Type::default()),
            TypedSymbol::new(VariableId::from(2), Type::default()),
            TypedSymbol::new(VariableId::from(3), Type::default()),
        ];
        let typed_list = builder.typed_variable_list(typed_symbols);

        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(disj, ctx, &mut state, &mut store);
        assert!(
            result.is_ok(),
            "Disjunction encoding with auxiliary predicate generation must succeed."
        );
        assert!(
            !state.aux_defs.is_empty(),
            "An auxiliary predicate definition should be generated for multi-branch disjunctions."
        );
    }

    /// # Objective
    /// Verify that encoding a temporal `AtStart` condition expression correctly unwraps its child
    /// and successfully returns an encoded atom.
    ///
    /// # Input
    /// - An expression store containing an `AtStart` temporal wrapper around a comparison expression used as a condition.
    /// - State, context, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Ok(Some(...))` containing the unwrapped condition atom.
    #[test]
    fn test_encode_condition_temporal_at_start() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let comp = builder.comparison(CompareOp::Equal, v1, v2);
        let temporal_expr = builder
            .at_start(comp)
            .expect("Failed to create at_start expression");

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let dummy_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: dummy_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(temporal_expr, ctx, &mut state, &mut store);
        assert!(result.is_ok(), "Temporal condition encoding must succeed.");
        assert!(
            result.unwrap().is_some(),
            "Temporal condition should unwrap and return an atom."
        );
    }

    /// # Objective
    /// Verify that encoding an arithmetic expression as a condition succeeds gracefully
    /// and evaluates to `None` (ignoring it in condition contexts).
    ///
    /// # Input
    /// - An expression store containing an arithmetic addition expression used as a condition.
    /// - State, context, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Ok(None)` indicating the arithmetic condition is safely ignored/skipped.
    #[test]
    fn test_encode_condition_arithmetic_ignored() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create an arithmetic expression via the builder
        let n1 = builder.number(1.0);
        let n2 = builder.number(2.0);
        let arith_expr = builder.add(&[n1, n2]);

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let dummy_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: dummy_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(arith_expr, ctx, &mut state, &mut store);
        println!("ERREUR RECUE : {:?}", result); // <-- Keep debugging print
        assert!(
            result.is_ok(),
            "Arithmetic condition encoding should succeed."
        );
        assert!(
            result.unwrap().is_none(),
            "Arithmetic expressions in conditions must evaluate to None."
        );
    }

    /// # Objective
    /// Verify that negating something other than an equality construct (such as an atomic formula)
    /// during condition encoding correctly returns a `FeatureNotSupported` error.
    ///
    /// # Input
    /// - An expression store containing an unsupported negation expression used as a condition.
    /// - State, context, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Err(DatalogError::FeatureNotSupported { .. })`
    #[test]
    fn test_encode_condition_unsupported_negation_error() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Negation of something other than an equality (e.g., an atomic formula or similar)
        let atom = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));
        let not_expr = builder.not(atom);

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let dummy_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: dummy_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(not_expr, ctx, &mut state, &mut store);
        assert!(
            matches!(result, Err(DatalogError::FeatureNotSupported { .. })),
            "Negation of non-equality constructs must return FeatureNotSupported error."
        );
    }

    /// # Objective
    /// Verify that an incompatible expression node type (such as an assignment)
    /// encountered during condition encoding correctly triggers a `DatalogError::IncompatibleNode` error.
    ///
    /// # Input
    /// - An expression store containing an incompatible assignment expression used as a condition.
    /// - State, context, and expression store.
    ///
    /// # Expected Output
    /// - `Result::Err(DatalogError::IncompatibleNode { .. })`
    #[test]
    fn test_encode_condition_incompatible_node_error() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Pass an assignment node which is not allowed in conditions
        let target = builder.variable(VariableId::from(1));
        let value = builder.number(1.0);
        let assign_expr = builder.assign(target, value);

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let dummy_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: dummy_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_condition(assign_expr, ctx, &mut state, &mut store);
        assert!(
            matches!(result, Err(DatalogError::IncompatibleNode { .. })),
            "Incompatible expression nodes in conditions must return IncompatibleNode error."
        );
    }

    /// # Objective
    /// Verify that encoding a basic atomic formula effect expression correctly extracts the atom,
    /// applies variable aliasing, and successfully accumulates it into the action effects vector.
    ///
    /// # Input
    /// - An expression store containing an atomic formula effect expression.
    /// - An action atom with arguments, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and a non-empty action effects vector containing the generated atomic effect.
    #[test]
    fn test_encode_effects_atomic() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let effect_expr = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));

        let action_atom = Atom::nary(
            AtomSkeletonId::from(1),
            vec![Term::Variable(VariableId::from(1))].into(),
        );
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            effect_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(result.is_ok(), "Atomic effect encoding must succeed.");
        assert!(
            !action_effects[0].is_empty(),
            "Action effects accumulator should contain the generated effect."
        );
    }

    /// # Objective
    /// Verify that encoding a conditional `When` effect expression correctly evaluates the condition,
    /// allocates necessary auxiliary predicates or handles caching, and succeeds without error.
    ///
    /// # Input
    /// - An expression store containing a conditional `When` effect expression.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` indicating successful encoding of the conditional effect.
    #[test]
    fn test_encode_effects_conditional_when() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));

        let condition = builder.comparison(CompareOp::Equal, v1, v2);
        let effect = builder.comparison(CompareOp::Equal, v1, v1);
        let when_expr = builder.when(condition, effect);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(10),
            AtomSkeletonId::from(20),
        ];
        let typed_list = builder.typed_variable_list(vec![
            TypedSymbol::new(VariableId::from(1), Type::default()),
            TypedSymbol::new(VariableId::from(2), Type::default()),
        ]);

        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            when_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            result.is_ok(),
            "Conditional 'When' effect encoding must succeed."
        );
    }

    /// # Objective
    /// Verify that encoding a deletion effect wrapped in a `Not` expression succeeds without error.
    ///
    /// # Input
    /// - An expression store containing a negated effect expression.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` indicating successful encoding of the deletion effect.
    #[test]
    fn test_encode_effects_deletion_not() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let comparison = builder.comparison(CompareOp::Equal, v1, v2);
        let not_effect = builder.not(comparison);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            not_effect,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(result.is_ok(), "Deletion effect encoding must succeed.");
    }

    /// # Objective
    /// Verify that encoding a temporal `AtStart` effect expression correctly unwraps its child
    /// and successfully accumulates the extracted effect into the action effects vector.
    ///
    /// # Input
    /// - An expression store containing an `AtStart` temporal wrapper around an atomic formula effect.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and a non-empty action effects vector containing the unwrapped effect.
    #[test]
    fn test_encode_effects_temporal_at_start() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let atom = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));
        let temporal_expr = builder
            .at_start(atom)
            .expect("Failed to create at_start temporal expression");

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            temporal_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(result.is_ok(), "Temporal effect encoding must succeed.");
        assert!(
            !action_effects[0].is_empty(),
            "Temporal effect should unwrap and be accumulated."
        );
    }

    /// # Objective
    /// Verify that encoding a temporal `AtEnd` effect expression correctly unwraps its child
    /// and successfully accumulates the extracted effect into the action effects vector.
    ///
    /// # Input
    /// - An expression store containing an `AtEnd` temporal wrapper around an atomic formula effect.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and a non-empty action effects vector containing the unwrapped effect.
    #[test]
    fn test_encode_effects_temporal_at_end() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let atom = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));
        let temporal_expr = builder
            .at_end(atom)
            .expect("Failed to create at_end temporal expression");

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            temporal_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            result.is_ok(),
            "Temporal at_end effect encoding must succeed."
        );
        assert!(
            !action_effects[0].is_empty(),
            "Temporal at_end effect should unwrap and be accumulated."
        );
    }

    /// # Objective
    /// Verify that encoding a temporal `Overall` effect expression correctly unwraps its child
    /// and successfully accumulates the extracted effect into the action effects vector.
    ///
    /// # Input
    /// - An expression store containing an `Overall` temporal wrapper around an atomic formula effect.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and a non-empty action effects vector containing the unwrapped effect.
    #[test]
    fn test_encode_effects_temporal_overall() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let atom = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));
        let temporal_expr = builder
            .overall(atom)
            .expect("Failed to create overall temporal expression");

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            temporal_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            result.is_ok(),
            "Temporal overall effect encoding must succeed."
        );
        assert!(
            !action_effects[0].is_empty(),
            "Temporal overall effect should unwrap and be accumulated."
        );
    }

    /// # Objective
    /// Verify that encoding a conjunction (`And`) effect expression correctly traverses
    /// both child effects and extracts them into the action effects vector.
    ///
    /// # Input
    /// - An expression store containing a conjunction (`And`) of two atomic formula effects.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and an action effects vector containing exactly two extracted effects.
    #[test]
    fn test_encode_effects_conjunction_and() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let atom1 = builder.atomic_formula(1, &[], AtomSkeletonId::from(1));
        let atom2 = builder.atomic_formula(2, &[], AtomSkeletonId::from(2));
        let and_expr = builder.and(&[atom1, atom2]);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            and_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            result.is_ok(),
            "Conjunction of effects encoding must succeed."
        );
        assert_eq!(
            action_effects[0].len(),
            2,
            "Both effects inside the 'And' block should be extracted."
        );
    }

    /// # Objective
    /// Verify that assignment and arithmetic expressions encountered during effect encoding
    /// are ignored gracefully without returning an error and without generating discrete effects.
    ///
    /// # Input
    /// - An expression store containing an assignment expression.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Ok(())` and an empty action effects vector.
    #[test]
    fn test_encode_effects_assignment_and_arithmetic_ignored() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let target = builder.variable(VariableId::from(1));
        let value = builder.number(5.0);
        let assign_expr = builder.assign(target, value);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            assign_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            result.is_ok(),
            "Assignment expression encoding should be skipped gracefully."
        );
        assert!(
            action_effects[0].is_empty(),
            "Assignments should not generate any discrete effects."
        );
    }

    /// # Objective
    /// Verify that an incompatible expression node type (such as a comparison)
    /// encountered during effect encoding correctly triggers a `DatalogError::IncompatibleNode` error.
    ///
    /// # Input
    /// - An expression store containing an incompatible comparison expression.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Err(DatalogError::IncompatibleNode { .. })`
    #[test]
    fn test_encode_effects_incompatible_node_error() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Use a comparison (e.g., v1 == v2) which is not a valid effect
        // and will therefore trigger the default case (_) in encode_effects.
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let incompatible_expr = builder.comparison(CompareOp::Equal, v1, v2);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![];
        let typed_list = builder.typed_variable_list(vec![]);
        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            incompatible_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            matches!(result, Err(DatalogError::IncompatibleNode { .. })),
            "Incompatible expression nodes in effects must return IncompatibleNode error."
        );
    }

    /// # Objective
    /// Verify that encoding effect expressions containing an unsupported direct ADL construct
    /// (such as `Imply`) correctly returns a `FeatureNotSupported` error.
    ///
    /// # Input
    /// - An expression store containing an unsupported `Imply` expression.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Err(DatalogError::FeatureNotSupported { .. })`
    #[test]
    fn test_encode_effects_unsupported_adl_error() {
        let mut store = ExprStore::new();

        // Directly create sub-nodes via intern to avoid the builder and borrow checker issues
        let v1 = store.intern(ExprKind::Variable(VariableId::from(1)), &[]);
        let c1 = store.intern(ExprKind::Comparison(CompareOp::Equal), &[v1, v1]);

        // An Imply takes two children: premise and consequence
        let adl_expr = store.intern(ExprKind::Imply, &[c1, c1]);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let type_skeletons = vec![];
        let typed_list = TypedListId::default(); // or the valid empty equivalent in your context
        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            adl_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            matches!(result, Err(DatalogError::FeatureNotSupported { .. })),
            "Unsupported ADL constructs in effects must return FeatureNotSupported error."
        );
    }

    /// # Objective
    /// Verify that encoding effect expressions containing nested unsupported ADL constructs
    /// (such as `Imply`) correctly returns a `FeatureNotSupported` error.
    ///
    /// # Input
    /// - An expression store containing an unsupported `Imply` expression nested inside a `When` condition.
    /// - An action atom, default action effects vector, state, context, and a pre-allocated scratchpad.
    ///
    /// # Expected Output
    /// - `Result::Err(DatalogError::FeatureNotSupported { .. })`
    #[test]
    fn test_encode_effects_nested_unsupported_adl_error() {
        let mut store = ExprStore::new();

        // 1. Isolate the builder in a block so its mutable borrow of `store`
        // ends as soon as it leaves the block.
        let (typed_list, condition, c1) = {
            let mut builder = ExprBuilder::new(&mut store);

            let v1 = builder.variable(VariableId::from(1));
            let v2 = builder.variable(VariableId::from(2));

            let condition = builder.comparison(CompareOp::Equal, v1, v2);
            let c1 = builder.comparison(CompareOp::Equal, v1, v1);

            let typed_list = builder.typed_variable_list(vec![
                TypedSymbol::new(VariableId::from(1), Type::default()),
                TypedSymbol::new(VariableId::from(2), Type::default()),
            ]);

            (typed_list, condition, c1)
        }; // <-- End of builder lifetime, `store` is fully free again

        // 2. Now `store` is no longer borrowed, we can safely use `store.intern`.
        let imply_expr = store.intern(ExprKind::Imply, &[c1, c1]);
        let when_expr = store.intern(ExprKind::When, &[condition, imply_expr]);

        let action_atom = Atom::nary(AtomSkeletonId::from(1), Default::default());
        let mut action_effects = vec![vec![]];

        let mut aliases = [Term::Variable(VariableId::from(0)); 64];
        let mut rules = Vec::new();
        let mut cache = rustc_hash::FxHashMap::default();
        let mut when_cache = rustc_hash::FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        let mut state = DatalogState {
            db: &mut db,
            rules: &mut rules,
            cache: &mut cache,
            when_cache: &mut when_cache,
            aux_defs: &mut aux_defs,
            next_aux_id: &mut next_aux_id,
            current_aliases: &mut aliases,
        };

        let type_skeletons = vec![
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(10),
            AtomSkeletonId::from(20),
        ];

        let ctx = DatalogContext {
            param_list_id: typed_list,
            type_to_skeleton: &type_skeletons,
            inertia_table: &InertiaTable::empty(),
            negation_offset: 100,
        };

        let result = encode_effects(
            when_expr,
            &action_atom,
            0,
            ctx,
            &mut state,
            &mut action_effects,
            &mut scratchpad,
            &mut store,
        );

        assert!(
            matches!(result, Err(DatalogError::FeatureNotSupported { .. })),
            "Nested unsupported ADL constructs in effects must return FeatureNotSupported error."
        );
    }
}
