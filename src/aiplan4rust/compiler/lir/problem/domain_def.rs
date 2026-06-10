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
//! - [`Display`] – debug string representation of the domain.

use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::compiler::lir::problem::problem::Problem;
use crate::aiplan4rust::compiler::lir::problem::skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton,
};
use crate::aiplan4rust::compiler::lir::problem::{
    ActionDef, DerivedPredicateDef, LiftedProblem, MethodDef,
};
use crate::aiplan4rust::compiler::lir::renderers;
use crate::aiplan4rust::compiler::lir::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext,
};
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::{
    ObjectId, Requirement, SymbolId, TypeId, TypedList, TypedSymbol,
};
use core::fmt::Formatter;

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
    problem: &'a Problem,
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
    /// The [`SymbolId`] representing the domain's name.
    pub fn domain_name(&self) -> SymbolId {
        self.problem.domain_name()
    }

    /// Returns the string interner associated with this domain.
    ///
    /// # Returns
    ///
    /// A reference to the [`SymbolInterner`] used by the lifted problem.
    pub fn interner(&self) -> &SymbolInterner {
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

    /// Returns an iter over all types in the domain.
    ///
    /// # Returns
    ///
    /// An iter over references to [`TypedSymbol`]s in the domain. Each
    /// [`TypedSymbol`] is associated with a unique typing identifier (`Ident`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// for ty in domain.types() {
    ///     println!("Type: {:?}", ty);
    /// }
    /// ```
    pub fn type_defs(&self) -> &TypedList<TypeId, TypeId> {
        self.problem.type_defs()
    }

    /// Checks if the problem contains any typing definitions.
    ///
    /// This is typically true for PDDL domains that use the `:typing` requirement.
    pub fn has_type_defs(&self) -> bool {
        self.problem.has_type_defs()
    }

    /// Returns a slice of constant definitions defined in the domain.
    ///
    /// Constants are "global" objects available across all problems
    /// associated with this domain.
    pub fn constant_defs(&self) -> &[TypedSymbol<ObjectId, TypeId>] {
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
    pub fn derived_predicates(&self) -> &[DerivedPredicateDef] {
        self.problem.derived_predicate_defs()
    }

    /// Returns a slice of all lifted action definitions.
    ///
    /// These are the primitive operators available to the planner.
    pub fn action_defs(&self) -> &[ActionDef] {
        self.problem.action_defs()
    }

    /// Returns a slice of all lifted method definitions.
    ///
    /// Methods define how an abstract task can be decomposed into
    /// sub-tasks within an HTN framework.
    pub fn method_defs(&self) -> &[MethodDef] {
        self.problem.method_defs()
    }

    /// Returns the global domain constraints.
    ///
    /// # Returns
    ///
    /// A reference to an [`Expr`] representing the domain-level constraints.
    pub fn constraints(&self) -> ExprId {
        self.problem.domain_constraints()
    }
}

impl<'a> LiftedSyntaxDisplay for DomainDef<'a> {
    /// Rendu PDDL du domaine complet en utilisant un contexte externe.
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::domain::render(f, self, ctx)
    }

    /// Rendu "Auto-géré" : crée le contexte à partir du problème interne.
    fn fmt_syntax_self(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ctx = RenderContext::new(self.problem);
        self.fmt_syntax(f, &ctx)
    }
}

impl<'a> LiftedDebugDisplay for DomainDef<'a> {
    /// Rendu structurel technique du domaine.
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        // Souvent, on délègue au renderer de problème car le domaine est une vue du problème
        renderers::debug::domain_def::render(f, self, ctx)
    }

    /// Rendu "Auto-géré" pour le debug.
    fn fmt_debug_self(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ctx = RenderContext::new(self.problem);
        self.fmt_debug(f, &ctx)
    }
}

/// Implémentation de Display pour faciliter l'usage de println!
/// Par défaut, on affiche la syntaxe PDDL/HDDL.
impl<'a> std::fmt::Display for DomainDef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_syntax_self(f)
    }
}
