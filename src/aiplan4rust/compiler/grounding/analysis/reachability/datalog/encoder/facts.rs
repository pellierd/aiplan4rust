//! Datalog Initialization and Database Population Module.
//!
//! This module provides utility functions for initializing and populating the core
//! Datalog data structures during the compilation phase of planning problems.
//!
//! # Core Responsibilities
//!
//! * **Type Mapping (`declare_type_defs`):** Translates PDDL domain types into unique
//!   auxiliary [`AtomSkeletonId`]s and appends a sentinel `ROOT` typing skeleton for
//!   top-level object classification.
//! * **Action Declaration (`declare_action_defs`):** Assigns unique Datalog predicate
//!   identifiers to domain actions sequentially, enabling $O(1)$ decoding during applicability checks.
//! * **Static Object Typing (`fill_db_from_objects`):** Populates the stable Datalog
//!   database with object typing facts across direct types, ancestor types, and the universal `ROOT` type.
//! * **Initial State Ingestion (`fill_db_from_init`):** Traverses initial state expression trees
//!   (AST) to extract atomic facts and ingest them into the Datalog delta database.
//!
//! # Invariants and Guarantees
//!
//! - **Monotonic Allocation:** ID allocation relies on a monotonically increasing counter
//!   stored within [`DatalogState`], ensuring identifiers are globally unique.
//! - **DAG Safety:** Traversal algorithms (such as AST ingestion) maintain visitation bit-sets
//!   to handle shared nodes within Directed Acyclic Graphs without redundant processing.
//! - **Infallibility:** Declarative setup functions operate without runtime errors, while
//!   AST ingestion validates object parameters against type structures.

use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, TypeId, TypedSymbol,
};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::state::DatalogState;

/// Encodes domain types into unique Datalog atom skeletons and appends a sentinel ROOT typing.
///
/// This function establishes a direct 1:1 mapping between PDDL domain type indices
/// and auxiliary Datalog skeleton IDs. It allocates skeleton IDs sequentially using
/// the monotonically increasing auxiliary counter within `state`.
///
/// Additionally, it appends a sentinel skeleton for the `ROOT` (universal) type at the
/// very last position of `type_to_skeleton` to simplify top-level object classification.
///
/// # Arguments
///
/// * `type_defs` - A slice of domain type definitions (`TypedSymbol`). Its length determines
///   how many sequential domain skeleton IDs will be generated.
/// * `type_to_skeleton` - A mutable vector populated with the allocated `AtomSkeletonId`s.
///   It is cleared implicitly via reallocation with a capacity of `type_defs.len() + 1`.
/// * `state` - The active [`DatalogState`], whose `next_aux_id` counter is read and incremented
///   for each registered type, ensuring globally unique auxiliary identifiers.
///
/// # Returns
///
/// This function returns `()` (unit). It operates via side-effects by modifying
/// `type_to_skeleton` and mutating `state.next_aux_id`.
///
/// # Errors
///
/// This function does not perform operations that fail at runtime and is guaranteed
/// not to return an error (infallible).
///
/// # Performance
///
/// - **Time Complexity:** $\mathcal{O}(N)$, where $N$ is the number of domain types.
/// - **Space Complexity:** $\mathcal{O}(N)$ allocation for `type_to_skeleton`.
pub(crate) fn declare_type_defs(
    type_defs: &[TypedSymbol<TypeId, TypeId>],
    type_to_skeleton: &mut Vec<AtomSkeletonId>,
    state: &mut DatalogState<'_>,
) {
    // Get total number of domain type definitions
    let num_types = type_defs.len();

    // Allocate memory ahead of time for N domain types and 1 ROOT sentinel
    *type_to_skeleton = Vec::with_capacity(num_types + 1);

    // Encode domain types sequentially to ensure direct index mapping
    for _ in 0..num_types {
        // Build skeleton ID using current state counter
        let sk_id = AtomSkeletonId::from(*state.next_aux_id);

        // Advance state counter for next allocation
        *state.next_aux_id += 1;

        // Store domain skeleton ID
        type_to_skeleton.push(sk_id);
    }

    // Allocate sentinel skeleton ID for universal ROOT typing
    let root_type_sk = AtomSkeletonId::from(*state.next_aux_id);

    // Advance state counter after ROOT sentinel allocation
    *state.next_aux_id += 1;

    // Append ROOT sentinel as the last element
    type_to_skeleton.push(root_type_sk);
}

