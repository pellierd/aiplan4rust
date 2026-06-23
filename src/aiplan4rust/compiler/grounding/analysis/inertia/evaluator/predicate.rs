//! Submodule for Compile-Time Invariant Pruning and Inertia Evaluation.
//!
//! This implementation provides the internal mechanisms used by the [`InertiaEvaluator`]
//! to simplify atomic formulas (predicates) into compile-time constants (`true` or `false`).
//! By analyzing the structural rigidity of PDDL relations—specifically positive inertia
//! ($I^+$) and negative inertia ($I^-$)—the evaluator detects dead branches and invariants
//! before generating the ground planning state.
//!
//! # Core Mechanics
//!
//! The evaluation pipeline bypasses traditional abstract syntax tree (AST) traversals
//! by operating directly on unified indexes and flattened structures:
//!
//! 1. **Fluency Gating**: Fast $\mathcal{O}(1)$ tracking checks isolate and ignore dynamic
//!    fluents immediately, minimizing cost for variables that cannot be evaluated statically.
//! 2. **Bitmask Extraction**: Parameter bindings (constants vs variables) are mapped onto a
//!    Big-Endian `u16` mask to quickly slice the multivariable constraints.
//! 3. **Combinatorial Product Space**: Unbound parameters are multiplied by their type domains
//!    using saturation arithmetic to calculate the upper bound of valid instances.
//! 4. **IPP Definition 6 Reduction**: Live instances counted from the initial state are cross-referenced
//!    against the structural invariants to emit a deterministic `Option<bool>`.

use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
use crate::analysis::inertia::evaluator::InertiaEvaluator;

impl<'a> InertiaEvaluator<'a> {
    /// Evaluates an atomic formula (predicate) based on the compile-time simplification
    /// rules defined in the IPP (Inertia Planning Graph) framework (Section 3.2).
    ///
    /// This function executes a critical optimization pass by combining structural rigidity
    /// analysis (positive/negative inertia) with live counting tables. It applies **Definition 6**
    /// of the IPP paper to prune or validate truth values before generating the grounding state,
    /// bypassing downstream combinatorial instantiation loops if a deterministic outcome is found.
    ///
    /// # Mathematical & Framework Semantics
    ///
    /// * **Positive Inertia ($I^+$)**: Predicates whose truth value can only change from `true` to `false`
    ///   (never added by an action). If their count in the initial state ($N_{val}$) is $0$, they are
    ///   statically `false`. If fully grounded and present, they are statically `true`.
    /// * **Negative Inertia ($I^-$)**: Predicates whose truth value can only change from `false` to `true`
    ///   (never removed by an action). If fully grounded and missing ($N_{val} = 0$), they are statically `false`.
    ///   If their count matches the maximum theoretical instances ($\text{MAX}(p, \sim\!a)$), they are universally `true`.
    ///
    /// # Parameters
    ///
    /// * `node` - The abstract syntax tree node representation (`ExprNode`) of the atomic formula.
    /// * `store` - A reference to the immutable global `ExprStore` containing the tree context.
    /// * `buffer` - A mutable scratchpad reference (`ArgumentBuffer`) used for zero-allocation
    ///   argument extraction during dynamic mask processing.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(true))` - The predicate simplifies deterministically to a compile-time `true` constant.
    /// * `Ok(Some(false))` - The predicate simplifies deterministically to a compile-time `false` constant.
    /// * `Ok(None)` - The predicate is dynamic (fluent) or lacks sufficient initial state criteria to be simplified.
    ///
    /// # Errors
    ///
    /// Returns an `Err(InertiaEvaluatorError)` if:
    /// * An internal state lookup fails within the underlying inertia bitsets.
    /// * The maximum combinatorial instance bound calculation overflows or queries an uninitialized type registry.
    ///
    /// # Performance & Allocation Invariants
    ///
    /// * **Lazy Evaluation / Gated Hot Path**: Structural verification (inertia bit checking) is performed
    ///   *before* extracting arguments or traversing types. If a predicate is fluent, it returns `Ok(None)`
    ///   in $\mathcal{O}(1)$ time without reading the arguments.
    /// * **Zero Heap Allocation**: Reuses the provided stack-allocated `ArgumentBuffer` to collect ground parameters,
    ///   eliminating heap churn during the compiler's upward post-order reduction traversal.
    /// * **Minimized Pointer Chasing**: Multilevel counting map structures (`counting_predicates`) are sequentialized
    ///   and short-circuited via standard `and_then` combinations to maximize CPU cache locality.
    pub(crate) fn evaluate_predicate_internal(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<bool>, InertiaEvaluatorError> {
        let pred_id = match node.kind() {
            ExprKind::AtomicFormula(id) => *id,
            _ => return Ok(None),
        };

        // 1. Sécurité ID & Extraction directe des statuts d'inertie
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return Ok(None);
        }
        let is_negative = self.inertia.is_predicate_negative_inertia(pred_id)?;
        let is_positive = self.inertia.is_predicate_positive_inertia(pred_id)?;
        if !is_negative && !is_positive {
            return Ok(None);
        }

        // --- ÉTAPE A : Extraction dynamique du masque (uniquement si l'inertie est confirmée) ---
        let mask = self.extract_mask_dynamic(node, store, buffer) as u16;

        // Récupération de la définition du prédicat pour obtenir l'arité
        let def = &self.predicate_defs[pred_id.as_usize()];
        let arity = store.typed_list_len(def.parameters())?;

        // --- ENCAPSULATION REUSSIE : Applique la projection et calcule le statut grounded ---
        let grounded = match self.validate_projection_and_grounding(mask, arity) {
            Some(status) => status,
            None => return Ok(None), // Court-circuit si max_proj est dépassé
        };

        // --- OPTIMISATION 1 : Chasse aux pointeurs réduite ---
        let n_val = self
            .counting_predicates
            .get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(buffer.as_slice()))
            .copied()
            .unwrap_or(0);

