use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
use crate::analysis::inertia::evaluator::InertiaEvaluator;

impl<'a> InertiaEvaluator<'a> {
    /// Évalue une formule atomique selon les règles de simplification du papier IPP (Section 3.2).
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

        // 1. Sécurité ID : On ne traite pas les prédicats générés dynamiquement (hors définitions PDDL)
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return Ok(None);
        }

        // 2. Détection de l'inertie (Section 3.4)
        let is_negative = self.inertia.is_predicate_negative_inertia(pred_id)?;
        let is_positive = self.inertia.is_predicate_positive_inertia(pred_id)?;

        // Si le prédicat n'est PAS inerte, c'est un Fluent (il change).
        if !is_negative && !is_positive {
            return Ok(None);
        }

        // --- ÉTAPE A : Calcul de N(p, ~a) ---
        let mask = self.extract_mask_dynamic(node, store, buffer);

        // Vérification de la limite de projection (max_proj)
        let bit_count = (mask as u32).count_ones() as usize;
        if mask != 0 && bit_count > self.max_proj {
            return Ok(None);
        }

        let lookup_slice = buffer.as_slice();

        // Récupération de la valeur N(p, a) dans les tables de comptage
        let n_p_a = self
            .counting_predicates
            .get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // --- ÉTAPE B : Application de la Définition 6 (Simplifications Atomiques) ---
        let n_val = n_p_a.unwrap_or(0);
        let grounded = self.all_args_grounded(node, store);

        // Règle 1 : Inertie Positive
        if is_positive {
            if n_val == 0 {
                return Ok(Some(false));
            }
            if grounded && n_val > 0 {
                return Ok(Some(true));
            }
            return Ok(None);
        }

        // Règle 2 : Inertie Négative
        if is_negative {
            let max_val = self.calculate_max_instances(node, store)?;

            if n_val == max_val {
                return Ok(Some(true));
            }
            if grounded && n_val == 0 {
                return Ok(Some(false));
            }
            return Ok(None);
        }

        Ok(None)
    }

    /// Calculates MAX(p, ~a) according to Definition 5 of the IPP paper.
    pub fn calculate_max_instances(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
    ) -> Result<usize, InertiaEvaluatorError> {
        let mut max_val: usize = 1;
        let children = node.children();

        if let ExprKind::AtomicFormula(pred_id) = node.kind() {
            if let Some(def) = self.predicate_defs.get(pred_id.as_usize()) {
                let arg_types = def.parameters();

                // RÈGLE : children[0] est le symbole. Les arguments commencent à l'index 1.
                for (i, &child_id) in children.iter().skip(1).enumerate() {
                    let child_entry = store.fetch(child_id)?;

                    if let ExprKind::Variable(_) = child_entry.kind() {
                        if let Some(param) = arg_types.get(i) {
                            let type_id = param.ty();
                            let domain_size = self.value_registry.get_type_domain(type_id)?.len();
                            max_val *= domain_size;
                        }
                    }
                }
            }
        }

        Ok(max_val)
    }

    /// Retourne true si le fait doit être inclus dans le BitVector d'état.
    fn is_fluent(&self, pred_id: AtomSkeletonId) -> bool {
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return false;
        }

        let is_static = self
            .inertia
            .is_predicate_positive_inertia(pred_id)
            .unwrap_or(false)
            || self
                .inertia
                .is_predicate_negative_inertia(pred_id)
                .unwrap_or(false);

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
        AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol,
        VariableId,
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
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let type_id = TypeId::from(0);

        // 2. Setup predicate definitions: P(?x)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
            ),
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
        let arg_var = builder.variable(0);

        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[arg_var],
            AtomSkeletonId::from(skel_id_raw),
        );

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
