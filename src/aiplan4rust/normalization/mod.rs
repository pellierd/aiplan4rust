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
//! the type_checker checker, lifted IR builder, and planner logic.
//!
//! Key goals of normalization include:
//!
//! - Flattening or rewriting nested constructs (e.g., desugaring `TypedList` and `EitherType`).
//! - Deduplicating or merging repeated entries (e.g., `:requirements`, type_checker definitions).
//! - Ensuring valid and well-scoped definitions are available to semantic analyzers.
//!
//! ## Structure
//!
//! This module is composed of several submodules and utilities:
//!
//! - [`passes`] — Contains the individual normalization passes:
//!   - [`typed_list`] — Expands and validates `TypedList` declarations.
//!   - [`either_type`] — Rewrites `either` expressions into concrete disjunctions.
//!   - [`require_def`] — Deduplicates and validates domain-level requirements.
//!   - [`type_def`] — Consolidates type_checker hierarchies and removes redundancies.
//!
//! - [`normalizer`] — Provides the [`Normalizer`] struct, the main interface to apply all normalization passes.
//!
//! - [`normalizer_result`] — Defines [`NormalizerResult`], the output of normalization containing:
//!   - The possibly modified AST,
//!   - A diagnostic log,
//!   - And an indicator whether any structural change occurred.
//!
//! - [`error`] — Defines the [`NormalizationError`] enum for critical internal normalization failures.
//!
//! ## Error Handling
//!
//! Errors during normalization are returned as a [`NormalizationError`], which can wrap:
//!
//! - [`SyntaxTreeError`] — Errors in the tree's internal structure.
//! - [`ArenaError`] — Memory arena allocation issues.
//! - [`InternerError`] — Identifier resolution or interning issues.
//!
//! Non-fatal issues (e.g., unsupported types, naming warnings) are collected as diagnostics
//! through the [`DiagnosticManager`] and can be reviewed post-normalization.
//!
//! ## Re-exports
//!
//! - [`Normalizer`] — Main entry point to apply normalization.
//! - [`NormalizerResult`] — The resulting structure returned after normalization.
//! - [`NormalizationError`] — The fatal error type_checker used when normalization cannot proceed.
//!
//! ## Example
//!
//! ```rust
//! use aiplan4rust::normalization::{Normalizer, NormalizerResult};
//! use aiplan4rust::syntax::ast::Ast;
//!
//! let ast: Ast = /* parsed AST */;
//! let mut normalizer = Normalizer::new();
//!
//! match normalizer.normalize(ast) {
//!     Ok(result) => {
//!         if result.changed() {
//!             println!("✅ AST was normalized and updated.");
//!         }
//!         for diag in result.diagnostics() {
//!             eprintln!("⚠️  Diagnostic: {}", diag);
//!         }
//!     }
//!     Err(err) => eprintln!("❌ Normalization failed: {:?}", err),
//! }
//! ```
//!
//! ## Notes
//!
//! - **AST validity is a precondition**: the input [`Ast`] must be structurally sound.
//!   Invalid ASTs may trigger internal normalization errors.
//!
//! - **Pass ordering is critical**: normalization passes are executed in a specific sequence,
//!   and skipping or reordering them may result in inconsistent or incorrect ASTs.
//!
//! - Diagnostics allow partial recovery: even if normalization completes, collected diagnostics
//!   may indicate semantic issues that require user attention.
//!
//! ## See Also
//!
//! - [`DiagnosticManager`] — Responsible for logging all non-fatal issues during normalization.
//! - [`Ast`] — The syntax tree structure being normalized.

pub mod normalizer_result;
pub mod normalizer;
pub mod passes;
pub mod error;

pub use normalizer::Normalizer;
pub use normalizer_result::NormalizerResult;
pub use error::NormalizationError;
