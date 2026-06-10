use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use crate::analysis::reachability::datalog::renderers::common;
use crate::analysis::reachability::datalog::tuple::Tuple;

pub fn render(aux_tuple: &Tuple<AtomSkeletonId>, lifted: &LiftedProblem) -> String {
    // 1. On ne peut pas récupérer de "Def" car c'est un prédicat généré.
    // On va donc utiliser une étiquette générique avec son ID de squelette.
    let name = format!("Aux_SK_{}", aux_tuple.symbol().as_usize());

    // 2. Par contre, common::render_args fonctionnera très bien !
    // Il va transformer les IDs d'objets (les arguments du tuple) en noms (valve, bracket, etc.)
    format!("{}{}", name, common::render_args(aux_tuple.args(), lifted))
}
