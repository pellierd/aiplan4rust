use crate::aiplan4rust::grounding::binding::evaluator::ExprConstant;
use crate::aiplan4rust::lir::expr::expr::Expr;

/// Extension trait providing a uniform interface for compile-time or static evaluation of
/// expressions within the Logical Intermediate Representation (LIR).
///
/// Implementations of this trait allow the `ExprBinder` and grounding engines to query
/// external domain knowledge, static state variables, or functional inertia (e.g.,
/// rigid PDDL relations) during the pre-processing and grounding pipeline. This architecture
/// enables aggressive branch-pruning (Köhler's optimization) on the fly, preventing the
/// allocation of statically dead branches inside the central `ExprStore`.
///
/// # Thread Safety
///
/// To integrate seamlessly with concurrent grounders or parallelized portfolio planners,
/// all implementations must be thread-safe, enforcing the `Send + Sync` bounds.
///
/// # Examples
///
/// Implementing a basic evaluator that simplifies specific rigid predicates to constant values:
///
/// ```rust
/// use crate::aiplan4rust::lir::expr::expr::Expr;
/// use crate::aiplan4rust::lir::expr::ExprEntryKind;
/// use crate::aiplan4rust::grounding::binding::evaluator::{ExprEvaluator, ExprConstant};
///
/// struct InertiaEvaluator {
///     target_skeleton: AtomSkeletonId,
/// }
///
/// impl ExprEvaluator for InertiaEvaluator {
///     fn evaluate(&self, expr: Expr<'_>) -> Option<ExprConstant> {
///         // Extract the underlying layout directly from the unified store index
///         let current_kind = expr.store()[expr.root_id()].kind();
///
///         match current_kind {
///             ExprEntryKind::AtomicFormula(skel_id) if *skel_id == self.target_skeleton => {
///                 // Prune this specific atomic formula dynamically to false
///                 Some(ExprConstant::Boolean(false))
///             }
///             _ => None, // Node is either not evaluable or not static in this context
///         }
///     }
/// }
/// ```
pub trait ExprEvaluator: Send + Sync {
    /// Evaluates a given expression against the evaluator's static context.
    ///
    /// # Parameters
    ///
    /// * `expr` - A zero-copy wrapper structure combining the active `ExprId` root node and
    ///   an immutable reference to the backing unified arena (`ExprStore`).
    ///
    /// # Returns
    ///
    /// * `Some(ExprConstant)` - The evaluated constant representation if the expression is
    ///   fully determined, static, or invariant under the current domain state.
    /// * `None` - If the expression contains unresolvable variables, dynamic fluents, or
    ///   is otherwise ineligible for immediate reduction.
    ///
    /// # Complexity & Optimization Guarantees
    ///
    /// The evaluation occurs during the second pass (upward post-order traversal) of the
    /// expression tree reduction. Returning a value here triggers an automated short-circuit
    /// rewrite, propagating identity elements (e.g., `And(..., False)` $\rightarrow$ `False`)
    /// up to the parent structures.
    fn evaluate(&self, expr: Expr<'_>) -> Option<ExprConstant>;
}
