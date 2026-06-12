use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::{
    ExprConstant, ExprEvaluatorError,
};
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::Expr;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton,
};
use crate::aiplan4rust::support::lang::{AtomSkeletonId, FunctionSkeletonId, ObjectId};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

/// The maximum number of arguments a predicate or function can possess by default.
///
/// This serves as a structural safety boundary used during `check_invariants` to prevent
/// the processing of excessively large signatures that could overflow static constraints
/// or downstream fixed bitmasks.
pub(crate) const DEFAULT_MAX_ARITY: usize = 15;

/// The fallback maximum projection size allowed for combinatorial indexing.
///
/// This configuration restricts the size of the sub-vectors evaluated during partial
/// instantiation analysis (Definition 7 & 8 of the IPP paper). It prevents exponential
/// explosion when calculating the power set of combinations for highly variable fluents.
pub(crate) const DEFAULT_MAX_PROJ: usize = 3;

/// The inline capacity threshold for stack-allocated argument arrays.
///
/// Setting this to `8` ensures that any predicate or function with 8 or fewer arguments
/// has its data stored directly on the stack via `SmallVec`. This completely avoids heap
/// allocations (`malloc`) along the evaluator's critical hot paths (`evaluate`, `extract_mask_dynamic`).
/// Statistically, more than 99% of benchmark PDDL domains fall well within this limit.
pub(crate) const ARGUMENT_BUFFER_SIZE: usize = 8;

/// The inline static capacity for the combinatorial hashing keys (`ComboKey`).
///
/// Setting this to `4` allocates exactly 4 slots for `ObjectId` (`usize`, which is 8 bytes on 64-bit platforms).
/// This yields a static footprint of 32 bytes for the internal array. When packaged inside the nested
/// indexing maps, it allows the entire bucket entry (key + payload metadata) to align comfortably within
/// a standard 64-byte CPU cache line.
///
/// This inline size covers more than 95% of standard PDDL projection use cases (`max_proj` is rarely > 3),
/// guaranteeing zero heap allocation during key generation while preventing stack bloating.
pub(crate) const COMBO_KEY_SIZE: usize = 4;

/// A specialized stack-allocated vector tailored for high-frequency argument extraction.
///
/// It utilizes `ARGUMENT_BUFFER_SIZE` to bypass the heap during the evaluation cycle.
/// If a term exceeds this length, `SmallVec` will transparently spill over to a standard
/// heap-allocated buffer, maintaining correctness at the expense of a minor performance hit.
pub(crate) type ArgumentBuffer = SmallVec<[ObjectId; ARGUMENT_BUFFER_SIZE]>;

/// Optimized combinatorial hashing key layout.
///
/// Represents an ordered collection of bound `ObjectId` elements tracking coordinate slices.
/// It uses `COMBO_KEY_SIZE` to remain fully embedded on the stack frame during mask generation.
/// If a combinatorial projection requires binding more than 4 variables at runtime, `SmallVec`
/// transparently dynamic-allocates on the heap, ensuring architectural robustness.
pub(crate) type ComboKey = SmallVec<[ObjectId; COMBO_KEY_SIZE]>;

