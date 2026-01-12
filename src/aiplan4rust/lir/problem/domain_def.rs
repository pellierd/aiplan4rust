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
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::problem::{LiftedProblem, DomainDef};
//!
//! # let problem: LiftedProblem = todo!();
//! let domain = DomainDef::new(&problem);
//!
//! // Access domain-level information
//! let name = domain.domain_name();
//! let requirements = domain.requirements();
//! let types = domain.types();
//! let constants = domain.constants();
//! let predicates = domain.predicates();
//! let functions = domain.functions();
//! let actions = domain.actions();
//! let methods = domain.methods();
//! let constraints = domain.domain_constraints();
//! ```

use std::fmt::{self, Display, Formatter};
use crate::aiplan4rust::lir::problem::{renderers, LiftedAction, LiftedMethod, LiftedProblem};
use crate::aiplan4rust::interner::{Ident, SelfInternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::serialization::syntax::SyntaxSerializable;
use crate::aiplan4rust::syntax::display::SyntaxDisplay;

/// Wrapper around the domain view of a lifted problem.
///
/// `DomainDef` provides convenient access to domain-level information extracted
/// from a [`LiftedProblem`]. It allows read-only inspection of all elements
/// defining a planning domain, including types, constants, predicates, functions,
/// actions, methods, requirements, and domain-level constraints.
///
/// This struct is typically used when you want to analyze or render the domain
/// of a problem without modifying it.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
/// use crate::aiplan4rust::lir::problem::DomainDef;
///
/// # let problem: LiftedProblem = todo!();
/// let domain = DomainDef::new(&problem);
///
/// // Access domain-level information
/// let name = domain.domain_name();
/// let requirements = domain.requirements();
/// let types = domain.types();
/// let constants = domain.constants();
/// let predicates = domain.predicates();
/// let functions = domain.functions();
/// let actions = domain.actions();
/// let methods = domain.methods();
/// let constraints = domain.domain_constraints();
/// ```
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
    /// The [`Ident`] representing the domain's name.
    pub fn domain_name(&self) -> Ident {
        self.problem.domain_id()
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
    pub fn types(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.problem.types()
    }

    /// Returns true if the problem contains any types.
    pub fn has_types(&self) -> bool {
        self.problem.has_types()
    }

    /// Returns an iterator over all constants in the domain.
    ///
    /// # Returns
    ///
    /// An iterator over references to [`TypedSymbol`]s representing domain-level constants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// for c in domain.constants() {
    ///     println!("Constant: {:?}", c);
    /// }
    /// ```
    pub fn constants(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.problem.constants()
    }

    /// Returns true if the domain contains any constants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// assert!(domain.has_constants());
    /// ```
    pub fn has_constants(&self) -> bool {
        self.problem.has_constants()
    }

    /// Returns the predicates declared in the domain.
    ///
    /// # Returns
    ///
    /// A reference to a [`Vec`] of [`AtomicFormulaSkeleton`] representing predicates.
    pub fn predicates(&self) -> &Vec<AtomicFormulaSkeleton> {
        self.problem.predicates()
    }

    /// Returns the functions declared in the domain.
    ///
    /// # Returns
    ///
    /// A reference to a [`Vec`] of [`AtomicFunctionSkeleton`] representing functions.
    pub fn functions(&self) -> &Vec<AtomicFunctionSkeleton> {
        self.problem.functions()
    }

    /// Returns the actions declared in the domain.
    ///
    /// # Returns
    ///
    /// A reference to a [`Vec`] of [`LiftedAction`] representing actions.
    pub fn actions(&self) -> &Vec<LiftedAction> {
        self.problem.actions()
    }

    /// Returns the methods declared in the domain.
    ///
    /// # Returns
    ///
    /// A reference to a [`Vec`] of [`LiftedMethod`] representing methods.
    pub fn methods(&self) -> &Vec<LiftedMethod> {
        self.problem.methods()
    }

    /// Returns the global domain constraints.
    ///
    /// # Returns
    ///
    /// A reference to an [`Expr`] representing the domain-level constraints.
    pub fn domain_constraints(&self) -> &Expr {
        self.problem.domain_constraints()
    }
}

/// Implements [`SyntaxDisplay`] for [`DomainDef`], providing a way to render
/// the domain as a syntax string without relying on indentation or external formatting.
///
/// This uses the `renderers::syntax::render_domain_def` function to generate
/// a textual representation of the domain's definitions (types, constants,
/// predicates, functions, actions, methods, and constraints) using the
/// domain's internal `StringInterner`.
impl<'a> SyntaxDisplay for DomainDef<'a> {
    /// Formats the domain as a syntax string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the syntax string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::syntax::render_domain_def(f, &self.problem.domain_def(), self.interner())
    }
}

/// Implements [`SelfInternerDisplay`] for [`DomainDef`], allowing the domain
/// to format itself using its own internal `StringInterner`.
///
/// This is useful for producing human-readable representations where
/// interned identifiers are resolved directly without requiring an external interner.
impl<'a> SelfInternerDisplay for DomainDef<'a> {
    /// Formats the domain using its internal interner.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_interner(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::interner::render_domain_def(f, &self.problem.domain_def(), self.interner())
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

impl<'a> SyntaxSerializable for DomainDef<'a> {
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
        // Use the existing SyntaxInternerDisplay implementation
        Ok(self.to_syntax_string())
    }
}
