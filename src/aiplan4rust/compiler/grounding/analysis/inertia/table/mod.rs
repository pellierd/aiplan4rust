//! Storage Infrastructure for Rigid Predicate Counts and Functional Invariants.
//!
//! This module orchestrates the memory layout, lifecycle, and query interfaces for
//! compile-time planning invariants. It exposes the concrete lookup tables populated
//! during the initial state analysis pass, separating the raw construction phase from
//! the immutable, read-heavy grounding evaluation phase.
//!
//! # Module Architecture
//!
//! The module is decoupled into three specialized sub-components:
//!
//! * [`table`] — Defines [`InertiaTable`], the core storage registry optimized for high-performance
//!   $\mathcal{O}(1)$ multi-level lookups, supporting short-circuited cache-local evaluations.
//! * [`builder`] — Implements the multi-pass compilation state machine used to ingest initial facts,
//!   generate combinatorial bitmasks, enforce `max_proj` bounds, and resolve functional value consensus.
//! * [`error`] — Contains [`InertiaTableError`], encapsulating edge cases such as map collision,
//!   structural domain corruption, or bounds overflow during tracking ingestion.
//!
//! # Data Flow & Lifecycles
//!
//! To eliminate lock contention and optimize CPU cache access, the memory architecture follows
//! a strict write-once, read-many lifecycle:
//!
//! 1. **Ingestion Phase ([`builder`])**: Gathers raw PDDL `:init` formulas, computes Big-Endian projection
//!    masks, asserts uniqueness bounds, and structures sparse matrix entries.
//! 2. **Consensus & Freeze**: Validates that overlapping functional mappings match existing records.
//!    Contradictory data causes eviction or triggers an [`InertiaTableError`].
//! 3. **Query Hot Path ([`InertiaTable`])**: The builder freezes into an immutable `InertiaTable`.
//!    Downstream evaluators query these tables concurrently via zero-allocation pointer slices.

pub mod table;

pub mod builder;
pub mod error;

pub use error::InertiaTableError;
pub use table::InertiaTable;