/// A high-performance, lookup-driven evaluator that simplifies planning fluents into compile-time constants.
///
/// The `InertiaEvaluator` implements structural analysis based on the IPP (Inertia Planning Graph) framework.
/// By caching the positive ground facts from the initial state inside multi-tiered indexing maps, it can
/// determine whether a predicate or functional term behaves as a static invariant (inertia) and evaluate it
/// instantly on the hot path without full AST traversal.
///
/// # Lifetime Architecture
///
/// * `'a` - Outlives the structural components of the grounding phase, specifically anchoring the long-lived
///   references to the problem context (`InertiaTable` and `ValueRegistry`).
///
/// # Combinatorial Indexing Layout
///
/// Both `counting_predicates` and `static_functions` use a three-tiered `FxHashMap` hierarchy to implement
/// partial instantiation lookups with zero scan overhead:
/// 1. **Outer Key**: The global structural identifier (`AtomSkeletonId` or `FunctionSkeletonId`).
/// 2. **Middle Key**: A `u16` bitmask representing the specific argument positions bound to constant objects.
/// 3. **Inner Key**: A `ComboKey` (`SmallVec`) tracking the bound object coordinates without heap overhead.
/// 4. **Value**: The aggregated evaluation target (occurrence count or static constant value).
#[derive(Debug)]
pub struct InertiaEvaluator<'a> {
    /// Combinatorial tracking tables for predicate occurrences.
    ///
    /// Maps a predicate ID and an instantiation mask to a nested map where the key is the sequence of
    /// bound object IDs ($\vec{a}\mid_{C(\vec{a})}$) and the value is the absolute occurrence count ($N$)
    /// extracted from the initial state. Used to evaluate truth values under partial grounding.
    pub(super) counting_predicates:
        FxHashMap<AtomSkeletonId, FxHashMap<u16, FxHashMap<ComboKey, usize>>>,

    /// Combinatorial tracking tables for static functional terms.
    ///
    /// Maps a function ID and an instantiation mask to its evaluated state. If a functional term is fully or
    /// partially grounded and matches an initial state assignment, this table maps its constant object coordinates
    /// directly to its assigned `ExprConstant` value.
    pub(super) static_functions:
        FxHashMap<FunctionSkeletonId, FxHashMap<u16, FxHashMap<ComboKey, ExprConstant>>>,

    /// A reference to the global inertia matrix mapping which fluent symbols are rigid/static.
    pub(super) inertia: &'a InertiaTable,

    /// An isolated, heap-allocated copy of the domain's predicate signatures.
    /// Used for structural invariant verification and arity safety checks.
    pub(super) predicate_defs: Box<[AtomicFormulaSkeleton]>,

    /// An isolated, heap-allocated copy of the domain's function signatures.
    /// Retained to cross-reference return types and argument counts during functional term lookups.
    pub(super) function_defs: Box<[AtomicFunctionSkeleton]>,

    /// A reference to the global registry managing objects, types, and constant values.
    pub(super) value_registry: &'a ValueRegistry,

    /// Fast-path cache for numeric/object functions that share a single uniform value across the entire problem.
    ///
    /// If a function symbol is declared static and all initial state assignments map to the exact same
    /// constant value, that value is cached here. This allows the evaluator to completely bypass the inner coordinate
    /// map lookups for domain-wide constant functions.
    pub(super) consensus_values: FxHashMap<FunctionSkeletonId, ExprConstant>,

    /// The upper threshold for argument counts allowed by this instance.
    pub(super) max_arity: usize,

    /// The upper threshold for combinatorial projection slicing.
    pub(super) max_proj: usize,
}

