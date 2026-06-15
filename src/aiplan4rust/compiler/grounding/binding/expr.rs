use crate::aiplan4rust::compiler::grounding::binding::error::BindingError;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::{ExprConstant, ExprEvaluator};
use crate::aiplan4rust::compiler::grounding::binding::Bindings;
use crate::aiplan4rust::compiler::lir::expr::expr::Expr;
use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId, ExprKind, ExprStore};

use super::scratchpad::BindingScratchpad;

/// **Version Standard** : Applique uniquement les substitutions de variables (Grounding classique).
/// Alloue temporairement un scratchpad local.
#[inline]
pub fn bind(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
) -> Result<ExprId, BindingError> {
    let mut scratchpad = BindingScratchpad::new();
    bind_with(expr_id, store, sub, None, &mut scratchpad)
}

/// **Version Avancée** : Moteur de parcours unique, ultra-optimisé et sans allocation.
/// Applique les substitutions et exécute l'élagage statique (Köhler) à la volée.
pub fn bind_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
    evaluator: Option<&dyn ExprEvaluator>,
    scratchpad: &mut BindingScratchpad,
) -> Result<ExprId, BindingError> {
    scratchpad.clear();

    // On initialise notre builder unique pour l'interning
    let mut builder = ExprBuilder::new(store);

    // Amorçage du parcours itératif avec la racine
    scratchpad.stack.push((expr_id, false));
    // CORRECTION 1 : Ne pas pré-remplir la map ici avec la racine (on le fera à la remontée)

    while let Some((old_id, children_pushed)) = scratchpad.stack.pop() {
        if !children_pushed {
            // --- PREMIER PASSAGE : Descente ---

            // Sécurité DAG : Si ce nœud a déjà été traité et résolu par un autre chemin, on passe.
            if scratchpad.substitution_map.contains_key(&old_id) {
                continue;
            }

            scratchpad.stack.push((old_id, true));

            let entry = &builder.store[old_id];
            for &child_id in entry.children().iter().rev() {
                // CORRECTION 2 : Utiliser contains_key() à la place de Entry::Vacant + v.insert
                // On pousse l'enfant sur la pile UNIQUEMENT s'il n'a pas encore sa valeur finale calculée.
                if !scratchpad.substitution_map.contains_key(&child_id) {
                    scratchpad.stack.push((child_id, false));
                }
            }
        } else {
            // --- SECOND PASSAGE : Remontée ---

            // Sécurité DAG : Si le nœud a été finalisé par une autre branche pendant qu'il attendait, on skip.
            if scratchpad.substitution_map.contains_key(&old_id) {
                continue;
            }

            // Isolation de l'emprunt immuable sur `builder.store` pour extraire les infos du nœud
            let (entry_kind, has_children) = {
                let entry = &builder.store[old_id];
                (entry.kind().clone(), !entry.children().is_empty())
            };

            let mut has_changed = false;
            scratchpad.children_buffer.clear();

            // Si le nœud a des enfants, on collecte leurs correspondances mises à jour
            if has_children {
                let entry = &builder.store[old_id];
                for &child_id in entry.children() {
                    let new_child_id = *scratchpad
                        .substitution_map
                        .get(&child_id)
                        .unwrap_or(&child_id);
                    if new_child_id != child_id {
                        has_changed = true;
                    }
                    scratchpad.children_buffer.push(new_child_id);
                }
            }

            // Évaluation et reconstruction du nœud courant
            let mut current_id = match entry_kind {
                ExprKind::Variable(var_id) => {
                    if let Some(obj_id) = sub.get(&var_id) {
                        builder.object(obj_id)
                    } else {
                        old_id
                    }
                }
                _ => {
                    if has_changed {
                        builder.reconstruct(entry_kind, &scratchpad.children_buffer)?
                    } else {
                        old_id
                    }
                }
            };

            // --- Élagage métier Köhler ---
            if let Some(eval) = evaluator {
                let current_kind = builder.store[current_id].kind();

                if is_evaluable(current_kind) {
                    let expr_wrapper = Expr::new(current_id, builder.store);

                    if let Some(static_val) = eval
                        .evaluate(expr_wrapper)
                        .map_err(|e| BindingError::Evaluator(e))?
                    {
                        current_id = match static_val {
                            ExprConstant::Boolean(true) => builder.store.empty_and(),
                            ExprConstant::Boolean(false) => builder.store.empty_or(),
                            ExprConstant::Number(n) => builder.number(*n),
                            ExprConstant::Object(o) => builder.object(o),
                        };
                    }
                }
            }

            // On écrase la valeur définitive calculée dans notre table
            scratchpad.substitution_map.insert(old_id, current_id);
        }
    }

    Ok(*scratchpad
        .substitution_map
        .get(&expr_id)
        .unwrap_or(&expr_id))
}

