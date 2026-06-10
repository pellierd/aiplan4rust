//! Utilities for merging [`SymbolInterner`] instances and managing identifier mappings.
//!
//! This module provides the [`InternerMergeResult`] type_checker, which encapsulates
//! the result of merging two [`SymbolInterner`]s — typically a domain and a problem interner.
//!
//! # Purpose
//!
//! When working with multiple interners representing different scopes (e.g., domain and problem),
//! it is often necessary to combine them into a single unified interner for global consistency.
//! This module supports that by merging strings and providing a mapping from problem identifiers
//! to their corresponding identifiers in the merged global interner.
//!
//! # Main type_checker
//!
//! - [`InternerMergeResult`]: Holds the merged interner and a mapping from problem [`SymbolId`]
//!   to global [`SymbolId`], enabling translation between the two contexts.
//!
//! # Example
//!
//! ```ignore
//! let merged = InternerMergeResult::from_domain_and_problem(&domain_interner, &problem_interner);
//! println!("Merged interner has {} strings", merged.interner().len());
//! if let Some(global_id) = merged.problem_ident_map().get(&problem_id) {
//!     println!("Problem id {:?} maps to global id {:?}", problem_id, global_id);
//! }
//! ```

use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::{LiteralId, SymbolId};

use std::collections::HashMap;
use std::fmt;
use std::mem::take;

/// Result of merging two [`SymbolInterner`] instances.
///
/// This structure contains the merged `interner` which holds the combined strings,
/// and a mapping from identifiers in the original "problem" interner to their corresponding
/// identifiers in the merged global interner.
///
/// This mapping is useful to translate identifiers from the problem context into the
/// unified global context after merging.
///
/// # Fields
///
/// - `interner`: The merged [`SymbolInterner`] containing all strings from both inputs.
/// - `ident_map`: A [`HashMap`] mapping original problem [`SymbolId`] values
///   to their corresponding global [`SymbolId`] in the merged interner.
///
/// # Example
///
/// ```ignore
/// let merged_result = InternerMergeResult::new(merged_interner, problem_to_global_map);
/// println!("Merged interner has {} strings", merged_result.interner().len());
/// if let Some(global_id) = merged_result.ident_map().get(&problem_id) {
///     println!("Problem id {:?} maps to global id {:?}", problem_id, global_id);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct InternerMergeResult {
    interner: SymbolInterner,
    symbol_map: HashMap<SymbolId, SymbolId>,
    literal_map: HashMap<LiteralId, LiteralId>,
}

impl InternerMergeResult {
    /// Creates a new `InternerMergeResult`.
    ///
    /// # Parameters
    ///
    /// - `interner`: The merged [`SymbolInterner`] instance.
    /// - `ident_map`: A mapping from problem identifiers to merged global identifiers.
    ///
    /// # Returns
    ///
    /// A new instance of `InternerMergeResult`.
    pub fn new(
        interner: SymbolInterner,
        ident_map: HashMap<SymbolId, SymbolId>,
        literal_map: HashMap<LiteralId, LiteralId>,
    ) -> Self {
        Self {
            interner,
            symbol_map: ident_map,
            literal_map,
        }
    }

    /// Returns a reference to the merged [`SymbolInterner`].
    ///
    /// This interner contains all strings from both merged interners.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Extracts the merged `StringInterner` by taking it out of `self`,
    /// leaving an empty/debug `StringInterner` in its place.
    ///
    /// Requires `&mut self`.
    pub fn take_interner(&mut self) -> SymbolInterner {
        take(&mut self.interner)
    }

    /// Returns a reference to the mapping from problem [`SymbolId`] to global [`SymbolId`].
    ///
    /// This map is used to translate identifiers from the problem interner
    /// into their equivalent in the merged global interner.
    pub fn symbol_map(&self) -> &HashMap<SymbolId, SymbolId> {
        &self.symbol_map
    }

    /// Takes (extracts) the mapping from problem [`SymbolId`] to global [`SymbolId`],
    /// leaving an empty map in its place.
    ///
    /// This allows consuming the map without cloning it.
    ///
    /// Requires a mutable reference to `self`.
    pub fn take_symbol_map(&mut self) -> HashMap<SymbolId, SymbolId> {
        take(&mut self.symbol_map)
    }