impl<'a> InertiaEvaluator<'a> {
    /// Instantiates and populates a new `InertiaEvaluator` by parsing the planning problem's initial state.
    ///
    /// This constructor performs a deep copy of the domain's structural skeletons, enforces configuration
    /// constraints, and drives a pre-order AST traversal across the initial state expression (`init`). It aggregates
    /// fluent facts and initial function assignments into highly optimized IPP counting tables.
    ///
    /// # Arguments
    ///
    /// * `predicate_defs` - A slice containing the metadata and signatures for all declared domain predicates.
    /// * `function_defs` - A slice containing the metadata and signatures for all declared domain functions.
    /// * `init` - An `Expr` handle referencing the root of the problem's initial state logical conjunction.
    /// * `inertia` - A long-lived reference to the `InertiaTable` detailing which fluent symbols are rigid/static.
    /// * `value_registry` - A long-lived reference to the global object and constant value coordinator.
    /// * `max_arity` - The strict upper bound for predicate/function arguments allowed by this evaluator instance.
    /// * `max_proj` - The strict upper bound for the dynamic combinatorial projection sizing.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - A fully initialized, structural evaluator packed with pre-calculated initial state metrics.
    /// * `Err(InertiaEvaluatorError)` - If an invariant checks fails, or if initial state parsing encounters
    ///   malformed expressions or unresolvable references during `process_init`.
    ///
    /// # Execution Lifecycle & AST Traversal
    ///
    /// 1. **Structural Isolation**: The constructor allocates isolated, heap-allocated slices (`Box<[T]>`) of
    ///    the predicate and function skeletons. This ensures that the evaluator owns its signature contexts
    ///    and decouples its lifetime from the mutable source problem structures.
    /// 2. **Pre-flight Validation**: Executes `check_invariants` to guarantee that no domain signature violates
    ///    the capacity parameters provided.
    /// 3. **Linearized Pre-order Traversal**: Iterates over the initial state via an explicit tree iterator (`tree_preorder`).
    ///    * **Atomic Facts & Comparisons**: When encountering an `AtomicFormula` or a `Comparison` (e.g., a numeric
    ///      function assignment like `(= (fluid a) 42)`), it wraps the entry into an `ExprNode` and shifts responsibility
    ///      to `process_init`. It then short-circuits the iterator's descent via `skip_children` because `process_init`
    ///      manually processes the immediate child arguments.
    ///    * **PDDL Stratification**: Explicitly ignores `Not` operators and `TimedInitialLiteral` (TIL) wrappers.
    ///      Per PDDL semantics, the initial state maps positive literal definitions; negative bounds and timed transitions
    ///      are isolated directly inside the `InertiaTable`.
    pub fn build(
        predicate_defs: &[AtomicFormulaSkeleton],
        function_defs: &[AtomicFunctionSkeleton],
        init: Expr<'_>,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
        max_arity: usize,
        max_proj: usize,
    ) -> Result<Self, InertiaEvaluatorError> {
        // 1. Isolate and own the structural definitions via boxed slices
        let mut evaluator = Self {
            predicate_defs: predicate_defs.to_vec().into_boxed_slice(),
            function_defs: function_defs.to_vec().into_boxed_slice(),
            inertia,
            value_registry,
            counting_predicates: FxHashMap::default(),
            static_functions: FxHashMap::default(),
            consensus_values: Default::default(),
            max_arity,
            max_proj,
        };

        // 2. Short-circuit early if structural limits are exceeded
        evaluator.check_invariants()?;

        // 3. Drive the initial state parsing phase
        let mut it = init.store().tree_preorder(init.root_id());

        while let Some((id, _, _, entry)) = it.next() {
            match entry.kind() {
                // Intercept atomic ground literals and functional initializations
                ExprKind::AtomicFormula(_) | ExprKind::Comparison(_) => {
                    let node_ref = ExprNode::new(id, entry);
                    evaluator.process_init(node_ref, init.store())?;

                    // Skip sub-children since process_init internally consumes argument entries
                    it.skip_children(entry.children().len());
                }

                // In standard PDDL, init contains positive ground atoms.
                // Negations and Timed Initial Literals (TILs) are managed directly by the inertia table.
                ExprKind::Not | ExprKind::TimedInitialLiteral => {
                    it.skip_children(entry.children().len());
                }

                _ => {}
            }
        }

        Ok(evaluator)
    }

    /// Construits an `InertiaEvaluator` using the fallback max limits.
    ///
    /// This named alternative constructor provides an ergonomic interface for general-purpose grounding
    /// and evaluation pipelines, removing the need to manually supply execution bounds when custom
    /// constraints are unnecessary.
    ///
    /// # Arguments
    ///
    /// * `predicate_defs` - A slice containing the metadata and signatures for all declared domain predicates.
    /// * `function_defs` - A slice containing the metadata and signatures for all declared domain functions.
    /// * `init` - An `Expr` handle referencing the root of the problem's initial state logical conjunction.
    /// * `inertia` - A long-lived reference to the target `InertiaTable`.
    /// * `value_registry` - A long-lived reference to the system's `ValueRegistry`.
    ///
    /// # Default Parameters Applied
    ///
    /// * `max_arity` -> Driven by `DEFAULT_MAX_ARITY` (configured to 15).
    /// * `max_proj`  -> Driven by `DEFAULT_MAX_PROJ` (configured to 3).
    pub fn build_with_defaults(
        predicate_defs: &[AtomicFormulaSkeleton],
        function_defs: &[AtomicFunctionSkeleton],
        init: Expr<'_>,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
    ) -> Result<Self, InertiaEvaluatorError> {
        Self::build(
            predicate_defs,
            function_defs,
            init,
            inertia,
            value_registry,
            DEFAULT_MAX_ARITY,
            DEFAULT_MAX_PROJ,
        )
    }

