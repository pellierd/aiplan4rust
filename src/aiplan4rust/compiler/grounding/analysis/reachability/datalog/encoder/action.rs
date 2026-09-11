//! PDDL action definition encoder for Datalog compilation.
//!
//! This module provides the compilation pipeline to translate domain action definitions
//! into logical Datalog rules, managing action signature naming, precondition extraction,
//! structural inertia analysis, parameter type-anchoring, bootstrap handling, and effect encoding.

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    ActionSymbolId, AtomSkeletonId, TypeId, TypedList, VariableId,
};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::core::atom::AtomArgs;
use crate::analysis::reachability::datalog::core::rule::RuleBody;
use crate::analysis::reachability::datalog::encoder;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
use crate::analysis::reachability::datalog::state::DatalogState;

/// Compiles all PDDL domain action definitions into logical Datalog rules.
///
/// This merged function iterates through each action, extracts its signature, compiles its body
/// (preconditions), handles parameterless actions (bootstrap), and translates its effects.
///
/// # Arguments
///
/// * `ctx` - The base translation context.
/// * `state` - A mutable reference to the compilation state.
/// * `action_effects` - A mutable vector accumulating the encoded effects for each action.
/// * `action_defs` - A slice of PDDL action definitions to compile.
/// * `action_base_id` - The base skeleton ID offset for actions.
/// * `scratchpad` - A mutable scratchpad for temporary traversal data.
/// * `store` - A mutable reference to the expression store.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `DatalogError` if an error occurs during encoding.
pub(crate) fn encode_action_defs(
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    action_effects: &mut Vec<Vec<(Atom, Cause)>>,
    action_defs: &[ActionDef],
    action_base_id: usize,
    scratchpad: &mut DatalogScratchpad,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    for (id, action) in action_defs.iter().enumerate() {
        // ID segmentation: compute the Datalog skeleton ID for this action
        let action_sk_id = AtomSkeletonId::from(action_base_id + id);
        let action_index = id;

        // A. Generate the name atom (Pivot: action(?p1, ?p2...))
        let action_atom = encode_action_name(action, action_sk_id, store)?;

        // 🌟 Locally update the context with the current action's parameters
        let action_ctx = DatalogContext {
            param_list_id: action.parameters(),
            ..ctx
        };

        // B. Generate the trigger rule (Preconditions -> Action)
        encode_action_body(
            action_ctx,
            state,
            action,
            action_atom.clone(),
            scratchpad,
            store,
        )?;

        // --- ACTION BOOTSTRAP ---
        // If the action has no parameters and an empty body, it is immediately applicable.
        let param_list_id = action.parameters();
        if let Some(trigger_rule) = state.rules.last() {
            if trigger_rule.body().is_empty() && store.fetch_typed_list(param_list_id)?.is_empty() {
                state.db.insert_delta_fact(action_sk_id, &[]);
            }
        }

        // C. Extract and encode the action's effects
        // 🌟 Call updated with the localized context and unified state
        encoder::expr::encode_effects(
            action.effect(),
            &action_atom,
            action_index,
            action_ctx,
            state,
            action_effects,
            scratchpad,
            store,
        )?;
    }

    Ok(())
}

/// Encodes the action name into a primary n-ary Datalog atom based on its parameter signature.
///
/// # Arguments
///
/// * `action` - The PDDL action definition being processed.
/// * `action_sk_id` - The atom skeleton identifier assigned to this action.
/// * `store` - A mutable reference to the expression store holding problem definitions.
///
/// # Returns
///
/// Returns the n-ary `Atom` representing the action name with its variables, or a `DatalogError` on failure.
fn encode_action_name(
    action: &ActionDef,
    action_sk_id: AtomSkeletonId,
    store: &mut ExprStore,
) -> Result<Atom, DatalogError> {
    // 1. Retrieve the parameter list identifier from the action definition
    let param_list_id = action.parameters();

    // 2. Extract the concrete parameter list from the problem store
    let parameters = store.fetch_typed_list(param_list_id)?;

    // OPTIMIZATION: Direct accumulation into the Atom's native SmallVec to avoid heap allocation
    let mut head_terms = AtomArgs::with_capacity(parameters.len());

    for param in parameters.iter() {
        head_terms.push(Term::Variable(param.symbol()));
    }

    // Use the n-ary constructor without heap transit
    Ok(Atom::nary(action_sk_id, head_terms))
}