    /// Returns an immutable reference to the mapping from problem [`Literal`]s to global [`Literal`]s.
    ///
    /// This map is produced during the linking phase, where identifiers from the `problem` file
    /// are reconciled against the `domain` definitions. It allows translating local identifiers
    /// (interned in the problem's interner) to their unified global form in the merged interner.
    ///
    /// # Usage
    ///
    /// This is typically used when resolving or rendering identifiers originating from the problem file,
    /// to ensure consistency with the domain-level symbol table and diagnostics.
    ///
    /// # Returns
    ///
    /// A reference to a [`HashMap`] that maps problem-local [`Literal`]s to their global equivalents.
    ///
    /// [`Literal`]: crate::interner::Literal
    pub fn literal_map(&self) -> &HashMap<LiteralId, LiteralId> {
        &self.literal_map
    }

    /// Consumes and returns the mapping from problem [`Literal`]s to global [`Literal`]s,
    /// replacing the internal map with an empty one.
    ///
    /// This is useful when the caller needs ownership of the entire map (e.g., for moving it
    /// into another structure) without incurring a clone.
    ///
    /// # Requirements
    ///
    /// This method requires a mutable reference to `self`, as it modifies internal state
    /// by emptying the original map.
    ///
    /// # Returns
    ///
    /// A [`HashMap`] containing the full identifier mapping from problem to global scope.
    ///
    /// # Example
    ///
    /// ```rust
    /// let map = linker.take_problem_literal_map();
    /// assert!(linker.problem_literal_map().is_empty());
    /// ```
    ///
    /// [`Literal`]: crate::interner::Literal
    pub fn take_literal_map(&mut self) -> HashMap<LiteralId, LiteralId> {
        take(&mut self.literal_map)
    }

    /// Merges a domain and problem [`SymbolInterner`] into a unified [`InternerMergeResult`].
    ///
    /// This function clones the domain interner and extends it with all strings from the
    /// problem interner. It also creates a mapping from each identifier in the problem
    /// to its new identifier in the merged interner.
    ///
    /// # Parameters
    ///
    /// - `domain_interner`: A reference to the domain's [`SymbolInterner`].
    /// - `problem_interner`: A reference to the problem's [`SymbolInterner`].
    ///
    /// # Returns
    ///
    /// A new [`InternerMergeResult`] containing:
    /// - the merged `StringInterner`
    /// - a `HashMap<Ident, Ident>` mapping problem identifiers to merged identifiers
    pub fn from_domain_and_problem(
        domain_interner: &SymbolInterner,
        problem_interner: &SymbolInterner,
    ) -> Self {
        let mut interner = domain_interner.clone();
        let mut problem_ident_map = HashMap::new();
        let mut problem_literal_map = HashMap::new();

        for (old_id, s) in problem_interner.iter_symbol_entries() {
            let new_id = interner.intern_symbol(s.to_string());
            problem_ident_map.insert(old_id, new_id);
        }

        for (old_id, s) in problem_interner.iter_literal_entries() {
            let new_id = interner.intern_literal(s.to_string());
            problem_literal_map.insert(old_id, new_id);
        }

        InternerMergeResult::new(interner, problem_ident_map, problem_literal_map)
    }
}

impl fmt::Display for InternerMergeResult {
    /// Formats the `InternerMergeResult` for display, showing the
    /// merged interner and all identifier mappings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "InternerMergeResult {{")?;
        writeln!(f, "  interner: {}", self.interner)?;
        writeln!(f, "  Ident map: [")?;
        for (problem_id, global_id) in &self.symbol_map {
            writeln!(f, "    {:?} -> {:?}", problem_id, global_id)?;
        }
        writeln!(f, "  ]")?;
        writeln!(f, "  Literal map: [")?;
        for (problem_id, global_id) in &self.literal_map {
            writeln!(f, "    {:?} -> {:?}", problem_id, global_id)?;
        }
        writeln!(f, "  ]")?;
        writeln!(f, "}}")
    }
}
