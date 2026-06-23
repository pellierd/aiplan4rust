//! Positive Normal Form (PNF) Transformation Pipeline.
//!
//! This module provides the infrastructure required to lower complex logical
//! expressions into their canonical **Positive Normal Form** (also known as Negation
//! Normal Form / NNF) across an entire planning problem.
//!
//! # Objective
//! The primary goal of this pipeline is to completely eliminate structural `Not`
//! operators by pushing negations down to the leaf nodes and absorbing them directly
//! into atomic formulas. This normalization heavily simplifies downstream tasks, such
//! as dialogue translation, grounding, and constraint satisfaction.
//!
//! # Organization
//!
//! * [`expr`]: Core iterative logic for walking and rewriting generic expression nodes into positive form.
//! * [`action`]: Sub-pipeline specializing in actions (handling preconditions vs. delete effects).
//! * [`method`]: Sub-pipeline tracking HTN methods and their explicit task network constraints.
//! * [`derived_predicate`]: Sub-pipeline optimizing pure evaluation conditions within derived bodies.
//! * [`problem`]: Orchestrating master module managing problem states and ownership demultiplexing.
//! * [`scratchpad`]: Memory recycling infrastructure providing [`PnfScratchpad`](scratchpad::PnfScratchpad).

mod action;
mod derived_predicate;
mod expr;
mod method;
pub(crate) mod problem;
mod scratchpad;
