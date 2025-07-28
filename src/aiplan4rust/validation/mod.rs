//! Core validation module for AST correctness and normalization.
//!
//! This module groups fundamental validation functionalities for abstract syntax trees (AST),
//! ensuring both structural correctness and well-normalized form of AST nodes.
//!
//! It provides submodules for different aspects of validation:
//!
//! - [`syntax`]: Validates structural and syntactic correctness of AST nodes, such as child counts,
//!   node kinds, and content validity.
//! - [`normalization`]: Checks that AST nodes conform to normalization rules,
//!   ensuring semantic consistency and canonical form.
//!
//! These modules and their functions can be used to enforce robust validation
//! pipelines during AST construction and transformation.
pub mod core;
pub mod syntax;
pub mod normalization;
