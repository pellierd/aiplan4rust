use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::{
    atom::Atom, database::Database, rule::Rule,
};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::analysis::reachability::datalog::encoder::AliasTable;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub(crate) struct DatalogState<'a> {
    pub(crate) rules: &'a mut Vec<Rule>,
    pub(crate) cache: &'a mut FxHashMap<Vec<Atom>, Atom>,
    pub(crate) when_cache: &'a mut FxHashMap<[Atom; 2], Atom>, // 🌟 [Atom; 2] au lieu de [Cause; 2]
    pub(crate) aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
    pub(crate) next_aux_id: &'a mut usize,
    pub(crate) db: &'a mut Database,
    pub(crate) current_aliases: &'a mut AliasTable,
}

impl<'a> DatalogState<'a> {
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
