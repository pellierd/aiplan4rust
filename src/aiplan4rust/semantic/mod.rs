//! Semantic analysis module for AIPlan4Rust.
//!
//! This module provides the common functionality and types for performing semantic analysis,
//! including AST validation, symbol table construction, typing checking, and context management.
//!
//! It is organized into several submodules:
//!
//! - [`analyzer`]: Contains the main semantic analyzer ops.
//! - [`symbol`]: Defines symbol representations and utilities.
//! - [`analyzer_result`]: Structures for representing analysis results.
//! - [`symbol_table`]: Symbol table construction and management.
//! - [`checks`]: Semantic checking rules and validations.
//! - [`context`]: Lightweight context wrappers for semantic verification.
//! - [`error`]: Semantic error types covering various failure scenarios.
//! - [`type_checker`]: Type checking functionality (internal).
//!
//! This module re-exports the key types and traits to provide a unified API:
//! - [`AnalyzerResult`]: Result of the semantic analysis process.
//! - [`Analyzer`]: The semantic analyzer struct.
//! - [`SymbolTable`]: Symbol table typing.
//! - [`TypeChecker`]: Type checking utility.
//! - [`SemanticError`], [`UnexpectedNodeKindError`], [`InvalidNodeArityError`]: Error types.
//! - [`SemanticContext`]: Context wrapper for semantic verification.
//!
//! # Examples
//! ```rust
//! use aiplan4rust::semantic::{Analyzer, SemanticContext};
//!
//! // Create and run a semantic analyzer...
//! ```
//!
//! [`analyzer`]: analyzer
//! [`symbol`]: symbol
//! [`analyzer_result`]: result
//! [`symbol_table`]: symbol_table
//! [`checks`]: checks
//! [`context`]: context
//! [`error`]: error
//! [`type_checker`]: type_checker
//! [`AnalyzerResult`]: result::Result
//! [`Analyzer`]: analyzer::Analyzer
//! [`SymbolTable`]: symbol_table::SymbolTable
//! [`TypeChecker`]: type_checker::type_checker::TypeChecker
//! [`SemanticError`]: error::SemanticError
//! [`UnexpectedNodeKindError`]: error::UnexpectedNodeKindError
//! [`InvalidNodeArityError`]: error::InvalidNodeArityError
//! [`SemanticContext`]: context::Context

/// Semantic analysis submodule managing the common analysis ops.
pub mod analyzer;

/// Module defining symbol representations and helper utilities.
pub mod symbol;

/// Structures representing the results of semantic analysis.
pub mod result;

/// Symbol table construction and query management.
pub mod symbol_table;

/// Semantic checking rules and validations for AST nodes.
pub mod checks;

/// Context wrappers used for passing semantic data in verification functions.
pub mod context;

/// Defines semantic error types used throughout the semantic analysis pipeline.
pub mod error;

pub mod finalization;
pub mod passes;
mod requirements;
pub mod rules;
pub mod signature_matcher;
/// Type checking module (internal, not publicly exposed).
pub mod type_checker;

pub use analyzer::Analyzer;
pub use context::Context as SemanticContext;
pub use error::SemanticError;
pub use result::Result as AnalyzerResult;
pub use symbol_table::SymbolTable;
pub use type_checker::type_checker::TypeChecker;
