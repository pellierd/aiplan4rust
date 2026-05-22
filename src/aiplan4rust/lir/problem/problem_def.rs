//! Module `problem_def`
//!
//! This module provides wrapper structs for domain and problem views of a
//! [`LiftedProblem`]. The wrappers offer convenient read-only access to
//! domain- or problem-specific information and specialized rendering capabilities.
//!
//! The primary wrapper in this module is [`ProblemDef`], which focuses on
//! problem-level details such as objects, initial/goal state, constraints,
//! metrics, length, and the initial task network.
//!
//! These wrappers implement formatting traits to allow flexible rendering:
//! - [`SyntaxDisplay`] – produces a syntax-oriented string representation.
//! - [`SelfInternerDisplay`] – uses the internal `StringInterner` for resolving identifiers.
//! - [`Display`] – debug string representation for convenience.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::problem::{LiftedProblem, ProblemDef};
//!
//! # let problem: LiftedProblem = todo!();
//! let problem_wrapper = ProblemDef::new(&problem);
//!
//! // Access problem-level information
//! let name = problem_wrapper.problem_name();
//! let domain = problem_wrapper.domain_name();
//! let objects = problem_wrapper.objects();
//! let init_state = problem_wrapper.init();
//! let goal_state = problem_wrapper.goal();
//! let constraints = problem_wrapper.problem_constraints();
//! let metric = problem_wrapper.metric_spec();
//! let length = problem_wrapper.length_spec();
//! let task_network = problem_wrapper.initial_task_network();
//! ```

use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{ObjectId, Requirement, SymbolId, TypeId, TypedSymbol};
use crate::aiplan4rust::lir::expr::ExprId;
use crate::aiplan4rust::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::lir::problem::NewLiftedProblem;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext};
use core::fmt::Formatter;
use std::fmt::Display;

/// Wrapper around a specific problem instance within a domain.
///
/// `ProblemDef` provides convenient access to problem-specific information
/// extracted from a [`LiftedProblem`]. It allows read-only inspection of all
/// elements defining a problem, including objects, initial/goal states,
/// constraints, metrics, length, and the initial task network.
///
/// This struct is typically used when analyzing or rendering a problem without
/// modifying it.
#[derive(Debug, Clone)]
pub struct ProblemDef<'a> {
    problem: &'a NewLiftedProblem,
}

impl<'a> ProblemDef<'a> {
    /// Constructs a new `ProblemDef` wrapper from a reference to a `LiftedProblem`.
    ///
    /// # Parameters
    /// - `problem`: Reference to the [`LiftedProblem`] to wrap.
    ///
    /// # Returns
    /// A new `ProblemDef` instance wrapping the provided problem.
    pub fn new(problem: &'a NewLiftedProblem) -> Self {
        Self { problem }
    }

    /// Returns the set of requirements declared in the domain.
    ///
    /// # Returns
    /// Reference to a [`HashSet`] of [`Requirement`] representing the domain requirements.
    pub fn requirements(&self) -> &std::collections::HashSet<Requirement> {
        self.problem.requirements()
    }

    /// Returns the problem name.
    ///
    /// # Returns
    /// An [`SymbolId`] representing the problem name.
    pub fn problem_name(&self) -> SymbolId {
        self.problem.problem_name()
    }

    /// Returns the domain name associated with this problem.
    ///
    /// # Returns
    /// An [`SymbolId`] representing the domain name.
    pub fn domain_name(&self) -> SymbolId {
        self.problem.domain_name()
    }

    /// Returns the string interner associated with this problem.
    ///
    /// # Returns
    /// Reference to the [`SymbolInterner`] used by the problem.
    pub fn interner(&self) -> &SymbolInterner {
        self.problem.interner()
    }

    pub fn object_defs(&self) -> &[TypedSymbol<ObjectId, TypeId>] {
        self.problem.problem_object_def()
    }

    /// Returns true if the problem contains any objects.
    ///
    /// # Example
    ///
    /// ```rust
    /// if problem.has_objects() {
    ///     println!("There are objects defined.");
    /// }
    /// ```
    pub fn has_object_defs(&self) -> bool {
        self.problem.has_problem_object_defs()
    }

    /// Returns the initial state expression.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the initial state of the problem.
    pub fn init(&self) -> ExprId {
        self.problem.init()
    }

    /// Returns the goal state expression.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the goal condition of the problem.
    pub fn goal(&self) -> ExprId {
        self.problem.goal()
    }

    /// Returns problem-specific constraints.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing constraints defined specifically for this problem.
    pub fn constraints(&self) -> ExprId {
        self.problem.problem_constraints()
    }

    /// Returns the metric specification for the problem.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the metric expression, if any.
    pub fn metric_spec(&self) -> ExprId {
        self.problem.metric_spec()
    }

    /// Returns the length specification for the problem.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the length specification, if any.
    pub fn length_spec(&self) -> ExprId {
        self.problem.length_spec()
    }

    /// Returns the initial task network of the problem.
    ///
    /// # Returns
    /// Reference to an [`InitialTaskNetwork`] representing the problem's initial task network.
    pub fn initial_task_network(&self) -> &InitialTaskNetwork {
        self.problem.initial_task_network()
    }
}

impl<'a> LiftedSyntaxDisplay for ProblemDef<'a> {
    /// Rendu PDDL/HDDL du problème (objets, init, goal, etc.) avec un contexte externe.
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::problem::render(f, self, ctx)
    }

    /// Rendu "Auto-géré" : crée le contexte à partir du LiftedProblem interne.
    fn fmt_syntax_self(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ctx = RenderContext::new(self.problem);
        self.fmt_syntax(f, &ctx)
    }
}

impl<'a> LiftedDebugDisplay for ProblemDef<'a> {
    /// Rendu structurel technique du problème (Store IDs, Interning, etc.).
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::problem_def::render(f, self, ctx)
    }

    /// Rendu "Auto-géré" pour le debug technique.
    fn fmt_debug_self(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ctx = RenderContext::new(self.problem);
        self.fmt_debug(f, &ctx)
    }
}

/// Implémentation de Display pour faciliter l'usage de println!
/// Par défaut, on affiche la syntaxe PDDL/HDDL.
impl<'a> std::fmt::Display for ProblemDef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_syntax_self(f)
    }
}