/// Compiles the body (preconditions and anchor rules) of a given action definition
/// into Datalog rules and appends them to the compilation state.
///
/// # Arguments
///
/// * `context` - The current global and local Datalog translation context.
/// * `state` - A mutable reference to the compilation state tracking rules and databases.
/// * `action` - The PDDL action definition being compiled.
/// * `head` - The primary Datalog atom representing the action's name/signature.
/// * `scratchpad` - A mutable scratchpad used for temporary traversal memory.
/// * `store` - A mutable reference to the expression store holding problem expressions.
///
/// # Returns
///
/// Returns `Ok(())` upon successful encoding, or a `DatalogError` if an invalid expression or argument is encountered.
fn encode_action_body(
    context: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    action: &ActionDef,
    head: Atom,
    scratchpad: &mut DatalogScratchpad,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // 1. Retrieve the parameter list ID from the action definition
    let param_list_id = action.parameters();

    // Borrow checker safety: capture the store length before immutable borrowing via fetch_expr
    let store_len = store.len();

    // Update context with the action's specific parameter list for precondition encoding
    let precond_ctx = DatalogContext {
        param_list_id,
        ..context
    };

    // Encode preconditions using the unified context and state
    let precond_opt = encoder::expr::encode_preconditions(
        action.precondition(),
        precond_ctx,
        state,
        scratchpad,
        store,
    )?;

    // 2. Initialize rule bodies for the final rule and anchor elements
    let mut final_action_body = RuleBody::new();
    let mut anchor_elements = RuleBody::new();

    // ==========================================================
    // TRAVERSAL GUARD (INERTIAL ATOM EXTRACTION)
    // ==========================================================
    scratchpad.prepare_visited(store_len);

    let precondition = store.fetch_expr(action.precondition())?;
    // Fetch atomic formulas in post-order traversal
    let atoms = precondition
        .postorder()
        .references()
        .filter(|node| matches!(node.kind(), ExprKind::AtomicFormula(_)));

    for atom_node in atoms {
        let idx = atom_node.id().as_usize();
        if scratchpad.visited[idx] {
            continue;
        }
        scratchpad.visited[idx] = true;

        let skel_id = match atom_node.kind() {
            ExprKind::AtomicFormula(sk) => *sk,
            _ => unreachable!(),
        };

        let positive_id = skel_id.strip_negation();

        if context
            .inertia_table
            .is_predicate_positive_negative_inertia(positive_id)?
        {
            let children = atom_node.children();

            // OPTIMIZATION: Direct stack accumulation via SmallVec
            let mut terms = AtomArgs::with_capacity(children.len().saturating_sub(1));

            for &term_id in children.iter().skip(1) {
                let term_node = precondition.fetch_node(term_id)?;

                let term = match term_node.kind() {
                    ExprKind::Variable(var_id) => {
                        let var_id = *var_id;
                        Term::Variable(var_id)
                    }
                    ExprKind::Object(obj_id) => Term::Constant(*obj_id),
                    _ => {
                        return Err(DatalogError::invalid_atom_argument(term_id));
                    }
                };
                terms.push(term);
            }

            // 🚀 OPTIMIZATION: Build n-ary static atom without intermediate heap allocation
            let static_atom = Atom::nary(skel_id, terms);
            anchor_elements.push(static_atom);
        }
    }

    // ==========================================================
    // PARAMETER TYPE-ANCHORING
    // ==========================================================
    // Fetch local parameters and build their unary type-anchoring atoms
    let parameters = store.fetch_typed_list(param_list_id)?;

    for (i, param) in parameters.iter().enumerate() {
        let var_id = VariableId::from(i);
        let var_term = Term::Variable(var_id);
        let type_id = param.ty().members()[0].as_usize();
        let type_sk = context.type_to_skeleton[type_id];
        anchor_elements.push(Atom::unary(type_sk, var_term));
    }

    // 4. Generate the anchor rule and final rule bodies
    if !anchor_elements.is_empty() {
        // 💡 Create the anchor head and register the intermediate anchor rule
        let anchor_head = create_anchor_atom(action.name(), parameters, &head, state.next_aux_id);

        state
            .rules
            .push(Rule::new(anchor_head.clone(), anchor_elements));
        final_action_body.push(anchor_head);
    }

    if let Some(p_atom) = precond_opt {
        final_action_body.push(p_atom);
    }

    state.rules.push(Rule::new(head, final_action_body));

    Ok(())
}

