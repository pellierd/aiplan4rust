//! Diagnostics module for semantic and syntactic analysis errors and warnings.
//!
//! This module defines the infrastructure used to construct, manage, and renderers diagnostics
//! during the parsing and semantic analysis phases of the AIPlan4Rust pipeline.
//! Diagnostics include both errors (which typically prevent further compilation) and warnings
//! (which indicate suspicious or potentially problematic constructs).
//!
//! # Overview
//!
//! The module is composed of several submodules:
//!
//! - [`kind`] defines the [`DiagnosticKind`] enum, which enumerates all diagnostic variants
//!   (e.g., typing mismatches, undeclared symbols, PDDL requirement violations).
//! - [`severity`] defines the [`Severity`] enum that categorizes diagnostics as errors or warnings.
//! - [`diagnostic`] provides the [`Diagnostic`] structure, the primary container for diagnostic information,
//!   including kind, severity, location, and optional suggestions.
//! - [`provider`] defines the [`Provider`] trait for objects that can emit diagnostics.
//! - [`diagnostic_manager`] implements the [`DiagnosticManager`], a central structure for collecting,
//!   storing, and querying diagnostics throughout analysis phases.
//! - [`renderer`] handles pretty-printing and formatting of diagnostics to a human-readable form.
//!
//! # Usage
//!
//! Most diagnostics are constructed through the [`Provider`] interface or directly via
//! [`Diagnostic::new`]. Diagnostics are collected using the [`DiagnosticManager`] and then rendered
//! via the [`Renderer`].
//!
//! ```rust
//! use aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, Severity};
//!
//! let diag = Diagnostic::new(
//!     DiagnosticKind::UndeclaredSymbol { usage: ... },
//!     Severity::Error,
//!     span,
//! );
//! ```
//!
//! # Exports
//! This module publicly re-exports the most important components for convenience:
//!
//! - [`Diagnostic`]
//! - [`DiagnosticKind`]
//! - [`Severity`]
//! - [`Provider`]
//! - [`DiagnosticManager`]
//! - [`Renderer`]
//!
//! # See Also
//! - [`DiagnosticKind`] for the full list of supported diagnostics
//! - [`Renderer`] for formatted output
//! - [`Provider`] for trait-based emission of diagnostics

pub mod severity;
pub mod kind;
pub mod diagnostic;
pub mod provider;
pub mod diagnostic_manager;
mod renderer;
pub mod error;

pub use diagnostic::Diagnostic;
pub use renderer::renderer::Renderer;
pub use diagnostic_manager::DiagnosticManager;
pub use kind::Kind as DiagnosticKind;
pub use provider::Provider;
pub use severity::Severity;
pub use error::DiagnosticError;