/// Filtre pour savoir si on doit soumettre le nœud à l'évaluateur statique.
#[inline]
fn is_evaluable(kind: &ExprKind) -> bool {
    matches!(kind, ExprKind::AtomicFormula(_) | ExprKind::Function(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluatorError;
    use crate::aiplan4rust::support::lang::{
        AtomSkeletonId, ObjectId, PredicateSymbolId, VariableId,
    };

    /// ### Test: Bind Variable Substitution
    ///
    /// **Objective:**
    /// Verify that a pure isolated `Variable` leaf node is correctly intercepted and substituted
    /// by the binding engine. This ensures that the base case of the grounding translation
    /// accurately morphs the variable descriptor into its corresponding primitive `Object`
    /// identifier when a valid mapping exists.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: Variable(?x)]
    /// ```
    /// *Substitution:* `?x -> Object(99)`
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Object(99)]
    /// ```
    /// The engine must return a distinct `ExprId` mapping to a structural `Object` variant,
    /// effectively replacing the variable placeholder with the concrete domain object reference.
    #[test]
    fn test_bind_variable_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_id = VariableId::new(42);
        let obj_id = ObjectId::new(99);

        // 1. Construct a standalone Variable leaf node
        let expr_var = builder.variable(var_id);

        // 2. Setup standard bindings map linking the variable to a concrete Object ID
        let mut bindings = Bindings::new();
        bindings.insert(var_id, obj_id);

        // 3. Execute the standard grounding binding pipeline
        let result = bind(expr_var, &mut store, &bindings);

        // 4. Assert successful execution and extract the resulting node identifier
        assert!(result.is_ok(), "The variable binding execution failed.");
        let final_id = result.unwrap();

        assert_ne!(
            final_id, expr_var,
            "The final identifier must differ from the original variable identifier."
        );

        // 5. Validate that the primitive leaf successfully transformed into an Object variant
        if let ExprKind::Object(o) = store[final_id].kind() {
            assert_eq!(
                *o, obj_id,
                "The variable leaf was not substituted with the correct Object ID."
            );
        } else {
            panic!("The final resolved expression node should be an Object variant.");
        }
    }

    /// ### Test: Bind No Change Returns Same ID
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly preserves structural identity and short-circuits
    /// when processing an immutable primitive literal leaf (such as a `Number`) against an empty
    /// bindings context. This guarantees optimal cache hits and zero redundant arena entries.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: Number(10)]
    /// ```
    /// *Bindings:* Empty (No variables to substitute)
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Same Number ID]
    /// ```
    /// Because a literal number possesses no inner variables, the change detection mechanism
    /// must flag `has_changed` as false, causing the engine to return the original `ExprId` unchanged.
    #[test]
    fn test_bind_no_change_returns_same_id() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Construct an immutable literal Number leaf node
        let expr_num = builder.number(10);

        // 2. Instantiate an empty bindings map context
        let bindings = Bindings::new();

        // 3. Run the standard binding execution pipeline
        let result = bind(expr_num, &mut store, &bindings).unwrap();

        // 4. Assert that the returned node identifier is strictly identical to the input identifier
        assert_eq!(
            result, expr_num,
            "The binding engine must return the exact same input ID when no structural changes occur."
        );
    }

    /// ### Test: Bind With Static Evaluation Pruning
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly intercepts evaluable atomic expressions and
    /// prunes them into the canonical `True` constant (an empty `And` node) when the static
    /// evaluator determines the expression evaluates unconditionally to true.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: AtomicFormula]
    /// ```
    /// *Bindings:* Empty (No variable substitution)
    /// *Evaluator:* Evaluates the target atomic formula to `true`
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Empty And (True)]
    /// ```
    /// The Köhler pruning pipeline must intercept the atomic formula node during the upward phase,
    /// query the evaluator, and successfully map the final node identifier to `store.empty_and()`,
    /// short-circuiting any further compound reconstruction.
    #[test]
    fn test_bind_with_static_evaluation_pruning() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let predicate_id = PredicateSymbolId::new(1);
        let skeleton_id = AtomSkeletonId::new(1);

        // 1. Construct a ground atomic formula with no arguments
        let expr_atomic = builder.atomic_formula(predicate_id, &[], skeleton_id);

        // 2. Setup empty bindings and the specialized evaluator to trigger pruning to True
        let bindings = Bindings::new();
        let evaluator = MockEvaluator {
            should_be_true: expr_atomic,
        };

        // 3. Execute the advanced binding pipeline with the active evaluator and scratchpad
        let mut scratchpad = BindingScratchpad::new();
        let result = bind_with(
            expr_atomic,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        );

        // 4. Assert execution success and extract the final node identifier
        assert!(result.is_ok(), "The advanced binding execution failed.");
        let final_id = result.unwrap();

        // 5. Validate that the root node was successfully replaced by the canonical empty And (True)
        let expected_empty_and = store.empty_and();
        assert_eq!(
            final_id, expected_empty_and,
            "The atomic formula should have been statically pruned into the canonical empty And (True)."
        );
    }

    /// A mock evaluator designed to intercept a specific target expression ID
    /// and force its resolution to a static logical `true` constant.
    struct MockEvaluator {
        should_be_true: ExprId,
    }

    impl ExprEvaluator for MockEvaluator {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            if expr.root_id() == self.should_be_true {
                Ok(Some(ExprConstant::Boolean(true)))
            } else {
                Ok(None)
            }
        }
    }

    /// ### Test: Bind Atomic Formula Basic Substitution
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly performs a straightforward nominal variable
    /// substitution inside a flat compound expression (an `AtomicFormula`). This confirms that
    /// leaf variables are accurately detected during the upward phase, mapped to their primitive
    /// object counterparts, and that the parent node is correctly reconstructed with the new argument.
    ///
    /// **Input Structure:**
    /// ```text
    ///       [Root: AtomicFormula]
    ///                 |
    ///       [Arg0: Variable(?x)]
    /// ```
    /// *Substitution:* `?x -> Object(100)`
    ///
    /// **Expected Output:**
    /// ```text
    ///       [Root: AtomicFormula]
    ///                 |
    ///       [Arg0: Object(100)]
    /// ```
    /// The substitution must trigger a full reconstruction of the `AtomicFormula` root node,
    /// yielding a new structural `ExprId` whose arguments slice points directly to the
    /// interned `Object(100)` leaf.
    #[test]
    fn test_bind_atomic_formula_basic_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);
        let pred_id = PredicateSymbolId::new(10);
        let skel_id = AtomSkeletonId::new(1);

        // 1. Build the leaf Variable node and wrap it inside an AtomicFormula root
        let v_node = builder.variable(var_x);
        let root = builder.atomic_formula(pred_id, &[v_node], skel_id);

        // 2. Setup standard bindings map for the target variable ?x
        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        // 3. Execute the grounding binding pipeline
        let result = bind(root, &mut store, &bindings);

        // 4. Assert execution success and check that a new node was generated
        assert!(result.is_ok(), "The basic binding execution failed.");
        let new_root = result.unwrap();

        assert_ne!(
            new_root, root,
            "The root identifier must mutate since its underlying child argument changed structure."
        );

        // 5. Inspect the newly interned root children to validate substitution integrity
        let children = store[new_root].children();

        // Note: index 0 is typically reserved for predicate/skeleton metadata, arguments follow at index 1.
        let arg_id = children[1];

        if let ExprKind::Object(id) = store[arg_id].kind() {
            assert_eq!(
                *id, obj_100,
                "The atomic formula argument was not substituted with the correct Object ID."
            );
        } else {
            panic!("The atomic formula argument slot failed to transform into an Object variant.");
        }
    }

    /// ### Test: Bind Partial Substitution
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly handles partial substitutions within a compound
    /// expression (like an `AtomicFormula` with multiple arguments). It ensures that variables
    /// present in the bindings map are successfully replaced by object primitives, while unmapped
    /// variables remain completely untouched and structurally intact.
    ///
    /// **Input Structure:**
    /// ```text
    ///         [Root: AtomicFormula]
    ///          /                 \
    ///  [Arg0: Variable(?x)]  [Arg1: Variable(?z)]
    /// ```
    /// *Substitution:* `?x -> Object(100)` (No binding provided for `?z`)
    ///
    /// **Expected Output:**
    /// ```text
    ///         [Root: AtomicFormula]
    ///          /                 \
    ///  [Arg0: Object(100)]   [Arg1: Variable(?z)]
    /// ```
    /// The upward phase must detect a structural change (`has_changed == true`) due to `?x` mutating,
    /// forcing a reconstruction of the root formula. However, the child slot for `?z` must retain
    /// its original variable identifier exactly as before.
    #[test]
    fn test_bind_partial_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x_id = VariableId::new(1);
        let z_id = VariableId::new(99);
        let obj_100 = ObjectId::new(100);

        // 1. Construct the target variables and link them as arguments to an AtomicFormula
        let var_x = builder.variable(x_id);
        let var_z = builder.variable(z_id);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[var_x, var_z],
            AtomSkeletonId::new(1),
        );

        // 2. Setup partial bindings map containing only the substitution for ?x
        let mut bindings = Bindings::new();
        bindings.insert(x_id, obj_100);

        // 3. Execute the standard binding pipeline
        let new_root = bind(root, &mut store, &bindings).unwrap();

        // 4. Assert that a structural change occurred and a new root node was interned
        assert_ne!(
            new_root, root,
            "The root ID must change since at least one argument was substituted."
        );
        let children = store[new_root].children();

        // 5. Validate that the first argument (?x) successfully mutated into an Object primitive
        // Note: index 0 is typically reserved for predicate/skeleton metadata, arguments follow.
        let arg_x = children[1];
        if let ExprKind::Object(id) = store[arg_x].kind() {
            assert_eq!(
                *id, obj_100,
                "The first argument variable ?x was not substituted with the correct Object ID."
            );
        } else {
            panic!("The first argument slot should have transformed into an Object variant.");
        }

        // 6. Validate that the second argument (?z) remained structurally untouched as a Variable
        let arg_z = children[2];
        if let ExprKind::Variable(id) = store[arg_z].kind() {
            assert_eq!(
                *id, z_id,
                "The unmapped variable ?z was incorrectly modified or altered."
            );
        } else {
            panic!("The second argument slot should have remained a Variable variant.");
        }
    }

    /// ### Test: Bind Zero Clone On Empty Bindings
    ///
    /// **Objective:**
    /// Verify that the binding engine operates with zero allocations and zero structural modifications
    /// when evaluated against an empty map of variable bindings. This ensures that the engine
    /// short-circuits safely and returns the exact same root identifier without wasting cycles or
    /// creating redundant entries in the `ExprStore`.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: AtomicFormula]
    ///           |
    ///  [Child: Variable(1)]
    /// ```
    /// *Bindings:* Empty (No variables to substitute)
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Same AtomicFormula ID]
    /// ```
    /// Because the bindings map contains no substitutions for `Variable(1)`, the upward phase
    /// must detect that `has_changed` is false for all sub-branches, preventing any call to
    /// `reconstruct` and returning the original `root` ID unchanged.
    #[test]
    fn test_bind_zero_clone_on_empty_bindings() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Construct a standard atomic formula containing a variable leaf
        let var_x = builder.variable(VariableId::new(1));
        let root =
            builder.atomic_formula(PredicateSymbolId::new(10), &[var_x], AtomSkeletonId::new(1));

        // 2. Instantiate an empty bindings map
        let bindings = Bindings::new();

        // 3. Run the binding process
        let new_root = bind(root, &mut store, &bindings).unwrap();

        // 4. Assert that the resulting node identity is strictly equal to the input root identity
        assert_eq!(
            new_root, root,
            "The binding engine must perform zero modifications and return the identical input ID when bindings are empty."
        );
    }

    /// ### Test: Bind With Evaluator Pruning Propagation False
    ///
    /// **Objective:**
    /// Verify that when the static evaluator determines an evaluable atomic expression is
    /// unconditionally false, the binding engine drops the constructed sub-tree and correctly
    /// replaces its root identifier with the store's canonical `False` constant (empty `Or`).
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: AtomicFormula]
    ///           |
    ///  [Child: Variable(1)]
    /// ```
    /// *Substitution:* `1 -> Object(100)`
    /// *Evaluator:* Evaluates the target atomic formula skeleton to `false`
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Empty Or (False)]
    /// ```
    /// Even though a valid variable substitution takes place during the upward phase, the
    /// subsequent Köhler pruning step must override the reconstructed atomic formula ID,
    /// mapping the final node identifier to `store.empty_or()`.
    #[test]
    fn test_bind_with_evaluator_pruning_propagation_false() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let target_predicate = PredicateSymbolId::new(10);
        let target_skeleton = AtomSkeletonId::new(1);

        // 1. Create a base atomic formula with a variable argument
        let var_x = builder.variable(VariableId::new(1));
        let expr_atomic = builder.atomic_formula(target_predicate, &[var_x], target_skeleton);

        // 2. Prepare bindings for the variable substitution phase
        let mut bindings = Bindings::new();
        bindings.insert(VariableId::new(1), ObjectId::new(100));

        // 3. Set up the static evaluator designed to yield Boolean(false)
        let evaluator = MockEvaluatorPruneFalse { target_skeleton };
        let mut scratchpad = BindingScratchpad::new();

        // 4. Execute the pipeline using an isolated scratchpad
        let new_root = bind_with(
            expr_atomic,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 5. Assert that the resulting root has successfully collapsed into the global False constant
        let expected_false = store.empty_or();
        assert_eq!(
            new_root, expected_false,
            "The atomic formula should have been statically pruned into the canonical empty Or (False)."
        );
    }

    /// A mock evaluator that intercepts specific atomic formula skeletons and forces
    /// them to evaluate to a structural logical `false` constant.
    struct MockEvaluatorPruneFalse {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorPruneFalse {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            let expr_id = expr.root_id();
            let entry_kind = expr.store()[expr_id].kind();

            match entry_kind {
                ExprKind::AtomicFormula(skel_id) => {
                    if *skel_id == self.target_skeleton {
                        Ok(Some(ExprConstant::Boolean(false)))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        }
    }

    /// ### Test: Bind Duplicate Children Diamond Sharing
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly handles localized diamond sharing where a single
    /// node (like an `AtomicFormula`) references the exact same `Variable` node multiple times
    /// in its arguments buffer. This ensures that the post-order substitution map resolves
    /// both argument slots to the same substituted object ID without duplicate processing.
    ///
    /// **Input Structure:**
    /// ```text
    ///       [Root: AtomicFormula]
    ///          /            \
    ///  [Arg0: Variable(?x)] [Arg1: Variable(?x)]
    /// ```
    /// *Substitution:* `?x -> Object(100)`
    ///
    /// **Expected Output:**
    /// ```text
    ///       [Root: AtomicFormula]
    ///          /            \
    ///  [Arg0: Object(100)]  [Arg1: Object(100)]
    /// ```
    /// After evaluation, both argument positions must point to the exact same interned
    /// `Object(100)` identifier, preserving the internal structural equality and proving
    /// that the diamond sharing resolution functions correctly under local duplicate arguments.
    #[test]
    fn test_bind_duplicate_children_diamond_sharing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);

        // 1. Create a unique Variable node (?x)
        let v_node = builder.variable(var_x);

        // 2. Assemble an AtomicFormula that replicates the same variable node across multiple arguments
        let target_skeleton = AtomSkeletonId::new(5);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(2),
            &[v_node, v_node],
            target_skeleton,
        );

        // 3. Set up the target substitution mapping ?x to Object(100)
        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        // 4. Run the binding pipeline
        let result = bind(root, &mut store, &bindings);

        assert!(result.is_ok(), "The binding process failed to execute.");
        let new_root = result.unwrap();

        // 5. Extract children and validate the shared structural identities
        let children = store[new_root].children();

        // Note: index 0 is typically reserved for the predicate/skeleton metadata, arguments start after.
        assert_eq!(
            children[1], children[2],
            "Both substituted arguments must point to the identical interned ExprId slot."
        );

        if let ExprKind::Object(id) = store[children[1]].kind() {
            assert_eq!(
                *id, obj_100,
                "The shared variable leaf was not substituted into the correct Object value."
            );
        } else {
            panic!("The diamond-shared argument failed to transform into an Object primitive.");
        }
    }

    /// ### Test: Bind With Static Evaluation To Number
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly intercepts evaluable fluent/functional expressions
    /// (like an `AtomicFormula` representing a numeric function) and replaces the entire sub-tree
    /// with a primitive numeric leaf node when the static evaluator yields a concrete float value.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: AtomicFormula (Function)]
    ///           |
    ///  [Child: Variable(?x)]
    /// ```
    /// *Bindings:* Empty (No variable substitution)
    /// *Evaluator:* Evaluates the target atomic formula skeleton to `42.0`
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Number(42.0)]
    /// ```
    /// The evaluator must recognize the functional atomic skeleton, compute its static numerical
    /// value, and force the engine to substitute the old compound root identifier with a clean,
    /// interned `Number` primitive leaf node.
    #[test]
    fn test_bind_with_static_evaluation_to_number() {
        let mut store = ExprStore::new();

        // 1. Initialize the tree structure with an atomic functional formula
        let (expr_function, function_skeleton) = {
            let mut builder = ExprBuilder::new(&mut store);
            let var_x = builder.variable(VariableId::new(1));
            let function_skeleton = AtomSkeletonId::new(42);
            let expr_function =
                builder.atomic_formula(PredicateSymbolId::new(9), &[var_x], function_skeleton);
            (expr_function, function_skeleton)
        };

        // 2. Setup an empty bindings map and the specialized numeric evaluator
        let bindings = Bindings::new();
        let evaluator = MockEvaluatorNumber {
            target_skeleton: function_skeleton,
        };

        // 3. Execute the binding and evaluation pipeline using a clean scratchpad
        let mut scratchpad = BindingScratchpad::new();
        let new_root = bind_with(
            expr_function,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 4. Assert that the returned ID matches a freshly interned Number(42.0) node
        let expected_number_node = ExprBuilder::new(&mut store).number(42.0);
        assert_eq!(
            new_root, expected_number_node,
            "The functional expression should have been statically pruned into a structural Number primitive."
        );
    }

    /// A mock evaluator that matches a specific atom skeleton ID and evaluates it
    /// into a static floating-point constant wrap.
    struct MockEvaluatorNumber {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorNumber {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            let expr_id = expr.root_id();
            if let ExprKind::AtomicFormula(skel_id) = expr.store()[expr_id].kind() {
                if *skel_id == self.target_skeleton {
                    return Ok(Some(ExprConstant::Number(42.0.into())));
                }
            }
            Ok(None)
        }
    }

    /// ### Test: Bind Non-Evaluable Nodes Bypass Evaluator
    ///
    /// **Objective:**
    /// Verify that nodes that are explicitly marked as non-evaluable (such as raw variables or
    /// leaf objects) completely bypass the static evaluator pipeline. This ensures that the
    /// engine does not waste cycles or incorrectly prune structural elements that require
    /// runtime evaluation.
    ///
    /// **Input Structure:**
    /// ```text
    ///  [Root: Variable(?x)]
    /// ```
    /// *Substitution:* `?x -> Object(88)`
    /// *Evaluator:* Aggressive (Attempts to prune everything to `True`)
    ///
    /// **Expected Output:**
    /// ```text
    ///  [Root: Object(88)]
    /// ```
    /// Even though the evaluator attempts to aggressively intercept and collapse every node
    /// into a boolean constant `True`, the `is_evaluable` guard must catch the `Variable` kind
    /// first, causing it to bypass pruning and correctly resolve to its substituted `Object` value.
    #[test]
    fn test_bind_non_evaluable_nodes_bypass_evaluator() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(7);
        let expr_var = builder.variable(var_x);

        // 1. Prepare variable bindings mapping ?x to Object(88)
        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(88));

        // 2. Instantiate an aggressive evaluator that forces everything to True
        let evaluator = MockEvaluatorAggressive;

        // 3. Execute the evaluation using an isolated scratchpad
        let mut scratchpad = BindingScratchpad::new();
        let new_root = bind_with(
            expr_var,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 4. Validate that the Variable node bypassed the evaluator and substituted correctly
        if let ExprKind::Object(id) = store[new_root].kind() {
            assert_eq!(
                *id,
                ObjectId::new(88),
                "The variable should have bypassed the evaluator and taken its bound object value."
            );
        } else {
            panic!("Variable should have bypassed the evaluator and transformed into an Object.");
        }
    }

    /// A mock evaluator designed to aggressively intercept nodes and force them to evaluate
    /// to a logical `True` constant, used to verify bypass filtering guards.
    struct MockEvaluatorAggressive;

    impl ExprEvaluator for MockEvaluatorAggressive {
        fn evaluate(
            &self,
            _expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            // Force a static pruning to True for any node that leaks into this method
            Ok(Some(ExprConstant::Boolean(true)))
        }
    }

    /// ### Test: Deep DAG Diamond Sharing Substitution
    ///
    /// **Objective:**
    /// Verify that the binding engine correctly traverses and substitutes variables within a
    /// deep Directed Acyclic Graph (DAG) structure featuring diamond sharing. This ensures
    /// that the post-order traversal cache (`substitution_map`) does not prematurely pollute
    /// unvisited branches with un-substituted placeholders during the downward phase.
    ///
    /// **Input Structure:**
    /// ```text
    ///          [Root: And]
    ///          /         \
    ///  [Left: Not]     [Right: Not]
    ///          \         /
    ///      [Leaf: Variable(?x)]
    /// ```
    /// *Substitution:* `?x -> Object(100)`
    ///
    /// **Expected Output (with Unary Collapse & Deduplication):**
    /// ```text
    ///  [Root: Not]
    ///       |
    ///  [Leaf: Object(100)]
    /// ```
    /// After substitution, both branches resolve to the exact same `Not(Object(100))` ID.
    /// The `ExprBuilder::reduce` engine deduplicates identical children (Idempotence).
    /// Since only one unique child remains (`len == 1`), it triggers a unary bypass,
    /// collapsing and removing the `And` wrapper entirely to return the `Not` node directly.
    #[test]
    fn test_bind_deep_dag_diamond_sharing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);

        // 1. Create the base leaf node: Variable(?x)
        let v_node = builder.variable(var_x);

        // 2. Create intermediate sub-trees (e.g., Not(?x))
        // Thanks to the store's Hash Consing, both identical operations will resolve
        // to the exact same ExprId, forcing a deep vertical diamond-sharing pattern.
        let sub_tree_left = builder.not(v_node);
        let sub_tree_right = builder.not(v_node);

        assert_eq!(
            sub_tree_left, sub_tree_right,
            "Hash Consing must merge identical structural sub-trees."
        );

        // 3. Assemble the root node linking both identical branches: And(Not(?x), Not(?x))
        let root = builder.and(&[sub_tree_left, sub_tree_right]);

        // 4. Prepare the variable bindings map
        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        // 5. Execute the evaluation using an isolated scratchpad
        let mut scratchpad = BindingScratchpad::new();
        let result = bind_with(root, &mut store, &bindings, None, &mut scratchpad);

        assert!(
            result.is_ok(),
            "The binding process should execute without errors."
        );
        let new_root = result.unwrap();

        // 6. Validate that the root has collapsed into the unique Not child
        let kind = store[new_root].kind().clone();
        let children = store[new_root].children();

        assert_eq!(
            kind,
            ExprKind::Not,
            "The And wrapper must be discarded, leaving the Not operator as the new root."
        );

        assert_eq!(
            children.len(),
            1,
            "The collapsed Not node must have exactly 1 child."
        );

        // Inspect the final substituted leaf inside the Not node
        let final_leaf = children[0];
        if let ExprKind::Object(id) = store[final_leaf].kind() {
            assert_eq!(
                *id, obj_100,
                "The deep variable inside the collapsed structure was not substituted correctly."
            );
        } else {
            panic!("The deep leaf node should have been successfully transformed into an Object.");
        }
    }
}
