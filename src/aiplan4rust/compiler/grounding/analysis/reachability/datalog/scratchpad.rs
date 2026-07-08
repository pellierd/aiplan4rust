use crate::aiplan4rust::compiler::lir::expr::ExprId;

/// Structure de stockage temporaire pour éviter les allocations au sein de l'encodeur Datalog.
pub struct DatalogScratchpad {
    /// Buffer pour le suivi des nœuds visités lors des parcours de graphes d'expressions.
    pub(crate) visited: Vec<bool>,
    /// Pile de travail générique pour les parcours de type DFS (non-récursifs).
    pub(crate) stack: Vec<ExprId>,
}

impl DatalogScratchpad {
    /// Crée un scratchpad avec une capacité initiale estimée pour limiter les réallocations futures.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            visited: Vec::with_capacity(capacity),
            stack: Vec::with_capacity(64), // Une profondeur d'arbre dépasse rarement 64
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
}
