use crate::aiplan4rust::grounding::binding::error::BindingError;
// Importation unique de l'évaluateur et de ses valeurs (ajuste selon ton déplacement final)
use crate::aiplan4rust::grounding::binding::Bindings;

use crate::aiplan4rust::grounding::binding::evaluator::{ExprConstant, ExprEvaluator};
use crate::aiplan4rust::lir::expr::expr::Expr;
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprEntryKind, ExprId, ExprStore};
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;

pub struct ExprBinder {
    /// Buffer interne pour la correspondance des IDs (Ancien ID -> Nouvel ID)
    substitution_map: FxHashMap<ExprId, ExprId>,
    /// Buffer interne pour accumuler les enfants traduits
    children_buffer: Vec<ExprId>,
    /// Pile de travail pour le parcours post-ordre sur-mesure (Évite tout parcours inutile)
    stack: Vec<(ExprId, bool)>,
}

impl ExprBinder {
    pub fn new() -> Self {
        Self {
            substitution_map: FxHashMap::with_capacity_and_hasher(32, Default::default()),
            children_buffer: Vec::with_capacity(8),
            stack: Vec::with_capacity(32),
        }
    }

    /// **Version Standard** : Applique uniquement les substitutions de variables (Grounding classique).
    #[inline]
    pub fn bind(
        &mut self,
        expr_id: ExprId,
        store: &mut ExprStore,
        sub: &Bindings,
    ) -> Result<ExprId, BindingError> {
        self.bind_internal(expr_id, store, sub, None)
    }

    /// **Version Avancée** : Applique les substitutions et exécute l'élagage statique (Köhler)
    /// à la volée en une seule passe, évitant la pollution du `ExprStore`.
    #[inline]
    pub fn bind_with(
        &mut self,
        expr_id: ExprId,
        store: &mut ExprStore,
        sub: &Bindings,
        evaluator: Option<&dyn ExprEvaluator>,
    ) -> Result<ExprId, BindingError> {
        self.bind_internal(expr_id, store, sub, evaluator)
    }

    /// Le moteur de parcours unique, privé et ultra-optimisé.
    fn bind_internal(
        &mut self,
        expr_id: ExprId,
        store: &mut ExprStore,
        sub: &Bindings,
        evaluator: Option<&dyn ExprEvaluator>,
    ) -> Result<ExprId, BindingError> {
        self.substitution_map.clear();
        self.stack.clear();

        // On initialise notre builder unique
        let mut builder = ExprBuilder::new(store);

        // On pousse la racine sur notre pile locale.
        self.stack.push((expr_id, false));
        // On marque également la racine comme planifiée
        self.substitution_map.insert(expr_id, expr_id);

        while let Some((old_id, children_pushed)) = self.stack.pop() {
            if !children_pushed {
                // --- PREMIER PASSAGE : On pousse les enfants sur la pile ---
                self.stack.push((old_id, true));

                let entry = &builder.store[old_id];
                for &child_id in entry.children().iter().rev() {
                    // MICRO-OPTIMISATION : Évite l'exploration multiple des nœuds dupliqués.
                    if let Entry::Vacant(v) = self.substitution_map.entry(child_id) {
                        v.insert(child_id);
                        self.stack.push((child_id, false));
                    }
                }
            } else {
                // --- SECOND PASSAGE : Tous les enfants sont résolus, on traite le nœud ---
                let mut has_changed = false;
                self.children_buffer.clear();

                // Scope éphémère pour inspecter le nœud sans bloquer builder.store
                let mut current_id = {
                    let entry = &builder.store[old_id];

                    for &child_id in entry.children() {
                        let new_child_id =
                            *self.substitution_map.get(&child_id).unwrap_or(&child_id);
                        if new_child_id != child_id {
                            has_changed = true;
                        }
                        self.children_buffer.push(new_child_id);
                    }

                    // Transformation ZÉRO-CLONE
                    match entry.kind() {
                        ExprEntryKind::Variable(var_id) => {
                            if let Some(obj_id) = sub.get(var_id) {
                                builder.object(obj_id)
                            } else {
                                old_id
                            }
                        }
                        _ => {
                            if has_changed {
                                let kind_to_reconstruct = entry.kind().clone();
                                builder.reconstruct(kind_to_reconstruct, &self.children_buffer)?
                            } else {
                                old_id
                            }
                        }
                    }
                };

                // --- Élagage métier Köhler ---
                if let Some(eval) = evaluator {
                    let current_kind = builder.store[current_id].kind();

                    if is_evaluable(current_kind) {
                        let expr_wrapper = Expr::new(current_id, builder.store);

                        if let Some(static_val) = eval.evaluate(expr_wrapper) {
                            current_id = match static_val {
                                ExprConstant::Boolean(true) => builder.store.empty_and(),
                                ExprConstant::Boolean(false) => builder.store.empty_or(),
                                ExprConstant::Number(n) => builder.number(*n),
                                ExprConstant::Object(o) => builder.object(o),
                            };
                        }
                    }
                }

                // On écrase la valeur sentinelle par le véritable ID final calculé
                self.substitution_map.insert(old_id, current_id);
            }
        }

        Ok(*self.substitution_map.get(&expr_id).unwrap_or(&expr_id))
    }
}

