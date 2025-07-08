//! The `aiplan4rust` crate provides the core components for parsing PDDL and HDDL planning domain languages.
//!
//! # Overview
//! This crate enables the reading, parsing, and structured error reporting of PDDL and HDDL files,
//! commonly used in AI planning. It provides high-level abstractions over the parser pipeline,
//! lexical analysis, abstract syntax tree (AST) representations, and diagnostic rendering.
//!
//! # Features
//! - Support for both PDDL and HDDL syntax
//! - Modular parser architecture with a shared diagnostic framework
//! - Span-based error messages with line/column resolution
//! - AST representations and utilities for syntax display
//!
//! # Example
//! Basic usage for parsing a domain file and rendering diagnostics:
//!
//! ```rust
//! use aiplan4rust::{Parser, Renderer};
//! use std::fs::File;
//! use std::io::Read;
//!
//! fn read_file(path: &str) -> Result<String, std::io::Error> {
//!     let mut file = File::open(path)?;
//!     let mut content = String::new();
//!     file.read_to_string(&mut content)?;
//!     Ok(content)
//! }
//!
//! let source_path = "domain.pddl"; // Replace with your PDDL or HDDL file
//! let content = read_file(source_path)?;
//!
//! let mut parser = Parser::new();
//! let parser_result = parser.parse(source_path, &content, aiplan4rust::Language::Pddl)?;
//!
//! let mut renderer = Renderer::new(parser_result.diagnostic_manager());
//! renderer.display();
//! ```
//!
//! # Architecture
//! The parser architecture is divided into the following stages:
//! 1. **Lexical Analysis** — Handled by the `lexer` module.
//! 2. **Grammar Parsing** — Rules and grammar trees are handled in `parser` and `grammar`.
//! 3. **AST Construction** — AST nodes and types are defined under `ast`.
//! 4. **Span Resolution** — File offsets are translated using `span` and `fast_line_table`.
//! 5. **Diagnostics** — Errors are collected and rendered with severity metadata.
//!
//! # Syntax Display Trait
//! The [`SyntaxDisplay`] trait allows converting AST nodes to language-specific
//! representations (pretty-printed or serialized). This is useful for pretty-printing or emitting code.
//!
//! # Notes
//! - The crate no longer depends internally on `string-interner`; interning logic has been modularized.
//! - `Language` must be explicitly passed when parsing a file to select between PDDL and HDDL.
//!
//! # Modules
//! - [`language`] – Definition of supported planning languages (PDDL/HDDL)
//! - [`lexer`] – Tokenizer for input streams
//! - [`parser`] – Entrypoint to the parsing pipeline
//! - [`parser_result`] – Wrapper for the output of the parsing process
//! - [`grammar`] – Grammar-specific logic
//! - [`ast`] – Abstract syntax tree definitions
//! - [`display`] – Provides the `SyntaxDisplay` trait
//! - [`span`] – Source position tracking with spans
//! - [`fast_line_table`] – Maps file offsets to line/column
//!
//! # Re-exports
//! The following are exposed for convenience:
//!
//! - [`Language`] — Language selector enum
//! - [`Parser`] — High-level parser interface
//! - [`ParserResult`] — Result type returned by the parser
//! - [`Span`] — Span utility for error reporting
//! - [`SyntaxDisplay`] — Trait for AST-to-string formatting
//! - [`FastLineTable`] — Efficient file line tracking

pub mod language;
pub mod lexer;
pub mod parser;
pub mod parser_result;
pub mod grammar;
pub mod span;
pub mod ast;
pub mod display;
pub mod fast_line_table;
pub mod context;
pub mod arena_parser_result;
pub mod lalrpop;
pub mod parser_error;

pub use language::Language;
pub use parser::Parser;
pub use parser_result::ParserResult;
pub use span::Span;
pub use display::PlanningSyntaxDisplay;
pub use fast_line_table::FastLineTable;
pub use context::ParseContext;
pub use arena_parser_result::ArenaParserResult;
pub use parser_error::ParserError;

pub use lalrpop::parse_hddl;
pub use lalrpop::parse_pddl;
