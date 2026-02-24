//! This module provides parsing functions for PDDL and HDDL syntax.
//!
//! It serves as an abstraction layer over the LALRPOP-generated parsers,
//! encapsulating parsing ops, error handling, and integration with
//! the parsing context and diagnostics management.
//!
//! # Overview
//! - Wraps calls to LALRPOP parser instances (`PDDLParser`, `HDDLParser`).
//! - Converts LALRPOP-specific errors into a unified `ParserError` type_checker.
//! - Manages the parse context, including the root syntax ID.
//! - Simplifies higher-level usage by exposing straightforward parsing functions.
//!
//! # Usage
//! Call `parse_pddl` or `parse_hddl` with a mutable `ParseContext` and a `Lexer`.
//! The functions return a `Result` containing the root AST syntax ID or a parse error.
//!
//! These functions abstract away the nested `Result` types produced by LALRPOP,
//! providing a cleaner and more ergonomic API for consumers of the parsing library.

use crate::aiplan4rust::syntax::ParseContext;
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::syntax::error::SyntaxError;
use crate::aiplan4rust::syntax::grammar::{PDDLParser, HDDLParser};

/// Parses a PDDL source using the LALRPOP-generated PDDL parser.
///
/// This function wraps the parser call, handling conversion of nested
/// `Result` types and integrating with the `ParseContext`.
///
/// # Arguments
/// * `ctx` - Mutable reference to the parsing context.
/// * `lexer` - The lexer providing tokenized input.
///
/// # Returns
/// * `Ok(NodeId)` - The root syntax ID of the parsed AST on success.
/// * `Err(ParserError)` - An error encountered during parsing.
pub fn parse_pddl(
    ctx: &mut ParseContext,
    lexer: Lexer,
) -> Result<NodeId, SyntaxError> {
    let parser = PDDLParser::new();

    let inner_result = parser
        .parse(ctx, lexer)
        .map_err(SyntaxError::ParseError)?;

    let root_id = inner_result?;

    ctx.set_root_id(root_id)?;

    Ok(root_id)
}

/// Parses an HDDL source using the LALRPOP-generated HDDL parser.
///
/// This function wraps the parser call, handling conversion of nested
/// `Result` types and integrating with the `ParseContext`.
///
/// # Arguments
/// * `ctx` - Mutable reference to the parsing context.
/// * `lexer` - The lexer providing tokenized input.
///
/// # Returns
/// * `Ok(NodeId)` - The root syntax ID of the parsed AST on success.
/// * `Err(ParserError)` - An error encountered during parsing.
pub fn parse_hddl(
    ctx: &mut ParseContext,
    lexer: Lexer,
) -> Result<NodeId, SyntaxError> {
    let parser = HDDLParser::new();

    let inner_result: Result<NodeId, _> = parser.parse(ctx, lexer)
        .map_err(SyntaxError::ParseError)?;

    let root_id = inner_result?;

    ctx.set_root_id(root_id)?;

    Ok(root_id)
}
