use std::collections::HashMap;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{ActionDefId, AtomSkeletonId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::tree::NodeId;

/// Maximum number of variables (parameters) allowed per action or rule.
///
/// This limit is set to 64 to allow high-performance variable tracking
/// using a single CPU register (u64 bitset).
const MAX_VARS: usize = 64;

/// A transformation engine that encodes complex PDDL formulas into Datalog rules.
///
/// The `DatalogEncoder` is responsible for **Normalization** and the final flattening
/// of logical connectives into Horn clauses.
///
/// # Pre-conditions (Crucial)
///
/// For this encoder to function correctly, the input [`Expr`] must have already passed
/// through the following transformation passes (see `crate::passes`):
///
/// 1. **Quantifier Expansion**: All `FORALL` and `EXISTS` nodes must be expanded into
///    their respective `AND`/`OR` equivalent grounded structures.
/// 2. **Type Flattening**: The PDDL type hierarchy must be flattened. Datalog operates
///    on flat sets; any complex inheritance must be resolved amont.
/// 3. **Object Fluent Flattening**: Functions (object fluents) must be converted into
///    relational predicates (e.g., `(at ?robot (location_of ?target))` -> `(at ?robot ?loc) ^ (is_at ?target ?loc)`).
///
/// # Internal Architecture
///
/// - **ID Space Separation**: Clearly partitions the predicate ID space into "Base"
///   (original PDDL domain) and "Auxiliary" (generated during encoding).
/// - **Structural Deduplication**: Uses a cache to ensure that logically identical
///   sub-formulas within the same context are mapped to the same auxiliary predicate,
///   minimizing the number of rules.
/// - **Schema Tracking**: Maintains a monotonic registry of generated signatures
///   ([`AtomicFormulaSkeleton`]), allowing the grounded results to be mapped back
///   to human-readable names after saturation.
pub struct DatalogEncoder {
    /// The starting offset for auxiliary predicate IDs.
    /// Typically set to the count of original predicates in the domain.
    base_aux_id: usize,
    /// Monotonic counter for generating the next unique auxiliary ID.
    next_aux_id: usize,
    /// Registry of auxiliary predicate signatures (skeletons).
    /// Used for debugging and reconstructing the logical state post-saturation.
    pub aux_defs: Vec<AtomicFormulaSkeleton>,
    /// Structural cache mapping a set of body atoms to a head atom.
    /// Prevents the redundant creation of multiple auxiliary predicates
    /// for the same logical sub-expression (Common Subexpression Elimination).
    cache: HashMap<Vec<Atom>, Atom>,
    /// Mapping: AtomSkeletonId (Datalog) -> ActionDefId (Index dans le Vec<LiftedAction>)
    action_map: HashMap<AtomSkeletonId, ActionDefId>,
    /// Mapping: AtomSkeletonId (Datalog) -> TypeId
    type_map: HashMap<AtomSkeletonId, TypeId>,
}
impl DatalogEncoder {

    /// Maximum number of variables (parameters) allowed per action or rule.
    ///
    /// This limit is set to 64 to allow high-performance variable tracking
    /// using a single CPU register (u64 bitset).
    pub const MAX_VARS: usize = 64;

    /// Initializes a new Datalog Encoder.
    ///
    /// # Arguments
    ///
    /// * `base_id` - The starting index for auxiliary predicates. This should
    ///   begin after the last standard predicate ID in the PDDL domain to
    ///   avoid ID collisions.
    ///
    /// # Process
    ///
    /// The encoder manages two distinct predicate ID spaces:
    /// 1. **Base Predicates**: IDs below `base_id`, which correspond to the
    ///    original domain symbols resolved via the global interner.
    /// 2. **Auxiliary Predicates**: IDs starting from `base_id`, which are
    ///    generated during the encoding process (e.g., for actions, types, or
    ///    skolemized formulas).
    ///
    /// # Performance
    ///
    /// This constructor pre-allocates space for 256 auxiliary definitions and
    /// cache entries. This heuristic strategy minimizes heap reallocations
    /// during the initial encoding phase of typical PDDL problems.
    pub fn new(base_id: usize) -> Self {
        Self {
            base_aux_id: base_id,
            next_aux_id: base_id,
            // Pre-allocation strategy to handle typical PDDL domain complexity
            aux_defs: Vec::with_capacity(256),
            cache: HashMap::with_capacity(256),
            action_map: HashMap::with_capacity(256),
            type_map: HashMap::with_capacity(256),
        }
    }

    /// Encodes a PDDL Type as a unary Datalog predicate and maintains a semantic mapping.
    ///
    /// This function creates a bridge between PDDL types and Datalog unary relations.
    /// It is a critical part of the **Type Guard** strategy, allowing the engine to
    /// restrict variable bindings to valid object domains during the saturation process.
    ///
    /// # Arguments
    ///
    /// * `type_id` - The original [`TypeId`] from the PDDL problem to be mapped.
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] representing this type in the Datalog database.
    ///
    /// # Process
    ///
    /// 1. **ID Allocation**: Assigns a new unique ID from the monotonic `next_aux_id` counter.
    /// 2. **Signature Creation**: Defines a unary predicate schema `(type_name ?v0)`.
    /// 3. **Semantic Mapping**: Registers the link between the Datalog `sk_id` and the
    ///    PDDL `type_id` in the `type_map`. This prevents information loss during grounding.
    /// 4. **Registration**: Adds the skeleton to `aux_defs` for schema-aware rule compilation.
    pub fn encode_type_as_unary_predicate(&mut self, type_id: TypeId) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        let sk_id = AtomSkeletonId::from(id);
        let predicate_id = PredicateSymbolId::from(id);

        // Schema: type_name(?v0) where ?v0 is of type 'object' (root)
        // We use the root type for the variable because this predicate
        // is what defines the membership of an object to a specific type.
        let arg = TypedSymbol::new(VariableId::from(0), Type::root());
        let mut arguments = TypedList::new();
        arguments.push(arg);

        // Register auxiliary definition for schema consistency in the encoder
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, arguments));

        // INTERNAL MAPPING: Link the Datalog relation ID back to the PDDL TypeId.
        // This allows the Engine to identify type-hierarchy facts during extraction.
        self.type_map.insert(sk_id, type_id);

        sk_id
    }

    /// Encodes a lifted action as a Datalog predicate.
    ///
    /// This allows the Datalog engine to represent action applicability as a relation.
    /// If the saturation process derives a fact for this predicate, the action is
    /// considered reachable with that specific grounding of parameters.
    ///
    /// # Arguments
    ///
    /// * `action` - A reference to the [`LiftedAction`] whose parameters define the
    ///   predicate's signature (arity and types).
    /// * `action_def_id` - The unique [`ActionDefId`] from the problem definition, used
    ///   to map the Datalog fact back to the original PDDL action.
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] assigned to this action's predicate.
    ///
    /// # Process
    ///
    /// 1. **ID Allocation**: Reserves a unique auxiliary ID for the action.
    /// 2. **Signature Extraction**: Clones the action's typed parameters to define the
    ///    predicate's structure within the Datalog engine.
    /// 3. **Registration**: Stores the [`AtomicFormulaSkeleton`] in `aux_defs`.
    /// 4. **Mapping**: Associates the resulting `AtomSkeletonId` with the `ActionDefId`
    ///    in the internal `action_map` for future decoding.
    ///
    /// # Performance
    ///
    /// - **Complexity**: $O(P)$ where $P$ is the number of parameters (cloning overhead).
    /// - **Inlining**: Marked with `#[inline]` to allow the compiler to optimize
    ///   the encoding loop during problem loading.
    #[inline]
    pub fn encode_action_as_predicate(
        &mut self,
        action: &LiftedAction,
        action_def_id: ActionDefId
    ) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        let sk_id = AtomSkeletonId::from(id);
        let predicate_id = PredicateSymbolId::from(id);

        // 1. Retrieve the action parameters to define the signature (arity and types)
        let parameters = action.parameters().clone();

        // 2. Register the skeleton definition
        // This is vital for mapping ground facts back to meaningful action names.
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, parameters));

        // 3. INTERNAL MAPPING: Link the Datalog relation ID back to the Action definition.
        // This allows the Engine to identify which action a grounded fact refers to.
        self.action_map.insert(sk_id, action_def_id);

        sk_id
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
    /// * `root_effect` - The expression tree representing the action's effects.
    /// * `action_atom` - The atom representing the execution of the action (the initial cause).
    /// * `rules_sink` - A vector where newly generated Datalog rules are stored.
    /// * `parameters` - The typed parameters of the action, used for auxiliary predicate signatures.
    ///
    /// # Errors
    ///
    /// Returns a [`DatalogError`] if:
    /// * An unsupported node kind is encountered (e.g., `Forall` or `Exists` not yet implemented).
    /// * There is a failure in auxiliary predicate generation or variable collection.
    pub fn encode_effects(
        &mut self,
        root_effect: &Expr,
        action_atom: &Atom,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<(), DatalogError> {
        // Work stack: (Node ID, Current Cause)
        let mut work_stack = vec![(root_effect.try_root_id()?, action_atom.clone())];

        while let Some((node_id, current_cause)) = work_stack.pop() {
            let node = root_effect.try_node(node_id)?;
            let kind = node.kind();

            match kind {
                // 1. Fact Production: Create the rule Effect :- Cause
                ExprKind::AtomicFormula => {
                    let effect_atom = self.extract_atom(root_effect, node)?;
                    rules_sink.push(Rule::new(effect_atom, vec![current_cause]));
                }

                // 2. Conjunction: Propagate the cause to all sub-effects
                ExprKind::And => {
                    for &child_id in node.children().iter().rev() {
                        work_stack.push((child_id, current_cause.clone()));
                    }
                }

                // 3. Conditional Effect: Create a pivot between Action and Condition
                ExprKind::When => {
                    let children = node.children();
                    let condition_id = children[0];
                    let sub_effect_id = children[1];

                    if let Some(cond_atom) = self.encode_expr(root_effect, condition_id, rules_sink, parameters)? {
                        // 1. Les variables qu'on A (Cause + Condition)
                        // Note le '?' à la fin car collect_mask peut échouer (VariableLimitExceeded)
                        let available_mask = self.collect_mask(&[current_cause.clone(), cond_atom.clone()]);

                        // 2. Les variables dont on a BESOIN (le futur de l'effet)
                        let required_mask = self.scan_required_vars_mask(root_effect, sub_effect_id);

                        // 3. LA PROJECTION : Intersection bit à bit
                        let final_mask = available_mask & required_mask;

                        // 4. On transforme le mask en Vec via ton code optimisé
                        let filtered_vars = self.mask_to_vars(final_mask);

                        // --- Logique de cache ---
                        let mut combined_body = vec![current_cause.clone(), cond_atom];
                        // On trie le corps pour que l'ordre des atomes n'impacte pas le cache
                        combined_body.sort_by_key(|a| a.skeleton_id());

                        let aux_when_atom = if let Some(existing_head) = self.cache.get(&combined_body) {
                            existing_head.clone()
                        } else {
                            // Création de l'atome avec uniquement les variables utiles (filtered_vars)
                            let head = self.create_aux_atom(filtered_vars, parameters);
                            rules_sink.push(Rule::new(head.clone(), combined_body.clone()));
                            self.cache.insert(combined_body, head.clone());
                            head
                        };

                        // On continue la pile avec le pivot comme nouvelle cause
                        work_stack.push((sub_effect_id, aux_when_atom));
                    }
                }

                // 4. Temporal Wrappers: Simply traverse through
                ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                    if let Some(&child_id) = node.children().first() {
                        work_stack.push((child_id, current_cause));
                    }
                }

                // 5. Explicitly Ignored Nodes (Numerical / Metrics)
                // These are skipped as they don't impact atomic fact reachability
                ExprKind::Assign | ExprKind::Operation | ExprKind::FComp | ExprKind::Not | ExprKind::Metric => {
                    // Note: 'Not' is ignored here because positive Datalog ignores delete-effects
                    continue;
                }

                // 6. Safety: Any other node kind triggers an error (e.g., Forall, Exists)
                _ => {
                    return Err(DatalogError::UnsupportedNode {
                        kind: kind.clone(),
                        node_id,
                    });
                }
            }
        }
        Ok(())
    }

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
    /// * `expr` - The expression tree representing the action's preconditions.
    /// * `rules_sink` - A vector where newly generated Datalog rules (auxiliary definitions) are stored.
    /// * `parameters` - The typed parameters of the action, used to define the signature of auxiliary predicates.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Atom))` - The head atom representing the unified precondition logic.
    /// * `Ok(None)` - If the expression is empty or contains only ignored nodes (e.g., empty AND).
    /// * `Err(DatalogError)` - If the expression tree is malformed or contains unsupported nodes.
    pub fn encode_preconditions(
        &mut self,
        expr: &Expr,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        if let Some(root_id) = expr.root_id() {
            self.encode_expr(expr, root_id, rules_sink, parameters)
        } else {
            Ok(None)
        }
    }

    /// Encodes a sub-expression starting from a specific node into Datalog atoms and rules.
    ///
    /// This is the internal engine used by both `encode_preconditions` and `encode_effects`.
    /// It performs an iterative post-order traversal starting at `node_id` to flatten
    /// complex logical structures (AND/OR) into auxiliary predicates.
    ///
    /// # Arguments
    ///
    /// * `expr` - The global expression tree containing the node.
    /// * `node_id` - The starting point for the encoding (root of the sub-tree).
    /// * `rules_sink` - A vector where newly generated Datalog rules (auxiliary definitions) are stored.
    /// * `parameters` - The typed parameters available in the current context (e.g., action parameters).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Atom))` - The head atom representing the encoded sub-expression.
    /// * `Ok(None)` - If the branch contains no logical content (e.g., empty AND, ignored nodes).
    /// * `Err(DatalogError)` - If the stack is inconsistent or an unsupported node is encountered.
    pub fn encode_expr(
        &mut self,
        expr: &Expr,
        node_id: NodeId,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        // Work stack: (Node ID, is_visited)
        // Start with the root node, initially unvisited.
        let mut work_stack = vec![(node_id, false)];

        // Result stack for post-order synthesis (stores computed Atoms or None).
        let mut results_stack: Vec<Option<Atom>> = Vec::with_capacity(32);

        // Iterative post-order traversal
        while let Some((node_id, visited)) = work_stack.pop() {
            let node = expr.try_node(node_id)?;
            let kind = node.kind();

            if !visited {
                // --- PHASE 1: Entry (Downward Traversal) ---
                match kind {
                    // Explore core logical branches
                    ExprKind::And | ExprKind::Or
                    | ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        work_stack.push((node_id, true));
                        // Push children in reverse order to maintain left-to-right processing
                        for &child_id in node.children().iter().rev() {
                            work_stack.push((child_id, false));
                        }
                    }

                    // L'AtomicFormula est traitée comme une feuille : on ne descend pas dans ses enfants !
                    ExprKind::AtomicFormula => {
                        work_stack.push((node_id, true));
                    }

                    // Ignore nodes that do not contribute to the Datalog logic mapping
                    ExprKind::Not | ExprKind::Operation | ExprKind::FComp => {
                        results_stack.push(None);
                        continue;
                    }

                    // Error on nodes that should have been pre-processed or are unexpected
                    _ => {
                        return Err(DatalogError::UnsupportedNode {
                            kind: kind.clone(),
                            node_id,
                        });
                    }
                }
            } else {
                // --- PHASE 2: Exit (Upward Synthesis / Post-order) ---
                let num_children = node.children().len();
                let result = match kind {
                    // Basic leaf node: extract the predicate and its terms
                    ExprKind::AtomicFormula => Some(self.extract_atom(expr, node)?),

                    ExprKind::And | ExprKind::Or => {
                        // Safety check: Ensure the results stack contains all child results
                        if results_stack.len() < num_children {
                            return Err(DatalogError::InconsistentStack(kind.clone()));
                        }

                        let start_idx = results_stack.len() - num_children;
                        // Collect all valid atoms computed by child branches
                        let mut atoms: Vec<Atom> = results_stack.drain(start_idx..).flatten().collect();

                        if atoms.is_empty() {
                            None
                        } else if atoms.len() == 1 {
                            // Optimization: A single-child is logically equivalent to the child itself
                            Some(atoms[0].clone())
                        } else {
                            // 1. Sort atoms by skeleton ID to ensure structural deduplication in cache
                            atoms.sort_by_key(|a| a.skeleton_id());

                            // 2. Structural Deduplication: Check if this logic already exists
                            if let Some(existing_head) = self.cache.get(&atoms) {
                                Some(existing_head.clone())
                            } else {
                                // 3. Cache Miss: Create a new auxiliary predicate signature
                                let head = self.encode_new_aux_predicate(&atoms, parameters)?;

                                // 4. Generate the Horn Clauses (Datalog rules)
                                match kind {
                                    ExprKind::And => {
                                        // Conjunction: One rule for all atoms (Head :- A, B, C)
                                        rules_sink.push(Rule::new(head.clone(), atoms.clone()));
                                    }
                                    ExprKind::Or => {
                                        // Disjunction: One rule per atom (Head :- A. Head :- B. ...)
                                        for atom in &atoms {
                                            rules_sink.push(Rule::new(head.clone(), vec![atom.clone()]));
                                        }
                                    }
                                    _ => unreachable!(),
                                }

                                // 5. Memorize the result for future identical expressions
                                self.cache.insert(atoms, head.clone());
                                Some(head)
                            }
                        }
                    }

                    // Temporal Wrappers: simply propagate the child result upwards
                    ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        results_stack.pop().flatten()
                    }

                    // Default: Cleanup the stack for unhandled or skipped node kinds
                    _ => {
                        if results_stack.len() >= num_children {
                            let start_idx = results_stack.len() - num_children;
                            results_stack.drain(start_idx..);
                        }
                        None
                    }
                };

                // Push the synthesized result back onto the stack
                results_stack.push(result);
            }
        }

        // The final result is the last remaining item on the stack
        Ok(results_stack.pop().flatten())
    }



    /// Extracts a logical [`Atom`] from a specific expression node.
    ///
    /// This function serves as the bridge between the high-level expression tree ([`Expr`][`ExprNode`][`Atom`]).
    /// It resolves the predicate identity and maps each child argument to its concrete Datalog representation.
    ///
    /// # Arguments
    ///
    /// * `expr` - The global expression tree used to resolve the nature of child nodes.
    /// * `node` - A reference to the current [`ExprNode`], which must represent an `AtomicFormula`.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the grounded or lifted [`Atom`], or a [`DatalogError`] if resolution fails.
    ///
    /// # Process
    ///
    /// 1. **Skeleton Extraction**: Retrieves the `AtomSkeletonId` from the node content.
    /// 2. **Term Mapping**: Iterates over child nodes to categorize arguments:
    ///    - **Variables**: Parameters (e.g., `?v0`) are converted to [`Term::Variable`].
    ///    - **Constants**: Fixed objects (e.g., `room_a`) are converted to [`Term::Constant`].
    /// 3. **Validation**: Ensures that all arguments are valid terminals for a Datalog relation.
    ///
    /// # Performance
    ///
    /// * **Complexity**: $O(N)$ where $N$ is the number of arguments (arity of the predicate).
    /// * **Efficiency**: Uses a fallible iterator (`collect::<Result<Vec<_>, _>>`) to populate
    ///   the term buffer in a single pass without redundant allocations.
    ///
    /// # Errors
    ///
    /// Returns a [`DatalogError`] if:
    /// * The node content cannot be converted to an atom skeleton.
    /// * An argument node is neither a `Variable` nor a `Constant` (e.g., a nested expression).
    /// * A node reference within the `expr` tree is invalid.
    fn extract_atom(&self, expr: &Expr, node: &ExprNode) -> Result<Atom, DatalogError> {
        let skeleton_id = node.content().try_atom_skeleton()?;
        let children = node.children();

        // Pre-allocate the vector for optimal performance
        let mut terms = Vec::with_capacity(children.len());

        // Explicit loop for mapping child nodes to Datalog terms (except 0 the predicate symbol)
        for &arg_id in children.iter().skip(1) {
            let arg_node = expr.try_node(arg_id)?;

            let term = match arg_node.kind() {
                ExprKind::Variable => {
                    let var_id = arg_node.content().try_variable()?;
                    Term::Variable(var_id)
                }
                ExprKind::Constant => {
                    let cons_id = arg_node.content().try_constant()?;
                    Term::Constant(cons_id)
                }
                // Datalog atoms must only contain terminals (Variables or Constants)
                _ => return Err(DatalogError::InvalidAtomArgument(arg_id)),
            };

            terms.push(term);
        }

        Ok(Atom::new(skeleton_id, terms))
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
    fn encode_new_aux_predicate(
        &mut self,
        atoms: &[Atom],
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Atom, DatalogError> {
        // Collect unique variables to define the new predicate's signature
        let vars = self.collect_variables(atoms)?;
        // Generate the unique auxiliary atom
        Ok(self.create_aux_atom(vars, parameters))
    }

    /// Creates a new auxiliary atom and registers its skeleton locally.
    ///
    /// This method is a core part of the **Skolemization** process during flattening.
    /// It generates a unique predicate ID for a sub-formula and maps the provided
    /// variables to their respective types based on the action's parameter list.
    ///
    /// # Arguments
    /// * `vars` - The subset of variables that will become the terms of this auxiliary atom.
    /// * `parameters` - The master list of typed variables from the current action/context
    ///   used to resolve the types of `vars`.
    ///
    /// # Returns
    /// A new [`Atom`] configured with an auxiliary [`AtomSkeletonId`] and variable terms.
    ///
    /// # Performance
    /// - **ID Management**: Increments an internal counter in $O(1)$.
    /// - **Type Resolution**: Direct $O(1)$ lookup per variable using the `parameters` list.
    /// - **Allocation**: Performs one allocation for the `aux_defs` storage and one for the `Atom` terms.
    fn create_aux_atom(
        &mut self,
        vars: Vec<VariableId>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Atom {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        // 1. Construct the signature (types) by mapping variables to their types
        let mut aux_params = TypedList::new();
        for &v_id in &vars {
            // O(1) direct access as VariableIds are used as indices
            let ty = parameters[v_id.as_usize()].ty();
            aux_params.push(TypedSymbol::new(v_id, ty.clone()));
        }

        // 2. Create the PredicateSymbolId (virtual ID, no interning required here)
        let predicate_id = PredicateSymbolId::from(id);

        // 3. Register the definition in the local auxiliary list
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, aux_params));

        // 4. Create the Atom for the Datalog engine
        let skeleton_id = AtomSkeletonId::from(id);
        let terms: Vec<Term> = vars.into_iter().map(Term::Variable).collect();

        Atom::new(skeleton_id, terms)
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
    pub fn collect_variables(&self, atoms: &[Atom]) -> Result<Vec<VariableId>, DatalogError> {
        let mask = self.collect_mask(atoms);
        Ok(self.mask_to_vars(mask))
    }


    fn scan_required_vars_mask(&self, expr: &Expr, start_node_id: NodeId) -> u64 {
        let mut mask: u64 = 0;
        let mut stack = vec![start_node_id];

        while let Some(node_id) = stack.pop() {
            if let Ok(node) = expr.try_node(node_id) {
                match node.kind() {
                    ExprKind::AtomicFormula => {
                        // --- CORRECTION ICI ---
                        // On utilise extract_atom pour obtenir l'objet qui possède la méthode .terms()
                        if let Ok(atom) = self.extract_atom(expr, node) {
                            for term in atom.terms() {
                                if let Term::Variable(v) = term {
                                    mask |= 1 << v.as_usize();
                                }
                            }
                        }
                    }
                    // On continue de descendre dans les enfants pour trouver tous les atomes
                    ExprKind::And | ExprKind::When | ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        for &child_id in node.children() {
                            stack.push(child_id);
                        }
                    }
                    _ => {}
                }
            }
        }
        mask
    }

    fn collect_mask(&self, atoms: &[Atom]) -> u64 {
        let mut mask: u64 = 0;
        for atom in atoms {
            for term in atom.terms() {
                if let Term::Variable(v) = term {
                    mask |= 1 << v.as_usize();
                }
            }
        }
        mask
    }

    fn mask_to_vars(&self, mut mask: u64) -> Vec<VariableId> {
        let mut vars = Vec::with_capacity(mask.count_ones() as usize);
        while mask != 0 {
            let bit = mask.trailing_zeros();
            vars.push(VariableId::from(bit as usize));
            mask &= mask - 1;
        }
        vars
    }
}