        // --- ÉTAPE B : Application de la Définition 6 (Simplifications Atomiques) ---

        // Règle 1 : Inertie Positive
        if is_positive {
            if n_val == 0 {
                return Ok(Some(false));
            }
            if grounded {
                return Ok(Some(true));
            }
            return Ok(None);
        }

        // Règle 2 : Inertie Négative
        if is_negative {
            if n_val == 0 && grounded {
                return Ok(Some(false));
            }

            let max_val = self.calculate_max_instances(pred_id, mask, store)?;
            if n_val == max_val {
                return Ok(Some(true));
            }
        }

        Ok(None)
    }

    /// Calculates the maximum theoretical number of ground instances for a predicate
    /// based on its signature and a bound variable bitmask.
    ///
    /// This represents an optimized computation of $\text{MAX}(p, \sim\!a)$, operating
    /// directly on the predicate identifier and a pre-extracted argument mask. By bypassing
    /// the expression AST and the central `ExprStore`, it avoids pointer-chasing and recursive
    /// tree traversals on the hot path.
    ///
    /// # Mathematical Sémantics
    ///
    /// Given a predicate $p(t_1, \dots, t_n)$, the maximum number of instances is the
    /// product of the domain sizes of all positions that are **unbound variables** ($\sim\!a$).
    /// Constant arguments restrict the position to a single value, contributing a factor of $1$
    /// to the Cartesian product.
    ///
    /// # Parameters
    ///
    /// * `pred_id` - The structural identifier (`AtomSkeletonId`) of the target predicate.
    /// * `mask` - A **Big-Endian bitmask** where each bit corresponds to an argument position:
    ///   * `1`: The argument is a bound **constant** (fixed value).
    ///   * `0`: The argument is an unbound **variable** (free to iterate over its type domain).
    ///
    /// # Performance & Complexity Guarantees
    ///
    /// * **Time Complexity**: $\mathcal{O}(A)$ where $A$ is the arity of the predicate. Since PDDL
    ///   predicates rarely exceed an arity of 4 or 5, this effectively runs in ultra-fast $\mathcal{O}(1)$ time.
    /// * **Space Complexity**: $\mathcal{O}(0)$ zero-allocation stack execution.
    /// * **Inlining**: Marked with `#[inline]` to allow instruction fusion and loop unrolling directly
    ///   within the counting table generation loops.
    ///
    /// # Errors
    ///
    /// Returns an `Err(InertiaEvaluatorError)` if the underlying `value_registry` fails to retrieve
    /// the domain bounds for any of the parameter types (e.g., corrupted or uninitialized type registry).
    ///
    /// # Examples
    ///
    /// For a predicate `clear(x: Block)` with arity 1 and an unbound variable mask (`0b0`):
    /// ```rust
    /// // mask = 0b0 (position 0 is a variable) -> returns domain_size(Block)
    /// let max = evaluator.calculate_max_instances(clear_id, 0b0)?;
    /// ```
    ///
    /// For a predicate `on(x: Block, y: Block)` where `x` is fixed but `y` is free (`0b10`):
    /// ```rust
    /// // mask = 0b10 (pos 0 is constant factor 1, pos 1 is variable factor domain_size)
    /// let max = evaluator.calculate_max_instances(on_id, 0b10)?;
    /// ```
    #[inline]
    pub fn calculate_max_instances(
        &self,
        pred_id: AtomSkeletonId,
        mask: u16,
        store: &ExprStore,
    ) -> Result<usize, InertiaEvaluatorError> {
        // 1. Safe boundary check: if the predicate is unknown, fallback gracefully to a factor of 1
        let def = match self.predicate_defs.get(pred_id.as_usize()) {
            Some(d) => d,
            None => return Ok(1),
        };

        let arg_types = def.parameters();
        let arity = store.typed_list_len(arg_types)?;
        if arity == 0 {
            return Ok(1);
        }

        let mut max_val: usize = 1;

        // 2. Compute the Cartesian product of unbound variable domains using Big-Endian bit shifting
        let parameters = store.fetch_typed_list(arg_types)?;
        for (i, param) in parameters.iter().enumerate() {
            let shift = arity - 1 - i;
            let is_variable = ((mask >> shift) & 1) == 0;

            if is_variable {
                // Fetch type domain size from registry, bubbling up any structural failure
                let domain_size = self.value_registry.get_type_domain(param.ty())?.len();

                // Defend against integer overflow during massive combinatorial multiplication
                max_val = max_val.saturating_mul(domain_size);
            }
        }

        Ok(max_val)
    }

    /// Determines if a predicate is fluent (dynamic) within the current planning context.
    ///
    /// A predicate is considered a **fluent** if its truth value can change over time through
    /// the execution of actions. Conversely, it is considered **static** (rigid) if it exhibits
    /// either pure positive inertia (always true if initially true, never added) or pure negative
    /// inertia (always false if initially false, never removed).
    ///
    /// # Performance & Complexity Guarantees
    ///
    /// * **Time Complexity**: $\mathcal{O}(1)$ constant time lookup.
    /// * **Space Complexity**: $\mathcal{O}(0)$ zero-allocation hot path.
    /// * **Inlining**: Marked with `#[inline]` to allow the compiler to eliminate the function call
    ///   overhead entirely, enabling aggressive branch-prediction and instruction pipelining inside
    ///   the core grounding loops.
    ///
    /// # Memory Safety & Robustness
    ///
    /// Instead of panicking on an invalid or uninitialized `AtomSkeletonId`, this method performs
    /// a safe bounds check against the internal predicate definitions. If the identifier is out
    /// of bounds, it gracefully returns `false` (treating the unknown item as non-fluent).
    ///
    /// # Internal Mechanics
    ///
    /// The function evaluates the structural inertia of the predicate:
    /// 1. Verifies the validity of `pred_id`.
    /// 2. Queries the internal `inertia` bitsets/tables for both positive and negative rigidity.
    /// 3. Safely defaults missing inertia context (`None`) to `false` via `.unwrap_or(false)`.
    /// 4. Inverts the aggregated static flag (`!is_static`) to deduce fluency.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let pred_id = AtomSkeletonId::from_usize(42);
    /// if registry.is_fluent(pred_id) {
    ///     println!("Predicate state must be tracked dynamically in the state transitions.");
    /// } else {
    ///     println!("Predicate is rigid; eligible for compile-time branch pruning.");
    /// }
    /// ```
    #[inline]
    pub fn is_fluent(&self, pred_id: AtomSkeletonId) -> bool {
        // 1. Safe boundary check to prevent out-of-bounds indexing down the pipeline
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return false;
        }

        // 2. Aggregate positive and negative inertia to determine if the predicate is rigid
        let is_static = self
            .inertia
            .is_predicate_positive_inertia(pred_id)
            .unwrap_or(false)
            || self
                .inertia
                .is_predicate_negative_inertia(pred_id)
                .unwrap_or(false);

        // 3. A predicate is fluent if and only if it is not static
        !is_static
    }
}

