// Dans src/grounding/context.rs (ou dans le mod.rs de ton dossier passes)

use crate::aiplan4rust::grounding::registry::value::ValueRegistry;
use crate::aiplan4rust::grounding::analysis::inertia::registry::InertiaRegistry;
use crate::aiplan4rust::lir::logic::LogicEngine;

pub struct GroundingContext<'a> {
    registry: &'a ValueRegistry,
    inertia: &'a InertiaRegistry<'a>,
    logic: LogicEngine<'a>, // Le moteur est stocké par valeur car c'est une vue légère
}

impl<'a> GroundingContext<'a> {
    /// Crée un nouveau contexte de grounding.
    /// On instancie le LogicEngine ici une seule fois.
    pub fn new(registry: &'a ValueRegistry, inertia: &'a InertiaRegistry) -> Self {
        Self {
            registry,
            inertia,
            logic: LogicEngine::with_evaluator(inertia),
        }
    }

    // --- Accesseurs ---

    pub fn registry(&self) -> &ValueRegistry {
        self.registry
    }

    pub fn inertia(&self) -> &InertiaRegistry {
        self.inertia
    }

    pub fn logic_engine(&self) -> &LogicEngine<'a> {
        &self.logic
    }
}
