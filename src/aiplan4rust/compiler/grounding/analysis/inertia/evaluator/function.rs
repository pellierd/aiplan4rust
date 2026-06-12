//! Inertia-Based Expression Evaluation and Branch Pruning Engine.
//!
//! This module implements the core static analysis and constant-folding dispatchers
//! for the grounding phase, leveraging structural invariants computed from the initial state.
//! It acts as the concrete implementation bridging the unified expression layout (`ExprStore`)
//! and the abstract `ExprEvaluator` trait interface.
//!
//! # Architecture & Theoretical Foundations
//!
//! The module splits evaluation into two highly specialized domains, both executing
//! post-order upward tree simplifications:
//!
//! 1. **Logical Predicate Pruning ([`evaluate_predicate_internal`])**: Implements Section 3.2
//!    (Definition 6) of the **IPP (Inertia Planning Graph)** framework. It cross-references
//!    positive ($I^+$) and negative ($I^-$) structural rigidities against multi-level live instance
//!    counting tables to deduce early `true` or `false` constants.
//! 2. **Functional Constant Folding ([`evaluate_function_internal`])**: Manages rigid
//!    functional fluents (e.g., unchanging object distances or action costs). It enforces
//!    PDDL functional defaults (such as totality fallbacks to `0.0` for unbound numeric terms)
//!    and propagates unanimous functional values up the evaluation tree.
//!
//! # Strategic Performance Layout
//!
//! Designed strictly for the compiler's hot path, the module guarantees heavy throughput
//! optimizations:
//!
//! * **Gated Lazy Filters**: Invariant flags (inertia bitsets) are evaluated as the absolute
//!   first step. If a term is dynamic (fluent), execution is aborted in $\mathcal{O}(1)$ time
//!   before reading token slices, extracting bitmasks, or querying data registries.
//! * **Zero-Allocation Scratchpads**: Rather than allocating dynamic arrays to inspect bound
//!   arguments, the module utilizes a mutable stack-allocated [`ArgumentBuffer`] (`SmallVec`).
//! * **Cache-Locality Lookup**: Counting structures and function registers use consecutive,
//!   short-circuited map chains (`and_then`) tailored to minimize CPU pointer-chasing.
//! * **Integer Safety**: Combinatorial space evaluation utilizes explicit saturating math
//!   (`saturating_mul`) to immunize the grounder against integer overflow during massive
//!   Cartesian product explosions.

use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
use crate::analysis::inertia::evaluator::InertiaEvaluator;
use ordered_float::OrderedFloat;

