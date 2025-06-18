//! # AST Normalization Framework
//!
//! This module provides the infrastructure for normalizing an Abstract Syntax Tree (AST)
//! through a set of composable, well-defined normalization passes.
//!
//! ## Purpose
//!
//! Normalization ensures that the AST is transformed into a clean, canonical, and structurally
//! consistent form, preparing it for semantic analysis. This includes:
//!
//! - Flattening or expanding nested constructs (e.g., `TypedItem` inside `TypedList`).
//! - Removing duplicates and merging redundant definitions (e.g., in `:requirements` or type blocks).
//! - Detecting and rewriting implicit structures (e.g., desugaring `either` types).
//!
//! These transformations simplify subsequent compiler or interpreter stages and enforce language invariants.
//!
//! ## Modules
//!
//! - [`passes`] — Contains the individual normalization passes:
//!   - [`typed_list`] — Expands and validates typed lists.
//!   - [`type_def`] — Merges type declarations with the same key.
//!   - [`either_type`] — Handles implicit `either` groupings.
//!   - [`require_def`] — Deduplicates requirements.
//! - [`normalizer`] — Orchestrates all normalization passes in the correct order.
//! - [`normalizer_result`] — Defines the result structure (`NormalizerResult`) returned after running normalization.
//!
//! ## Re-exports
//!
//! - [`Normalizer`] — Main entry point to normalize an AST.
//! - [`NormalizerResult`] — Wraps the final state of the AST along with change tracking and diagnostics.
//!
//! ## Example
//!
//! ```rust,ignore
//! use normalizer::{Normalizer, NormalizerResult};
//!
//! let mut ast_old = ...
//! let mut normalizer = Normalizer::new();
//! let result: NormalizerResult = normalizer.normalize(ast_old)?;
//!
//! if result.changed() {
//!     println!("✅ AST was normalized and updated.");
//! }
//!
//! for diagnostic in result.diagnostics() {
//!     eprintln!("⚠️  {}", diagnostic);
//! }
//! ```
//!
//! ## Notes
//!
//! - The input AST must be **structurally valid** before normalization begins. Invalid or partial trees
//!   may cause internal errors during normalization passes.
//! - Normalization is **order-sensitive**. Certain passes (e.g., [`normalize_type_def`]) assume previous
//!   passes (e.g., [`normalize_typed_list`]) have already run successfully.
//! - Errors during normalization are reported through [`ParserInternalError`] or as diagnostics.
//!
//! ## See Also
//!
//! - [`DiagnosticManager`] — Used throughout to collect and report non-fatal warnings.
//! - [`Ast`] — The structure being normalized.
pub mod normalizer_result;
pub mod normalizer;
pub mod passes;

pub use normalizer::Normalizer;
pub use normalizer_result::NormalizerResult;
