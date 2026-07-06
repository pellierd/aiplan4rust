use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Objet contexte pour le rendu, contenant uniquement les données nécessaires
/// pour traduire les IDs bruts en symboles lisibles.
pub struct DatalogRenderContext<'a> {
    pub problem: &'a LiftedProblem,
    pub type_to_skeleton: &'a [AtomSkeletonId],
    pub fluence_threshold: usize,
    pub action_base_id: usize,
    pub action_threshold: usize,
}

impl<'a> DatalogRenderContext<'a> {
    /// Crée un contexte à partir des accesseurs publics de l'Engine.
    pub fn new(
        problem: &'a LiftedProblem,
        type_to_skeleton: &'a [AtomSkeletonId],
        fluence_threshold: usize,
        action_base_id: usize,
        action_threshold: usize,
    ) -> Self {
        Self {
            problem,
            type_to_skeleton,
            fluence_threshold,
            action_base_id,
            action_threshold,
        }
    }

    /// Identifie la catégorie PDDL et résout le nom du symbole pour un ID donné.
    pub fn identify_segment(&self, sk_id: AtomSkeletonId) -> (&'static str, String) {
        let raw_id = sk_id.as_usize();
        let interner = self.problem.interner();

        // 1. Détection des TYPES
        if let Some(type_idx) = self.type_to_skeleton.iter().position(|&s| s == sk_id) {
            let name = self
                .problem
                .type_symbols()
                .get_ident(crate::aiplan4rust::support::lang::ids::TypeId::from(
                    type_idx,
                ))
                .and_then(|&sym| interner.resolve_symbol(sym))
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if type_idx == self.type_to_skeleton.len() - 1 {
                        "ROOT_SENTINEL".into()
                    } else {
                        format!("TYPE_{}", type_idx)
                    }
                });
            return ("TYPE", name);
        }

        // 2. Détection des FLUENTS (Prédicats PDDL)
        if raw_id < self.fluence_threshold {
            let name = self
                .problem
                .predicate_symbols()
                .get_ident(crate::aiplan4rust::support::lang::ids::PredicateSymbolId::from(raw_id))
                .and_then(|&sym| interner.resolve_symbol(sym))
                .unwrap_or("Unknown_Predicate");
            return ("FLUENT", name.to_string());
        }

        // 3. Détection des ACTIONS
        if raw_id >= self.action_base_id && raw_id < self.action_threshold {
            let act_idx = raw_id - self.action_base_id;
            let name = self
                .problem
                .action_symbols()
                .get_ident(crate::aiplan4rust::support::lang::ids::ActionSymbolId::from(act_idx))
                .and_then(|&sym| interner.resolve_symbol(sym))
                .unwrap_or("Unknown_Action");
            return ("ACTION", name.to_string());
        }

        // 4. Par défaut : Inférences Datalog / Auxiliaires
        ("AUX", format!("Inference_{}", raw_id))
    }
}
