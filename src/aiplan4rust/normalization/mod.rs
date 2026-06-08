//! # AST Normalization Framework
//!
//! This module provides the infrastructure for normalizing an Abstract Syntax Tree (AST)
//! in the AIPlan4Rust compiler pipeline. It ensures ASTs are transformed into a canonical,
//! consistent, and simplified form that is easier to analyze and compile.
//!
//! ## Purpose
//!
//! Normalization eliminates syntactic ambiguities, resolves implicit constructs, and ensures
//! structural uniformity of AST nodes. This is crucial for downstream components such as
//! the type_checker checker, lifted IR builder, and planner ops.
//!
//! Key goals of logic include:
//!
//! - Flattening or simplification nested constructs (e.g., desugaring `TypedList` and `EitherType`).
//! - Deduplicating or merging repeated entries (e.g., `:requirements`, type_checker definitions).
//! - Ensuring valid and well-scoped definitions are available to semantic analyzers.
//!
//! ## Structure
//!
//! This module is composed of several submodules and utilities:
//!
//! - [`passes`] — Contains the individual logic logic:
//!   - [`typed_list`] — Expands and validates `TypedList` declarations.
//!   - [`either_type`] — Rewrites `either` logic into concrete disjunctions.
//!   - [`require_def`] — Deduplicates and validates domain-level requirements.
//!   - [`type_def`] — Consolidates type_checker hierarchies and removes redundancies.
//!
//! - [`normalizer`] — Provides the [`Normalizer`] struct, the main interface to apply all logic logic.
//!
//! - [`result`] — Defines [`NormalizerResult`], the output of logic containing:
//!   - The possibly modified AST,
//!   - A diagnostic log,
//!   - And an indicator whether any structural change occurred.
//!
//! - [`error`] — Defines the [`NormalizationError`] enum for critical internal logic failures.
//!
//! ## Error Handling
//!
//! Errors during logic are returned as a [`NormalizationError`], which can wrap:
//!
//! - [`SyntaxTreeError`] — Errors in the tree's internal structure.
//! - [`ArenaError`] — Memory arena allocation issues.
//! - [`InternerError`] — Identifier resolution or interning issues.
//!
//! Non-fatal issues (e.g., unsupported types, naming warnings) are collected as diagnostics
//! through the [`DiagnosticManager`] and can be reviewed post-logic.
//!
//! ## Re-exports
//!
//! - [`Normalizer`] — Main entry point to apply logic.
//! - [`NormalizerResult`] — The resulting structure returned after logic.
//! - [`NormalizationError`] — The fatal error type_checker used when logic cannot proceed.
//!
//! ## Example
//!
//! ```rust
//! use aiplan4rust::logic::{Normalizer, NormalizerResult};
//! use aiplan4rust::syntax::ast::Ast;
//!
//! let ast: Ast = /* parsed AST */;
//! let mut normalizer = Normalizer::new();
//!
//! match normalizer.normalize(ast) {
//!     Ok(result) => {
//!         if result.changed() {
//!             println!("AST was normalized and updated.");
//!         }
//!         for diag in result.diagnostics() {
//!             eprintln!("  Diagnostic: {}", diag);
//!         }
//!     }
//!     Err(err) => eprintln!(" Normalization failed: {:?}", err),
//! }
//! ```
//!
//! ## Notes
//!
//! - **AST validity is a precondition**: the input [`Ast`] must be structurally sound.
//!   Invalid ASTs may trigger internal logic errors.
//!
//! - **Pass ordering is critical**: logic logic are executed in a specific sequence,
//!   and skipping or reordering them may result in inconsistent or incorrect ASTs.
//!
//! - Diagnostics allow partial recovery: even if logic completes, collected diagnostics
//!   may indicate semantic issues that require user attention.
//!
//! ## See Also
//!
//! - [`DiagnosticManager`] — Responsible for logging all non-fatal issues during logic.
//! - [`Ast`] — The syntax tree structure being normalized.

pub mod error;
pub mod normalizer;
pub mod passes;
pub mod result;
pub mod validation;

pub use error::NormalizationError;
pub use normalizer::Normalizer;
pub use result::Result as NormalizerResult;
