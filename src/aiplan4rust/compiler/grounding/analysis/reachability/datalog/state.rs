//! Datalog Compilation State Module.
//!
//! This module provides the `DatalogState` structure, which aggregates mutable references
//! to rules, caches, auxiliary definitions, databases, and alias tables required during
//! the translation and grounding execution phases of the Datalog pipeline.

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::{
    atom::Atom, database::Database, rule::Rule,
};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::analysis::reachability::datalog::encoder::AliasTable;
use rustc_hash::FxHashMap;

/// Encapsulates mutable state containers and references utilized during Datalog expression encoding.
///
/// `DatalogState` acts as a unified mutable bundle passed across encoder routines to manage
/// rule accumulation, deduplication caches, auxiliary predicate generation, and active alias resolutions.
#[derive(Debug)]
pub(crate) struct DatalogState<'a> {
    pub(crate) rules: &'a mut Vec<Rule>,
    pub(crate) cache: &'a mut FxHashMap<Vec<Atom>, Atom>,
    pub(crate) when_cache: &'a mut FxHashMap<[Atom; 2], Atom>,
    pub(crate) aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
    pub(crate) next_aux_id: &'a mut usize,
    pub(crate) db: &'a mut Database,
    pub(crate) current_aliases: &'a mut AliasTable,
}

impl<'a> DatalogState<'a> {
    /// Creates a new instance of `DatalogState` for the encoding pipeline.
    ///
    /// # Arguments
    ///
    /// * `rules` - A mutable reference to the vector storing generated Datalog rules.
    /// * `cache` - A mutable reference to the general expression caching hash map.
    /// * `when_cache` - A mutable reference to the specialized conditional caching hash map.
    /// * `aux_defs` - A mutable reference to the auxiliary formula skeleton definitions.
    /// * `next_aux_id` - A mutable reference tracking the next available auxiliary identifier.
    /// * `db` - A mutable reference to the underlying Datalog database.
    /// * `current_aliases` - A mutable reference to the active alias resolution table.
    ///
    /// # Returns
    ///
    /// Returns a new, initialized `DatalogState` instance wrapping the provided mutable references.
    pub(crate) fn new(
        rules: &'a mut Vec<Rule>,
        cache: &'a mut FxHashMap<Vec<Atom>, Atom>,
        when_cache: &'a mut FxHashMap<[Atom; 2], Atom>,
        aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
        next_aux_id: &'a mut usize,
        db: &'a mut Database,
        current_aliases: &'a mut AliasTable,
    ) -> Self {
        Self {
            rules,
            cache,
            when_cache,
            aux_defs,
            next_aux_id,
            db,
            current_aliases,
        }
    }
}

/// Tests the creation and field initialization of the Datalog compilation state.
///
/// # Objective
/// Verifies that `DatalogState::new` correctly captures and exposes all mutable references
/// provided to rules, caches, auxiliary structures, databases, and aliases.
///
/// # Inputs
/// - A vector for rules, standard hash maps for general and conditional caching,
///   an auxiliary definition vector, a counter tracker for auxiliary identifiers,
///   an empty database, and a default alias table.
///
/// # Expected Output
/// Returns a properly constructed `DatalogState` whose internal mutable fields point to the supplied instances.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::reachability::datalog::encoder::aliasing;

    #[test]
    fn test_datalog_state_creation() {
        let mut rules = Vec::new();
        let mut cache = FxHashMap::default();
        let mut when_cache = FxHashMap::default();
        let mut aux_defs = Vec::new();
        let mut next_aux_id = 0;
        let mut db = Database::default();
        let mut current_aliases = aliasing::new_alias_table();

        let state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut next_aux_id,
            &mut db,
            &mut current_aliases,
        );

        assert!(state.rules.is_empty());
        assert!(state.cache.is_empty());
        assert!(state.when_cache.is_empty());
        assert!(state.aux_defs.is_empty());
        assert_eq!(*state.next_aux_id, 0);
    }
}