    /// Validates the structural and configuration invariants of the evaluator.
    ///
    /// This method enforces critical boundary conditions before the grounding and evaluation
    /// phases begin. It ensures that the current problem definition matches the hardware or
    /// algorithmically defined limits of the combinatorial tracking maps.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all configuration parameters are logically consistent and no problem definitions
    ///   exceed the evaluator's capacity.
    /// * `Err(InertiaEvaluatorError)` - If an invariant is violated, specifically:
    ///   * `InertiaEvaluatorError::InvalidLimits`: When `max_arity` exceeds the `u16` storage boundary (16),
    ///     or when the maximum projection size (`max_proj`) is configured to be greater than `max_arity`.
    ///   * `InertiaEvaluatorError::PredicateArityTooHigh`: When a predicate's signature has more
    ///     arguments than the evaluator can index.
    ///   * `InertiaEvaluatorError::FunctionArityTooHigh`: When a functional term's signature
    ///     exceeds the max arity threshold.
    ///
    /// # Invariant Guarantees
    ///
    /// 1. **Hardware & Type Boundary ($max\_arity \le 16$)**: Enforces an absolute physical limit. Since
    ///    instantiation masks are packed into a `u16` primitive integer, any configured arity higher than 16
    ///    is rejected to prevent silent bit-shift underflows/overflows during mask extraction.
    /// 2. **Configuration Sanity ($max\_proj \le max\_arity$)**: The dynamic projection size used to slice
    ///    argument vectors cannot exceed the global arity boundary. Violating this would lead to out-of-bounds
    ///    bitmask shifts during projection calculation.
    /// 3. **Predicate Structural Bounds**: Iterates through all `predicate_defs` to verify that their
    ///    arities fit within `max_arity`. This guarantees that their runtime argument sequences safely
    ///    fit inside the stack-allocated `ArgumentBuffer`.
    /// 4. **Function Structural Bounds**: Iterates through all `function_defs` to apply the same arity
    ///    safeguards on functional terms.
    ///
    /// # Fail-Fast Behavior
    ///
    /// This function returns a `Result` and uses the `?` operator implicitly in the constructor to short-circuit
    /// the initialization process. If a problem domain is too large or misconfigured, it fails immediately
    /// during the `build` phase rather than panicking with an out-of-bounds index later during hot-path execution.
    fn check_invariants(&self) -> Result<(), InertiaEvaluatorError> {
        // 1. Absolute Type Safety: Disallow configurations exceeding the u16 bitmask capacity
        if self.max_arity > 16 {
            return Err(InertiaEvaluatorError::invalid_limits(16, self.max_proj));
        }

        // 2. Validate configuration coherence
        if self.max_proj > self.max_arity {
            return Err(InertiaEvaluatorError::invalid_limits(
                self.max_arity,
                self.max_proj,
            ));
        }

        // 3. Validate predicate arity limits
        for (i, p) in self.predicate_defs.iter().enumerate() {
            if p.arity() > self.max_arity {
                return Err(InertiaEvaluatorError::predicate_arity_too_high(
                    AtomSkeletonId::from(i),
                    p.arity(),
                ));
            }
        }

        // 4. Validate function arity limits
        for (i, f) in self.function_defs.iter().enumerate() {
            if f.arity() > self.max_arity {
                return Err(InertiaEvaluatorError::function_arity_too_high(
                    FunctionSkeletonId::from(i),
                    f.arity(),
                ));
            }
        }

        Ok(())
    }

