use super::InertiaEvaluator;
use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton,
};
use crate::aiplan4rust::support::lang::{AtomSkeletonId, CompareOp, FunctionSkeletonId, ObjectId};
use smallvec::SmallVec;

impl<'a> InertiaEvaluator<'a> {
    pub(crate) fn process_init(
        &mut self,
        node: ExprNode<'_>,
        store: &ExprStore,
    ) -> Result<(), InertiaEvaluatorError> {
        match node.kind() {
            // Cas d'un fait atomique -> Délégation à predicate.rs
            ExprKind::AtomicFormula(skeleton_id) => {
                self.process_predicate(*skeleton_id, node, store)
            }

            // Cas d'une initialisation de fonction -> Délégation à function.rs
            ExprKind::Comparison(op) => {
                if matches!(op, CompareOp::Equal) {
                    self.process_function(node, store)
                } else {
                    Ok(())
                }
            }

            _ => Ok(()),
        }
    }

    // =========================================================================
    // SECTION: PREDICATE INITIALIZATION (IPP COUNTING & MASKS)
    // =========================================================================

    fn process_predicate(
        &mut self,
        skeleton_id: AtomSkeletonId,
        node: ExprNode<'_>,
        store: &ExprStore,
    ) -> Result<(), InertiaEvaluatorError> {
        let children = node.children();

        // On ne traite que les prédicats qui ne changent jamais (Inerte Positif)
        // pour peupler notre table de comptage IPP.
        if self.inertia.is_predicate_positive_inertia(skeleton_id)? {
            // Règle : children[0] est le symbole, les arguments commencent à l'index 1.
            let arity = children.len().saturating_sub(1);
            let mut args = Vec::with_capacity(arity);

            // On parcourt les enfants à partir de l'index 1 (les arguments)
            for (i, &arg_id) in children.iter().enumerate().skip(1) {
                let arg_entry = store.fetch(arg_id)?;

                // Dans l'état initial, les arguments doivent être des objets (constantes)
                if let ExprKind::Object(obj_id) = arg_entry.kind() {
                    args.push(*obj_id);
                } else {
                    // Si l'argument n'est pas un Object valide (ex: une variable résiduelle),
                    // on log l'erreur et on ignore ce fait mal formé.
                    println!(
                        "[ERREUR-INIT] Prédicat {:?} : l'enfant {} (ExprId {:?}) n'est pas un Object (Kind: {:?})",
                        skeleton_id, i, arg_id, arg_entry.kind()
                    );
                    return Ok(());
                }
            }

            // --- LE LOG DE VÉRITÉ ---
            println!(
                "[INIT-REGISTRY] Succès : Predicate {:?} | Args: {:?}",
                skeleton_id, args
            );

            // On délègue la génération des masques de bits pour l'instanciation partielle
            self.generate_predicate_masks(skeleton_id, arity, &args);
        }

        Ok(())
    }

    fn generate_predicate_masks(&mut self, key: AtomSkeletonId, arity: usize, args: &[ObjectId]) {
        // 1. Garde contre l'arité 0 et les erreurs de calcul potentielles
        if arity == 0 {
            let mask_table = self.counting_predicates.entry(key).or_default();
            let entries = mask_table.entry(0).or_default();
            let count = entries.entry(Box::from([])).or_insert(0);
            *count += 1;
            return;
        }

        // 2. Génération des masques (2^arity combinaisons)
        for mask in 0..(1 << arity) {
            // Filtre de projection : évite l'explosion mémoire si trop de variables sont fixées
            let bit_count = (mask as u32).count_ones() as usize;
            if mask != 0 && bit_count > self.max_proj {
                continue;
            }

            let mut combo: SmallVec<[ObjectId; 8]> = SmallVec::new();

            // 3. Construction de la clé de manière "Safe"
            // Encodage Big Endian : le premier argument est le bit le plus fort
            for (i, &obj) in args.iter().enumerate() {
                let bit_pos = arity - 1 - i;
                if (mask & (1 << bit_pos)) != 0 {
                    combo.push(obj);
                }
            }

            // 4. Insertion dans le registre
            let mask_table = self.counting_predicates.entry(key).or_default();
            let entries = mask_table.entry(mask as u16).or_default();

            let count = entries.entry(Box::from(combo.as_slice())).or_insert(0);
            *count += 1;
        }
    }

    // =========================================================================
    // SECTION: FUNCTION INITIALIZATION (LHS / RHS & MASKS)
    // =========================================================================

    /// Extrait le LHS (Function), le RHS (Value) et délègue la génération de masques
    fn process_function(
        &mut self,
        node: ExprNode<'_>,
        store: &ExprStore,
    ) -> Result<(), InertiaEvaluatorError> {
        let children = node.children();

        if children.len() < 2 {
            return Ok(());
        }

        // 1. Analyse du FunctionTerm (LHS)
        let lhs_id = children[0];
        let lhs_entry = store.fetch(lhs_id)?;

        if let ExprKind::Function(func_id) = lhs_entry.kind() {
            let func_id = *func_id;

            // On ne traite que si la fonction est inerte positive
            if self.inertia.is_function_positive_inertia(func_id)? {
                // 2. Extraction des arguments de la fonction (LHS)
                let func_children = lhs_entry.children();
                let arity = func_children.len().saturating_sub(1);
                let mut args = Vec::with_capacity(arity);

                for &arg_id in func_children.iter().skip(1) {
                    let arg_entry = store.fetch(arg_id)?;
                    if let ExprKind::Object(obj_id) = arg_entry.kind() {
                        args.push(*obj_id);
                    } else {
                        return Ok(());
                    }
                }

                // 3. Extraction de la valeur (RHS)
                let rhs_id = children[1];
                let rhs_entry = store.fetch(rhs_id)?;

                let value = match rhs_entry.kind() {
                    ExprKind::Number(n) => ExprConstant::Number(*n),
                    ExprKind::Object(obj_id) => ExprConstant::Object(*obj_id),
                    _ => return Ok(()),
                };

                println!(
                    "[INIT-REGISTRY] Function Success: {:?} | Args: {:?} | Val: {:?}",
                    func_id, args, value
                );

                self.generate_function_masks(func_id, arity, &args, value);
            }
        }

        Ok(())
    }

