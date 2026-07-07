use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::{
    atom::Atom, database::Database, rule::Rule,
};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::support::lang::VariableId;
// 🌟 Ajout de l'import pour les variables
use crate::analysis::reachability::datalog::core::Term;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct DatalogState<'a> {
    pub(crate) rules: &'a mut Vec<Rule>,
    pub(crate) cache: &'a mut HashMap<Vec<Atom>, Atom>,
    pub(crate) aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
    pub(crate) next_aux_id: &'a mut usize,
    pub(crate) db: &'a mut Database,
    pub(crate) current_aliases: &'a mut HashMap<VariableId, Term>, // 🌟 Ajout de la table d'alias
}

impl<'a> DatalogState<'a> {
    /// Crée une nouvelle instance de `DatalogState` en regroupant les buffers d'accumulation.
    pub(crate) fn new(
        rules: &'a mut Vec<Rule>,
        cache: &'a mut HashMap<Vec<Atom>, Atom>,
        aux_defs: &'a mut Vec<AtomicFormulaSkeleton>,
        next_aux_id: &'a mut usize,
        db: &'a mut Database,
        current_aliases: &'a mut HashMap<VariableId, Term>, // 🌟 Ajouté au constructeur
    ) -> Self {
        Self {
            rules,
            cache,
            aux_defs,
            next_aux_id,
            db,
            current_aliases,
        }
    }
}