/// Iterates over and declares all domain action definitions in Datalog.
///
/// This function sequentially allocates a unique, monotonically increasing
/// Datalog predicate ID for each action. Representing action applicability
/// as relations enables O(1) decoding without hash table lookups.
///
/// # Arguments
///
/// * `action_defs` - A slice containing the domain action definitions (`ActionDef`).
/// * `state` - The active [`DatalogState`] where action predicate skeletons are
///   registered and whose `next_aux_id` counter is updated.
///
/// # Returns
///
/// Returns `Ok(())` on successful registration of all action definitions.
///
/// # Errors
///
/// This function is currently infallible and always returns `Ok(())`.
///
/// # Performance
///
/// - **Time Complexity:** O(N), where N is the number of action definitions.
/// - **Space Complexity:** O(N) memory allocations appended to `state.aux_defs`.
pub(crate) fn declare_action_defs(
    action_defs: &[ActionDef],
    state: &mut DatalogState<'_>,
) -> Result<(), DatalogError> {
    state.aux_defs.reserve(action_defs.len());
    // Process each action definition sequentially
    for action in action_defs {
        // Extract parameter list identifier from action
        let list_id = action.parameters();

        // Read current state counter as anchor ID
        let anchor_id = *state.next_aux_id;

        // Advance state counter for next allocation
        *state.next_aux_id += 1;

        // Map anchor ID to predicate symbol identifier
        let predicate_id = PredicateSymbolId::from(anchor_id);

        // Push new formula skeleton directly into state accumulator
        state
            .aux_defs
            .push(AtomicFormulaSkeleton::new(predicate_id, list_id));
    }

    Ok(())
}

/// Populates the static Datalog database with typing facts derived from domain objects.
///
/// This function processes every declared object and inserts its static typing facts
/// into the database. Each object is inserted into the universal `ROOT` sentinel
/// type, its declared direct type, and all corresponding ancestor/parent types.
///
/// # Arguments
///
/// * `ctx` - The contextual read-only reference containing the mapping from
///   type indices to skeleton identifiers (`type_to_skeleton`).
/// * `state` - The active mutable [`DatalogState`] containing the database (`db`)
///   where static facts are recorded.
/// * `object_defs` - A slice of defined objects associated with their respective types.
/// * `type_defs` - A slice of type definitions used to resolve hierarchical type inheritance.
///
/// # Returns
///
/// Returns `Ok(())` upon successfully inserting all object typing facts into the database.
///
/// # Errors
///
/// Returns [`DatalogError::InternalState`] if `ctx.type_to_skeleton` is empty and the
/// `ROOT` typing skeleton cannot be retrieved.
///
/// # Performance
///
/// - **Time Complexity:** O(O * (1 + T * P)), where O is the number of objects,
///   T is the average number of types per object, and P is the average number of parent types.
/// - **Space Complexity:** O(1) auxiliary allocation beyond database mutations.
pub(crate) fn fill_db_from_objects(
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    object_defs: &[TypedSymbol<ObjectId, TypeId>],
    type_defs: &[TypedSymbol<TypeId, TypeId>],
) -> Result<(), DatalogError> {
    // Retrieve ROOT sentinel typing skeleton ID from context
    let root_sk_id = *ctx
        .type_to_skeleton
        .last()
        .ok_or_else(|| DatalogError::internal_state("Root typing skeleton missing".to_string()))?;

    // Iterate over each declared domain object
    for object in object_defs {
        // Get unique object identifier symbol
        let obj_id = object.symbol();

        // Insert object into universal ROOT typing relation
        state.db.insert_stable_fact(root_sk_id, &[obj_id]);

        // Iterate over all direct types assigned to the object
        for &type_id in object.ty() {
            // Map type identifier to target skeleton ID
            let sk_id = ctx.type_to_skeleton[type_id.as_usize()];

            // Insert object into its direct typing relation
            state.db.insert_stable_fact(sk_id, &[obj_id]);

            // Resolve parent types and insert object into ancestor relations
            if let Some(ty_def) = type_defs.get(type_id.as_usize()) {
                // Iterate through all ancestor member types
                for &parent_id in ty_def.ty().members() {
                    // Map parent type identifier to skeleton ID
                    let parent_sk_id = ctx.type_to_skeleton[parent_id.as_usize()];

                    // Insert object into parent typing relation
                    state.db.insert_stable_fact(parent_sk_id, &[obj_id]);
                }
            }
        }
    }

    Ok(())
}