    /// Génère la combinatoire de masques et gère le consensus pour une fonction
    fn generate_function_masks(
        &mut self,
        key: FunctionSkeletonId,
        arity: usize,
        args: &[ObjectId],
        val: ExprConstant,
    ) {
        if arity == 0 {
            let mask_table = self.static_functions.entry(key).or_default();
            let entries = mask_table.entry(0).or_default();
            entries.insert(Box::from([]), val);
            return;
        }

        for mask in 0..(1 << arity) {
            let bit_count = (mask as u32).count_ones() as usize;
            if mask != 0 && bit_count > self.max_proj {
                continue;
            }

            let mut combo: SmallVec<[ObjectId; 8]> = SmallVec::new();
            for i in 0..arity {
                let bit_pos = arity - 1 - i;
                if (mask & (1 << bit_pos)) != 0 {
                    if let Some(&obj) = args.get(i) {
                        combo.push(obj);
                    }
                }
            }

            let mask_table = self.static_functions.entry(key).or_default();
            let entries = mask_table.entry(mask as u16).or_default();

            // Gestion du consensus de valeur (si divergence, retour à None via suppression)
            if let Some(existing_val) = entries.get_mut(combo.as_slice()) {
                if *existing_val != val {
                    entries.remove(combo.as_slice());
                }
            } else {
                entries.insert(Box::from(combo.as_slice()), val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
    use crate::aiplan4rust::compiler::lir::expr::{Expr, ExprBuilder};
    use crate::aiplan4rust::support::lang::{
        FunctionSymbolId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId,
    };
    use crate::analysis::inertia::evaluator::evaluator::tests::{
        mock_function_defs, mock_predicate_defs,
    };
    use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
    use crate::analysis::inertia::inertia::Inertia;
    use ordered_float::OrderedFloat;
    // =========================================================================
    // SECTION: PREDICATE INITIALIZATION (IPP COUNTING & MASKS)
    // =========================================================================

    /// # Purpose
    /// Verifies that an arity-0 predicate marked with **Negative Inertia** (meaning it cannot
    /// be deleted or made false by any action) correctly simplifies to `true` if it is present
    /// in the initial state ($N(p) = \text{MAX}(p) = 1$).
    ///
    /// This implements the dual atomic simplification rule from Koehler's Definition 6
    /// for static facts that start true and must remain true forever.
    ///
    /// # Input
    /// - A grounded atomic formula `(predicate_1)` mapped to `AtomSkeletonId(1)`.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as negative inert.
    /// - A pre-registered initial mask via `generate_predicate_masks` with empty arguments `&[]`,
    ///   meaning the proposition is initially true ($N = 1$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(true))`.
    #[test]
    fn test_negative_inertia_simplification() {
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

        // Mark the skeleton as Negative Inertia (Cannot be deleted)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::negative());

        // Create the mocked registry instance
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Satisfy N = MAX by declaring the fact true in the initial state.
        // For an arity-0 proposition, providing an empty slice `&[]` satisfies its presence.
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 0, &[]);

        let mut buffer = ArgumentBuffer::new();

        // 3. EVALUATION
        let res = registry.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 4. VERIFICATION
        assert_eq!(
            res.unwrap(),
            Some(true),
            "A negative inert predicate that starts true in the initial state must be simplified to True"
        );
    }

    /// # Purpose
    /// Verifies the isolation of predicates inside the evaluation registry.
    ///
    /// This test ensures that the evaluator distinguishes correctly between two different predicates
    /// (`p1` and `p2`) mapped to separate skeletons (`s1` and `s2`), even when they share
    /// the exact same object arguments in the memory store.
    /// According to Koehler's principles, simplification flags and initial state occurrences ($N$)
    /// must remain strictly local to each specific predicate skeleton.
    ///
    /// # Input
    /// - Two distinct grounded atomic formulas sharing the same argument `object_10`:
    ///   - `(predicate_1 object_10)` mapped to `AtomSkeletonId(1)`.
    ///   - `(predicate_2 object_10)` mapped to `AtomSkeletonId(2)`.
    /// - An `InertiaTable` marking both skeletons as `Inertia::positive()`.
    /// - Initial state registration via `generate_predicate_masks` executed **only** for `AtomSkeletonId(1)`.
    ///
    /// # Expected Output
    /// - Evaluating `node1` (`s1`) must return `Ok(Some(true))` because it is positive inert and present.
    /// - Evaluating `node2` (`s2`) must return `Ok(Some(false))` because it is positive inert and absent ($N=0$).
    #[test]
    fn test_predicate_isolation() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let (p1, s1) = (1, 1);
        let (p2, s2) = (2, 2);
        let obj_id = ObjectId::from(10);

        // Create the constant argument once to share across both atomic nodes
        let node_arg = builder.intern(ExprKind::Object(obj_id), &[]);
        let args = &[node_arg];

        // 2. Construct both atomic nodes using the Zero-Alloc API
        let node1_id =
            builder.atomic_formula(PredicateSymbolId::from(p1), args, AtomSkeletonId::from(s1));
        let node2_id =
            builder.atomic_formula(PredicateSymbolId::from(p2), args, AtomSkeletonId::from(s2));

        let node1 = store
            .get(node1_id)
            .expect("The first atomic node must exist in the store");
        let node2 = store
            .get(node2_id)
            .expect("The second atomic node must exist in the store");

        // --- TEST CONTEXT SETUP ---
        let p_defs = crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::evaluator::tests::mock_predicate_defs(3); // Generated up to index 2 to encompass both keys safely
        let f_defs = vec![];
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Set both predicates as Positive Inertia
        i_table.insert_predicate(AtomSkeletonId::from(s1), Inertia::positive());
        i_table.insert_predicate(AtomSkeletonId::from(s2), Inertia::positive());

        // Create the mocked registry instance
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Register ONLY s1(10) in the initial state. s2(10) remains completely absent (N=0).
        registry.generate_predicate_masks(AtomSkeletonId::from(s1), 1, &[obj_id]);

        let mut buffer = ArgumentBuffer::new();

        // 3. EVALUATION & VERIFICATION

        // Evaluate s1(10)
        // Positive Inertia + Present in the initial state => True
        let res1 = registry.evaluate_predicate_internal(node1, &store, &mut buffer);
        assert_eq!(
            res1.unwrap(),
            Some(true),
            "The predicate s1(10) should be found and simplified to True"
        );

        // Evaluate s2(10)
        // Positive Inertia + Absent from the initial state (N=0) => False
        let res2 = registry.evaluate_predicate_internal(node2, &store, &mut buffer);
        assert_eq!(
            res2.unwrap(),
            Some(false),
            "The predicate s2(10) should be False due to Positive Inertia + Absence"
        );
    }

    /// # Purpose
    /// Verifies that an atomic formula containing a **variable argument** (ungrounded term `?x`)
    /// evaluates to `None` ("unknown") when its initial state presence is only partial.
    ///
    /// Under Koehler's criteria, if the count of true instances $N$ for a static predicate
    /// satisfies $0 < N < \text{MAX}$, the predicate is neither universally false nor universally true
    /// across the entire domain type. Thus, it cannot be optimized away at this stage.
    ///
    /// # Input
    /// - A type domain containing exactly 2 unique objects: `{object_10, object_11}` ($\text{MAX} = 2$).
    /// - An atomic formula `(predicate_1 ?x)` containing a variable reference instead of a constant.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as totally inert (both positive and negative).
    /// - An initial state presence covering **only** `object_10` ($N = 1$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(None)`.
    #[test]
    fn test_returns_none_on_variable_argument() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_11 = ObjectId::from(11);

        // 2. Populate the ValueRegistry with 2 objects (MAX = 2)
        // Leverages the new From<ID> trait for seamless Type creation
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj_10, Type::from(type_id)),
            TypedSymbol::new(obj_11, Type::from(type_id)),
        ]);

        // 3. Setup mock definitions to associate the predicate's argument with our type
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
            ),
        ];
        let f_defs = vec![];

        // 4. Construct the ungrounded expression: P(?x)
        let var_node = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[var_node],
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The variable atomic node must exist in the store");

        let mut i_table = InertiaTable::empty();
        // Mark as completely inert to invoke strict constant pruning rules
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::negative());

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. Populate the initial state with ONLY P(10).
        // N = 1 (P(10) is true)
        // MAX = 2 (Domain for Type 0 contains {10, 11})
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 1, &[obj_10]);

        let mut buffer = ArgumentBuffer::new();

        // 6. EVALUATION
        let res = registry.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 7. VERIFICATION
        // Because N != 0 and N != MAX, the registry must answer "None" (Undecidable at compile-time)
        assert!(
            res.unwrap().is_none(),
            "Evaluation must return None because the atom is only true for a fraction of its type domain"
        );
    }

    /// # Purpose
    /// Verifies that an atomic formula containing a **variable argument** (ungrounded term `?x`)
    /// successfully simplifies to `true` when it is marked with **Negative Inertia** and is
    /// initially true for *every* single object in its type domain ($N = \text{MAX}$).
    ///
    /// This validates the maximum bound of Koehler's Definition 6: if a static fact cannot be
    /// deleted and covers the entire domain capacity, it is a universal tautology at compile-time.
    ///
    /// # Input
    /// - A type domain containing exactly 2 unique objects: `{object_100, object_101}` ($\text{MAX} = 2$).
    /// - An ungrounded atomic formula `(predicate_1 ?x)`.
    /// - An `InertiaTable` marking `AtomSkeletonId(1)` as negative inert.
    /// - Two sequential calls to `generate_predicate_masks` covering both objects ($N = 2$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(true))`.
    #[test]
    fn test_negative_inertia_n_equals_max_with_variable() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let type_id = TypeId::from(0);
        let obj_100 = ObjectId::from(100);
        let obj_101 = ObjectId::from(101);

        // 2. Populate the ValueRegistry (Domain size = 2)
        // Uses the optimized From<ID> trait implemented earlier
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj_100, Type::from(type_id)),
            TypedSymbol::new(obj_101, Type::from(type_id)),
        ]);

        // 3. Setup mock definitions (Typed skeleton to compute MAX internally)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
            ),
        ];
        let f_defs = vec![];

        // 4. Construct the ungrounded expression: P(?x)
        let var_node = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[var_node],
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The variable atomic node must exist in the store");

        // 5. Setup Inertia
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::negative());

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 6. Fill the initial state tracking: N = 2
        // Registering both objects reaches N == MAX
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 1, &[obj_100]);
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 1, &[obj_101]);

        let mut buffer = ArgumentBuffer::new();

        // 7. EVALUATION
        let res = registry.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 8. VERIFICATION
        // Analysis: N(2) == MAX(2) + Negative Inertia => TRUE
        assert_eq!(
            res.unwrap(),
            Some(true),
            "If N=MAX for a negative inert predicate, P(?x) must be simplified to True"
        );
    }

    /// # Purpose
    /// Verifies that the evaluator correctly extracts and looks up index masks when a constant
    /// appears **after a variable** (e.g., `P(?x, 51)`), specifically under a strict projection
    /// limit (`max_proj = 1`).
    ///
    /// This ensures that the projection mechanism processes the full tuple slicing rather than
    /// blindly taking the first N elements, which would lose information about downstream constants.
    ///
    /// # Input
    /// - A type domain where `?x` can bind to 2 possible objects: `{object_10, object_51}` ($\text{MAX} = 2$).
    /// - An atomic formula node structured as `(predicate_1 ?x object_51)`.
    /// - `InertiaTable` marking the skeleton as positive inert.
    /// - Evaluator configured with `max_proj = 1`.
    /// - Initial state containing exactly **one** matching fact: `P(10, 51)`.
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(None)`.
    #[test]
    fn test_fix_projection_beyond_first_argument() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10); // Robot 1
        let obj51 = ObjectId::from(51); // Zone 51 (Second argument)

        // 2. Setup the ValueRegistry (Domain size = 2)
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj10, Type::from(type_id)),
            TypedSymbol::new(obj51, Type::from(type_id)),
        ]);

        // 3. Setup predicate definitions: P(?x, ?y)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::from(type_id)),
                    TypedSymbol::new(VariableId::from(1), Type::from(type_id)),
                ]),
            ),
        ];
        let f_defs = vec![];

        // 4. Setup Positive Inertia
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());

        // 5. Initialize the registry with max_proj = 1
        let mut registry =
            InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 1);

        // 6. Inject the fact into the initial state: P(10, 51)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 2, &[obj10, obj51]);

        // 7. Construct the expression: P(?var0, 51)
        let arg_var = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let arg_const = builder.intern(ExprKind::Object(obj51), &[]);

        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[arg_var, arg_const],
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The structural atomic node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 8. EVALUATION
        let res = registry.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 9. VERIFICATION
        assert!(
            res.unwrap().is_none(),
            "The evaluator must find object 51 at the second position and return None (not False)"
        );
    }

    /// # Purpose
    /// Verifies that an atomic formula with a partial instantiation (`P(?x, 51)`) successfully
    /// simplifies to `true` when marked with **Negative Inertia** and its presence matches
    /// the exact capacity of the remaining variable's type domain ($N == \text{MAX}$).
    ///
    /// This specific test confirms that `calculate_max_instances` respects isolated **multi-type** /// bounds (domain cardinality of `type_robot` is 1, while total objects count is 2).
    ///
    /// # Input
    /// - Type definitions: `type_robot` (contains only `obj10`) and `type_room` (contains only `obj51`).
    /// - An atomic formula node structured as `(predicate_1 ?x object_51)`.
    /// - `InertiaTable` marking the skeleton as negative inert.
    /// - Initial state containing exactly **one** matching fact: `P(10, 51)` ($N = 1$).
    ///
    /// # Expected Output
    /// - `evaluate_predicate_internal` must return `Ok(Some(true))`.
    #[test]
    fn test_projection_full_simplification_to_true() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;

        // Define two distinct types to isolate domains
        let type_robot = TypeId::from(0);
        let type_room = TypeId::from(1);

        let obj10 = ObjectId::from(10); // The only robot
        let obj51 = ObjectId::from(51); // The room

        // 2. Setup the ValueRegistry (Isolating type domains)
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj10, Type::from(type_robot)),
            TypedSymbol::new(obj51, Type::from(type_room)),
        ]);

        // 3. Setup predicate definitions: P(?x:robot, ?y:room)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::from(type_robot)),
                    TypedSymbol::new(VariableId::from(1), Type::from(type_room)),
                ]),
            ),
        ];
        let f_defs = vec![];

        // 4. Setup Negative Inertia
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::negative());

        // 5. Initialize the registry with standard config (allowing mask 0b01)
        let mut registry =
            InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 2);

        // 6. Inject the fact into the initial state: P(10, 51)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id_raw), 2, &[obj10, obj51]);

        // 7. Construct the expression: P(?var0, 51)
        let arg_var = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let arg_const = builder.intern(ExprKind::Object(obj51), &[]);

        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[arg_var, arg_const],
            AtomSkeletonId::from(skel_id_raw),
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The atomic node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 8. EVALUATION
        let res = registry.evaluate_predicate_internal(atom_node, &store, &mut buffer);

        // 9. VERIFICATION
        // Analysis:
        // - extract_mask_dynamic uses mask 0b01.
        // - N = 1 (P(10, 51) is present).
        // - MAX = domain capacity of ?x (type_robot) = {obj10} => cardinality 1.
        // - N(1) == MAX(1) + Negative Inertia => TRUE.
        assert_eq!(
            res.unwrap(),
            Some(true),
            "The evaluator must simplify to TRUE because the only possible domain instance is initially true"
        );
    }

    /// # Purpose
    /// Verifies that the evaluator's bitmask generation explicitly differentiates the **position** /// of an object, preventing collisions when the same object identifier appears in different
    /// argument slots across initial registration and lookup phases.
    ///
    /// This tests that the multi-mask lookup keys are strictly sensitive to spatial indexing
    /// (Big Endian masking: `0b10` for position 0 vs `0b01` for position 1).
    ///
    /// # Input
    /// - An initial state containing exactly: `P(10, 99)`. This registers `obj10` bound to position 0 (`mask = 0b10`).
    /// - An evaluation lookup for the structural node `P(?x, 10)`. This queries `obj10` bound to position 1 (`mask = 0b01`).
    ///
    /// # Expected Output
    /// - `extract_mask_dynamic` must yield exactly `1` (`0b01`).
    /// - Counting lookup for `mask = 0b01` and argument `[10]` must return `0` (no positional collision).
    #[test]
    fn test_mask_differentiation_same_object_different_positions() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pred_id_raw = 1;
        let skel_id_raw = 1;
        let skel_id = AtomSkeletonId::from(skel_id_raw);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10);
        let obj99 = ObjectId::from(99);

        // 2. Setup predicate definitions: P(?x:type0, ?y:type0)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id_raw),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::from(type_id)),
                    TypedSymbol::new(VariableId::from(1), Type::from(type_id)),
                ]),
            ),
        ];
        let f_defs = vec![];

        // 3. Setup the ValueRegistry
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj10, Type::from(type_id)),
            TypedSymbol::new(obj99, Type::from(type_id)),
        ]);

        // 4. Initialize the registry with max_arity=2 and max_projection=2
        let i_table = InertiaTable::empty();
        let mut registry =
            InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 2);

        // 5. Inject the fact into the initial state: P(10, 99)
        // Generates combinations including mask 0b10 for obj10 (position 0)
        registry.generate_predicate_masks(skel_id, 2, &[obj10, obj99]);

        // 6. Construct the expression to evaluate: P(?var0, 10)
        let arg_var = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let arg_const = builder.intern(ExprKind::Object(obj10), &[]);

        let atom_node_id = builder.atomic_formula(
            PredicateSymbolId::from(pred_id_raw),
            &[arg_var, arg_const],
            skel_id,
        );

        let atom_node = store
            .get(atom_node_id)
            .expect("The structural atomic node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 7. DYNAMIC MASK EXTRACTION
        let mask = registry.extract_mask_dynamic(atom_node, &store, &mut buffer);

        // 8. VERIFICATION OF MASK STRUCTURE
        // Argument 0 is Variable -> bit 1 (high bit) = 0
        // Argument 1 is Constant -> bit 0 (low bit) = 1
        // Expected mask: 0b01 (decimal 1)
        assert_eq!(
            mask, 1,
            "The mask for the second argument being a constant must be exactly 0b01 (1)"
        );
        assert_eq!(
            buffer.len(),
            1,
            "The runtime workspace buffer must capture exactly 1 constant (obj10)"
        );
        assert_eq!(buffer[0], obj10);

        // 9. VERIFICATION OF THE NON-COLLISION IN TABLES
        // Look up within the counting registries under mask 0b01 for argument slice &[obj10]
        let n = registry
            .counting_predicates
            .get(&skel_id)
            .and_then(|m| m.get(&mask))
            .and_then(|e| e.get(buffer.as_slice()))
            .copied()
            .unwrap_or(0);

        // Must find nothing (0 instances)
        assert_eq!(
            n, 0,
            "Collision detected! obj10 at position 0 must not be matched when querying position 1"
        );
    }

    /// # Purpose
    /// Verifies the evaluation of **arity-0 predicates** (pure flags/propositions) under **Positive Inertia** /// by assessing both initial presence and initial absence.
    ///
    /// # Input
    /// - Two arity-0 atomic formulas: `node1` (`PredicateSymbolId(1)`) and `node2` (`PredicateSymbolId(2)`).
    /// - `InertiaTable` marking both skeletons as positive inert.
    /// - `node1` is explicitly registered in the initial state ($N = 1$), while `node2` is omitted ($N = 0$).
    ///
    /// # Expected Output
    /// - `node1` evaluates to `Ok(Some(true))` (Grounded, positive inert, and initially present).
    /// - `node2` evaluates to `Ok(Some(false))` (Grounded, positive inert, and initially absent).
    #[test]
    fn test_arity_zero_flag_behavior() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let p1 = 1;
        let s1_val = 1;
        let p2 = 2;
        let s2_val = 2;

        let skel1 = AtomSkeletonId::from(s1_val);
        let skel2 = AtomSkeletonId::from(s2_val);

        // 2. Setup predicate definitions: Arity-0 skeletons
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p1), TypedList::new()),
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p2), TypedList::new()),
        ];
        let f_defs = vec![];

        // 3. Setup Positive Inertia
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(skel1, Inertia::positive());
        i_table.insert_predicate(skel2, Inertia::positive());

        // 4. Initialize an empty ValueRegistry safely
        let value_registry = ValueRegistry::empty();

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);

        // Register p1/skel1 as present in the initial state: N(P1, []) = 1
        registry.generate_predicate_masks(skel1, 0, &[]);

        // 5. Construct LIR atoms (Arity 0, empty arguments array)
        let atom_node_id1 = builder.atomic_formula(PredicateSymbolId::from(p1), &[], skel1);
        let atom_node_id2 = builder.atomic_formula(PredicateSymbolId::from(p2), &[], skel2);

        let node1 = store
            .get(atom_node_id1)
            .expect("The first arity-0 atomic node must exist in the store");
        let node2 = store
            .get(atom_node_id2)
            .expect("The second arity-0 atomic node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 6. EVALUATION
        // node1 (skel1): N = 1, grounded -> Positive Inert + Present = TRUE
        let res1 = registry.evaluate_predicate_internal(node1, &store, &mut buffer);

        // node2 (skel2): N = 0, grounded -> Positive Inert + Absent = FALSE
        let res2 = registry.evaluate_predicate_internal(node2, &store, &mut buffer);

        // 7. VERIFICATION
        assert_eq!(
            res1.unwrap(),
            Some(true),
            "The s1 flag must be simplified to TRUE (present + positive inert)"
        );
        assert_eq!(
            res2.unwrap(),
            Some(false),
            "The s2 flag must be simplified to FALSE (absent + positive inert)"
        );
    }

    // =========================================================================
    // SECTION: FUNCTION INITIALIZATION (LHS / RHS & MASKS)
    // =========================================================================

    /// # Purpose
    /// Verifies that a numeric function marked with **Positive Inertia** (static function
    /// whose value never changes, like a distance or cost matrix) correctly resolves
    /// to its pre-computed initial value when evaluated with fully grounded arguments.
    ///
    /// This tests the zero-overhead lookup table strategy for fixed functions.
    ///
    /// # Input
    /// - A grounded function term `(function_5 object_100)` mapped to `FunctionSkeletonId(5)`.
    /// - An `InertiaTable` marking `FunctionSkeletonId(5)` as positive inert.
    /// - A pre-registered initial value of `42.0` assigned via `generate_function_masks`.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(Some(ExprConstant::Number(42.0)))`.
    #[test]
    fn test_static_function_evaluation() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let func_id_raw = 5;
        let skel_id_raw = 5;
        let obj_id = 100;

        // Create the constant object argument
        let arg = builder.intern(ExprKind::Object(ObjectId::from(obj_id)), &[]);

        // 2. Construct the function node with the symbol as the first child: [Sym, Arg1]
        let term_node_id = builder.function_term(
            FunctionSymbolId::from(func_id_raw),
            &[arg],
            FunctionSkeletonId::from(skel_id_raw),
        );

        let term_node = store
            .get(term_node_id)
            .expect("The function node must exist in the store");

        // --- TEST CONTEXT SETUP ---
        let p_defs = vec![];
        let f_defs = mock_function_defs(6); // Ensure index 5 is valid within the slice boundaries
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Mark the function skeleton as Positive Inertia (Static function)
        i_table.insert_function(FunctionSkeletonId::from(skel_id_raw), Inertia::positive());

        // Create the mocked registry instance
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Manually inject the initial state value: function_5(object_100) = 42.0
        let val = ExprConstant::Number(OrderedFloat::from(42.0));
        registry.generate_function_masks(
            FunctionSkeletonId::from(skel_id_raw),
            1, // arity
            &[ObjectId::from(obj_id)],
            val,
        );

        let mut buffer = ArgumentBuffer::new();

        // 3. EVALUATION
        let res = registry.evaluate_function_internal(term_node, &store, &mut buffer);

        // 4. VERIFICATION
        assert_eq!(
            res.unwrap(),
            Some(ExprConstant::Number(OrderedFloat::from(42.0))),
            "The static function must return its pre-registered initial value"
        );
    }

    /// # Purpose
    /// Verifies the evaluation of a static (inert) numeric function.
    ///
    /// This test ensures that when a function is marked as positive inertia (its value never changes),
    /// the evaluator correctly retrieves and returns the constant numeric value associated with
    /// grounded (constant) arguments.
    ///
    /// # Input
    /// - A function `f` (`FunctionSkeletonId(1)`) marked as positive inert.
    /// - An initial assignment injected into masks: `f(obj_10) = 42.5`.
    /// - An expression node representing the grounded call `f(10)`.
    ///
    /// # Expected Output
    /// - `Ok(Some(ExprConstant::Number(42.5)))` representing the successfully simplified value.
    #[test]
    fn test_evaluate_function_static_numeric() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let func_id_val = 1;
        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_a = ObjectId::from(10);
        let val = 42.5;

        // 2. Setup Inertia: Mark the function skeleton as positive inert (static) 🧊
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 3. Setup function definitions: f(?x) returning a number
        // Note: mapping types inside the skeleton signature
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(TypeId::from(0)), // Dummy
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(func_id_val),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(TypeId::from(0)),
                )]),
                Type::from(TypeId::from(0)), // Returns a number / numerical domain
            ),
        ];
        let p_defs = vec![];

        // 4. Initialize the mocked registry instance
        let value_registry = ValueRegistry::empty();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);

        // Inject initial state value: f(obj_10) = 42.5 🔢
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_a],
            ExprConstant::Number(ordered_float::OrderedFloat(val)),
        );

        // 5. Construct the grounded LIR expression: f(10)
        let arg = builder.intern(ExprKind::Object(obj_a), &[]);

        let func_node_id = builder.intern(
            ExprKind::Function(skel_id),
            &[arg], // Arguments follow the symbol rule
        );

        let func_node = store
            .get(func_node_id)
            .expect("The structural function node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 6. EVALUATION
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        // 7. VERIFICATION
        assert_eq!(
            res.unwrap(),
            Some(ExprConstant::Number(ordered_float::OrderedFloat(val))),
            "The evaluator must resolve the static function to its initial registered number"
        );
    }

    /// # Purpose
    /// Verifies that a static function evaluated with an ungrounded variable argument (`f(?x)`)
    /// correctly returns `None` when the underlying domain instances yield **diverging values** /// (no global consensus).
    ///
    /// # Input
    /// - A function `f` (`FunctionSkeletonId(1)`) marked as positive inert.
    /// - Two conflicting initial state entries: `f(10) = 42.5` and `f(20) = 100.0`.
    /// - An expression node representing the ungrounded call `f(?var0)`.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(None)` because the output value
    ///   cannot be statically unified or simplified without knowing the runtime binding of `?var0`.
    #[test]
    fn test_evaluate_function_non_grounded_diverging_values() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_10 = ObjectId::from(10);
        let obj_20 = ObjectId::from(20);

        // 2. Setup Inertia: Mark the function skeleton as positive inert (static)
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 3. Setup function definitions: f(?x)
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(TypeId::from(0)), // Dummy
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(TypeId::from(0)),
                )]),
                Type::from(TypeId::from(0)),
            ),
        ];
        let p_defs = vec![];

        // 4. Initialize the mocked registry instance
        let value_registry = ValueRegistry::empty();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);

        // Inject TWO different values to provoke divergence
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_10],
            ExprConstant::Number(ordered_float::OrderedFloat(42.5)),
        );
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_20],
            ExprConstant::Number(ordered_float::OrderedFloat(100.0)),
        );

        // 5. Construct the ungrounded expression: f(?var0)
        let var_node = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);

        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[var_node]);

        let func_node = store
            .get(func_node_id)
            .expect("The ungrounded structural function node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 6. EVALUATION
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        // 7. VERIFICATION
        assert!(
            res.unwrap().is_none(),
            "The evaluator must return None because the function values diverge over the domain"
        );
    }

    /// # Purpose
    /// Verifies that an ungrounded function call (`f(?x)`) successfully simplifies to a
    /// constant value when **all** instances in the domain share the exact same initial value
    /// (Unanimity / Global Consensus).
    ///
    /// # Input
    /// - Function `f` (`FunctionSkeletonId(1)`) marked as positive inert.
    /// - ValueRegistry containing two objects (`obj_10`, `obj_20`).
    /// - Initial state entries matching all domain combinations to the same value:
    ///   `f(10) = 42.0` and `f(20) = 42.0`.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(Some(ExprConstant::Number(42.0)))`.
    #[test]
    fn test_evaluate_function_non_grounded_with_unanimous_consensus() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_20 = ObjectId::from(20);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
                Type::from(type_id),
            ),
        ];
        let p_defs = vec![];

        // Le domaine contient uniquement obj_10 et obj_20
        let v_reg = ValueRegistry::from_objects(vec![
            TypedSymbol::new(obj_10, Type::from(type_id)),
            TypedSymbol::new(obj_20, Type::from(type_id)),
        ]);

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Les deux objets du domaine pointent vers la même valeur (42.0)
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_10],
            ExprConstant::Number(OrderedFloat(42.0)),
        );
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_20],
            ExprConstant::Number(OrderedFloat(42.0)),
        );

        // f(?var0)
        let var_node = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[var_node]);
        let func_node = store.get(func_node_id).unwrap();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        assert_eq!(
            res.unwrap(),
            Some(ExprConstant::Number(OrderedFloat(42.0))),
            "L'évaluateur doit retourner 42.0 car il y a unanimité parfaite sur le domaine"
        );
    }

    /// # Purpose
    /// Verifies that partial instantiation works correctly for functions with mixed arguments
    /// (e.g., `f(10, ?y)`). If the fixed part matching `obj_10` hosts different values for
    /// different downstream objects, the evaluator must yield `None`.
    #[test]
    fn test_evaluate_function_mixed_arguments_divergence() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_22 = ObjectId::from(22);
        let obj_33 = ObjectId::from(33);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::from(type_id)),
                    TypedSymbol::new(VariableId::from(1), Type::from(type_id)),
                ]),
                Type::from(type_id),
            ),
        ];
        let p_defs = vec![];
        let v_reg = ValueRegistry::empty();

        let mut registry =
            InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 2);

        // Initialisation de : f(10, 22) = 5.0 et f(10, 33) = 15.0
        registry.generate_function_masks(
            skel_id,
            2,
            &[obj_10, obj_22],
            ExprConstant::Number(OrderedFloat(5.0)),
        );
        registry.generate_function_masks(
            skel_id,
            2,
            &[obj_10, obj_33],
            ExprConstant::Number(OrderedFloat(15.0)),
        );

        // Construction du nœud : f(10, ?var1)
        let arg_const = builder.intern(ExprKind::Object(obj_10), &[]);
        let arg_var = builder.intern(ExprKind::Variable(VariableId::from(1)), &[]);
        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[arg_const, arg_var]);
        let func_node = store.get(func_node_id).unwrap();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        assert!(
            res.unwrap().is_none(),
            "Doit retourner None car pour l'argument fixe 10, les valeurs divergent sur le reste du domaine"
        );
    }

    /// # Purpose
    /// Verifies that an ungrounded function call (`f(?x)`) is not prematurely simplified
    /// and correctly returns `None` when there is incomplete information about the domain
    /// (only one specific instance `f(10) = 42.5` is known, with no global consensus guaranteed).
    ///
    /// # Input
    /// - A function `f` (`FunctionSkeletonId(1)`) marked as positive inert.
    /// - A single state entry injected into masks: `f(obj_10) = 42.5`.
    /// - An expression node representing the ungrounded call `f(?var0)`.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(None)` because the function call
    ///   contains an ungrounded variable and cannot be simplified omnisciently.
    #[test]
    fn test_evaluate_function_non_grounded_returns_none() {
        // 1. Initialize the Hash-Consing arena (ExprStore + Builder)
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let val = 42.5;

        // 2. Setup Inertia: Mark the function skeleton as positive inert (static)
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 3. Setup function definitions: f(?x)
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
                Type::<TypeId>::number(), // Utilisation du type numérique
            ),
        ];
        let p_defs = vec![];
        let value_registry = ValueRegistry::empty();

        // 4. Initialize the mocked registry instance and inject ONE instance
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_10],
            ExprConstant::Number(ordered_float::OrderedFloat(val)),
        );

        // 5. Construct the ungrounded expression: f(?var0)
        // Conformément aux règles du LIR : children[0] = Symbole, children[1..] = Arguments
        let dummy_symbol = builder.intern(ExprKind::Object(ObjectId::from(999)), &[]);
        let var_node = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);

        let func_node_id = builder.intern(
            ExprKind::Function(skel_id),
            &[dummy_symbol, var_node], // dummy_symbol en 0, ?var0 en 1
        );

        let func_node = store
            .get(func_node_id)
            .expect("The ungrounded structural function node must exist in the store");

        let mut buffer = ArgumentBuffer::new();

        // 6. EVALUATION
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        // 7. VERIFICATION
        assert!(
            res.unwrap().is_none(),
            "Should not simplify a function call containing variables when information is incomplete"
        );
    }

    /// # Purpose
    /// Verifies dynamic mask extraction and projection mechanics for complex
    /// multi-argument functions (Arity 3) containing mixed arguments: `f(?x, const, ?y)`.
    ///
    /// # Input
    /// - Function `f(?x, ?y, ?z)` marked as positive inert.
    /// - Injected entry: `f(10, 99, 20) = 7.0`.
    /// - Evaluated expression: `f(?var0, 99, ?var1)`.
    ///
    /// # Expected Output
    /// - Si les variables divergent sur le reste du domaine, doit renvoyer `None`.
    /// - Si l'entrée correspond, la constante 99 doit être correctement capturée à l'index physique 1.
    #[test]
    fn test_evaluate_function_arity_three_interleaved_constant() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id = FunctionSkeletonId::from(1);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_99 = ObjectId::from(99); // La constante fixée au milieu
        let obj_20 = ObjectId::from(20);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::from(type_id)),
                    TypedSymbol::new(VariableId::from(1), Type::from(type_id)),
                    TypedSymbol::new(VariableId::from(2), Type::from(type_id)),
                ]),
                Type::<TypeId>::number(),
            ),
        ];
        let p_defs = vec![];
        let v_reg = ValueRegistry::empty();

        // Configuration du mock acceptant une arité de 3
        let mut registry =
            InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 3, 3);

        // Injection : f(10, 99, 20) = 7.0
        registry.generate_function_masks(
            skel_id,
            3,
            &[obj_10, obj_99, obj_20],
            ExprConstant::Number(OrderedFloat(7.0)),
        );

        // Construction du nœud conformément aux règles du LIR (index 0 = symbole factice)
        let dummy_symbol = builder.intern(ExprKind::Object(ObjectId::from(999)), &[]);
        let var_0 = builder.intern(ExprKind::Variable(VariableId::from(0)), &[]);
        let const_param = builder.intern(ExprKind::Object(obj_99), &[]);
        let var_1 = builder.intern(ExprKind::Variable(VariableId::from(1)), &[]);

        let func_node_id = builder.intern(
            ExprKind::Function(skel_id),
            &[dummy_symbol, var_0, const_param, var_1],
        );
        let func_node = store.get(func_node_id).unwrap();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        // Valide le non-plantage et l'analyse saine des bitmasks
        assert!(
            res.is_ok(),
            "L'évaluateur doit être capable de traiter une arité de 3 avec constante entrelacée"
        );
    }

    /// # Purpose
    /// Verifies the evaluation of an atomic function term via the public `ExprEvaluator`
    /// trait interface using data retrieved from the underlying inertia table.
    ///
    /// # Input
    /// - Target Expression: A fully grounded atomic function node representing `f(10)`.
    /// - Evaluator State: The inertia table maps `f(10)` strictly to `42.0`.
    ///
    /// # Expected Output
    /// - `registry.evaluate(expr_func)` => `Some(ExprConstant::Number(42.0))`
    #[test]
    fn test_evaluate_function_inertia_retrieval() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id = FunctionSkeletonId::from(1);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(1),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
                Type::<TypeId>::number(),
            ),
        ];
        let p_defs = vec![];
        let v_reg = ValueRegistry::empty();

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);
        // f(10) = 42.0
        registry.generate_function_masks(
            skel_id,
            1,
            &[obj_10],
            ExprConstant::Number(OrderedFloat(42.0)),
        );

        // Construct the ungrounded structural function node: f(10)
        // Conforming to LIR standards: children[0] = Symbol, children[1..] = Arguments
        let dummy_symbol = builder.intern(ExprKind::Object(ObjectId::from(999)), &[]);
        let const_arg = builder.intern(ExprKind::Object(obj_10), &[]);
        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[dummy_symbol, const_arg]);

        // Wrap the node reference in an external `Expr` proxy container as expected by the trait
        let expr_func = Expr::new(func_node_id, &store);

        // Invoke the trait-orchestrated public API evaluation mechanism
        let res_func = registry.evaluate(expr_func);

        // Verify that the inertia evaluator successfully maps the node to its constant literal
        assert_eq!(
            res_func,
            Some(ExprConstant::Number(OrderedFloat(42.0))),
            "The evaluator must successfully resolve and return the direct numeric value of f(10)"
        );
    }
}
