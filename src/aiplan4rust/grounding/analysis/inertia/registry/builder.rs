use crate::aiplan4rust::grounding::analysis::inertia::registry::{InertiaRegistry, InertiaRegistryError};
use crate::aiplan4rust::grounding::analysis::inertia::InertiaTable;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};

/// Default maximum number of arguments a predicate or function can have.
const DEFAULT_MAX_ARITY: usize = 15;
/// Default maximum number of constants to project for index-based lookups.
const DEFAULT_MAX_PROJ: usize = 3;

/// A builder pattern implementation for configuring and constructing an [`InertiaRegistry`].
pub struct InertiaRegistryBuilder<'a> {
    // On ne stocke plus le LiftedProblem entier, mais ses composants
    p_defs: &'a [AtomicFormulaSkeleton],
    f_defs: &'a [AtomicFunctionSkeleton],
    init_expr: &'a Expr,

    // Objets externes
    inertia: &'a InertiaTable,
    value_registry: &'a ValueRegistry,

    max_arity: usize,
    max_proj: usize,
}

impl<'a> InertiaRegistryBuilder<'a> {
    /// Creates a new builder with default configuration.
    /// Note: On prend les composants séparément pour permettre le disjoint borrowing.
    pub(crate) fn new(
        p_defs: &'a [AtomicFormulaSkeleton],
        f_defs: &'a [AtomicFunctionSkeleton],
        init_expr: &'a Expr,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry
    ) -> Self {
        Self {
            p_defs,
            f_defs,
            init_expr,
            inertia,
            value_registry,
            max_arity: DEFAULT_MAX_ARITY,
            max_proj: DEFAULT_MAX_PROJ,
        }
    }

    pub fn with_max_arity(mut self, arity: usize) -> Self {
        self.max_arity = arity;
        self
    }

    pub fn with_max_proj(mut self, proj: usize) -> Self {
        self.max_proj = proj;
        self
    }

    /// Consumes the builder and constructs the [`InertiaRegistry`].
    pub fn build(self) -> Result<InertiaRegistry<'a>, InertiaRegistryError> {
        // Appelle la version de build_with_config qui prend les composants
        InertiaRegistry::build_with_config(
            self.p_defs,
            self.f_defs,
            self.init_expr,
            self.inertia,
            self.value_registry,
            self.max_arity,
            self.max_proj,
        )
    }
}
