//! Module `domain_def`
//!
//! This module defines the [`DomainDef`] struct, a wrapper around the domain view
//! of a lifted problem (`LiftedProblem`). It provides convenient, read-only access
//! to all domain-level information required for planning and rendering.
//!
//! `DomainDef` allows inspection of:
//! - The domain name
//! - Requirements
//! - Types and constants
//! - Predicates and functions
//! - Actions and methods
//! - Domain-level constraints
//!
//! This module also provides implementations of formatting traits for [`DomainDef`]:
//! - [`SyntaxDisplay`] – formats the domain as a syntax string without interner or indentation.
//! - [`SelfInternerDisplay`] – formats the domain using its internal `StringInterner`.
//! - [`Display`] – default string representation of the domain.

use core::fmt::Display;
use std::fmt::{self, Formatter};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{ObjectID, Requirement, StringID, TypeID, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::{renderers, LiftedAction, LiftedDerivedPredicate, LiftedDurativeAction, LiftedMethod};
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};

/// Wrapper around the domain view of a lifted problem.
///
/// `DomainDef` provides convenient access to domain-level information extracted
/// from a [`LiftedProblem`]. It allows read-only inspection of all elements
/// defining a planning domain, including types, constants, predicates, functions,
/// actions, methods, requirements, and domain-level constraints.
///
/// This struct is typically used when you want to analyze or render the domain
/// of a problem without modifying it.
#[derive(Debug, Clone)]
pub struct DomainDef<'a> {
    problem: &'a LiftedProblem,
}

impl<'a> DomainDef<'a> {
    /// Constructs a new `DomainDef` wrapper from a reference to a [`LiftedProblem`].
    ///
    /// # Arguments
    ///
    /// * `problem` - A reference to the lifted problem whose domain should be wrapped.
    ///
    /// # Returns
    ///
    /// A new `DomainDef` instance providing read-only access to domain information.
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self { problem }
    }

    /// Returns the domain name.
    ///
    /// # Returns
    ///
    /// The [`StringID`] representing the domain's name.
    pub fn domain_name(&self) -> StringID {
        self.problem.domain_name()
    }

    /// Returns the string interner associated with this domain.
    ///
    /// # Returns
    ///
    /// A reference to the [`StringInterner`] used by the lifted problem.
    pub fn interner(&self) -> &StringInterner {
        self.problem.interner()
    }

    /// Returns the set of requirements declared in the domain.
    ///
    /// # Returns
    ///
    /// A reference to a [`HashSet`] of [`Requirement`]s.
    pub fn requirements(&self) -> &std::collections::HashSet<Requirement> {
        self.problem.requirements()
    }

    /// Returns an iterator over all types in the domain.
    ///
    /// # Returns
    ///
    /// An iterator over references to [`TypedSymbol`]s in the domain. Each
    /// [`TypedSymbol`] is associated with a unique type identifier (`Ident`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// for ty in domain.types() {
    ///     println!("Type: {:?}", ty);
    /// }
    /// ```
    pub fn type_defs(&self) -> &[TypedSymbol<TypeID, TypeID>] {
        self.problem.type_defs()
    }

    /// Checks if the problem contains any type definitions.
    ///
    /// This is typically true for PDDL domains that use the `:typing` requirement.
    pub fn has_type_defs(&self) -> bool {
        self.problem.has_type_defs()
    }

    /// Returns a slice of constant definitions defined in the domain.
    ///
    /// Constants are "global" objects available across all problems
    /// associated with this domain.
    pub fn constant_defs(&self) -> &[TypedSymbol<ObjectID, TypeID>] {
        self.problem.domain_constant_def()
    }

    /// Checks if the domain defines any constants.
    pub fn has_constant_defs(&self) -> bool {
        self.problem.has_domain_constant_defs()
    }

    /// Returns a slice of the predicate definitions (skeletons).
    ///
    /// Each skeleton defines the symbol and the expected parameter types
    /// for a boolean fluent.
    pub fn predicate_defs(&self) -> &[AtomicFormulaSkeleton] {
        self.problem.predicate_defs()
    }

    /// Checks if the problem has any predicate definitions.
    pub fn has_predicate_defs(&self) -> bool {
        self.problem.has_predicate_defs()
    }

    /// Returns a slice of the functional definitions (numeric fluents).
    ///
    /// These represent functions that map objects to numeric values,
    /// often used with the `:fluents` requirement.
    pub fn functions_defs(&self) -> &[AtomicFunctionSkeleton] {
        self.problem.function_defs()
    }

    /// Checks if the problem contains any function definitions.
    pub fn has_function_defs(&self) -> bool {
        self.problem.has_function_defs()
    }

    /// Returns a slice of all task definitions (skeletons).
    ///
    /// In HTN planning, these represent the abstract or primitive tasks
    /// that can be part of a task network.
    pub fn task_defs(&self) -> &[AtomicTaskSkeleton] {
        self.problem.task_defs()
    }

    /// Checks if the problem defines any tasks.
    pub fn has_task_defs(&self) -> bool {
        self.problem.has_task_defs()
    }

    /// Returns a slice of all derived predicate definitions (axioms).
    ///
    /// Derived predicates are evaluated based on the current state and
    /// other predicates, rather than being modified directly by actions.
    pub fn derived_predicates(&self) -> &[LiftedDerivedPredicate] {
        self.problem.derived_predicate_defs()
    }

    /// Returns a slice of all lifted action definitions.
    ///
    /// These are the primitive operators available to the planner.
    pub fn action_defs(&self) -> &[LiftedAction] {
        self.problem.action_defs()
    }

    /// Returns a slice of all lifted durative action definitions.
    ///
    /// These actions have a temporal component, including duration
    /// and conditions/effects applied at different time points (start, end, over all).
    pub fn durative_action_defs(&self) -> &[LiftedDurativeAction] {
        self.problem.durative_action_defs()
    }

    /// Returns a slice of all lifted method definitions.
    ///
    /// Methods define how an abstract task can be decomposed into
    /// sub-tasks within an HTN framework.
    pub fn method_defs(&self) -> &[LiftedMethod] {
        self.problem.method_defs()
    }

    /// Returns the global domain constraints.
    ///
    /// # Returns
    ///
    /// A reference to an [`Expr`] representing the domain-level constraints.
    pub fn constraints(&self) -> &Expr {
        self.problem.domain_constraints()
    }
}

