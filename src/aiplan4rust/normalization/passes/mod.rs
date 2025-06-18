//! # AST Normalization Passes
//!
//! This module defines and re-exports several normalization passes that operate
//! on the Abstract Syntax Tree (AST) of the language. Each pass is responsible for
//! cleaning up or transforming specific structural aspects of the AST to ensure
//! consistent, deduplicated, and well-formed representations before further
//! semantic analysis or code generation.
//!
//! ## Available Passes
//!
//! - [`normalize_typed_list`] — Flattens and validates all `TypedList` nodes by ensuring each
//!   `TypedItem` has exactly one element and an optional type.
//! - [`normalize_type_def`] — Merges `TypedItem` nodes within `TypesDef` that share the same
//!   `PrimitiveType` key, deduplicating and combining their types.
//! - [`normalize_require_def`] — Removes duplicate requirement declarations from the `RequireDef` node
//!   and reports them as diagnostics.
//! - [`normalize_either_type`] — Normalizes `Type` nodes that use implicit union (either) types by
//!   sorting and deduplicating type elements.
//!
//! ## Usage
//!
//! These functions are intended to be called after parsing and before type-checking
//! or interpretation. Each pass assumes a valid AST and may depend on earlier
//! passes. For example, [`normalize_type_def`] assumes that [`normalize_typed_list`] has already
//! been applied.
//!
//! ```rust,ignore
//! let mut ast = parse_source_code(source)?;
//!
//! // Apply normalizations in the correct order
//! normalize_typed_list(&mut ast)?;
//! normalize_type_def(&mut ast, &mut diagnostics)?;
//! normalize_require_def(&mut ast, &mut diagnostics)?;
//! normalize_either_type(&mut ast)?;
//! ```
//!
//! ## Error Handling
//!
//! Each normalization function returns a `Result` and may produce
//! `ParserInternalError` if the AST structure deviates from expected patterns.
//!
//! ## Re-exports
//!
//! These passes are available for external use via re-export:
//!
//! - [`normalize_require_def`]
//! - [`normalize_type_def`]
//! - [`normalize_either_type`]
//! - [`normalize_typed_list`]

pub mod require_def;
pub mod types_def;
pub mod either_type;
pub mod typed_list;

pub use require_def::normalize_require_def;
pub use types_def::normalize_type_def;
pub use either_type::normalize_either_type;
pub use typed_list::normalize_typed_list;
