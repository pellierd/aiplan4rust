/// Core components of the Datalog grounding engine.
///
/// This module implements a lightweight, specialized Datalog engine tailored for
/// PDDL reachability analysis and fact saturation. It provides the fundamental
/// logical abstractions (terms, atoms, rules) alongside concrete data structures
/// (tuples, relations, databases) optimized for high-performance unification
/// and join processing.
///
/// # Architecture Overview
///
/// The engine is architected around two layers:
///
/// 1. **The Syntax Layer (`term`, `atom`, `rule`)**: Represents the logical structures
///    and implications of the PDDL domain translation.
/// 2. **The Storage & Evaluation Layer (`tuple`, `relation`, `database`, `cause`)**:
///    Manages the concrete ground facts, relational indexing, and tracking of derivation histories.
pub mod atom;
pub mod cause;
pub mod database;
pub mod relation;
pub mod rule;
pub mod term;
pub mod tuple;

#[doc(inline)]
pub use atom::Atom;

#[doc(inline)]
pub use cause::Cause;

#[doc(inline)]
pub use database::Database;

#[doc(inline)]
pub use relation::Relation;

#[doc(inline)]
pub use rule::Rule;

#[doc(inline)]
pub use term::Term;

#[doc(inline)]
pub use tuple::Tuple;