/// Implements the standard [`Display`] trait for [`DomainDef`].
///
/// This provides a default string representation of the domain, typically
/// using the default rendering logic for the lifted problem (`renderers::default::render_problem`).
///
/// # Example
///
/// ```rust
/// use std::fmt::Display;
/// let domain_def: DomainDef = ...;
/// println!("{}", domain_def);
/// ```
impl<'a> Display for DomainDef<'a> {
    /// Formats the domain using the default renderer.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_problem(f, self.problem)
    }
}

impl<'a>  LiftedSyntaxDisplay for DomainDef<'a> {
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        renderers::syntax::domain::render(f, self, ctx)
    }

    fn fmt_syntax_self(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctx = RenderContext::new(self.problem);
        self.fmt_syntax(f, &ctx)
    }
}

/*impl<'a> SyntaxSerializable for DomainDef<'a> {
    /// Serializes the domain definition into a syntax string.
    ///
    /// This uses the internal [`StringInterner`] of the domain to resolve
    /// all identifiers into their string representations. The resulting
    /// string is a normalized, human-readable representation of the domain,
    /// suitable for saving to a file or for comparison with other serialized domains.
    ///
    /// # Returns
    ///
    /// A `String` containing the serialized domain.
    ///
    /// # Errors
    ///
    /// This method may return a [`SerializationError`] if any internal
    /// formatting fails, although in the current implementation this is
    /// unlikely since `to_syntax_string` is infallible.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::aiplan4rust::lir::problem::{DomainDef, LiftedProblem};
    /// # let problem: LiftedProblem = todo!();
    /// let domain = DomainDef::new(&problem);
    /// let serialized = domain.serialize_to_string().unwrap();
    /// println!("{}", serialized);
    /// ```
    fn serialize_to_string(
        &self,
    ) -> Result<String, SerializationError> {
        Ok(self.to_syntax_string())
    }
}*/