impl<'a> InertiaEvaluator<'a> {
    /// Evaluates a functional expression (static fluent) using structural rigidity
    /// and pre-computed static functional maps.
    ///
    /// This function handles the evaluation and constant-folding of numeric or symbolic
    /// functions (e.g., rigid costs, distances, or capacities) that exhibit positive inertia.
    /// It avoids pointer-chasing and runtime lookups by querying localized static registers,
    /// enforcing PDDL functional defaults where applicable.
    ///
    /// # Mathematical & Functional Semantics
    ///
    /// * **Functional Positive Inertia**: A function whose assignments are strictly static
    ///   and immutable after the initial state initialization (never modified by numerical effects).
    /// * **Grounded Total Resolution**: If all parameters are bound constants (`grounded == true`)
    ///   and no entry is recorded in the map, numerical functions safely default to `0.0` to maintain
    ///   structural totality.
    /// * **Partial Evaluation / Unanimity**: For non-grounded terms containing free variables,
    ///   if the underlying registry stores a universal or partial invariant value, it is
    ///   short-circuited and returned immediately.
    ///
    /// # Parameters
    ///
    /// * `node` - The abstract syntax tree node representation (`ExprNode`) of the functional term.
    /// * `store` - A reference to the immutable global `ExprStore` holding the tree context.
    /// * `buffer` - A mutable scratchpad reference (`ArgumentBuffer`) used for zero-allocation
    ///   argument extraction during dynamic mask processing.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ExprConstant))` - The evaluated constant numeric or symbolic representation if
    ///   the function resolves deterministically.
    /// * `Ok(None)` - If the function is dynamic (fluent) or cannot be conclusively simplified.
    ///
    /// # Errors
    ///
    /// Returns an `Err(InertiaEvaluatorError)` if an internal structural lookup fails
    /// within the functional inertia bitsets.
    ///
    /// # Performance & Allocation Invariants
    ///
    /// * **Gated Execution**: Evaluates the inertia status *before* parsing parameters or
    ///   allocating slice ranges. Dynamic functional fluents exit immediately in $\mathcal{O}(1)$ time.
    /// * **Bounded Slice Windowing**: Constrains the argument lookup window using `self.max_proj`
    ///   to prevent out-of-bounds pointer reads and ensure highly localized CPU cache hits during map lookups.
    /// * **Zero Heap Allocation**: Completely reuses the provided stack-allocated `ArgumentBuffer`.
    pub(super) fn evaluate_function_internal(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<ExprConstant>, InertiaEvaluatorError> {
        // 1. Isolate the functional identifier from the node layout
        let func_id = match node.kind() {
            ExprKind::Function(id) => *id,
            _ => return Ok(None),
        };

        // 2. Structural Gate: ignore immediately if the function is not static (positive inertia)
        if !self.inertia.is_function_positive_inertia(func_id)? {
            return Ok(None);
        }

        // --- STEP A: Dynamic Mask Extraction ---
        let mask = self.extract_mask_dynamic(node, store, buffer) as u16;

        let def = &self.function_defs[func_id.as_usize()];
        let arity = def.parameters().len();

        // --- STEP B: Validate projection bounds and extract grounding status ---
        let grounded = match self.validate_projection_and_grounding(mask, arity) {
            Some(status) => status,
            None => return Ok(None), // Hard circuit-break if max_proj threshold is breached
        };

        // 3. Prevent slicing out of bounds by capping at max_proj threshold
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        // --- STEP C: Direct O(1) multi-level map lookups for function values ---
        let value = self
            .static_functions
            .get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // --- STEP D: Core Decision Logic & Totality Fallbacks ---
        if grounded {
            if value.is_some() {
                return Ok(value);
            }
            // PDDL/Functional totality fallback: uninitialized numeric items default to 0.0
            if def.ty().is_number() {
                return Ok(Some(ExprConstant::Number(OrderedFloat(0.0))));
            }
        } else {
            // For terms containing free variables:
            // If the static registry contains a unanimous value, propagate it upward
            if value.is_some() {
                return Ok(value);
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::inertia::inertia::Inertia;
    use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFunctionSkeleton;
    use crate::aiplan4rust::support::lang::{
        FunctionSkeletonId, FunctionSymbolId, ObjectId, Type, TypeId, TypedList, TypedSymbol,
        VariableId,
    };

    /// # Purpose
    /// Verifies the PDDL fallback rule for uninitialized grounded numeric functions.
    /// When a function is fully grounded (`f(99)`) but has no entry in the initial state,
    /// the evaluator must yield `0.0` as a default value if the return type is numeric.
    ///
    /// # Input
    /// - Function `f` marked as positive inert.
    /// - Grounded call expression `f(99)`.
    /// - The function skeleton's return type is explicitly set to a **Numeric** type.
    /// - No registration in the initial state masks.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(Some(ExprConstant::Number(0.0)))`.
    #[test]
    fn test_evaluate_function_grounded_missing_returns_pddl_default_zero() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let type_id = TypeId::from(0);
        let obj_99 = ObjectId::from(99);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        let numeric_type = Type::<TypeId>::number();

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
                numeric_type,
            ),
        ];
        let p_defs = vec![];
        let v_reg = ValueRegistry::empty();

        let registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Build the structure: index 0 = Function metadata symbol, index 1 = object parameter
        let symbol_node = builder.intern(ExprKind::Object(ObjectId::from(0)), &[]);
        let const_node = builder.intern(ExprKind::Object(obj_99), &[]);
        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[symbol_node, const_node]);
        let func_node = store.get(func_node_id).unwrap();

        // Simulate real runtime evaluation conditions by populating the buffer
        let mut buffer = ArgumentBuffer::new();
        buffer.push(obj_99);

        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        assert_eq!(
            res.unwrap(),
            Some(ExprConstant::Number(ordered_float::OrderedFloat(0.0))),
            "A grounded numeric function missing from the initial state must return 0.0 by default"
        );
    }
}