/// Ingests the initial problem state expression tree into the dynamic delta database.
///
/// This function performs a preorder traversal over the expression AST starting
/// from the root `init` expression node. It extracts all valid atomic formulas,
/// validates that their arguments are object symbols, and populates the Datalog
/// database with delta facts. Duplicate subtrees are skipped via a visitation bit-set.
///
/// # Arguments
///
/// * `state` - The active mutable [`DatalogState`] containing the target database (`db`).
/// * `init` - The root [`ExprId`] representing the initial state AST.
/// * `store` - A mutable reference to the [`ExprStore`] containing expression nodes.
///
/// # Returns
///
/// Returns `Ok(())` upon successfully ingesting all atomic initial state facts.
///
/// # Errors
///
/// * Returns [`DatalogError::InvalidAtomArgument`] if an atomic formula child node
///   is not a concrete object (e.g., a variable node).
/// * Returns an error if an expression node cannot be fetched from `store`.
///
/// # Performance
///
/// - **Time Complexity:** O(V + E), where V is the number of AST nodes and E is the
///   number of child edges traversed.
/// - **Space Complexity:** O(N) bit-set vector allocation where N is the total
///   number of entries in `store`.
pub(crate) fn fill_db_from_init(
    state: &mut DatalogState<'_>,
    init: ExprId,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // Initialize preorder iterator over expression tree starting at root init node
    let mut iter = store.preorder(init);

    // Track visited nodes using a direct boolean lookup vector indexed by ExprId
    let mut visited = vec![false; store.len()];

    // Traverse AST nodes in preorder
    while let Some((id, _depth, entry)) = iter.next() {
        // Convert node ExprId to array index
        let idx = id.as_usize();

        // Skip node if already processed in DAG traversal
        if visited[idx] {
            continue;
        }

        // Mark current node index as visited
        visited[idx] = true;

        // Wrap current store entry into expression node helper
        let node = ExprNode::new(id, entry);

        // Process node if it represents an atomic formula
        if let ExprKind::AtomicFormula(sk_id) = node.kind() {
            // Dereference skeleton identifier
            let sk_id = *sk_id;

            // Retrieve child expression identifiers
            let children = node.children();

            // Reserve vector capacity for argument object identifiers
            let mut args = Vec::with_capacity(children.len().saturating_sub(1));

            // Iterate over child argument nodes skipping predicate symbol at index 0
            for &arg_id in children.iter().skip(1) {
                // Fetch argument expression node from store
                let child_node = store.fetch(arg_id)?;

                // Verify argument node is a concrete object
                if let ExprKind::Object(object_id) = child_node.kind() {
                    // Push object identifier to argument list
                    args.push(*object_id);
                } else {
                    // Return error if argument node is not a concrete object
                    return Err(DatalogError::invalid_atom_argument(arg_id));
                }
            }

            // Insert extracted atomic formula into Datalog delta database
            state.db.insert_delta_fact(sk_id, &args);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
    use crate::aiplan4rust::support::lang::{
        ActionSymbolId, PredicateSymbolId, Type, TypedListId, VariableId,
    };
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::core::{Atom, Database, Rule};
    use crate::analysis::reachability::datalog::encoder::aliasing::new_alias_table;
    use crate::analysis::reachability::datalog::encoder::AliasTable;
    use rustc_hash::FxHashMap;

    /// Helper to instantiate a `DatalogState` for testing purposes.
    fn create_test_state<'a>(
        rules: &'a mut Vec<Rule>,
        cache: &'a mut FxHashMap<Vec<Atom>, Atom>,
        when_cache: &'a mut FxHashMap<[Atom; 2], Atom>, // 🌟 Ajout du paramètre
        aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
        next_aux_id: &'a mut usize,
        db: &'a mut Database,
        aliases: &'a mut AliasTable,
    ) -> DatalogState<'a> {
        DatalogState::new(rules, cache, when_cache, aux_defs, next_aux_id, db, aliases)
    }

    // =========================================================================
    // 1. TYPE AND ACTION DECLARATION TESTS
    // =========================================================================

    /// Objective: Verify that `declare_type_defs` sequentially allocates skeleton IDs
    ///            for domain types and appends the sentinel ROOT typing skeleton at the end.
    /// Input:
    ///   - `type_defs`: A slice of 2 domain type definitions.
    ///   - Initial `next_aux_id`: Set to `100`.
    /// Expected Output:
    ///   - `type_to_skeleton` contains 3 elements (2 domain types + 1 ROOT sentinel).
    ///   - Skeleton IDs mapped sequentially: 100, 101, and 102 for ROOT.
    ///   - `state.next_aux_id` is incremented to `103`.
    #[test]
    fn test_declare_type_defs_allocates_sequential_ids_and_root_sentinel() {
        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 100;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let type_defs = vec![
            TypedSymbol::new(TypeId::from(0), Type::primitive(TypeId::from(0))),
            TypedSymbol::new(TypeId::from(1), Type::primitive(TypeId::from(1))),
        ];
        let mut type_to_skeleton = Vec::new();

        declare_type_defs(&type_defs, &mut type_to_skeleton, &mut state);

        assert_eq!(type_to_skeleton.len(), 3);
        assert_eq!(type_to_skeleton[0], AtomSkeletonId::from(100));
        assert_eq!(type_to_skeleton[1], AtomSkeletonId::from(101));
        assert_eq!(type_to_skeleton[2], AtomSkeletonId::from(102));
        assert_eq!(*state.next_aux_id, 103);
    }

    /// Objective: Verify that `declare_action_defs` sequentially registers action
    ///            skeletons with unique predicate symbols into the state.
    /// Input:
    ///   - `action_defs`: A vector of 2 `Action` items constructed via `Action::new_snap`.
    ///   - Initial `next_aux_id`: Set to `50`.
    /// Expected Output:
    ///   - Returns `Ok(())`.
    ///   - `state.aux_defs` contains 2 registered atomic formula skeletons.
    ///   - Assigned predicate IDs are `50` and `51`.
    ///   - `state.next_aux_id` is updated to `52`.
    #[test]
    fn test_declare_action_defs_registers_skeletons() {
        let params_id = TypedListId::from(0);
        let dummy_expr = ExprId::from(0);

        let action_defs = vec![
            ActionDef::new_snap(ActionSymbolId::from(0), params_id, dummy_expr, dummy_expr),
            ActionDef::new_snap(ActionSymbolId::from(1), params_id, dummy_expr, dummy_expr),
        ];

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 50;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = declare_action_defs(&action_defs, &mut state);

        assert!(result.is_ok());
        assert_eq!(state.aux_defs.len(), 2);
        assert_eq!(state.aux_defs[0].symbol(), PredicateSymbolId::from(50));
        assert_eq!(state.aux_defs[1].symbol(), PredicateSymbolId::from(51));
        assert_eq!(*state.next_aux_id, 52);
    }

    // =========================================================================
    // 2. DATABASE POPULATION FROM OBJECTS TESTS
    // =========================================================================

    /// Objective: Verify that `fill_db_from_objects` correctly inserts static object facts
    ///            into the ROOT sentinel, the direct type, and parent hierarchy types.
    /// Input:
    ///   - `object_defs`: One object `ObjectId(7)` defined under `TypeId(0)`.
    ///   - `type_defs`: `TypeId(0)` inheriting from/having member `TypeId(1)`.
    ///   - `type_to_skeleton`: Mapped skeleton IDs `[10, 20, 999]` (where 999 is ROOT).
    /// Expected Output:
    ///   - Returns `Ok(())`.
    ///   - `state.db` contains stable facts for `ObjectId(7)` in ROOT skeleton (`999`),
    ///     direct type skeleton (`10`), and parent type skeleton (`20`).
    #[test]
    fn test_fill_db_from_objects_inserts_root_direct_and_parent_types() {
        let root_sk_id = AtomSkeletonId::from(999);
        let type0_sk_id = AtomSkeletonId::from(10);
        let type1_sk_id = AtomSkeletonId::from(20);

        let type_to_skeleton = vec![type0_sk_id, type1_sk_id, root_sk_id];

        let type_defs = vec![
            TypedSymbol::new(TypeId::from(0), Type::primitive(TypeId::from(1))),
            TypedSymbol::new(TypeId::from(1), Type::primitive(TypeId::from(1))),
        ];

        let obj0 = ObjectId::from(7);
        let object_defs = vec![TypedSymbol::new(obj0, Type::primitive(TypeId::from(0)))];

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let dummy_inertia = InertiaTable::default();
        let ctx = DatalogContext {
            type_to_skeleton: &type_to_skeleton,
            inertia_table: &dummy_inertia,
            negation_offset: 0,
            param_list_id: TypedListId::from(0),
        };

        let result = fill_db_from_objects(ctx, &mut state, &object_defs, &type_defs);
        assert!(result.is_ok());

        assert!(state.db.contains_stable(root_sk_id, &[obj0]));
        assert!(state.db.contains_stable(type0_sk_id, &[obj0]));
        assert!(state.db.contains_stable(type1_sk_id, &[obj0]));
    }

    // =========================================================================
    // 3. DATABASE POPULATION FROM INITIAL STATE TESTS
    // =========================================================================

    /// Objective: Verify that `fill_db_from_init` extracts atomic formulas from the initial
    ///            expression tree and populates the delta database facts with their arguments.
    /// Input:
    ///   - An initial AST expression containing an `AtomicFormula` with `AtomSkeletonId(42)`
    ///     and children `[ObjectId(100), ObjectId(101)]`.
    /// Expected Output:
    ///   - Returns `Ok(())`.
    ///   - `state.db` contains a delta fact with skeleton `42` and arguments `[100, 101]`.
    #[test]
    fn test_fill_db_from_init_populates_delta_facts() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sk_id = AtomSkeletonId::from(42);
        let pred_sym_id = PredicateSymbolId::from(1);
        let obj1 = ObjectId::from(100);
        let obj2 = ObjectId::from(101);

        let expr_obj1 = builder.object(obj1);
        let expr_obj2 = builder.object(obj2);

        let init_expr_id = builder.atomic_formula(pred_sym_id, &[expr_obj1, expr_obj2], sk_id);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = fill_db_from_init(&mut state, init_expr_id, builder.store);
        assert!(result.is_ok());

        assert!(state.db.contains_delta(sk_id, &[obj1, obj2]));
    }

    /// Objective: Verify that `fill_db_from_init` returns an error when encountering
    ///            a non-object node (e.g., a variable) inside an atomic formula argument.
    /// Input:
    ///   - An initial AST expression containing an `AtomicFormula` with an argument of type `VariableId`.
    /// Expected Output:
    ///   - Returns `Err(DatalogError::InvalidAtomArgument { .. })`.
    ///   - Ingestion terminates immediately without writing corrupt facts to the database.
    #[test]
    fn test_fill_db_from_init_returns_error_on_invalid_argument() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sk_id = AtomSkeletonId::from(42);
        let pred_sym_id = PredicateSymbolId::from(1);
        let var_id = VariableId::from(0);

        let expr_var = builder.variable(var_id);
        let init_expr_id = builder.atomic_formula(pred_sym_id, &[expr_var], sk_id);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = fill_db_from_init(&mut state, init_expr_id, builder.store);

        assert!(result.is_err());
        match result.unwrap_err() {
            DatalogError::InvalidAtomArgument { .. } => {}
            other => panic!("Expected InvalidAtomArgument, got {:?}", other),
        }
    }

    // =========================================================================
    // 4. EDGE CASES & ERROR PATHS TESTS
    // =========================================================================

    /// Objective: Verify that `fill_db_from_objects` fails cleanly with a state error
    ///            when `type_to_skeleton` is empty (missing ROOT sentinel).
    /// Input:
    ///   - An empty `type_to_skeleton` vector.
    /// Expected Output:
    ///   - Returns `Err(DatalogError::InternalState { .. })`.
    #[test]
    fn test_fill_db_from_objects_fails_on_empty_skeleton_mapping() {
        let type_to_skeleton = vec![];
        let type_defs = vec![];
        let obj0 = ObjectId::from(7);
        let object_defs = vec![TypedSymbol::new(obj0, Type::primitive(TypeId::from(0)))];

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let dummy_inertia = InertiaTable::default();
        let ctx = DatalogContext {
            type_to_skeleton: &type_to_skeleton,
            inertia_table: &dummy_inertia,
            negation_offset: 0,
            param_list_id: TypedListId::from(0),
        };

        let result = fill_db_from_objects(ctx, &mut state, &object_defs, &type_defs);

        assert!(result.is_err());
    }

    /// Objective: Verify that `declare_type_defs` still allocates the ROOT sentinel
    ///            even if the input `type_defs` list is empty.
    /// Input:
    ///   - An empty slice `&[]` for `type_defs`.
    /// Expected Output:
    ///   - `type_to_skeleton` contains exactly 1 element (the ROOT sentinel).
    ///   - `next_aux_id` is incremented by 1.
    #[test]
    fn test_declare_type_defs_with_empty_types_creates_root_sentinel_only() {
        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let mut type_to_skeleton = Vec::new();

        declare_type_defs(&[], &mut type_to_skeleton, &mut state);

        assert_eq!(type_to_skeleton.len(), 1);
        assert_eq!(type_to_skeleton[0], AtomSkeletonId::from(0));
        assert_eq!(*state.next_aux_id, 1);
    }

    /// Objective: Verify that `fill_db_from_init` correctly skips already visited AST
    ///            nodes without re-processing or corrupting the Datalog state.
    /// Input:
    ///   - An AST where an expression is evaluated/traversed twice (DAG graph pattern).
    /// Expected Output:
    ///   - `fill_db_from_init` succeeds with `Ok(())`.
    ///   - The fact is ingested properly without duplicate side-effects.
    #[test]
    fn test_fill_db_from_init_skips_already_visited_nodes() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sk_id = AtomSkeletonId::from(10);
        let pred_sym_id = PredicateSymbolId::from(1);
        let obj = ObjectId::from(5);

        let expr_obj = builder.object(obj);
        let atom_expr_id = builder.atomic_formula(pred_sym_id, &[expr_obj], sk_id);

        // Root referencing the same atomic formula twice (e.g. AND node or shared DAG sub-tree)
        let root_expr_id = builder.and(&[atom_expr_id, atom_expr_id]);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = fill_db_from_init(&mut state, root_expr_id, builder.store);

        assert!(result.is_ok());
        assert!(state.db.contains_delta(sk_id, &[obj]));
    }

    // =========================================================================
    // 5. ADDITIONAL AST & TYPING EDGE CASES
    // =========================================================================

    /// Objective: Verify that `fill_db_from_init` extracts atomic formulas nested
    ///            inside complex AST operators (like AND/NOT) while ignoring the operators.
    /// Input:
    ///   - An AST with an `AND` node containing an `AtomicFormula`.
    /// Expected Output:
    ///   - `fill_db_from_init` traverses past the operator node and registers the atomic fact.
    #[test]
    fn test_fill_db_from_init_ignores_non_atomic_expression_nodes() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sk_id = AtomSkeletonId::from(15);
        let pred_sym_id = PredicateSymbolId::from(2);
        let obj = ObjectId::from(9);

        let expr_obj = builder.object(obj);
        let atom_expr = builder.atomic_formula(pred_sym_id, &[expr_obj], sk_id);

        // Wrap inside a logical operator
        let root_expr = builder.and(&[atom_expr]);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = fill_db_from_init(&mut state, root_expr, builder.store);

        assert!(result.is_ok());
        assert!(state.db.contains_delta(sk_id, &[obj]));
    }

    /// Objective: Verify that `fill_db_from_init` supports 0-arity (nullary) atomic formulas.
    /// Input:
    ///   - An `AtomicFormula` with 0 object arguments.
    /// Expected Output:
    ///   - A delta fact with an empty argument slice `&[]` is added to the database.
    #[test]
    fn test_fill_db_from_init_handles_zero_arity_atoms() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sk_id = AtomSkeletonId::from(88);
        let pred_sym_id = PredicateSymbolId::from(3);

        // Atomic formula without arguments
        let init_expr_id = builder.atomic_formula(pred_sym_id, &[], sk_id);

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let result = fill_db_from_init(&mut state, init_expr_id, builder.store);

        assert!(result.is_ok());
        assert!(state.db.contains_delta(sk_id, &[]));
    }

    /// Objective: Verify that `fill_db_from_objects` gracefully handles objects whose
    ///            type definition index is missing from `type_defs`.
    /// Input:
    ///   - An object with `TypeId(5)`, but `type_defs` is empty.
    /// Expected Output:
    ///   - Fact inserted into ROOT and direct skeleton, no panic on parent lookup.
    #[test]
    fn test_fill_db_from_objects_handles_unmapped_parent_types() {
        let root_sk_id = AtomSkeletonId::from(999);
        let type0_sk_id = AtomSkeletonId::from(10);

        let type_to_skeleton = vec![type0_sk_id, root_sk_id];
        let empty_type_defs = vec![];

        let obj0 = ObjectId::from(3);
        let object_defs = vec![TypedSymbol::new(obj0, Type::primitive(TypeId::from(0)))];

        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::new();
        let mut aliases = new_alias_table();

        let mut state = create_test_state(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut aliases,
        );

        let dummy_inertia = InertiaTable::default();
        let ctx = DatalogContext {
            type_to_skeleton: &type_to_skeleton,
            inertia_table: &dummy_inertia,
            negation_offset: 0,
            param_list_id: TypedListId::from(0),
        };

        let result = fill_db_from_objects(ctx, &mut state, &object_defs, &empty_type_defs);

        assert!(result.is_ok());
        assert!(state.db.contains_stable(root_sk_id, &[obj0]));
        assert!(state.db.contains_stable(type0_sk_id, &[obj0]));
    }
}
