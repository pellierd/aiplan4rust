//! Intermediate Representation (IR) Module
//!
//! This module provides the types, utilities, and abstractions for working with
//! **Intermediate Representations (IR)** of planning artifacts within the AI planning pipeline.
//!
//! IR files represent parsed or transformed versions of planning domains and problems,
//! facilitating downstream reasoning, linking, and serialization.
//!
//! # Concepts
//!
//! - **IRKind**: Semantic classification of an IR artifact, distinguishing between
//!   parsed domains, parsed problems, and lifted problems.
//! - **Header**: Metadata associated with serialized IR files, including magic number,
//!   format, IR kind, version, and generation timestamp. Ensures file validity and
//!   compatibility across versions and tools.
//! - **IRContent**: The actual content of an IR file, such as parsed domains, parsed problems,
//!   or lifted problems.
//!
//! # Workflow
//!
//! 1. **Parsing**: Convert raw PDDL/HDDL files into `IRContent` of either_type `ParsedDomain` or `ParsedProblem`.
//! 2. **Lifting**: Transform parsed problems into `LiftedProblem` for lifted reasoning.
//! 3. **Serialization**: Store IR artifacts with a `Header` to ensure correctness and interoperability.
//! 4. **Deserialization**: Read IR files back into `IRContent`, validating the `Header` and `IRKind`.
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::ir::{IRKind, Header};
//! use aiplan4rust::serialization::serde::SerdeFormat;
//!
//! // Create a header for a parsed domain IR file
//! let header = Header::new(SerdeFormat::Json, 1, IRKind::ParsedDomain);
//!
//! println!("Header info: {}", header);
//!
//! // Access IR kind
//! let kind = header.ir_kind();
//! match kind {
//!     IRKind::ParsedDomain => println!("Parsed domain"),
//!     IRKind::ParsedProblem => println!("Parsed problem"),
//!     IRKind::LiftedProblem => println!("Lifted problem"),
//! }
//! ```
//!
//! # Notes
//!
//! - IR files enable decoupling of parsing from problem solving, improving
//!   modularity and performance.
//! - The module ensures safety by validating magic numbers, formats, and IR kinds
//!   during deserialization.
//! - Designed to integrate seamlessly with `Source` abstractions for raw and unknown inputs.
pub(super) mod kind;
pub(super) mod content;
pub(super) mod header;
