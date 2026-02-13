//! # AST Normalization Passes
//!
//! This module defines and re-exports several expr passes that operate
//! on the Abstract Syntax Tree (AST) of the language. Each pass is responsible for
//! transforming or cleaning up specific structural aspects of the AST to ensure
//! consistency, deduplication, and well-formedness before further
//! semantic analysis or code generation.
//!
//! ## Available Passes
//!
//! - [`normalize_typed_list`] — Flattens and validates all `TypedList` nodes by ensuring each
//!   `TypedItem` has exactly one element and, optionally, a `type_checker`.
//! - [`normalize_type_def`] — Merges `TypedItem` nodes within `TypesDef` that share the same
//!   `PrimitiveType`, deduplicating and combining their associated types.
//! - [`normalize_require_def`] — Eliminates duplicate requirement declarations from the `RequireDef`
//!   syntax node and reports them via diagnostics.
//! - [`normalize_either_type`] — Normalizes `Type` nodes using implicit union (either) types by
//!   sorting and deduplicating their `type_checker` components.
//!
//! ## Usage
//!
//! These expr functions are intended to be invoked after parsing
//! and before type checking or interpretation. Each pass expects a valid AST
//! and may rely on earlier expr passes.
//!
//! For example, [`normalize_type_def`] assumes that [`normalize_typed_list`] has already run.
//!
//! ```rust,ignore
//! let mut ast = parse_source_code(source)?;
//!
//! // Apply expr passes in the correct order
//! normalize_typed_list(&mut ast)?;
//! normalize_type_def(&mut ast, &mut diagnostics)?;
//! normalize_require_def(&mut ast, &mut diagnostics)?;
//! normalize_either_type(&mut ast)?;
//! ```
//!
//! ## Error Handling
//!
//! Each expr function returns a `Result` and may produce a
//! [`NormalizationPassError`] if the AST structure deviates from the expected shape.
//!
//! ## Re-exports
//!
//! These expr passes are publicly re-exported for use in other modules:
//!
//! - [`normalize_typed_list`]
//! - [`normalize_type_def`]
//! - [`normalize_require_def`]
//! - [`normalize_either_type`]
//! - [`NormalizationPassError`]


pub mod require_def;
pub mod types_def;
pub mod either_type;
pub mod typed_list;
pub mod error;

pub use require_def::normalize_require_def;
pub use types_def::normalize_type_def;
pub use either_type::normalize_either_type;
pub use typed_list::normalize_typed_list;
pub use error::NormalizationPassError;
