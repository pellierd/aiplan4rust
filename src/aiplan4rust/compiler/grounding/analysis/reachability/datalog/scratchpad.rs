use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use ahash::HashSetExt;
use rustc_hash::FxHashSet;

/// Structure de stockage temporaire pour éviter les allocations au sein de l'encodeur Datalog.
pub struct DatalogScratchpad {
    /// Buffer pour le suivi des nœuds visités lors des parcours de graphes d'expressions.
    pub(crate) visited: Vec<bool>,
    /// Pile générique pour les parcours simples (ex: aliasing).
    pub(crate) stack: Vec<ExprId>,
    /// Pile pour le parcours des conditions (état post-ordre : id + visited).
    pub(crate) condition_stack: Vec<(ExprId, bool)>,
    /// Pile pour le parcours des effets (id + cause).
    pub(crate) effect_stack: Vec<(ExprId, Atom)>,
    /// Ensemble de suivi des paires (expression, cause) pour éviter les allocations dans encode_effects.
    pub(crate) visited_effects: FxHashSet<(ExprId, AtomSkeletonId)>,
}

impl DatalogScratchpad {
    /// Crée un scratchpad avec une capacité initiale estimée pour limiter les réallocations futures.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            visited: Vec::with_capacity(capacity),
            stack: Vec::with_capacity(64),
            condition_stack: Vec::with_capacity(64),
            effect_stack: Vec::with_capacity(64),
            visited_effects: FxHashSet::with_capacity(64),
        }
    }

    /// Prépare le buffer `visited` pour un store spécifique et le remet à zéro.
    #[inline]
    pub fn prepare_visited(&mut self, size: usize) {
        self.visited.resize(size, false);
        self.visited.fill(false);
    }

    /// Nettoie la pile générique pour un nouveau parcours.
    #[inline]
    pub fn prepare_stack(&mut self, root: ExprId) {
        self.stack.clear();
        self.stack.push(root);
    }

    /// Nettoie la pile de conditions pour un nouveau parcours.
    #[inline]
    pub fn prepare_condition_stack(&mut self, root: ExprId) {
        self.condition_stack.clear();
        self.condition_stack.push((root, false));
    }

    /// Nettoie la pile d'effets pour un nouveau parcours.
    #[inline]
    pub fn prepare_effect_stack(&mut self, root: ExprId, root_cause: Atom) {
        self.effect_stack.clear();
        self.effect_stack.push((root, root_cause));
    }

    /// Nettoie l'ensemble des effets visités pour un nouveau parcours d'effets.
    #[inline]
    pub fn prepare_effects_visited(&mut self) {
        self.visited_effects.clear();
    }
}
