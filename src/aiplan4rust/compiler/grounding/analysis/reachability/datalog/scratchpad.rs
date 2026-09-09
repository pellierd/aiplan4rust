use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use ahash::HashSetExt;
use rustc_hash::FxHashSet;

/// Structure de stockage temporaire pour éviter les allocations au sein de l'encodeur Datalog.
pub struct DatalogScratchpad {
    /// Buffer pour le suivi des nœuds visités lors des parcours de graphes d'expressions.
    pub(crate) visited: Vec<bool>,
    /// Pile de travail générique pour les parcours de type DFS (non-récursifs).
    pub(crate) stack: Vec<ExprId>,
    /// Ensemble de suivi des paires (expression, cause) pour éviter les allocations dans encode_effects.
    pub(crate) visited_effects: FxHashSet<(ExprId, AtomSkeletonId)>,
}

impl DatalogScratchpad {
    /// Crée un scratchpad avec une capacité initiale estimée pour limiter les réallocations futures.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            visited: Vec::with_capacity(capacity),
            stack: Vec::with_capacity(64), // Une profondeur d'arbre dépasse rarement 64
            visited_effects: FxHashSet::with_capacity(64),
        }
    }

    /// Prépare le buffer `visited` pour un store spécifique et le remet à zéro.
    #[inline]
    pub fn prepare_visited(&mut self, size: usize) {
        self.visited.resize(size, false);
        self.visited.fill(false);
    }

    /// Nettoie la pile de travail pour un nouveau parcours.
    #[inline]
    pub fn prepare_stack(&mut self, root: ExprId) {
        self.stack.clear();
        self.stack.push(root);
    }

    /// Nettoie l'ensemble des effets visités pour un nouveau parcours d'effets.
    #[inline]
    pub fn prepare_effects_visited(&mut self) {
        self.visited_effects.clear();
    }
}
