//! This module provides parsing functions for PDDL and HDDL syntax.
//!
//! It serves as an abstraction layer over the LALRPOP-generated parsers,
//! encapsulating parsing logic, error handling, and integration with
//! the parsing context and diagnostics management.
//!
//! # Overview
//! - Wraps calls to LALRPOP parser instances (`PDDLParser`, `HDDLParser`).
//! - Converts LALRPOP-specific errors into a unified `ParserError` type.
//! - Manages the parse context, including the root node ID.
//! - Simplifies higher-level usage by exposing straightforward parsing functions.
//!
//! # Usage
//! Call `parse_pddl` or `parse_hddl` with a mutable `ParseContext` and a `Lexer`.
//! The functions return a `Result` containing the root AST node ID or a parse error.
//!
//! These functions abstract away the nested `Result` types produced by LALRPOP,
//! providing a cleaner and more ergonomic API for consumers of the parsing library.

use crate::aiplan4rust::syntax::{ParseContext, ParserError};
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::arena::NodeId;
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
/// * `Ok(NodeId)` - The root node ID of the parsed AST on success.
/// * `Err(ParserError)` - An error encountered during parsing.
pub fn parse_pddl(
    ctx: &mut ParseContext,
    lexer: Lexer,
) -> Result<NodeId, ParserError> {
    let parser = PDDLParser::new();

    // Call the LALRPOP parser, converting ParseError into ParserError.
    let inner_result = parser.parse(ctx, lexer)?;

    // Convert ParserInternalError into ParserError and unwrap the root ID.
    let root_id = inner_result?;

    // Save the root node ID into the parse context.
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
/// * `Ok(NodeId)` - The root node ID of the parsed AST on success.
/// * `Err(ParserError)` - An error encountered during parsing.
pub fn parse_hddl(
    ctx: &mut ParseContext,
    lexer: Lexer,
) -> Result<NodeId, ParserError> {
    let parser = HDDLParser::new();

    // Call the LALRPOP parser, converting ParseError into ParserError.
    let inner_result = parser.parse(ctx, lexer)?;

    // Convert ParserInternalError into ParserError and unwrap the root ID.
    let root_id = inner_result?;

    // Save the root node ID into the parse context.
    ctx.set_root_id(root_id)?;

    Ok(root_id)
}
