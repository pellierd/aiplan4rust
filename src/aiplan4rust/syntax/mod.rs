//! The `aiplan4rust` crate provides tree components for parsing PDDL and HDDL syntax domain languages.
//!
//! # Overview
//! This crate enables reading, parsing, and structured error reporting of PDDL and HDDL files,
//! which are commonly used in AI syntax systems. It offers high-level abstractions over the
//! parser pipeline, lexical analysis, abstract syntax tree (AST) arena representations, and diagnostic rendering.
//!
//! # Features
//! - Support for both PDDL and HDDL syntax
//! - Modular parser architecture with a shared diagnostic framework
//! - Span-based error messages with line and column resolution
//! - AST representations and utilities for syntax display and formatting
//!
//! # Example
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
//! let source_path = "domain.pddl"; // Replace with your PDDL or HDDL file path
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
//! The parsing process is divided into these stages:
//! 1. **Lexical Analysis** — Implemented in the [`lexer`] module.
//! 2. **Grammar Parsing** — Managed by [`parser`] and [`grammar`] modules.
//! 3. **AST Construction** — Defined under the [`ast`] module.
//! 4. **Span Resolution** — File offsets are mapped to line/column using [`span`] and [`fast_line_table`].
//! 5. **Diagnostics** — Error reporting with severity metadata and spans.
//!
//! # Syntax Display Trait
//! The [`SyntaxInternerDisplay`] trait facilitates conversion of AST nodes into language-specific
//! representations for pretty-printing or serialization purposes.
//!
//! # Notes
//! - The crate no longer depends internally on `string-interner`; interning is modularized.
//! - The [`Language`] enum must be explicitly specified when parsing to distinguish between PDDL and HDDL.
//!
//! # Modules
//! - [`language`] — Definitions of supported syntax languages (PDDL/HDDL)
//! - [`lexer`] — Tokenizer for input streams
//! - [`parser`] — Entrypoint to the parsing pipeline
//! - [`parser_result`] — Output wrapper from the parsing process
//! - [`grammar`] — Grammar-specific parsing logic
//! - [`ast`] — Abstract syntax tree arena definitions
//! - [`display`] — Provides the [`SyntaxInternerDisplay`] trait and formatting utilities
//! - [`span`] — Source position tracking using spans
//! - [`fast_line_table`] — Efficient mapping from file offsets to line/column numbers
//!
//! # Re-exports
//! For convenience, these types and traits are publicly re-exported:
//! - [`Language`] — Language selector enum
//! - [`Parser`] — Main parser interface
//! - [`ParserResult`] — Parser output type_checker
//! - [`Span`] — Source span utility
//! - [`SyntaxInternerDisplay`] — Trait for AST formatting
//! - [`FastLineTable`] — File line tracking utility

pub mod lexer;
pub mod parser;
pub mod grammar;
pub mod span;
pub mod ast;
pub mod display;
pub(crate) mod fast_line_table;
pub mod parser_result;
pub mod lalrpop;
pub mod error;
pub mod tree;
pub mod context;

pub use ast::Ast;
pub use context::ParseContext;
pub use context::ParseContextError;
pub use display::write_indent;
pub use display::SyntaxInternerDisplay;
pub use display::SyntaxDisplay;
pub use error::SyntaxError;
pub use fast_line_table::FastLineTable;
pub use parser::Parser;
pub use parser_result::ParserResult;
pub use span::Span;


pub use lalrpop::parse_hddl;
pub use lalrpop::parse_pddl;