/// Creates an anchor atom for an action rule based on its header.
///
/// # Arguments
///
/// * `_action_id` - The symbol identifier of the action (retained for future debugging and naming).
/// * `_parameters` - The typed parameter list of the action (retained for future structural context).
/// * `action_head` - The reference atom representing the head of the action.
/// * `next_aux_id` - A mutable reference to the auxiliary ID counter used for allocation.
///
/// # Returns
///
/// Returns a new n-ary `Atom` acting as the anchor for the action rule.
fn create_anchor_atom(
    _action_id: ActionSymbolId, // Retained for future debugging/naming
    _parameters: &TypedList<VariableId, TypeId>,
    action_head: &Atom,
    next_aux_id: &mut usize,
) -> Atom {
    // 1. Allocate a new auxiliary ID using the local counter
    let anchor_id = *next_aux_id;
    *next_aux_id += 1;

    let sk_id = AtomSkeletonId::from(anchor_id);

    // 2. OPTIMIZATION: Extract and clone terms directly into a SmallVec
    let terms = AtomArgs::from_slice(action_head.arguments());

    // 3. Create the anchor atom via the n-ary constructor
    Atom::nary(sk_id, terms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId, ExprStore};
    use crate::aiplan4rust::compiler::lir::problem::action::Action;
    use crate::aiplan4rust::compiler::lir::problem::ActionDef;
    use crate::aiplan4rust::support::lang::{
        ActionSymbolId, AtomSkeletonId, Type, TypedListId, VariableId,
    };
    use crate::analysis::inertia::inertia::Inertia;
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::context::DatalogContext;
    use crate::analysis::reachability::datalog::core::Database;
    use crate::analysis::reachability::datalog::encoder::aliasing::new_alias_table;
    use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
    use crate::analysis::reachability::datalog::state::DatalogState;
    use rustc_hash::FxHashMap;

    /// Tests the generation of anchor atoms used in rule optimization.
    ///
    /// # Objective
    /// Verifies that the anchor atom creation helper correctly assigns the next available auxiliary skeleton ID,
    /// increments the auxiliary ID counter, and properly clones the arguments from the action head atom.
    ///
    /// # Inputs
    /// - An initial auxiliary ID counter (`10`).
    /// - An action head unary atom containing a variable term.
    /// - An empty parameter list and an action symbol ID.
    ///
    /// # Expected Output
    /// Returns an anchor atom whose symbol matches the expected auxiliary skeleton ID (`10`), increments
    /// the auxiliary ID to `11`, and retains the exact arity and arguments of the action head.
    #[test]
    fn test_create_anchor_atom_generation() {
        let mut next_aux_id = 10;
        let head = Atom::unary(AtomSkeletonId::from(1), Term::Variable(VariableId::from(0)));
        let parameters = TypedList::new();

        let anchor = create_anchor_atom(
            ActionSymbolId::from(1),
            &parameters,
            &head,
            &mut next_aux_id,
        );

        // Fixed: use .symbol() instead of .skeleton_id()
        assert_eq!(anchor.symbol(), AtomSkeletonId::from(10));
        assert_eq!(next_aux_id, 11);
        assert_eq!(anchor.arguments().len(), head.arguments().len());
        assert_eq!(anchor.arguments()[0], head.arguments()[0]);
    }

    /// Tests action definition encoding with an empty action list.
    ///
    /// # Objective
    /// Verifies that encoding an empty set of action definitions processes cleanly without errors
    /// and leaves the state rules collection empty.
    ///
    /// # Inputs
    /// - An empty vector of action definitions (`ActionDef`).
    /// - An empty type skeleton list and an empty inertia table.
    /// - Properly initialized state and context components.
    ///
    /// # Expected Output
    /// Returns `Ok(())` and ensures that the state rules remain empty.
    #[test]
    fn test_encode_action_defs_empty() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        // Proper initialization for DatalogState and DatalogContext components
        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        let mut action_effects = Vec::new();
        let action_defs: Vec<ActionDef> = Vec::new();

        let type_to_skeleton = Vec::new();
        let inertia_table = InertiaTable::empty();
        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result = encode_action_defs(
            ctx,
            &mut state,
            &mut action_effects,
            &action_defs,
            0,
            &mut scratchpad,
            &mut store,
        );

        assert!(result.is_ok());
        assert!(state.rules.is_empty());
    }

    /// Tests action name extraction and atom creation from action parameters.
    ///
    /// # Objective
    /// Verifies that the action name encoder correctly builds an n-ary atom matching the action's skeleton ID,
    /// mapping its parameters directly to variable terms with the correct arity.
    ///
    /// # Inputs
    /// - An action definition with a registered parameter list containing a single typed variable.
    /// - A specified target action skeleton ID (`77`).
    ///
    /// # Expected Output
    /// Returns `Ok(Atom)` where the atom's symbol equals the expected skeleton ID, its arity is 1, and its argument matches the parameter variable.
    #[test]
    fn test_encode_action_name_extraction() {
        let mut store = ExprStore::new();
        let action_sk_id = AtomSkeletonId::from(77);
        let mut action = ActionDef::default();

        let mut parameters = TypedList::new();
        parameters.push(crate::aiplan4rust::support::lang::TypedSymbol::new(
            VariableId::from(0),
            Type::primitive(0),
        ));
        let param_list_id = store.intern_typed_list(parameters);
        action.set_parameters(param_list_id);

        let result = encode_action_name(&action, action_sk_id, &mut store);
        assert!(result.is_ok());

        let atom = result.unwrap();
        assert_eq!(atom.symbol(), action_sk_id);
        assert_eq!(atom.arity(), 1);
        assert_eq!(atom.arguments()[0], Term::Variable(VariableId::from(0)));
    }

    /// Tests action body encoding with parameters and type anchors.
    ///
    /// # Objective
    /// Verifies that action parameters correctly trigger type anchor generation and are successfully encoded alongside the final action rule.
    ///
    /// # Inputs
    /// - An action definition with a valid empty precondition and a registered parameter list containing a primitive type.
    /// - A type skeleton mapping matching the parameter type.
    /// - An empty inertia table.
    ///
    /// # Expected Output
    /// Returns `Ok(())` and ensures that rules are successfully added to the state.
    #[test]
    fn test_encode_action_body_with_parameters_and_anchors() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        let mut action = ActionDef::default();

        // 1. Define a valid precondition (e.g., an empty and / true)
        action.set_precondition(store.empty_and());

        // 2. Define valid parameters registered in the store
        let mut parameters = TypedList::new();
        parameters.push(crate::aiplan4rust::support::lang::TypedSymbol::new(
            VariableId::from(0),
            Type::primitive(0),
        ));
        let param_list_id = store.intern_typed_list(parameters);
        action.set_parameters(param_list_id);

        let head = Atom::unary(AtomSkeletonId::from(1), Term::Variable(VariableId::from(0)));
        let type_to_skeleton = vec![AtomSkeletonId::from(50)];
        let inertia_table = InertiaTable::empty();

        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result =
            encode_action_body(ctx, &mut state, &action, head, &mut scratchpad, &mut store);

        assert!(result.is_ok());
        assert!(!state.rules.is_empty());
    }

    /// Tests the bootstrap scenario for parameterless actions during definition encoding.
    ///
    /// # Objective
    /// Verifies that actions with no parameters and trivial conditions are immediately applicable and correctly bootstrapped as delta facts into the database.
    ///
    /// # Inputs
    /// - A snapshot action definition with no parameters and `ExprId::TRUE` conditions.
    /// - An empty type skeleton list and an empty inertia table.
    ///
    /// # Expected Output
    /// Executes successfully without error (`Ok(())`), completing the bootstrap process for the action.
    #[test]
    fn test_encode_action_defs_bootstrap() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id: usize = 0;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        let mut action_effects = Vec::new();

        let typed_list = builder.typed_variable_list(vec![]);

        // Using ExprId::TRUE (index 0) to respect the alias table of size 2
        let action = Action::new_snap(
            ActionSymbolId::from(0),
            typed_list,
            ExprId::TRUE,
            ExprId::TRUE,
        );
        let action_defs = vec![action];

        let type_to_skeleton = Vec::new();
        let inertia_table = InertiaTable::empty();
        let ctx = DatalogContext::new(typed_list, &type_to_skeleton, 0, &inertia_table);

        let result = encode_action_defs(
            ctx,
            &mut state,
            &mut action_effects,
            &action_defs,
            0,
            &mut scratchpad,
            &mut store,
        );

        match result {
            Ok(_) => (),
            Err(e) => panic!("Error in encode_action_defs: {:?}", e),
        }
    }

    /// Tests the action definition encoding flow with action effects.
    ///
    /// # Objective
    /// Verifies that the action effects vector is properly sized and processed alongside action definitions during the compilation flow.
    ///
    /// # Inputs
    /// - An action definition representing a snapshot action with trivial `TRUE` conditions and effects.
    /// - A pre-allocated action effects vector sized to match the number of action definitions.
    /// - An empty type skeleton list and an empty inertia table.
    ///
    /// # Expected Output
    /// Returns `Ok(())` and ensures that the action effects vector size matches the expected count for the encoded actions.
    #[test]
    fn test_encode_action_defs_with_effects_flow() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id: usize = 0;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        let typed_list = builder.typed_variable_list(vec![]);
        let action = Action::new_snap(
            ActionSymbolId::from(0),
            typed_list,
            ExprId::TRUE,
            ExprId::TRUE,
        );
        let action_defs = vec![action];

        // Pre-allocate the effects vector to the size of the number of actions
        let mut action_effects = vec![Vec::new(); action_defs.len()];

        let type_to_skeleton = Vec::new();
        let inertia_table = InertiaTable::empty();
        let ctx = DatalogContext::new(typed_list, &type_to_skeleton, 0, &inertia_table);

        let result = encode_action_defs(
            ctx,
            &mut state,
            &mut action_effects,
            &action_defs,
            0,
            &mut scratchpad,
            &mut store,
        );

        assert!(result.is_ok());
        // Verifies that the action effects vector has been properly sized/populated for the encoded action
        assert_eq!(action_effects.len(), 1);
    }

    /// Tests action body encoding when no anchors are required (empty parameters and empty preconditions).
    ///
    /// # Objective
    /// Verifies that action bodies lacking parameters and static-inert predicates skip anchor generation
    /// and correctly append only the final action rule to the state.
    ///
    /// # Inputs
    /// - An action definition with an empty precondition (`empty_and`) and an empty parameter list.
    /// - An empty type skeleton list and an empty inertia table.
    ///
    /// # Expected Output
    /// Returns `Ok(())` and ensures that exactly one rule (the final action rule) is added to the state rules without any intermediate anchors.
    #[test]
    fn test_encode_action_body_without_anchors() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        let mut action = ActionDef::default();
        action.set_precondition(store.empty_and());

        // Empty parameters to avoid automatic type anchor addition
        let param_list_id = store.intern_typed_list(TypedList::new());
        action.set_parameters(param_list_id);

        let head = Atom::nary(AtomSkeletonId::from(1), AtomArgs::new());
        let type_to_skeleton = Vec::new();
        let inertia_table = InertiaTable::empty();

        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result =
            encode_action_body(ctx, &mut state, &action, head, &mut scratchpad, &mut store);

        assert!(result.is_ok());
        // No anchor generated, a single final rule added
        assert_eq!(state.rules.len(), 1);
    }

    /// Tests error propagation when an invalid term expression is encountered as an atom argument.
    ///
    /// # Objective
    /// Verifies that trying to encode an action body with an invalid term (e.g., an internal `And` expression)
    /// inside a static-inert atom's arguments correctly triggers and propagates a `DatalogError`.
    ///
    /// # Inputs
    /// - An action definition with a precondition containing an atomic formula (ID 10) that holds a valid term followed by an invalid term expression.
    /// - An empty parameter list.
    /// - An inertia table where predicate 10 is configured as `Inertia::positive_negative()`.
    ///
    /// # Expected Output
    /// Returns a `Result::Err`, indicating that error propagation handled the invalid argument successfully.
    #[test]
    fn test_encode_action_body_invalid_atom_argument_error() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        // Dummy term at index 0 (skipped by .skip(1)) and invalid expression as the target argument at index 1
        let dummy_term = store.intern(
            ExprKind::Object(crate::aiplan4rust::support::lang::ObjectId::from(0)),
            &[],
        );
        let invalid_term_expr = store.intern(ExprKind::And, &[]);
        let atomic_formula = store.intern(
            ExprKind::AtomicFormula(AtomSkeletonId::from(10)),
            &[dummy_term, invalid_term_expr],
        );

        let mut action = ActionDef::default();
        action.set_precondition(atomic_formula);

        let param_list_id = store.intern_typed_list(TypedList::new());
        action.set_parameters(param_list_id);

        let head = Atom::nary(AtomSkeletonId::from(1), AtomArgs::new());
        let type_to_skeleton = Vec::new();

        // Activate inertia to force reading the atom's children
        let mut inertia_table = InertiaTable::empty();
        inertia_table.insert_predicate(AtomSkeletonId::from(10), Inertia::positive_negative());

        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result =
            encode_action_body(ctx, &mut state, &action, head, &mut scratchpad, &mut store);

        assert!(result.is_err());
    }

    /// Tests action body encoding when both action parameters and a static-inert atom are present in the precondition.
    ///
    /// # Objective
    /// Verifies that both parameter-based types and static-inert atoms are correctly extracted and combined
    /// to generate an intermediate anchor rule alongside the final action rule.
    ///
    /// # Inputs
    /// - An action definition with a registered parameter list and a precondition containing a static-inert atomic formula (ID 20).
    /// - An inertia table where predicate 20 is configured as `Inertia::positive_negative()`.
    /// - A type skeleton mapping for parameter type anchoring.
    ///
    /// # Expected Output
    /// Returns `Ok(())` and populates the state rules with at least two rules (the generated anchor rule and the final rule).
    #[test]
    fn test_encode_action_body_with_parameters_and_static_inertia() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        // Static-inert atom in the precondition combined with a parameter
        let dummy_term = store.intern(
            ExprKind::Object(crate::aiplan4rust::support::lang::ObjectId::from(0)),
            &[],
        );
        let static_atom_expr = store.intern(
            ExprKind::AtomicFormula(AtomSkeletonId::from(20)),
            &[dummy_term],
        );
        let precondition = store.intern(ExprKind::And, &[static_atom_expr]);

        let mut action = ActionDef::default();
        action.set_precondition(precondition);

        let mut parameters = TypedList::new();
        parameters.push(crate::aiplan4rust::support::lang::TypedSymbol::new(
            VariableId::from(0),
            Type::primitive(0),
        ));
        let param_list_id = store.intern_typed_list(parameters);
        action.set_parameters(param_list_id);

        let head = Atom::unary(AtomSkeletonId::from(1), Term::Variable(VariableId::from(0)));
        let type_to_skeleton = vec![AtomSkeletonId::from(50)];

        let mut inertia_table = InertiaTable::empty();
        inertia_table.insert_predicate(AtomSkeletonId::from(20), Inertia::positive_negative());

        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result =
            encode_action_body(ctx, &mut state, &action, head, &mut scratchpad, &mut store);

        assert!(result.is_ok());
        assert!(state.rules.len() >= 2);
    }

    /// Tests action body encoding with a precondition containing a fluent (non-inert) predicate.
    ///
    /// # Objective
    /// Verifies that a fluent atom in the precondition is correctly handled—bypassing static anchor
    /// generation while successfully incorporating into the final rule body—without returning an error.
    ///
    /// # Inputs
    /// - An action definition with a precondition containing a fluent atomic formula (ID 30).
    /// - An empty parameter list.
    /// - An inertia table where predicate 30 is explicitly configured as `Inertia::fluent()`.
    ///
    /// # Expected Output
    /// Returns `Ok(())`, ensuring the encoding pipeline completes successfully.
    #[test]
    fn test_encode_action_body_with_fluent_precondition() {
        let mut store = ExprStore::new();
        let mut scratchpad = DatalogScratchpad::with_capacity(32);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut current_aliases = new_alias_table();

        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        // Fluent atom in the precondition (inertia set to false)
        let dummy_term = store.intern(
            ExprKind::Object(crate::aiplan4rust::support::lang::ObjectId::from(0)),
            &[],
        );
        let fluent_atom_expr = store.intern(
            ExprKind::AtomicFormula(AtomSkeletonId::from(30)),
            &[dummy_term],
        );

        let mut action = ActionDef::default();
        action.set_precondition(fluent_atom_expr);

        let param_list_id = store.intern_typed_list(TypedList::new());
        action.set_parameters(param_list_id);

        let head = Atom::nary(AtomSkeletonId::from(1), AtomArgs::new());
        let type_to_skeleton = Vec::new();

        let mut inertia_table = InertiaTable::empty();
        // Explicitly force the predicate to FLUENT (neither positive nor negative inert)
        inertia_table.insert_predicate(AtomSkeletonId::from(30), Inertia::fluent());

        let ctx = DatalogContext::new(TypedListId::default(), &type_to_skeleton, 0, &inertia_table);

        let result =
            encode_action_body(ctx, &mut state, &action, head, &mut scratchpad, &mut store);

        assert!(result.is_ok());
    }
}
