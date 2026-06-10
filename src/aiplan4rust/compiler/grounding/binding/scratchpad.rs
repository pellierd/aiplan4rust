use crate::aiplan4rust::compiler::lir::expr::ExprId;
use fxhash::FxHashMap;

/// Carnet de notes passif pour les opérations de substitution (binding).
/// Aligné sur le modèle de conception sans allocation à chaud.
pub struct BindingScratchpad {
    /// Buffer interne pour la correspondance des IDs (Ancien ID -> Nouvel ID)
    pub(in crate::aiplan4rust) substitution_map: FxHashMap<ExprId, ExprId>,
    /// Buffer interne pour accumuler les enfants traduits
    pub(in crate::aiplan4rust) children_buffer: Vec<ExprId>,
    /// Pile de travail pour le parcours post-ordre itératif manuel
    pub(in crate::aiplan4rust) stack: Vec<(ExprId, bool)>,
}

impl BindingScratchpad {
    /// Crée un nouveau scratchpad de binding avec des capacités initiales.
    pub fn new() -> Self {
        Self {
            substitution_map: FxHashMap::with_capacity_and_hasher(32, Default::default()),
            children_buffer: Vec::with_capacity(8),
            stack: Vec::with_capacity(32),
        }
    }

    /// Réinitialise les buffers pour réutilisation immédiate sans libérer la mémoire.
    pub fn clear(&mut self) {
        self.substitution_map.clear();
        self.children_buffer.clear();
        self.stack.clear();
    }
}
