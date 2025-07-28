//! Module hierarchy for syntax tree support in `aiplan4rust`.
//!
//! This module provides components and utilities to work with
//! abstract syntax trees (ASTs) for PDDL/HDDL parsing and analysis.
//!
//! # Submodules
//!
//! - [`tree`]: Core tree data structures and traversal utilities.
//! - [`default`]: Default implementations and helpers for AST components.
//! - [`syntax`]: Syntax-specific definitions, nodes, kinds, and rendering.

pub mod tree;
pub mod default;
pub mod syntax;