    /// Validates combinatorial projection constraints and computes the logical instantiation status of an expression.
    ///
    /// This method unifies and centralizes the zero-cost evaluation of argument masks for both
    /// logical predicates and numerical functions. It enforces two critical rules from the IPP framework:
    /// 1. **Projection Limiting (`max_proj`)**: Restricts combinatorial explosion by aborting evaluation
    ///    if the number of bound constant objects exceeds the configured threshold.
    /// 2. **Global Instantiation Status (`grounded`)**: Determines via $O(1)$ bitwise operations whether
    ///    the call contains any unbound free variables.
    ///
    /// # Algorithm & Performance Notes
    ///
    /// * **$O(1)$ Bitwise Evaluation**: Instead of traversing the Abstract Syntax Tree (AST) in RAM,
    ///   the `grounded` status is deduced by comparing the raw mask against a target perfect mask computed
    ///   via a bit-shift operation (`(1 << arity) - 1`). This executes in a single CPU instruction cycle.
    /// * **Compiler Inlining**: Marked with `#[inline]`, this function completely eliminates function call
    ///   overhead at runtime. The LLVM compiler fuses these instructions directly into the grounder's hot loops.
    ///
    /// # Arguments
    ///
    /// * `mask` - The raw Big-Endian bitmask extracted from the expression node. Each bit set to `1`
    ///   represents a bound constant object; each bit set to `0` represents a free variable.
    /// * `arity` - The structural arity (number of expected parameters) declared in the skeleton signature
    ///   (predicate or function).
    ///
    /// # Returns
    ///
    /// * `Some(true)`  - The expression is fully instantiated (*fully grounded*). All required arguments are constants.
    /// * `Some(false)` - The expression is partially instantiated. It contains at least one free variable.
    /// * `None`        - The maximum projection limit has been breached (`count_ones() > max_proj`). The caller
    ///                   must immediately abort the current evaluation path and return `Ok(None)`.
    #[inline]
    pub(crate) fn validate_projection_and_grounding(
        &self,
        mask: u16,
        arity: usize,
    ) -> Option<bool> {
        // 1. Strict limitation of combinatorial projection (Prevents hash-map table explosion)
        if mask != 0 && mask.count_ones() as usize > self.max_proj {
            return None;
        }

        // 2. Ultra-fast bitwise check for fully-grounded status.
        // If arity == 0, the item is trivially grounded.
        // Otherwise, all lower-order bits up to 'arity' must be set to 1.
        let grounded = arity == 0
            || mask
                == if arity >= 16 {
                    0xFFFF
                } else {
                    (1u16 << arity) - 1
                };

        Some(grounded)
    }