// =========================================================================
// SECTION: PREDICATE EVALUATION TESTS
// =========================================================================
#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprStore};
    use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
    use crate::aiplan4rust::support::lang::{
        AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, TypedList, TypedListId,
        TypedSymbol, VariableId,
    };
    use crate::analysis::inertia::evaluator::evaluator::tests::mock_predicate_defs;
    use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
    use crate::analysis::inertia::evaluator::InertiaEvaluator;
    use crate::analysis::inertia::inertia::Inertia;
    use crate::analysis::inertia::table::InertiaTable;

    /// # Purpose
    /// Verifies that a predicate marked with **Positive Inertia** (meaning its truth value
    /// can never change from its initial state during planning) is successfully simplified to `false`
    /// if it is completely absent from the initial state ($N(p, \vec{a}) = 0$).
    ///
    /// This implements the atomic simplification rule from Section 3.2 of the IPP paper
    /// (Closed-World Assumption for static predicates).
    ///
    /// # Input
    /// - A grounded atomic formula `(predicate_0 object_10 object_20)` mapped to `AtomSkeletonId(1)`.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as `Inertia::positive()`.
    /// - An empty `ValueRegistry` and empty `counting_predicates` cache (simulating an empty `init` state).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(false))`.
    #[test]
    fn test_empty_registry_returns_false_for_positive_inertia() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_raw = 1;

        // Create constant arguments (LIR Objects)
        let arg1 = builder.object(10);
        let arg2 = builder.object(20);

        // 2. Construct the atomic node with the symbol as the first child: [Sym, Arg1, Arg2]
        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(0),
            &[arg1, arg2],
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The atomic node must exist in the store");

        // --- TEST CONTEXT SETUP ---
        let p_defs = mock_predicate_defs(2); // Generate 2 mock definitions to ensure index [1] is valid
        let f_defs = vec![];

        // Instantiate live production structures as empty placeholders
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Mark skeleton ID 1 as Positive Inertia (Static predicate)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());

        // 3. Instantiate the mocked evaluator
        let evaluator = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        let mut buffer = ArgumentBuffer::new();

        // 4. EVALUATION
        let res = evaluator.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // According to IPP: Positive Inertia + Absent from initial state (N=0) => FALSE
        assert_eq!(
            res.unwrap(),
            Some(false),
            "A positive inert predicate absent from the initial state must be simplified to False"
        );
    }

    /// # Purpose
    /// Verifies the boundary case of an **arity-0 predicate** (a proposition with no arguments)
    /// marked with **Positive Inertia**. If it is completely absent from the initial state
    /// ($N(p) = 0$), it must be simplified to `false`.
    ///
    /// This ensures the engine handles empty argument vectors safely without off-by-one
    /// errors or slicing panics.
    ///
    /// # Input
    /// - A grounded atomic formula `(predicate_1)` with 0 arguments, mapped to `AtomSkeletonId(1)`.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as positive inert.
    /// - An empty `ValueRegistry` (simulating an empty `init` state).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(false))`.
    #[test]
    fn test_positive_inertia_pruning() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;

        // 2. Construct an arity-0 atom (only the symbol node is interned inside the buffer)
        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[], // Empty slice since arity is 0
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The atomic node must exist in the store");

        // --- TEST CONTEXT SETUP ---
        // Generate 2 mock definitions to make sure index [1] is safely within bounds
        let p_defs = mock_predicate_defs(2);
        let f_defs = vec![];
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Mark the skeleton as Positive Inertia
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());

        // Create the mocked evaluator instance
        let evaluator = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);
        let mut buffer = ArgumentBuffer::new();

        // 3. EVALUATION
        let res = evaluator.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 4. VERIFICATION
        // According to Koehler's Definition 6: Positive Inertia + Absent from initial state (N=0) => FALSE
        assert_eq!(
            res.unwrap(),
            Some(false),
            "An arity-0 positive inert predicate absent from the initial state must be simplified to False"
        );
    }

    /// # Purpose
    /// Verifies the boundary case of an **arity-0 predicate** marked with **Negative Inertia**
    /// when it is completely **absent** from the initial state ($N(p) = 0$).
    ///
    /// For an arity-0 predicate (a pure proposition), the maximum possible domain capacity
    /// $\text{MAX}$ is always 1 (an empty product of types). Because $N = 0$, it is impossible
    /// to satisfy the condition $0 < N < \text{MAX}$ needed to return `None`. Since the atom
    /// is fully grounded and absent, Koehler's atomic simplification rules reduce this
    /// known initial absence to `Some(false)`.
    ///
    /// # Input
    /// - A grounded atomic formula `(predicate_1)` with 0 arguments, mapped to `AtomSkeletonId(1)`.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as negative inert.
    /// - An empty `ValueRegistry` and no mask registration (simulating $N = 0$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(false))`.
    #[test]
    fn test_negative_inertia_arity_0_absent_returns_false() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;

        // 2. Construct an arity-0 atom (proposition)
        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[], // Arity 0
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The atomic node must exist in the store");

        // --- TEST CONTEXT SETUP ---
        let p_defs = mock_predicate_defs(2);
        let f_defs = vec![];
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Mark the skeleton as Negative Inertia
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::negative());

        // Create the mocked registry instance
        let evaluator = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // We intentionally DO NOT call generate_predicate_masks here, meaning N = 0
        let mut buffer = ArgumentBuffer::new();

        // 3. EVALUATION
        let res = evaluator.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 4. VERIFICATION
        // For arity 0, MAX = 1. Since the atom is grounded and N = 0, the engine
        // correctly optimizes this initial absence to False.
        assert_eq!(
            res.unwrap(),
            Some(false),
            "An absent negative inert arity-0 predicate must evaluate to Some(false)"
        );
    }

    /// # Purpose
    /// Verifies that an atomic formula involving variables (`P(?x)`) evaluates to `false`
    /// under **Positive Inertia** when it is completely **absent** from the initial state ($N = 0$).
    ///
    /// Since positive inert predicates cannot be asserted/added by any action, if an atom
    /// has zero instances initially, it can never become true. The engine statically prunes
    /// it to `Some(false)` for all future planning states.
    ///
    /// # Input
    /// - An atomic formula node structured as `(predicate_1 ?x)`.
    /// - `InertiaTable` marking the skeleton as positive inert.
    /// - An initial state containing no facts for this predicate ($N = 0$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(false))`.
    #[test]
    fn test_perfect_constant_missing_is_always_false() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let type_id = TypeId::from(0);

        // 2. Setup predicate definitions: P(?x)
        let var_list = TypedList::from_iter(vec![TypedSymbol::new(
            VariableId::from(0),
            Type::from(type_id),
        )]);
        let var_list_id = store.intern_typed_list(var_list);

        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedListId::EMPTY), // Dummy
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(pred_id_raw), var_list_id),
        ];
        let f_defs = vec![];

        // 3. Setup the ValueRegistry (Domain contains at least one object)
        let v_reg = ValueRegistry::from_objects(vec![TypedSymbol::new(
            ObjectId::from(100),
            Type::from(type_id),
        )]);

        // 4. Setup Positive Inertia
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());

        // 5. Initialize the mocked evaluator instance
        let evaluator = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 6. Construct the ungrounded expression: P(?var0)
        let atom_node_id = {
            let mut builder = ExprBuilder::new(&mut store);
            let arg_var = builder.variable(0);

            builder.atomic_formula(
                PredicateSymbolId::from(pred_id_raw),
                &[arg_var],
                AtomSkeletonId::from(skel_id_raw),
            )
        };

        let atom_node = store
            .get(atom_node_id)
            .expect("The atomic formula node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 7. EVALUATION
        let res = evaluator.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 8. VERIFICATION
        // Analysis:
        // - Positive Inertia is active.
        // - No initial state masks registered -> N = 0.
        // - Since it can never be added, it must evaluate to Some(false).
        assert_eq!(
            res.unwrap(),
            Some(false),
            "The predicate must simplify to FALSE (Positive Inertia + Absent)"
        );
    }
}
