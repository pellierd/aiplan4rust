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
//! - [`Display`] – default string representation for convenience.
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

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use crate::aiplan4rust::lir::problem::{renderers, InitialTaskNetwork, LiftedProblem};
use crate::aiplan4rust::interner::{Ident, SelfInternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::serialization::syntax::SyntaxSerializable;
use crate::aiplan4rust::syntax::SyntaxDisplay;

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
    problem: &'a LiftedProblem,
}

impl<'a> ProblemDef<'a> {
    /// Constructs a new `ProblemDef` wrapper from a reference to a `LiftedProblem`.
    ///
    /// # Parameters
    /// - `problem`: Reference to the [`LiftedProblem`] to wrap.
    ///
    /// # Returns
    /// A new `ProblemDef` instance wrapping the provided problem.
    pub fn new(problem: &'a LiftedProblem) -> Self {
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
    /// An [`Ident`] representing the problem name.
    pub fn problem_name(&self) -> Ident {
        self.problem.problem_id()
    }

    /// Returns the domain name associated with this problem.
    ///
    /// # Returns
    /// An [`Ident`] representing the domain name.
    pub fn domain_name(&self) -> Ident {
        self.problem.domain_id()
    }

    /// Returns the string interner associated with this problem.
    ///
    /// # Returns
    /// Reference to the [`StringInterner`] used by the problem.
    pub fn interner(&self) -> &StringInterner {
        self.problem.interner()
    }

    /// Returns an iterator over all objects in this problem.
    ///
    /// # Example
    ///
    /// ```rust
    /// for obj in problem.objects() {
    ///     println!("Object: {:?}", obj);
    /// }
    /// ```
    pub fn objects(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.problem.objects()
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
    pub fn has_objects(&self) -> bool {
        self.problem.has_objects()
    }

    /// Returns the initial state expression.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the initial state of the problem.
    pub fn init(&self) -> &Expr {
        self.problem.init()
    }

    /// Returns the goal state expression.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the goal condition of the problem.
    pub fn goal(&self) -> &Expr {
        self.problem.goal()
    }

    /// Returns problem-specific constraints.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing constraints defined specifically for this problem.
    pub fn problem_constraints(&self) -> &Expr {
        self.problem.problem_constraints()
    }

    /// Returns the metric specification for the problem.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the metric expression, if any.
    pub fn metric_spec(&self) -> &Expr {
        self.problem.metric_spec()
    }

    /// Returns the length specification for the problem.
    ///
    /// # Returns
    /// Reference to an [`Expr`] representing the length specification, if any.
    pub fn length_spec(&self) -> &Expr {
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

impl<'a> SyntaxDisplay for ProblemDef<'a> {
    /// Formats the problem as a syntax string suitable for output or serialization.
    ///
    /// This implementation delegates to `renderers::syntax::render_problem_def`,
    /// passing the underlying [`LiftedProblem`] and its associated [`StringInterner`].
    ///
    /// # Parameters
    /// - `f`: The [`Formatter`] to write the formatted output into.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating whether formatting succeeded or failed.
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::syntax::render_problem_def(f, &self.problem.problem_def(), self.interner())
    }
}

impl<'a> SelfInternerDisplay for ProblemDef<'a> {
    /// Formats the problem using its internal [`StringInterner`].
    ///
    /// This allows resolving interned identifiers when rendering the problem.
    /// Delegates to `renderers::interner::render_problem`.
    ///
    /// # Parameters
    /// - `f`: The [`Formatter`] to write the output into.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating success or failure.
    fn fmt_interner(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::interner::render_problem(f, self.problem, self.problem.interner())
    }
}

impl<'a> Display for ProblemDef<'a> {
    /// Provides the default human-readable string representation of the problem.
    ///
    /// This implementation delegates to `renderers::default::render_problem`.
    ///
    /// # Parameters
    /// - `f`: The [`Formatter`] to write the output into.
    ///
    /// # Returns
    /// [`fmt::Result`] indicating success or failure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fmt::Display;
    /// let problem_def: ProblemDef = /* obtain ProblemDef */;
    /// println!("{}", problem_def); // Uses this Display implementation
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_problem(f, self.problem)
    }
}

impl<'a> SyntaxSerializable for ProblemDef<'a> {
    /// Serializes the problem definition into a syntax string.
    ///
    /// This method produces a normalized, human-readable representation of the problem,
    /// including all objects, initial state, goals, and tasks. It uses the internal
    /// [`StringInterner`] of the problem to resolve all identifiers.
    ///
    /// The resulting string is suitable for saving to a file, re-parsing, or comparing
    /// problem definitions in a normalized form.
    ///
    /// # Returns
    ///
    /// A `String` containing the serialized problem.
    ///
    /// # Errors
    ///
    /// This method may return a [`SerializationError`] if internal formatting fails,
    /// although with the current implementation this is unlikely because `to_syntax_string`
    /// is infallible.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::aiplan4rust::lir::problem::{ProblemDef, LiftedProblem};
    /// # let problem: LiftedProblem = todo!();
    /// let problem_def = ProblemDef::new(&problem);
    /// let serialized = problem_def.serialize_to_string().unwrap();
    /// println!("{}", serialized);
    /// ```
    fn serialize_to_string(
        &self,
    ) -> Result<String, SerializationError> {
        // Use the existing SyntaxInternerDisplay implementation
        Ok(self.to_syntax_string())
    }
}
