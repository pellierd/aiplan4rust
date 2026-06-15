use super::scratchpad::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::binding::error::BindingError;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::{ExprConstant, ExprEvaluator};
use crate::aiplan4rust::compiler::grounding::binding::Bindings;
use crate::aiplan4rust::compiler::lir::expr::expr::Expr;
use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::VariableId;

/// **Version Standard** : Applique uniquement les substitutions de variables (Grounding classique).
/// Alloue temporairement un scratchpad local.
#[inline]
pub fn bind(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
) -> Result<(ExprId, Option<VariableId>), BindingError> {
    let mut scratchpad = BindingScratchpad::new();
    bind_with(expr_id, store, sub, None, &mut scratchpad)
}

/// **Version Avancée** : Moteur de parcours unique, ultra-optimisé et sans allocation.
/// Applique les substitutions et exécute l'élagage statique (Köhler) à la volée.
/// Retourne l'ExprId résultant et potentiellement la VariableId responsable de l'effondrement.
/// **Version Avancée** : Moteur de parcours unique, ultra-optimisé et sans allocation.
/// Applique les substitutions et exécute l'élagage statique (Köhler) à la volée.
/// Retourne l'ExprId résultant et la VariableId responsable de l'effondrement (si unique et valide).
/// **Version Avancée** : Moteur de parcours unique, ultra-optimisé et sans allocation.
/// Applique les substitutions et exécute l'élagage statique (Köhler) à la volée.
/// Retourne l'ExprId résultant et la VariableId responsable de l'effondrement (si unique et valide).
pub fn bind_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
    evaluator: Option<&dyn ExprEvaluator>,
    scratchpad: &mut BindingScratchpad,
) -> Result<(ExprId, Option<VariableId>), BindingError> {
    scratchpad.clear();

    // On initialise notre builder unique pour l'interning
    let mut builder = ExprBuilder::new(store);

    // Amorçage du parcours itératif avec la racine
    scratchpad.stack.push((expr_id, false));

    while let Some((old_id, children_pushed)) = scratchpad.stack.pop() {
        if !children_pushed {
            // --- PREMIER PASSAGE : Descente ---

            // Sécurité DAG
            if scratchpad.substitution_map.contains_key(&old_id) {
                continue;
            }

            scratchpad.stack.push((old_id, true));

            let entry = &builder.store[old_id];
            for &child_id in entry.children().iter().rev() {
                if !scratchpad.substitution_map.contains_key(&child_id) {
                    scratchpad.stack.push((child_id, false));
                }
            }
        } else {
            // --- SECOND PASSAGE : Remontée ---

            // Sécurité DAG
            if scratchpad.substitution_map.contains_key(&old_id) {
                continue;
            }

            // Isolation de l'emprunt immuable
            let (entry_kind, has_children) = {
                let entry = &builder.store[old_id];
                (entry.kind().clone(), !entry.children().is_empty())
            };

            let mut has_changed = false;
            let mut inherited_culprit = None;
            let mut multiple_or_culprits = false; // Flag de sécurité pour la disjonction
            scratchpad.children_buffer.clear();

            // Si le nœud a des enfants, on collecte leurs correspondances mises à jour
            if has_children {
                let entry = &builder.store[old_id];
                for &child_id in entry.children() {
                    if let Some(&(new_child_id, child_culprit)) =
                        scratchpad.substitution_map.get(&child_id)
                    {
                        if new_child_id != child_id {
                            has_changed = true;

                            if let ExprKind::Variable(var_id) = builder.store[child_id].kind() {
                                if inherited_culprit.is_none() {
                                    inherited_culprit = Some(*var_id);
                                }
                            }
                        }

                        // Analyse et propagation fine du coupable selon l'opérateur courant
                        if let Some(var_id) = child_culprit {
                            match entry_kind {
                                ExprKind::And => {
                                    // CORRECTION : On ne capture le coupable d'effondrement que si
                                    // aucun coupable n'a encore été enregistré pour ce AND.
                                    if builder.store[new_child_id].kind() == &ExprKind::Or
                                        && builder.store[new_child_id].children().is_empty()
                                    {
                                        if inherited_culprit.is_none() {
                                            inherited_culprit = Some(var_id);
                                        }
                                    } else if inherited_culprit.is_none() {
                                        inherited_culprit = Some(var_id);
                                    }
                                }
                                ExprKind::Or => {
                                    if inherited_culprit.is_some()
                                        && inherited_culprit != Some(var_id)
                                    {
                                        multiple_or_culprits = true;
                                    }
                                    inherited_culprit = Some(var_id);
                                }
                                _ => {
                                    inherited_culprit = Some(var_id);
                                }
                            }
                        }
                        scratchpad.children_buffer.push(new_child_id);
                    } else {
                        scratchpad.children_buffer.push(child_id);
                    }
                }
            }

            // Évaluation et reconstruction du nœud courant
            let local_variable = match entry_kind {
                ExprKind::Variable(var_id) => Some(var_id),
                _ => None,
            };

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
                        builder.reconstruct(entry_kind.clone(), &scratchpad.children_buffer)?
                    } else {
                        old_id
                    }
                }
            };

            let mut final_culprit = None;

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

                        // Effondrement direct via l'évaluateur externe (ex: Prédicat Statique Faux)
                        final_culprit = local_variable.or(inherited_culprit);
                    }
                }
            }

            // Si le nœud s'est effondré structurellement par remontée d'enfants dominants
            if final_culprit.is_none() {
                let current_kind = builder.store[current_id].kind();
                let is_collapsed = matches!(current_kind, ExprKind::And | ExprKind::Or)
                    && builder.store[current_id].children().is_empty();

                if is_collapsed {
                    if current_kind == &ExprKind::Or && multiple_or_culprits {
                        // Sécurité critique : Un OR mort né de variables hétérogènes n'a pas de coupable unique.
                        final_culprit = None;
                    } else {
                        final_culprit = inherited_culprit;
                    }
                }
            }

            // On stocke le tuple définitif pour ce sous-arbre
            scratchpad
                .substitution_map
                .insert(old_id, (current_id, final_culprit));
        }
    }

    // Extraction du résultat final pour la racine
    let final_res = scratchpad
        .substitution_map
        .get(&expr_id)
        .copied()
        .unwrap_or((expr_id, None));

    Ok(final_res)
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

        // CORRECTION 1 : Déstructuration du tuple (ExprId, Option<VariableId>) renvoyé par bind
        let (final_id, culprit) = result.unwrap();

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

        // CORRECTION 2 : Validation qu'aucun coupable de saut n'est extrait pour une substitution nominale
        assert!(
            culprit.is_none(),
            "No variable culprit should be returned when a variable is successfully substituted without a structural collapse."
        );
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
        // CORRECTION : Déstructuration du tuple (ExprId, Option<VariableId>)
        let (new_root, culprit) = bind(expr_num, &mut store, &bindings).unwrap();

        // 4. Assert that the returned node identifier is strictly identical to the input identifier
        assert_eq!(
            new_root, expr_num,
            "The binding engine must return the exact same input ID when no structural changes occur."
        );

        // AJOUT DE SÉCURITÉ : On s'assure qu'aucun coupable n'est extrait puisqu'il n'y a aucun effondrement
        assert!(
            culprit.is_none(),
            "No variable culprit should be returned for literal nodes without variables."
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

        // CORRECTION : Déstructuration du tuple pour séparer l'ExprId du potentiel coupable
        let (final_id, culprit) = result.unwrap();

        // 5. Validate that the root node was successfully replaced by the canonical empty And (True)
        let expected_empty_and = store.empty_and();
        assert_eq!(
            final_id, expected_empty_and,
            "The atomic formula should have been statically pruned into the canonical empty And (True)."
        );

        // AJOUT DE SÉCURITÉ : On s'assure qu'aucun skip erroné n'est levé pour une réduction à True
        assert!(
            culprit.is_none(),
            "An evaluation to a valid static True constant must not yield a variable culprit for odometer skipping."
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

        // CORRECTION : Déstructuration du tuple (ExprId, Option<VariableId>) renvoyé par bind
        let (new_root, culprit) = result.unwrap();

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

        // AJOUT DE SÉCURITÉ : Validation qu'aucun coupable n'est extrait (pas d'effondrement)
        assert!(
            culprit.is_none(),
            "No variable culprit should be returned when the formula is successfully grounded without collapsing."
        );
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
        let (new_root, culprit) = bind(root, &mut store, &bindings).unwrap();

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
    /// short-circuits safely, returning the exact same root identifier and confirming that no
    /// variable culprit is flagged when no collapse occurs.
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
        // CORRECTION : Déstructuration du tuple (ExprId, Option<VariableId>)
        let (new_root, culprit) = bind(root, &mut store, &bindings).unwrap();

        // 4. Assert that the resulting node identity is strictly equal to the input root identity
        assert_eq!(
            new_root, root,
            "The binding engine must perform zero modifications and return the identical input ID when bindings are empty."
        );

        // AJOUT DE SÉCURITÉ : On valide qu'aucun coupable n'est retourné (pas d'effondrement)
        assert!(
            culprit.is_none(),
            "No variable culprit should be returned when the tree remains intact."
        );
    }

    /// ### Test: Bind With Evaluator Pruning Propagation False
    ///
    /// **Objective:**
    /// Verify that when the static evaluator determines an evaluable atomic expression is
    /// unconditionally false, the binding engine drops the constructed sub-tree, correctly
    /// replaces its root identifier with the store's canonical `False` constant (empty `Or`),
    /// and accurately identifies the variable responsible for the collapse.
    #[test]
    fn test_bind_with_evaluator_pruning_propagation_false() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let target_predicate = PredicateSymbolId::new(10);
        let target_skeleton = AtomSkeletonId::new(1);
        let target_var = VariableId::new(1);

        // 1. Create a base atomic formula with a variable argument
        let var_x = builder.variable(target_var);
        let expr_atomic = builder.atomic_formula(target_predicate, &[var_x], target_skeleton);

        // 2. Prepare bindings for the variable substitution phase
        let mut bindings = Bindings::new();
        bindings.insert(target_var, ObjectId::new(100));

        // 3. Set up the static evaluator designed to yield Boolean(false)
        let evaluator = MockEvaluatorPruneFalse { target_skeleton };
        let mut scratchpad = BindingScratchpad::new();

        // 4. Execute the pipeline using an isolated scratchpad
        let (new_root, culprit) = bind_with(
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

        // CORRECTION 2 : Validation cruciale du mécanisme de "Culprit Tracking"
        assert_eq!(
            culprit,
            Some(target_var),
            "The engine must flag Variable(1) as the culprit responsible for the false collapse to allow smart odometer skipping."
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

        // CORRECTION 1 : Déstructuration du tuple pour isoler l'ExprId pur
        let (new_root, culprit) = result.unwrap();

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

        // CORRECTION 2 : Validation qu'aucun coupable n'est extrait (pas d'effondrement ou d'élagage destructif)
        assert!(
            culprit.is_none(),
            "No variable culprit should be returned when performing nominal substitution in diamond sharing."
        );
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
    /// interned `Number` primitive leaf node without reporting a variable culprit.
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
        let (new_root, culprit) = bind_with(
            expr_function,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 4. Assert that the returned ID matches a freshly interned Number(42.0) node
        let expected_number_node = ExprBuilder::new(&mut store).number(42.0);

        // 5. Validate node identity and ensure no variable culprit is flagged for successful evaluations
        assert_eq!(
            new_root, expected_number_node,
            "The functional expression should have been statically pruned into a structural Number primitive."
        );

        assert!(
            culprit.is_none(),
            "An evaluation to a valid static Number should not report a variable culprit for skips."
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
        let (new_root, culprit) = bind_with(
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

        assert!(
            culprit.is_none(),
            "Bypassing the evaluator for a nominal variable substitution must not report a culprit."
        );
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

        let (new_root, culprit) = result.unwrap();

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

        assert!(
            culprit.is_none(),
            "An algebraic unary reduction must not produce a variable culprit for odometer skips."
        );
    }

    /// ### Test: Bind Or Collapse With Heterogeneous Culprits Extinguishes Skip
    ///
    /// **Objective:**
    /// Verify that when a disjunction (an `Or` node) completely collapses into `False` because
    /// multiple sub-branches failed due to *different* variables, the engine safely extinguishes
    /// the culprit (`None`). This prevents the odometer from blindly skipping variables when
    /// a multi-variable conflict requires standard synchronous iteration.
    #[test]
    fn test_bind_or_collapse_with_heterogeneous_culprits_extinguishes_skip() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let var_y = VariableId::new(2);

        // CORRECTION : On extrait les variables dans des locales pour calmer le Borrow Checker
        let v_node_x = builder.variable(var_x);
        let atom_x = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[v_node_x],
            AtomSkeletonId::new(1),
        );

        let v_node_y = builder.variable(var_y);
        let atom_y = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[v_node_y],
            AtomSkeletonId::new(2),
        );

        // 2. Wrap them under a root Or node: Or(Atom(?x), Atom(?y))
        let root = builder.or(&[atom_x, atom_y]);

        // 3. Setup bindings and an evaluator that forces BOTH specific skeletons to False
        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(100));
        bindings.insert(var_y, ObjectId::new(200));

        let evaluator = MockEvaluatorPruneHeterogeneousFalse;
        let mut scratchpad = BindingScratchpad::new();

        // 4. Run the binding pipeline
        let (new_root, culprit) = bind_with(
            root,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 5. Assert that the entire root collapsed into False
        assert_eq!(
            new_root,
            store.empty_or(),
            "The Or root should have collapsed into the canonical False constant."
        );

        // 6. CRITICAL: The culprit MUST be None due to ambiguity (Heterogeneous variables)
        assert!(
            culprit.is_none(),
            "The culprit must be cleared (None) when multiple distinct variables cause a disjunction failure."
        );
    }

    struct MockEvaluatorPruneHeterogeneousFalse;

    impl ExprEvaluator for MockEvaluatorPruneHeterogeneousFalse {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            let expr_id = expr.root_id();
            if let ExprKind::AtomicFormula(_) = expr.store()[expr_id].kind() {
                // Prune toutes les formules atomiques de ce contexte à False
                Ok(Some(ExprConstant::Boolean(false)))
            } else {
                Ok(None)
            }
        }
    }

    /// ### Test: Bind And Collapse Favor First Culprit
    ///
    /// **Objective:**
    /// Verify that when a conjunction (an `And` node) collapses into `False` because multiple
    /// sub-branches fail, the engine deterministically retains the *first* variable culprit
    /// that triggered the failure (or follows the deterministic evaluation order). This ensures
    /// the odometer skips at the correct, most restrictive pivot.
    #[test]
    fn test_bind_and_collapse_favor_first_culprit() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let var_y = VariableId::new(2);

        let v_node_x = builder.variable(var_x);
        let atom_x = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[v_node_x],
            AtomSkeletonId::new(1),
        );

        let v_node_y = builder.variable(var_y);
        let atom_y = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[v_node_y],
            AtomSkeletonId::new(2),
        );

        // 1. Wrap them under a root And node: And(Atom(?x), Atom(?y))
        let root = builder.and(&[atom_x, atom_y]);

        // 2. Setup bindings and an evaluator that forces BOTH to False
        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(100));
        bindings.insert(var_y, ObjectId::new(200));

        let evaluator = MockEvaluatorAndDoubleFalse;
        let mut scratchpad = BindingScratchpad::new();

        // 3. Run the binding pipeline
        let (new_root, culprit) = bind_with(
            root,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 4. Assert that the entire root collapsed into False (empty_or)
        assert_eq!(
            new_root,
            store.empty_or(),
            "The And root should have collapsed into the canonical False constant."
        );

        // 5. CRITICAL: The engine must pick the first culprit (Variable(1)) and not overwrite it with Variable(2)
        assert_eq!(
            culprit,
            Some(var_x),
            "The engine should retain the first variable responsible for the conjunction's collapse."
        );
    }

    /// ### Test: Bind And Successful Pruning Extinguishes Culprit
    ///
    /// **Objective:**
    /// Verify that when a sub-branch containing a variable is statically pruned into `True`
    /// (an empty `And`), it does *not* leak its variable ID as a culprit to the parent `And` node
    /// if the parent successfully survives. A valid branch must never report a culprit.
    #[test]
    fn test_bind_and_successful_pruning_extinguishes_culprit() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let v_node_x = builder.variable(var_x);

        // 1. Create an atomic formula that will evaluate to True
        let atom_x = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[v_node_x],
            AtomSkeletonId::new(1),
        );

        // 2. Create a normal ground literal that stays True/Valid
        let ground_num = builder.number(5.0);

        // Root: And(Atom(?x), 5.0)
        let root = builder.and(&[atom_x, ground_num]);

        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(100));

        let evaluator = MockEvaluatorAndTrue;
        let mut scratchpad = BindingScratchpad::new();

        // 3. Run the binding pipeline
        let (new_root, culprit) = bind_with(
            root,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        // 4. Ensure the root is successfully reconstructed (it didn't collapse to False)
        assert_ne!(
            new_root,
            store.empty_or(),
            "The And root must not collapse to False since its branches are true/valid."
        );

        // 5. CRITICAL: Culprit must be None because no failure or structural collapse to False occurred.
        assert!(
            culprit.is_none(),
            "A successful evaluation to True must never leak a variable culprit to the parent context."
        );
    }

    // --- Mocks Utilitaires pour les tests ---

    struct MockEvaluatorAndDoubleFalse;
    impl ExprEvaluator for MockEvaluatorAndDoubleFalse {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            let expr_id = expr.root_id();
            if let ExprKind::AtomicFormula(_) = expr.store()[expr_id].kind() {
                Ok(Some(ExprConstant::Boolean(false)))
            } else {
                Ok(None)
            }
        }
    }

    struct MockEvaluatorAndTrue;
    impl ExprEvaluator for MockEvaluatorAndTrue {
        fn evaluate(
            &self,
            expr: Expr,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            let expr_id = expr.root_id();
            if let ExprKind::AtomicFormula(_) = expr.store()[expr_id].kind() {
                Ok(Some(ExprConstant::Boolean(true)))
            } else {
                Ok(None)
            }
        }
    }
}
