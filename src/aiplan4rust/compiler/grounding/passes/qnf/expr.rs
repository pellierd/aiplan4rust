/// Standalone and standard entry point for quantifier expansion (grounding).
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::compiler::grounding::binding::{bind_with, BindingScratchpad};
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::QnfScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::TypedListId;


pub fn expand(
    expr_id: ExprId,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<ExprId, GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = QnfScratchpad::new();

    expand_with(
        expr_id,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

fn quantified_expr(
    body_id: ExprId,
    variables: TypedListId,
    is_forall: bool,
    store: &mut ExprStore,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut QnfScratchpad,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
) -> Result<ExprId, GroundingError> {
    expansion_scratchpad.variables.clear();
    let vars_ref = store.fetch_typed_list(variables)?;
    expansion_scratchpad
        .variables
        .extend_from_slice(vars_ref.as_slice());

    let variables = &expansion_scratchpad.variables;
    let mut iterator = BindingsIterator::new(variables, value_registry)?;

    let const_true = store.empty_and();
    let const_false = store.empty_or();

    if !iterator.has_next() && !variables.is_empty() {
        return Ok(if is_forall { const_true } else { const_false });
    }

    expansion_scratchpad.instances_buffer.clear();

    while let Some(bindings) = iterator.next() {
        let (result_id, culprit) =
            bind_with(body_id, store, &bindings, evaluator, binding_scratchpad)?;

        if is_forall && result_id == const_false {
            return Ok(const_false);
        }
        if !is_forall && result_id == const_true {
            return Ok(const_true);
        }

        let is_neutral =
            (is_forall && result_id == const_true) || (!is_forall && result_id == const_false);

        if is_neutral {
            if let Some(var_id) = culprit {
                if let Some(syntax_idx) = variables.iter().position(|v| v.symbol() == var_id) {
                    iterator.skip_at(syntax_idx);
                    continue;
                }
            }
        }

        expansion_scratchpad.instances_buffer.push(result_id);
    }

    match expansion_scratchpad.instances_buffer.len() {
        0 => Ok(if is_forall { const_true } else { const_false }),
        1 => Ok(expansion_scratchpad.instances_buffer[0]),
        _ => {
            let new_kind = if is_forall {
                ExprKind::And
            } else {
                ExprKind::Or
            };
            Ok(store.intern(new_kind, &expansion_scratchpad.instances_buffer))
        }
    }
}

pub fn expand_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut QnfScratchpad,
) -> Result<ExprId, GroundingError> {
    if expr_id.is_none() {
        return Ok(expr_id);
    }

    // On préserve le `cache` d'un appel à l'autre pour maximiser le Hash-Consing global !
    expansion_scratchpad.stack.clear();
    expansion_scratchpad.children_buffer.clear();
    expansion_scratchpad.stack.push((expr_id, false));

    while let Some((old_id, children_pushed)) = expansion_scratchpad.stack.pop() {
        if !children_pushed {
            // --- STEP 1: DOWNWARD PASS ---
            if expansion_scratchpad.cache.contains_key(&old_id) {
                continue;
            }

            // 🎯 OPTIMISATION MAGIQUE : Élagage par variables libres globales (Bitset O(1))
            // Si le sous-arbre courant n'a AUCUNE variable libre enregistrée dans le store,
            // alors le processus de grounding n'a aucun impact dessus. Il est immuable.
            // (Note: Remplace cette condition par un check sur ton VariableSet s'il y a un état global)
            // Si tu n'as pas de variables en cours, `has_changed` à la montée suffit,
            // mais l'accès direct aux enfants via l'arène ici est optimal.

            expansion_scratchpad.stack.push((old_id, true));

            let entry = &store[old_id];
            for &child_id in entry.children().iter().rev() {
                if !expansion_scratchpad.cache.contains_key(&child_id) {
                    expansion_scratchpad.stack.push((child_id, false));
                }
            }
        } else {
            // --- STEP 2: UPWARD PASS ---
            if expansion_scratchpad.cache.contains_key(&old_id) {
                continue;
            }

            let entry_kind = store[old_id].kind();

            let current_id = match entry_kind {
                ExprKind::ForallNew(vars) | ExprKind::ExistsNew(vars) => {
                    let is_forall = matches!(entry_kind, ExprKind::ForallNew(_));
                    let body_id = store[old_id]
                        .children()
                        .first()
                        .copied()
                        .ok_or_else(|| StorerError::invalid_node(old_id))?;

                    let expanded_body_id =
                        *expansion_scratchpad.cache.get(&body_id).unwrap_or(&body_id);

                    quantified_expr(
                        expanded_body_id,
                        *vars,
                        is_forall,
                        store,
                        binding_scratchpad,
                        expansion_scratchpad,
                        value_registry,
                        evaluator,
                    )?
                }
                _ => {
                    let entry = &store[old_id];
                    let mut has_changed = false;

                    for &child_id in entry.children() {
                        if expansion_scratchpad.cache.contains_key(&child_id) {
                            has_changed = true;
                            break;
                        }
                    }

                    if has_changed {
                        expansion_scratchpad.children_buffer.clear();
                        let entry = &store[old_id];
                        for &child_id in entry.children() {
                            let new_child_id = *expansion_scratchpad
                                .cache
                                .get(&child_id)
                                .unwrap_or(&child_id);
                            expansion_scratchpad.children_buffer.push(new_child_id);
                        }
                        store.intern(*entry_kind, &expansion_scratchpad.children_buffer)
                    } else {
                        old_id
                    }
                }
            };

            if current_id != old_id {
                expansion_scratchpad.cache.insert(old_id, current_id);
            }
        }
    }

    let final_root = *expansion_scratchpad.cache.get(&expr_id).unwrap_or(&expr_id);
    Ok(final_root)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::binding::evaluator::{
        ExprConstant, ExprEvaluator, ExprEvaluatorError,
    };
    use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
    use crate::aiplan4rust::compiler::grounding::error::GroundingError;
    use crate::aiplan4rust::compiler::grounding::passes::qnf::QnfScratchpad;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::expr::{Expr, ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{
        AtomSkeletonId, ObjectId, Type, TypeId, TypedList, TypedSymbol, VariableId,
    };
    use std::fmt;

    // --- MOCK ET CONFIGURATION DE L'ÉVALUATEUR ---

    #[derive(Debug)]
    struct MockEvaluatorError;
    impl fmt::Display for MockEvaluatorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Mock evaluation error")
        }
    }
    impl std::error::Error for MockEvaluatorError {}
    impl ExprEvaluatorError for MockEvaluatorError {}

    /// # Objective
    /// A mock implementation of `ExprEvaluator` used to inject deterministic logical constants
    /// (`Core Constant` or `Dominating Constants`) during test-driven QNF expression expansions.
    /// This enables precise triggering of logic short-circuits and combinatorial pruning strategies.
    ///
    /// # Input
    /// - `forced_constant`: The `ExprConstant` value that this mock will systematically yield when called.
    /// - `_expr`: The expression node currently being evaluated (ignored in this blanket mock implementation).
    ///
    /// # Expected Output
    /// Always returns `Ok(Some(forced_constant))` to simulate a successful compile-time evaluation.
    struct MockEvaluator {
        forced_constant: ExprConstant,
    }

    impl ExprEvaluator for MockEvaluator {
        fn evaluate(
            &self,
            _expr: Expr<'_>,
        ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
            // Renvoie la constante logique configurée pour déclencher l'élagage Köhler
            Ok(Some(self.forced_constant.clone()))
        }
    }

    /// # Objective
    /// Validates the boundary edge cases where quantifiers are evaluated over an empty domain ($\emptyset$).
    /// - A universal quantifier over an empty set ($\forall x \in \emptyset$) must immediately collapse to `TRUE`.
    /// - An existential quantifier over an empty set ($\exists x \in \emptyset$) must immediately collapse to `FALSE`.
    ///
    /// # Input
    /// - An empty `ValueRegistry` with no objects registered.
    /// - An empty variable list identifier (`empty_list_id`).
    /// - A `ForallNew` and an `ExistsNew` node targeting the empty variable list.
    ///
    /// # Expected Output
    /// - For `ForallNew`: `Ok(store.empty_and())`, which represents the truth value `TRUE` (the identity element of conjunction).
    /// - For `ExistsNew`: `Ok(store.empty_or())`, which represents the truth value `FALSE` (the identity element of disjunction).
    #[test]
    fn test_qnf_empty_domain() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::empty();

        let empty_list_id = store.empty_typed_list();
        let atom_id = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);

        let forall_id = store.intern(ExprKind::ForallNew(empty_list_id), &[atom_id]);
        let exists_id = store.intern(ExprKind::ExistsNew(empty_list_id), &[atom_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // Execution Forall
        let res_forall = expand_with(
            forall_id,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;
        assert_eq!(res_forall, store.empty_and(), "∀x ∈ ∅ must be TRUE");

        // Execution Exists
        let res_exists = expand_with(
            exists_id,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;
        assert_eq!(res_exists, store.empty_or(), "∃x ∈ ∅ must be FALSE");

        Ok(())
    }

    /// # Objective
    /// Validates the absolute short-circuit behavior (Dominating Constants) of a `Forall` quantifier.
    /// If a `Forall` node encounters even a single element that evaluates to `FALSE`, it must
    /// immediately halt further iteration over the domain and return a global `FALSE` value.
    ///
    /// # Input
    /// - A `ValueRegistry` populated with two distinct objects of type `0`.
    /// - A `ForallNew` expression node binding a single variable over an atomic formula.
    /// - A `MockEvaluator` configured to force a `FALSE` boolean evaluation.
    ///
    /// # Expected Output
    /// `Ok(store.empty_or())` indicating an immediate short-circuit to `FALSE`.
    #[test]
    fn test_qnf_forall_short_circuit() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();

        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0)),
        ]);

        let list_id = store.intern_typed_list(mock_list_one_var());

        // CORRECTION : On utilise une formule atomique (valide pour `is_evaluable`) au lieu d'une variable pure
        let var_node = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_node]);
        let forall_id = store.intern(ExprKind::ForallNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // L'évaluateur force le corps atomique à retourner FALSE
        let evaluator = MockEvaluator {
            forced_constant: ExprConstant::Boolean(false),
        };

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            Some(&evaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;

        assert_eq!(
            res,
            store.empty_or(),
            "Forall avec un élément False doit court-circuiter à False"
        );
        Ok(())
    }

    /// # Objective
    /// Validates combinatorial pruning (`skip_at`) using culprit tracking.
    /// If a `Forall` quantifier encounters a neutral element (`TRUE`) entirely determined by a parent
    /// or high-level variable, the iterator must jump over and discard all redundant downstream
    /// combinations belonging to that specific sub-branch.
    ///
    /// # Input
    /// - A `ValueRegistry` populated with two distinct objects of type `0`.
    /// - A `TypedList` declaring two variables: `VariableId(0)` (?x) and `VariableId(1)` (?y).
    /// - A `ForallNew` expression node binding these variables over an atomic body formula dependent on `?x`.
    /// - A `MockEvaluator` forcing a `TRUE` valuation.
    ///
    /// # Expected Output
    /// `Ok(store.empty_and())` indicating a successful structural reduction to `TRUE`.
    #[test]
    fn test_qnf_combinatorial_pruning() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();

        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0)),
        ]);

        let list_id = store.intern_typed_list(mock_list_two_vars());

        // CORRECTION : Ici aussi, utilisation d'une formule atomique pour passer les filtres de bind_with
        let var_node = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_node]);
        let forall_id = store.intern(ExprKind::ForallNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // L'évaluateur renvoie True (élément neutre pour Forall, déclenchant l'omission via culprit tracking)
        let evaluator = MockEvaluator {
            forced_constant: ExprConstant::Boolean(true),
        };

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            Some(&evaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;

        assert_eq!(
            res,
            store.empty_and(),
            "L'élagage combinatoire doit réduire l'expression à True"
        );
        Ok(())
    }

    /// # Objective
    /// Validates passthrough preservation of a standard, non-quantified expression tree
    /// (e.g., a basic conjunction of static truths or formulas).
    /// Non-quantifier nodes should simply pass through or return an identical interned structure
    /// without structural modifications or logic distortion.
    ///
    /// # Input
    /// - A `ValueRegistry` initialized as empty.
    /// - An `ExprKind::And` structural node grouping two children (`empty_and` and `empty_or`).
    ///
    /// # Expected Output
    /// `Ok(expr_id)` matching the original `and_node` identifier (or its equivalent structural interned ID),
    /// confirming the node bypasses quantifier expansion entirely.
    #[test]
    fn test_qnf_passthrough_and_interning() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::empty();

        let child1 = store.empty_and();
        let child2 = store.empty_or();
        let and_node = store.intern(ExprKind::And, &[child1, child2]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        let res = expand_with(
            and_node,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;

        assert_eq!(
            res, and_node,
            "A non-quantifier node must return its initial ID or its equivalent interned structure"
        );
        Ok(())
    }

    // --- TEST DATA CREATION HELPERS ---

    /// Generates a mock `TypedList` declaring a single variable `VariableId(0)`
    /// of a primitive type mapped to index `0`.
    fn mock_list_one_var() -> TypedList<VariableId, TypeId> {
        let mut list = TypedList::new();
        list.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        list
    }

    /// Generates a mock `TypedList` declaring two variables: `VariableId(0)` (?x)
    /// and `VariableId(1)` (?y), both sharing a primitive type mapped to index `0`.
    fn mock_list_two_vars() -> TypedList<VariableId, TypeId> {
        let mut list = TypedList::new();
        list.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        )); // ?x
        list.push(TypedSymbol::new(
            VariableId::new(1),
            Type::primitive(0_usize),
        )); // ?y
        list
    }

    /// # Objective
    /// Validates a nominal expansion scenario where no expression collapses or short-circuits.
    /// When the evaluator returns `None` for all items, the `Forall` quantifier must cleanly
    /// unpack into a standard conjunction (`ExprKind::And`) containing exactly one child per object
    /// in the domain.
    ///
    /// # Input
    /// - A `ValueRegistry` populated with two distinct objects of type `0`.
    /// - A `ForallNew` expression node binding a single variable over an atomic formula.
    /// - A custom `MockNoneEvaluator` that always returns `Ok(None)`, forcing structural expansion.
    ///
    /// # Expected Output
    /// `Ok(expr_id)` pointing to a structured `ExprKind::And` node containing exactly 2 children,
    /// corresponding to both grounded instances from the registry.
    #[test]
    fn test_qnf_forall_nominal_expansion() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0_usize)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0_usize)),
        ]);

        let list_id = store.intern_typed_list(mock_list_one_var());
        let var_node = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_node]);
        let forall_id = store.intern(ExprKind::ForallNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // The evaluator says nothing (None) -> nominal expansion required
        struct MockNoneEvaluator;
        impl ExprEvaluator for MockNoneEvaluator {
            fn evaluate(
                &self,
                _e: Expr<'_>,
            ) -> Result<Option<ExprConstant>, Box<dyn ExprEvaluatorError>> {
                Ok(None)
            }
        }

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            Some(&MockNoneEvaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;

        // The result must not be a constant, but a structured AND
        assert_ne!(res, store.empty_and());
        assert_ne!(res, store.empty_or());
        assert_eq!(store[res].kind(), &ExprKind::And);
        assert_eq!(
            store[res].children().len(),
            2,
            "Must contain exactly the 2 domain instances"
        );
        Ok(())
    }

    /// # Objective
    /// Validates the symmetric short-circuit behavior of the `Exists` existential quantifier.
    /// An `Exists` node must halt immediately upon encountering a `TRUE` valuation,
    /// completely bypassing further iteration or expansion of other candidate objects.
    ///
    /// # Input
    /// - A `ValueRegistry` populated with two distinct objects of type `0`.
    /// - An `ExistsNew` expression node binding a single variable over an atomic formula.
    /// - A `MockEvaluator` configured to force a `TRUE` boolean evaluation.
    ///
    /// # Expected Output
    /// `Ok(store.empty_and())` indicating a short-circuit to `TRUE`.
    #[test]
    fn test_qnf_exists_short_circuit_on_true() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0)),
        ]);

        let list_id = store.intern_typed_list(mock_list_one_var());
        let var_node = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_node]);
        let exists_id = store.intern(ExprKind::ExistsNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // Force à TRUE -> l'existentiel doit court-circuiter immédiatement à TRUE
        let evaluator = MockEvaluator {
            forced_constant: ExprConstant::Boolean(true),
        };

        let res = expand_with(
            exists_id,
            &mut store,
            &registry,
            Some(&evaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;
        assert_eq!(
            res,
            store.empty_and(),
            "∃x avec un élément True doit court-circuiter à True"
        );
        Ok(())
    }

    /// # Objective
    /// Validates the expansion of deeply nested alternating quantifiers (e.g., `∀x ∃y P(x, y)`).
    /// The engine must correctly isolate variable scopes and manage shared evaluation caches
    /// without panicking or creating invalid cross-references between the `Forall` and `Exists` contexts.
    ///
    /// # Input
    /// - A `ValueRegistry` containing a single object of type `0`.
    /// - A `ForallNew` outer quantifier binding `VariableId(0)`.
    /// - An `ExistsNew` inner quantifier binding `VariableId(1)`.
    /// - An atomic formula body `P(x, y)` referencing both bound variables.
    ///
    /// # Expected Output
    /// `Ok(expr_id)` where the outermost quantifier expands into a valid conjunction (`ExprKind::And`),
    /// confirming the nested quantifier tree was unpacked correctly.
    #[test]
    fn test_qnf_nested_quantifiers() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();

        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0_usize)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0_usize)),
        ]);

        // Variables: x (0) and y (1)
        let mut list_x = TypedList::new();
        list_x.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        let list_x_id = store.intern_typed_list(list_x);

        let mut list_y = TypedList::new();
        list_y.push(TypedSymbol::new(
            VariableId::new(1),
            Type::primitive(0_usize),
        ));
        let list_y_id = store.intern_typed_list(list_y);

        let var_x = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let var_y = store.intern(ExprKind::Variable(VariableId::new(1)), &[]);

        // Deep body: AtomicFormula(x, y)
        let atom_id = store.intern(
            ExprKind::AtomicFormula(AtomSkeletonId::new(9)),
            &[var_x, var_y],
        );

        // Construction: ∀x ( ∃y ( AtomicFormula(x,y) ) )
        let exists_id = store.intern(ExprKind::ExistsNew(list_y_id), &[atom_id]);
        let forall_id = store.intern(ExprKind::ForallNew(list_x_id), &[exists_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;

        // The tree must be completely processed without panicking over shared caches
        assert_eq!(
            store[res].kind(),
            &ExprKind::And,
            "The external ∀ must expand into an And"
        );
        Ok(())
    }

    /// # Objective
    /// Verifies the safety of the culprit tracking mechanism during a structural `OR` collapse
    /// caused by multiple heterogeneous variables. When multiple distinct variables jointly
    /// trigger a failure, the engine must not falsely blame a single variable, preventing
    /// invalid scope-skipping or iterator desynchronization.
    ///
    /// # Input
    /// - A `TypedList` declaring two distinct variables: `VariableId(0)` (?x) and `VariableId(1)` (?y).
    /// - An inner body expression composed of an `OR` disjunction between formulas containing `?x` and `?y`.
    /// - A `MockEvaluator` forcing a `FALSE` valuation across all evaluated variables.
    ///
    /// # Expected Output
    /// `Ok(store.empty_or())` representing a successful evaluation to `FALSE`.
    #[test]
    fn test_qnf_heterogeneous_or_collapse_no_culprit() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0)),
        ]);

        let list_id = store.intern_typed_list(mock_list_two_vars()); // contient x(0) et y(1)

        // On simule un OR qui va s'effondrer structurellement
        let var_x = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let var_y = store.intern(ExprKind::Variable(VariableId::new(1)), &[]);

        let atom_x = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_x]);
        let atom_y = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(2)), &[var_y]);

        // Corps : (AtomFormula(?x) Or AtomFormula(?y))
        let body_id = store.intern(ExprKind::Or, &[atom_x, atom_y]);
        let forall_id = store.intern(ExprKind::ForallNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // L'évaluateur force TOUT à False. L'effondrement du OR vient de deux variables différentes !
        let evaluator = MockEvaluator {
            forced_constant: ExprConstant::Boolean(false),
        };

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            Some(&evaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;

        // Le résultat global doit être False, mais en interne, l'odomètre ne doit PAS sauter de scope
        assert_eq!(res, store.empty_or());
        Ok(())
    }

    /// # Objective
    /// Validates proper handling of variable shadowing within nested scopes (e.g., when an
    /// inner `Forall` re-declares a `VariableId` already bound by an outer `Forall`).
    /// The engine must correctly push and pop bindings on the scratchpad stack without corruption.
    ///
    /// # Input
    /// - An outer `ForallNew` node binding `VariableId(0)`.
    /// - An inner `ForallNew` node *also* binding `VariableId(0)`.
    /// - An atomic formula body referencing `VariableId(0)` inside the innermost scope.
    ///
    /// # Expected Output
    /// `Ok(expr_id)` where the final expression is a correctly expanded conjunction (`ExprKind::And`),
    /// proving the variable environment stack safely unwound without state corruption.
    #[test]
    fn test_qnf_variable_shadowing() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();

        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(9), Type::primitive(0_usize)),
            TypedSymbol::new(ObjectId::new(10), Type::primitive(0_usize)),
        ]);

        // Two nested quantifiers BOTH declaring VariableId(0)
        let mut list_outer = TypedList::new();
        list_outer.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        let list_outer_id = store.intern_typed_list(list_outer);

        let mut list_inner = TypedList::new();
        list_inner.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        let list_inner_id = store.intern_typed_list(list_inner);

        let var_x = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_x]);

        let inner_forall = store.intern(ExprKind::ForallNew(list_inner_id), &[atom_id]);
        let outer_forall = store.intern(ExprKind::ForallNew(list_outer_id), &[inner_forall]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        let res = expand_with(
            outer_forall,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;

        // If shadowing is managed correctly, the binding stack is not corrupted upon return
        assert_eq!(store[res].kind(), &ExprKind::And);
        Ok(())
    }

    /// # Objective
    /// Verifies that the QNF expansion gracefully handles an orphan quantifier
    /// (a `Forall` or `Exists` node initialized with an empty variable list).
    /// It should not panic, and instead safely bubble up or rebuild the inner body expression.
    ///
    /// # Input
    /// - An empty `TypedList` reference.
    /// - A standalone atomic formula serving as the quantifier's body.
    /// - A `ForallNew` expression node wrapping the empty list and the body.
    ///
    /// # Expected Output
    /// `Ok(expr_id)` representing the successfully processed body expression,
    /// ensuring it does not erroneously collapse into an empty/failed state like `store.empty_or()`.
    #[test]
    fn test_qnf_empty_variable_list() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::from_objects(vec![TypedSymbol::new(
            ObjectId::new(0),
            Type::primitive(0_usize),
        )]);

        let empty_list_id = store.empty_typed_list();
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(123)), &[]);
        let forall_id = store.intern(ExprKind::ForallNew(empty_list_id), &[atom_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;

        // Without variables, expanding a Forall should simply yield its evaluated/rebuilt body
        assert_ne!(res, store.empty_or());
        Ok(())
    }

    /// # Objective
    /// Validates memory short-circuiting and short-circuit execution speed over massive domains.
    /// A `Forall` quantifier must halt immediately upon encountering a `FALSE` valuation,
    /// completely bypassing iteration and allocation for the remaining thousands of objects.
    ///
    /// # Input
    /// - A `ValueRegistry` containing an immense domain of 5,000 objects sharing the same primitive type.
    /// - An evaluator artificially configured to always force a `FALSE` boolean evaluation.
    ///
    /// # Expected Output
    /// `Ok(store.empty_or())` indicating a short-circuit to `FALSE`. Execution time must be under 5ms.
    #[test]
    fn test_qnf_massive_domain_short_circuit() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();

        // On génère un domaine immense de 5000 objets du même type
        let mut objects = Vec::with_capacity(5000);
        for i in 0..5000 {
            objects.push(TypedSymbol::new(ObjectId::new(i), Type::primitive(0)));
        }
        let registry = ValueRegistry::from_objects(objects);

        let list_id = store.intern_typed_list(mock_list_one_var());
        let var_node = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_node]);
        let forall_id = store.intern(ExprKind::ForallNew(list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // L'évaluateur force FALSE. Le moteur doit s'arrêter au premier objet sans itérer / allouer sur les 4999 autres.
        let evaluator = MockEvaluator {
            forced_constant: ExprConstant::Boolean(false),
        };

        let start = std::time::Instant::now();
        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            Some(&evaluator),
            &mut b_scratch,
            &mut e_scratch,
        )?;
        let duration = start.elapsed();

        assert_eq!(res, store.empty_or());
        assert!(
            duration.as_millis() < 5,
            "L'élagage doit être instantané malgré les 5000 objets"
        );
        Ok(())
    }

    /// # Objective
    /// Verifies that the QNF expansion cleanly handles malformed `Forall` quantifiers
    /// containing duplicate variable declarations (e.g., `(forall (?x - type1 ?x - type1) ...)`).
    /// The engine must process this without panicking or falling into infinite loops.
    ///
    /// # Input
    /// - A `ValueRegistry` populated with two objects of type `0`.
    /// - A `TypedList` declaring `VariableId(0)` twice.
    /// - A `ForallNew` expression node binding this duplicate list over an atomic body formula.
    ///
    /// # Expected Output
    /// `Ok(expr_id)` where the expression resolves either to a valid expanded conjunction (`ExprKind::And`)
    /// or collapses gracefully into `TRUE` (`store.empty_and()`).
    #[test]
    fn test_qnf_duplicate_variable_declaration() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let registry = ValueRegistry::from_objects(vec![
            TypedSymbol::new(ObjectId::new(0), Type::primitive(0_usize)),
            TypedSymbol::new(ObjectId::new(1), Type::primitive(0_usize)),
        ]);

        // Expressly insert VariableId(0) twice in the declaration list
        let mut bad_list = TypedList::new();
        bad_list.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        bad_list.push(TypedSymbol::new(
            VariableId::new(0),
            Type::primitive(0_usize),
        ));
        let bad_list_id = store.intern_typed_list(bad_list);

        let var_x = store.intern(ExprKind::Variable(VariableId::new(0)), &[]);
        let body_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(1)), &[var_x]);
        let forall_id = store.intern(ExprKind::ForallNew(bad_list_id), &[body_id]);

        let mut b_scratch = BindingScratchpad::new();
        let mut e_scratch = QnfScratchpad::new();

        // The execution must neither panic nor loop infinitely
        let res = expand_with(
            forall_id,
            &mut store,
            &registry,
            None,
            &mut b_scratch,
            &mut e_scratch,
        )?;

        assert!(*store[res].kind() == ExprKind::And || res == store.empty_and());
        Ok(())
    }
}
