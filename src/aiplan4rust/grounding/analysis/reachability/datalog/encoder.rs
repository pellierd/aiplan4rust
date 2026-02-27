use std::collections::HashMap;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{AtomSkeletonId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;

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
        }
    }

    /// Encodes a PDDL Type as a unary Datalog predicate.
    ///
    /// This function reserves a unique auxiliary ID and registers a unary signature
    /// (arity 1) used to represent type-membership facts (e.g., `(is_truck ?x)`)
    /// within the Datalog engine.
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] assigned to this type's unary relation.
    ///
    /// # Process
    ///
    /// 1. **ID Allocation**: Increments the monotonic auxiliary counter for a new predicate ID.
    /// 2. **Unary Signature**: Defines a [`TypedList`] with exactly one argument
    ///    associated with the root type.
    /// 3. **Registration**: Stores the signature in `aux_defs` to maintain
    ///    schema awareness during grounding.
    ///
    /// # Performance
    ///
    /// - **Complexity**: $O(1)$ constant time.
    /// - **Memory**: Minimal allocation for the single-argument [`TypedList`].
    pub fn encode_type_as_unary_predicate(&mut self) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        let predicate_id = PredicateSymbolId::from(id);

        // Arity 1: Every type is represented as a unary relation
        let arg = TypedSymbol::new(VariableId::from(0), Type::root());
        let mut arguments = TypedList::new();
        arguments.push(arg);

        // Register the skeleton for schema consistency
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, arguments));

        AtomSkeletonId::from(id)
    }

    /// Encodes a lifted action as a Datalog predicate.
    ///
    /// This allows the Datalog engine to represent action applicability as a relation.
    /// If the saturation process derives a fact for this predicate, the action is
    /// considered reachable with that specific grounding of parameters.
    ///
    /// # Arguments
    ///
    /// * `action` - A reference to the [`LiftedAction`] whose signature (name and parameters)
    ///   will be encoded.
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] assigned to this action's predicate.
    ///
    /// # Process
    ///
    /// 1. **ID Allocation**: Reserves a unique auxiliary ID for the action.
    /// 2. **Signature Extraction**: Clones the action's typed parameters to define the
    ///    predicate's arity and internal types.
    /// 3. **Registration**: Stores the [`AtomicFormulaSkeleton`] in `aux_defs`. This mapping
    ///    is essential for decoding grounded facts back into executable action instances.
    ///
    /// # Performance
    ///
    /// - **Complexity**: $O(P)$ where $P$ is the number of parameters (cloning overhead).
    /// - **ID Management**: $O(1)$ monotonic increment.
    pub fn encode_action_as_predicate(&mut self, action: &LiftedAction) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        // 1. Retrieve the action parameters to define the signature
        let parameters = action.parameters().clone();

        // 2. Create the virtual PredicateSymbolId
        let predicate_id = PredicateSymbolId::from(id);

        // 3. Register the skeleton definition
        // This is vital for mapping ground facts back to meaningful action names.
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, parameters));

        // 4. Return the corresponding AtomSkeletonId
        AtomSkeletonId::from(id)
    }


    /*fn extract_effect_atom(&self, effect: &Expr, params: &TypedList<VariableId, TypeId>) -> Result<Option<Atom>, DatalogError> {
        match effect.kind() {
            // Effet simple : (at ?robot ?loc)
            ExprKind::AtomicFormula => {
                let atom = self.extract_atom(effect.expr(), effect.root_node())?;
                Ok(Some(atom))
            },
            // Effet conditionnel : (when (condition) (effect))
            // INDUSTRIEL : FD traite souvent cela en créant une action virtuelle
            // avec (condition AND action_params) comme précondition.
            ExprKind::When => {
                // Pour l'instant, on peut ignorer ou logger
                Ok(None)
            },
            // On ignore les effets numériques pour la reachability pure
            _ => Ok(None),
        }
    }*/

    /// Iteratively encodes an expression into Datalog atoms and rules.
    ///
    /// This function performs a post-order traversal of the expression tree using
    /// an explicit work stack to flatten complex logical structures (AND/OR)
    /// into auxiliary predicates (Tseitin-like transformation).
    ///
    /// # Arguments
    /// * `expr` - The expression tree to encode.
    /// * `rules_sink` - A vector where newly generated Datalog rules are stored.
    /// * `parameters` - The typed parameters available in the current context.
    ///
    /// # Returns
    /// * `Ok(Some(Atom))` - The head atom representing the encoded expression.
    /// * `Ok(None)` - If the expression branch is ignored (e.g., unsupported or non-logical nodes).
    /// * `Err(DatalogError)` - If the stack is inconsistent or an unsupported node is encountered.
    pub fn encode_expr(
        &mut self,
        expr: &Expr,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        // Work stack: (Node ID, is_visited)
        // Start with the root node, initially unvisited.
        let mut work_stack = vec![(expr.try_root_id()?, false)];

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
                    ExprKind::Not | ExprKind::Assign | ExprKind::Operation | ExprKind::Metric
                    | ExprKind::Task | ExprKind::FunctionTerm | ExprKind::FComp => {
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
    fn collect_variables(&self, atoms: &[Atom]) -> Result<Vec<VariableId>, DatalogError> {
        // The 'mask' serves as a bitset where the n-th bit represents VariableId(n).
        let mut mask: u64 = 0;

        // --- PHASE 1: Marking Presence ---
        for atom in atoms {
            for term in atom.terms() {
                if let Term::Variable(v) = term {
                    let id = v.as_usize();

                    // Check if the variable fits in our 64-bit registry
                    if id >= Self::MAX_VARS {
                        return Err(DatalogError::VariableLimitExceeded(id as u32));
                    }

                    // Set the bit corresponding to the variable ID.
                    mask |= 1 << id;
                }
            }
        }

        // --- PHASE 2: Extraction ---
        // Pre-allocate the vector with the exact capacity needed using the
        // CPU's population count (popcount) instruction.
        let mut vars = Vec::with_capacity(mask.count_ones() as usize);
        let mut temp_mask = mask;

        // High-performance bit-scanning loop.
        while temp_mask != 0 {
            // trailing_zeros() uses the CPU's CTZ/BSF instruction to find the
            // index of the lowest set bit in O(1).
            let bit = temp_mask.trailing_zeros();

            // Convert the bit position back to a VariableId.
            vars.push(VariableId::from(bit as usize));

            // Clear the lowest set bit (BLSR trick: n & (n - 1)).
            // This is more efficient than shifting because it jumps directly
            // to the next set bit.
            temp_mask &= temp_mask - 1;
        }

        Ok(vars)
    }
}