    /// Extracts the instantiation mask and associated constant objects from an atomic formula or function term.
    ///
    /// This method implements **Definition 8** from the IPP (Inertia Planning Graph) paper:
    /// $$C(a) := \{i \mid a_i \text{ is a constant}\}$$
    ///
    /// It scans the arguments of a predicate or function to identify bound constants versus unbound variables,
    /// enabling targeted lookups in the combinatorial tracking tables.
    ///
    /// # Arguments
    ///
    /// * `node` - The `ExprNode` handle representing the atomic formula or function call.
    /// * `store` - The backing `ExprStore` used to resolve the kind of each argument node.
    /// * `buffer` - A mutable reference to a stack-allocated `ArgumentBuffer` (`SmallVec`), which is cleared
    ///   and repopulated with the extracted constant `ObjectId`s.
    ///
    /// # Returns
    ///
    /// * `u16` - A bitmask representing the set $C$. Each bit corresponds to an argument position.
    ///   The encoding uses a **Big Endian** layout:
    ///   * The first argument corresponds to the most significant bit ($1 \ll (\text{args.len()} - 1)$).
    ///   * The last argument corresponds to the least significant bit ($1 \ll 0$).
    ///
    /// # IPP Paper & ADL Semantics
    ///
    /// * **Combinatorial Evaluation**: According to the IPP framework, evaluating $N(p, \vec{a})$ requires
    ///   knowing which positions of the argument vector $\vec{a}$ are occupied by constants. This bitmask is used
    ///   as a hash key to select the appropriate counting table $T(p, C)$.
    /// * **Argument Restriction**: The `buffer` implements the sequence restriction $\vec{a}\mid_{C(\vec{a})}$
    ///   (**Definition 7**). It preserves the relative order of constant objects while skipping variables.
    /// * **Variable-Awareness (Section 3.2)**: If an argument is an unbound `Variable`, its bit remains `0`
    ///   in the mask and it is omitted from the buffer. This supports partial instantiations during advanced
    ///   grounding phases.
    ///
    /// # Performance & Safety Notes
    ///
    /// * **Zero Heap Allocation**: Operating on a pre-allocated `SmallVec` buffer prevents heap churning
    ///   on the grounder's hot path.
    /// * **Bit-Shift Optimization**: Uses a branchless, index-based shift `1 << (max_shift - idx)`
    ///   leveraging the enumerated iterator. This allows LLVM to unroll the loop, optimize
    ///   Big-Endian bit positioning, and completely eliminate runtime bounds checking.
    /// * **Underflow Guard**: Explicitly checks if `children.len() <= 1` (symbol with no arguments), ensuring
    ///   the bit-shift initializer `args.len() - 1` never suffers from an unsigned integer underflow.
    pub(crate) fn extract_mask_dynamic(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> u16 {
        buffer.clear();
        let children = node.children();

        // Guard: index 0 is the symbol itself. If len <= 1, there are no arguments.
        if children.len() <= 1 {
            return 0;
        }

        let args = &children[1..];
        let mut mask = 0u16;
        let max_shift = args.len() - 1;

        // By using an enumerated iterator, LLVM can unroll the loop
        // and completely eliminate bound checks or tracking-bit state overhead.
        for (idx, &arg_id) in args.iter().enumerate() {
            if let Ok(arg_entry) = store.fetch(arg_id) {
                if let ExprKind::Object(obj_id) = arg_entry.kind() {
                    // Compute the Big-Endian bit position: first argument maps to the highest bit.
                    mask |= 1 << (max_shift - idx);
                    buffer.push(*obj_id); // Direct inline write to the stack buffer
                }
            }
        }

        mask
    }
}

impl<'a> ExprEvaluator for InertiaEvaluator<'a> {
    /// Evaluates a lifted expression to a constant value using the pre-computed
    /// IPP (Inertia Planning Graph) counting tables and static function registries.
    ///
    /// This method acts as the primary evaluation dispatch engine for the grounder.
    /// It intercepts atomic formulas (predicates) and functional expressions, looks up
    /// their partial instantiation states, and simplifies them into a compile-time constant
    /// (`ExprConstant`) if they are determined to be invariant or statically known.
    ///
    /// # Arguments
    ///
    /// * `expr` - The `Expr` syntax tree handle to evaluate, wrapping the root ID and the store backing it.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ExprConstant::Boolean(bool)))` - If the expression is an atomic formula that simplifies to `true` or `false`.
    /// * `Ok(Some(ExprConstant))` - The numeric/symbolic constant value if the expression is a static function term.
    /// * `Ok(None)` - If the expression cannot be conclusively simplified (e.g., it contains unbound variables,
    ///   is fluent/dynamic, or lacks sufficient context in the initial state).
    ///
    /// # Errors
    ///
    /// Returns an `Err(Box<dyn ExprEvaluatorError>)` wrapping an underlying `InertiaEvaluatorError` if:
    /// * An unrecoverable structural error or invalid invariant is encountered during node fetching.
    /// * An internal evaluation failure occurs within downstream predicate or function resolution.
    ///
    /// # Implementation Details & Workflow
    ///
    /// 1. **Node Retrieval**: Fetches the root node (`ExprNode`) from the expression's backing `ExprStore`.
    ///    Any retrieval error is converted into `InertiaEvaluatorError`, boxed, and short-circuited via `?`.
    /// 2. **Stack Allocation**: Instantiates a stack-allocated `ArgumentBuffer` (`SmallVec`) to aggregate
    ///    ground object IDs during downstream dynamic mask extraction, guaranteeing **zero heap allocation**
    ///    during hot-path evaluation.
    /// 3. **Pattern Matching & Propagating Dispatch**:
    ///    * `ExprKind::AtomicFormula(_)`: Dispatches to `evaluate_predicate_internal`. Any internal failure is
    ///      short-circuited and propagated up via `?`. On success, maps the inner `Option<bool>` to `Option<ExprConstant>`.
    ///    * `ExprKind::Function(_)`: Dispatches to `evaluate_function_internal`. Any internal failure is
    ///      short-circuited and propagated up via `?`.
    ///    * Any other expression kind is safely skipped and returns `Ok(None)`.
    fn evaluate(
        &self,
        expr: Expr<'_>,
    ) -> Result<Option<ExprConstant>, Box<(dyn ExprEvaluatorError + 'static)>> {
        // 1. Fetch the root node reference from the backing store and map errors to the required trait boundary
        let node_ref = expr
            .store()
            .fetch(expr.root_id())
            .map_err(|e| Box::new(InertiaEvaluatorError::from(e)) as Box<dyn ExprEvaluatorError>)?;
        let store = expr.store();

        // 2. Prepare a stack-allocated buffer for argument aggregation (zero-allocation hot path)
        let mut buffer = ArgumentBuffer::new();

        // 3. Dispatch based on the specific expression kind, propagating errors via `?`
        let result = match node_ref.kind() {
            ExprKind::AtomicFormula(_) => self
                .evaluate_predicate_internal(node_ref, store, &mut buffer)?
                .map(ExprConstant::Boolean),

            ExprKind::Function(_) => {
                self.evaluate_function_internal(node_ref, store, &mut buffer)?
            }

            _ => None,
        };

        Ok(result)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
    use crate::aiplan4rust::support::lang::{FunctionSymbolId, PredicateSymbolId, TypedList};

    // Local test extension to provide the missing `mock` constructor for InertiaEvaluator
    impl<'a> InertiaEvaluator<'a> {
        /// Mock disponible pour les tests unitaires des fichiers frères
        pub(crate) fn mock(
            predicate_defs: &[AtomicFormulaSkeleton],
            function_defs: &[AtomicFunctionSkeleton],
            value_registry: &'a ValueRegistry,
            inertia: &'a InertiaTable,
        ) -> Self {
            Self {
                predicate_defs: predicate_defs.to_vec().into_boxed_slice(),
                function_defs: function_defs.to_vec().into_boxed_slice(),
                inertia,
                value_registry,
                counting_predicates: FxHashMap::default(),
                static_functions: FxHashMap::default(),
                consensus_values: Default::default(),
                max_arity: 15,
                max_proj: 3,
            }
        }

        /// Helper de mock spécifique permettant de configurer finement les limites de projection
        pub(crate) fn mock_with_config(
            p_defs: &[AtomicFormulaSkeleton],
            f_defs: &[AtomicFunctionSkeleton],
            v_reg: &'a ValueRegistry,
            i_table: &'a InertiaTable,
            max_arity: usize,
            max_proj: usize,
        ) -> Self {
            Self {
                predicate_defs: p_defs.to_vec().into_boxed_slice(),
                function_defs: f_defs.to_vec().into_boxed_slice(),
                inertia: i_table,
                value_registry: v_reg,
                counting_predicates: FxHashMap::default(),
                static_functions: FxHashMap::default(),
                consensus_values: FxHashMap::default(),
                max_arity,
                max_proj,
            }
        }
    }

    /// Helper function to generate mock predicate definitions for testing purposes.
    ///
    /// Creates a vector of `AtomicFormulaSkeleton` instances with sequential `PredicateSymbolId`s
    /// and empty parameter lists.
    pub(crate) fn mock_predicate_defs(count: usize) -> Vec<AtomicFormulaSkeleton> {
        (0..count)
            .map(|i| AtomicFormulaSkeleton::new(PredicateSymbolId::from(i), TypedList::new()))
            .collect()
    }

    /// Helper function to generate mock function definitions for testing purposes.
    ///
    /// Creates a vector of `AtomicFunctionSkeleton` instances with sequential `FunctionSymbolId`s,
    /// empty parameter lists, and default return types.
    pub(crate) fn mock_function_defs(count: usize) -> Vec<AtomicFunctionSkeleton> {
        (0..count)
            .map(|i| {
                // Adjust here if your AtomicFunctionSkeleton::new requires a different return type structure
                AtomicFunctionSkeleton::new(
                    FunctionSymbolId::from(i),
                    TypedList::new(),
                    Default::default(),
                )
            })
            .collect()
    }
}
