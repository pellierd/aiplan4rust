//! Wrappers for domain and problem views.
//!
//! This module provides lightweight wrapper structs for `Domain` and `Problem`
//! that delegate access to a `LiftedProblem` while allowing specialized rendering
//! and domain-/problem-specific logic.

use std::fmt::{self, Display, Formatter};
use crate::aiplan4rust::lir::problem::{renderers, InitialTaskNetwork,LiftedProblem};
use crate::aiplan4rust::interner::{Ident, InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::syntax::SyntaxDisplay;

/// Wrapper around a specific problem instance within a domain.
///
/// Provides access to problem-specific information such as:
/// - Problem name
/// - Objects
/// - Initial state
/// - Goal state
/// - Problem-specific constraints
/// - Metric and length specifications
/// - Initial task network
#[derive(Debug, Clone)]
pub struct ProblemDef<'a> {
    problem: &'a LiftedProblem,
}

impl<'a> ProblemDef<'a> {
    /// Constructs a new `Problem` wrapper from a reference to a `LiftedProblem`.
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self { problem }
    }

    /// Returns the set of requirements declared in the domain.
    pub fn requirements(&self) -> &std::collections::HashSet<Requirement> {
        self.problem.requirements()
    }

    /// Returns the problem name.
    pub fn problem_name(&self) -> Ident {
        self.problem.problem_name()
    }

    pub fn domain_name(&self) -> Ident {
        self.problem.domain_name()
    }

    /// Returns the string interner associated with this problem.
    pub fn interner(&self) -> &StringInterner {
        self.problem.interner()
    }

    /// Returns the objects defined in this problem.
    pub fn objects(&self) -> &std::collections::HashSet<TypedSymbol> {
        self.problem.objects()
    }

    /// Returns the initial state.
    pub fn init(&self) -> &Expr {
        self.problem.init()
    }

    /// Returns the goal expression.
    pub fn goal(&self) -> &Expr {
        self.problem.goal()
    }

    /// Returns the problem-specific constraints.
    pub fn problem_constraints(&self) -> &Expr {
        self.problem.problem_constraints()
    }

    /// Returns the metric specification.
    pub fn metric_spec(&self) -> &Expr {
        self.problem.metric_spec()
    }

    /// Returns the length specification.
    pub fn length_spec(&self) -> &Expr {
        self.problem.length_spec()
    }

    /// Returns the initial task network.
    pub fn initial_task_network(&self) -> &InitialTaskNetwork {
        self.problem.initial_task_network()
    }
}

impl<'a> SyntaxDisplay for ProblemDef<'a> {
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, _interner: &StringInterner, _indent: usize) -> fmt::Result {
        renderers::syntax::render_problem_def(f, &self.problem.problem_def(), self.interner())
    }
}

impl<'a> InternerDisplay for ProblemDef<'a> {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        // To do
        renderers::interner::render_problem(f, self.problem, self.problem.interner())
    }
}
impl<'a> Display for ProblemDef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // To do
        renderers::default::render_problem(f, self.problem)
    }
}
