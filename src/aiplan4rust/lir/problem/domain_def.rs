use std::fmt::{self, Display, Formatter};
use crate::aiplan4rust::lir::problem::{renderers, LiftedAction, LiftedMethod, LiftedProblem};
use crate::aiplan4rust::interner::{Ident, SelfInternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::syntax::display::SyntaxDisplay;

/// Wrapper around the domain view of a lifted problem.
///
/// Provides access to domain-level information such as:
/// - Domain name
/// - Requirements
/// - Types, constants
/// - Predicates, functions
/// - Actions and methods
/// - Domain-level constraints
#[derive(Debug, Clone)]
pub struct DomainDef<'a> {
    problem: &'a LiftedProblem,
}

impl<'a> DomainDef<'a> {
    /// Constructs a new `Domain` wrapper from a reference to a `LiftedProblem`.
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self { problem }
    }

    /// Returns the domain name.
    pub fn domain_name(&self) -> Ident {
        self.problem.domain_name()
    }

    /// Returns the string interner associated with this domain.
    pub fn interner(&self) -> &StringInterner {
        self.problem.interner()
    }

    /// Returns the set of requirements declared in the domain.
    pub fn requirements(&self) -> &std::collections::HashSet<Requirement> {
        self.problem.requirements()
    }

    /// Returns a slice of the domain's types.
    pub fn types(&self) -> &std::collections::HashSet<TypedSymbol> {
        self.problem.types()
    }

    /// Returns the domain-level constants.
    pub fn constants(&self) -> &std::collections::HashSet<TypedSymbol> {
        self.problem.constants()
    }

    /// Returns the domain predicates.
    pub fn predicates(&self) -> &Vec<AtomicFormulaSkeleton> {
        self.problem.predicates()
    }

    /// Returns the domain functions.
    pub fn functions(&self) -> &Vec<AtomicFunctionSkeleton> {
        self.problem.functions()
    }

    /// Returns the actions declared in the domain.
    pub fn actions(&self) -> &Vec<LiftedAction> {
        self.problem.actions()
    }

    /// Returns the methods declared in the domain.
    pub fn methods(&self) -> &Vec<LiftedMethod> {
        self.problem.methods()
    }

    /// Returns the global domain constraints.
    pub fn domain_constraints(&self) -> &Expr {
        self.problem.domain_constraints()
    }
}

impl<'a> SyntaxDisplay for DomainDef<'a> {
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::syntax::render_domain_def(f, &self.problem.domain_def(), self.interner())
    }
}

impl<'a> SelfInternerDisplay for DomainDef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::interner::render_domain_def(f, &self.problem.domain_def(), self.interner())
    }
}

impl<'a> Display for DomainDef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_problem(f, self.problem)
    }
}