/// Filtre pour savoir si on doit soumettre le nœud à l'évaluateur statique.
#[inline]
fn is_evaluable(kind: &ExprEntryKind) -> bool {
    match kind {
        ExprEntryKind::AtomicFormula(_) | ExprEntryKind::Function(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, ObjectId, PredicateSymbolId, VariableId};

    /// Explanation: Verifies that a variable expression is correctly replaced
    ///              by its corresponding object mapping defined in the bindings.
    /// Input:       An expression node of type `ExprEntryKind::Variable` and a
    ///              `Bindings` map containing the variable-to-object association.
    /// Expected Output: The new `ExprId` pointing to an `ExprEntryKind::Object`
    ///                  containing the mapped `ObjectId`.
    #[test]
    fn test_bind_variable_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create a variable and its binding definition
        let var_id = VariableId::new(42);
        let obj_id = ObjectId::new(99);
        let expr_var = builder.variable(var_id);

        let mut bindings = Bindings::new();
        bindings.insert(var_id, obj_id);

        // 2. Execute the binder
        let mut binder = ExprBinder::new();
        let result = binder.bind(expr_var, &mut store, &bindings);

        // 3. Assertions
        assert!(result.is_ok());
        let final_id = result.unwrap();

        assert_ne!(final_id, expr_var);
        if let ExprEntryKind::Object(o) = store[final_id].kind() {
            assert_eq!(*o, obj_id);
        } else {
            panic!("The final node should be an Object.");
        }
    }

    /// Explanation: Ensures the ZERO-CLONE optimization works properly. If an expression
    ///              contains no variables, it should not trigger any reconstruction or
    ///              store pollution, returning the exact same node ID.
    /// Input:       An expression node with no variables (e.g., a constant number)
    ///              and an empty `Bindings` map.
    /// Expected Output: The exact same `ExprId` as the input, meaning no change occurred.
    #[test]
    fn test_bind_no_change_returns_same_id() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create a static node with no variables
        let expr_num = builder.number(10);
        let bindings = Bindings::new();

        let mut binder = ExprBinder::new();
        let result = binder.bind(expr_num, &mut store, &bindings).unwrap();

        // ZERO-CLONE optimization check
        assert_eq!(result, expr_num);
    }

    /// Explanation: Validates the advanced Köhler optimization pass. When an evaluable node
    ///              (like an atomic formula) is processed with a `StaticEvaluator` that resolves
    ///              it to a static boolean value, the node should be pruned directly on the fly.
    /// Input:       An evaluable atomic formula expression and a mock evaluator configured
    ///              to return `StaticValue::Boolean(true)` for this specific node.
    /// Expected Output: The `ExprId` corresponding to the empty "AND" expression (`store.empty_and()`),
    ///                  which represents a statically true condition.
    #[test]
    fn test_bind_with_static_evaluation_pruning() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create a simulated atomic formula matching the new signature requirement
        let predicate_id = PredicateSymbolId::new(1);
        let skeleton_id = AtomSkeletonId::new(1); // Added to satisfy the third argument `skel_id`

        // Updated call with 3 arguments: symbol, arguments slice, and skeleton ID
        let expr_atomic = builder.atomic_formula(predicate_id, &[], skeleton_id);

        let bindings = Bindings::new();

        // Configure the mock evaluator to return true for this node
        let evaluator = MockEvaluator {
            should_be_true: expr_atomic,
        };

        // 2. Call the advanced `bind_with` pipeline
        let mut binder = ExprBinder::new();
        let result = binder.bind_with(expr_atomic, &mut store, &bindings, Some(&evaluator));

        assert!(result.is_ok());
        let final_id = result.unwrap();

        // 3. The node should be pruned and replaced by the identity element of AND
        let expected_empty_and = store.empty_and();
        assert_eq!(final_id, expected_empty_and);
    }

    // --- MOCK DEFINITION FOR THE STATIC EVALUATOR ---
    struct MockEvaluator {
        should_be_true: ExprId,
    }

    impl ExprEvaluator for MockEvaluator {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            // Using root_id() as per your updated Mock definition
            if expr.root_id() == self.should_be_true {
                Some(ExprConstant::Boolean(true))
            } else {
                None
            }
        }
    }

    /// Explanation: Verifies basic variable substitution within an atomic formula structure.
    /// Input:       An atomic formula expression containing a variable `?x` and a
    ///              binding map `{ ?x -> obj_100 }`.
    /// Expected Output: A new expression ID where the variable is replaced by the object.
    #[test]
    fn test_bind_atomic_formula_basic_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);
        let pred_id = PredicateSymbolId::new(10);
        let skel_id = AtomSkeletonId::new(1);

        let v_node = builder.variable(var_x);
        let root = builder.atomic_formula(pred_id, &[v_node], skel_id);

        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        let mut binder = ExprBinder::new();
        let result = binder.bind(root, &mut store, &bindings);

        assert!(result.is_ok());
        let new_root = result.unwrap();

        assert_ne!(
            new_root, root,
            "Root should change because a child was substituted"
        );

        let children = store[new_root].children();
        let arg_id = children[1]; // Index 1 assuming predicate symbol node is at index 0

        if let ExprEntryKind::Object(id) = store[arg_id].kind() {
            assert_eq!(*id, obj_100, "The substituted object ID is incorrect");
        } else {
            panic!("The argument was not substituted into an Object.");
        }
    }

    /// Explanation: Validates that a multi-variable atomic formula can be partially
    ///              grounded when only one of the variables is present in the bindings.
    /// Input:       An atomic formula with variables `?x` and `?z`, and a binding map containing only `?x`.
    /// Expected Output: A new node where `?x` became an Object, but `?z` remains a Variable.
    #[test]
    fn test_bind_partial_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x_id = VariableId::new(1);
        let z_id = VariableId::new(99);
        let obj_100 = ObjectId::new(100);

        let var_x = builder.variable(x_id);
        let var_z = builder.variable(z_id);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[var_x, var_z],
            AtomSkeletonId::new(1),
        );

        let mut bindings = Bindings::new();
        bindings.insert(x_id, obj_100);

        let mut binder = ExprBinder::new();
        let new_root = binder.bind(root, &mut store, &bindings).unwrap();

        assert_ne!(new_root, root);
        let children = store[new_root].children();

        // Verify ?x -> Object(100)
        let arg_x = children[1];
        if let ExprEntryKind::Object(id) = store[arg_x].kind() {
            assert_eq!(*id, obj_100);
        } else {
            panic!("First argument should be an Object");
        }

        // Verify ?z remains Variable(99)
        let arg_z = children[2];
        if let ExprEntryKind::Variable(id) = store[arg_z].kind() {
            assert_eq!(*id, z_id);
        } else {
            panic!("Second argument should remain a Variable");
        }
    }

    /// Explanation: Confirms ZERO-CLONE efficiency on complex formulas. If bindings are empty,
    ///              the binder must return the exact same root ID without modifying the store.
    /// Input:       An expression tree and an empty bindings map.
    /// Expected Output: The returned `ExprId` is strictly equal to the input `root` ID.
    #[test]
    fn test_bind_zero_clone_on_empty_bindings() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::new(1));
        let root =
            builder.atomic_formula(PredicateSymbolId::new(10), &[var_x], AtomSkeletonId::new(1));

        let bindings = Bindings::new(); // Empty
        let mut binder = ExprBinder::new();

        let new_root = binder.bind(root, &mut store, &bindings).unwrap();

        // Crucial optimization check: IDs must match
        assert_eq!(
            new_root, root,
            "Zero-clone optimization failed: identical structures must preserve IDs"
        );
    }

    /// Explanation: Tests structural pruning propagation via a static evaluator during the binding pass.
    ///              Simulates a branch getting optimized to `False` (EmptyOr) by targeting a specific PDDL definition ID.
    /// Input:       A compound expression containing an evaluable atomic formula, paired with an evaluator
    ///              configured to prune atomic formulas carrying the targeted PDDL definition ID (AtomSkeletonId).
    /// Expected Output: The root is instantly rewritten to the store's static false indicator (`empty_or`).
    #[test]
    fn test_bind_with_evaluator_pruning_propagation_false() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let target_predicate = PredicateSymbolId::new(10);
        let target_skeleton = AtomSkeletonId::new(1); // The PDDL definition argument

        let var_x = builder.variable(VariableId::new(1));

        // The first child will contain the predicate symbol, while the enum holds the target_skeleton
        let expr_atomic = builder.atomic_formula(target_predicate, &[var_x], target_skeleton);

        let mut bindings = Bindings::new();
        bindings.insert(VariableId::new(1), ObjectId::new(100));

        // Setup the target-aware mock evaluator with the skeleton ID
        let evaluator = MockEvaluatorPruneFalse { target_skeleton };

        let mut binder = ExprBinder::new();
        let new_root = binder
            .bind_with(expr_atomic, &mut store, &bindings, Some(&evaluator))
            .unwrap();

        // An evaluation to false should translate to empty_or
        let expected_false = store.empty_or();
        assert_eq!(
            new_root, expected_false,
            "The expression should be compiled down to False (empty_or)"
        );
    }

    // --- MOCK DEFINITION (MATCHING THE PDDL DEFINITION ARGUMENT) ---
    struct MockEvaluatorPruneFalse {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorPruneFalse {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            let expr_id = expr.root_id();
            let entry_kind = expr.store()[expr_id].kind();

            // We match directly on the AtomSkeletonId argument carried by the enum variant
            match entry_kind {
                ExprEntryKind::AtomicFormula(skel_id) => {
                    if *skel_id == self.target_skeleton {
                        Some(ExprConstant::Boolean(false))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    /// Explanation: Tests that a node sharing duplicate children (diamond structure) is processed
    ///              efficiently without double-traversing the child branch, and reconstructs correctly.
    /// Input:       A parent node (e.g., an imaginary logical group or connector) pointing twice to
    ///              the exact same variable child node ID, with a valid substitution.
    /// Expected Output: The parent node is successfully reconstructed with both children pointing
    ///                  to the newly substituted object ID.
    #[test]
    fn test_bind_duplicate_children_diamond_sharing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);
        let v_node = builder.variable(var_x);

        // We simulate a node holding the same child twice (using a custom kind or standard container if applicable)
        // For testing, let's assume builder.and or a similar array-based container takes a slice of children.
        // If your builder layout differs for generic containers, adapt this to a multi-child kind.
        let target_skeleton = AtomSkeletonId::new(5);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(2),
            &[v_node, v_node],
            target_skeleton,
        );

        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        let mut binder = ExprBinder::new();
        let result = binder.bind(root, &mut store, &bindings);

        assert!(result.is_ok());
        let new_root = result.unwrap();

        let children = store[new_root].children();
        // Skip predicate index 0, check arguments at index 1 and 2
        assert_eq!(
            children[1], children[2],
            "Both arguments should point to the exact same new object entry via Hash-Consing"
        );
        if let ExprEntryKind::Object(id) = store[children[1]].kind() {
            assert_eq!(*id, obj_100);
        } else {
            panic!("Diamond substitution failed.");
        }
    }

    /// Explanation: Verifies that the advanced evaluator pass can rewrite an evaluable node
    ///              into a non-boolean constant, specifically a StaticValue::Number.
    /// Input:       An evaluable node (e.g., a Function node) and a mock evaluator returning a Number(42.0).
    /// Expected Output: The binder intercepts the number evaluation and returns the corresponding store number ID.
    #[test]
    fn test_bind_with_static_evaluation_to_number() {
        let mut store = ExprStore::new();

        // 1. Scope isolated to release the mutable borrow on `store` before binding
        let (expr_function, function_skeleton) = {
            let mut builder = ExprBuilder::new(&mut store);
            let var_x = builder.variable(VariableId::new(1));
            let function_skeleton = AtomSkeletonId::new(42);
            let expr_function =
                builder.atomic_formula(PredicateSymbolId::new(9), &[var_x], function_skeleton);
            (expr_function, function_skeleton)
        }; // `builder` dropped here -> `store` can be borrowed again

        let bindings = Bindings::new();
        let evaluator = MockEvaluatorNumber {
            target_skeleton: function_skeleton,
        };

        // 2. Execute the binder safely
        let mut binder = ExprBinder::new();
        let new_root = binder
            .bind_with(expr_function, &mut store, &bindings, Some(&evaluator))
            .unwrap();

        // 3. Re-create a short-lived builder to fetch the expected constant ID
        let expected_number_node = ExprBuilder::new(&mut store).number(42.0);

        assert_eq!(
            new_root, expected_number_node,
            "The function node should be pruned down to a constant Number entry"
        );
    }

    /// Explanation: Ensures that non-evaluable nodes are completely shielded from the evaluator
    ///              pipeline, bypassing any accidental static pruning.
    /// Input:       A Variable node (which is NOT evaluable according to `is_evaluable`)
    ///              and an aggressive mock evaluator that tries to prune everything.
    /// Expected Output: The evaluator is ignored, and standard grounding occurs.
    #[test]
    fn test_bind_non_evaluable_nodes_bypass_evaluator() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(7);
        let expr_var = builder.variable(var_x);

        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(88));

        let evaluator = MockEvaluatorAggressive;

        let mut binder = ExprBinder::new();
        let new_root = binder
            .bind_with(expr_var, &mut store, &bindings, Some(&evaluator))
            .unwrap();

        // Standard object transformation should happen, ignoring the evaluator's attempts to make it true
        if let ExprEntryKind::Object(id) = store[new_root].kind() {
            assert_eq!(*id, ObjectId::new(88));
        } else {
            panic!("Variable should have bypassed the evaluator and turned into an Object.");
        }
    }

    // --- ADDITIONAL MOCKS FOR THE EXTENDED EDGE CASES ---

    struct MockEvaluatorNumber {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorNumber {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            let expr_id = expr.root_id();
            if let ExprEntryKind::AtomicFormula(skel_id) = expr.store()[expr_id].kind() {
                if *skel_id == self.target_skeleton {
                    // Injecting a non-boolean static value type
                    return Some(ExprConstant::Number(42.0.into()));
                }
            }
            None
        }
    }

    struct MockEvaluatorAggressive;

    impl ExprEvaluator for MockEvaluatorAggressive {
        fn evaluate(&self, _expr: Expr) -> Option<ExprConstant> {
            // Aggressively tries to turn every incoming node into True
            Some(ExprConstant::Boolean(true))
        }
    }
}
