use crate::aiplan4rust::grounding::analysis::inertia::{InertiaTable, InertiaError};
use crate::aiplan4rust::grounding::analysis::inertia::registry::{InertiaRegistry, InertiaRegistryError};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Default maximum number of arguments a predicate or function can have.
const DEFAULT_MAX_ARITY: usize = 15;
/// Default maximum number of constants to project for index-based lookups.
const DEFAULT_MAX_PROJ: usize = 3;

/// A builder pattern implementation for configuring and constructing an [`InertiaRegistry`].
///
/// This builder allows users to fine-tune performance and memory usage by adjusting
/// arity limits and projection depth before parsing the initial state of a problem.
///
/// # Examples
///
/// Basic usage with default settings:
/// ```ignore
/// let registry = InertiaRegistry::builder(problem, inertia_table).build()?;
/// ```
///
/// Expert usage with custom limits:
/// ```ignore
/// let registry = InertiaRegistry::builder(problem, inertia_table)
///     .with_max_arity(32)
///     .with_max_proj(5)
///     .build()?;
/// ```
pub struct InertiaRegistryBuilder<'a> {
    problem: &'a LiftedProblem,
    inertia: &'a InertiaTable,
    value_registry: &'a ValueRegistry,
    max_arity: usize,
    max_proj: usize,
}

impl<'a> InertiaRegistryBuilder<'a> {
    /// Creates a new builder with default configuration.
    ///
    /// * `problem`: The lifted representation of the PDDL problem.
    /// * `inertia`: The table identifying which fluents are static/inertial.
    pub(crate) fn new(problem: &'a LiftedProblem, inertia: &'a InertiaTable, value_registry: &'a ValueRegistry) -> Self {
        Self {
            problem,
            inertia,
            value_registry,
            max_arity: DEFAULT_MAX_ARITY,
            max_proj: DEFAULT_MAX_PROJ,
        }
    }

    /// Sets the maximum supported arity for predicates and functions.
    ///
    /// This value is used during the build phase to validate the problem's domain.
    /// If a predicate or function exceeds this arity, [`InertiaRegistry::build`] will return an error.
    ///
    /// # Arguments
    /// * `arity`: The maximum number of parameters allowed.
    pub fn with_max_arity(mut self, arity: usize) -> Self {
        self.max_arity = arity;
        self
    }

    /// Sets the maximum projection depth for inertial indexing.
    ///
    /// The registry creates bitmask-based indexes for constant arguments. This value
    /// determines how many arguments are indexed (the $k$ in $k$-combinations).
    /// Increasing this value improves lookup precision but significantly increases memory usage.
    ///
    /// # Arguments
    /// * `proj`: The maximum number of constant arguments to index.
    pub fn with_max_proj(mut self, proj: usize) -> Self {
        self.max_proj = proj;
        self
    }

    /// Consumes the builder and constructs the [`InertiaRegistry`].
    ///
    /// This method triggers the preorder traversal of the problem's initial state
    /// to populate the internal counting and static value maps.
    ///
    /// # Errors
    /// Returns [`InertiaRegistryError`] if the problem arity exceeds `max_arity` or if
    /// the initial state contains malformed expressions.
    pub fn build(self) -> Result<InertiaRegistry<'a>, InertiaRegistryError> {
        // Correction de l'ordre des arguments pour matcher build_with_config
        InertiaRegistry::build_with_config(
            self.problem,
            self.inertia,
            self.value_registry,
            self.max_arity,
            self.max_proj,
        )
    }
}
