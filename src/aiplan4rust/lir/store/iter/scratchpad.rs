use crate::aiplan4rust::lir::store::ExprId;
use bit_set::BitSet;
use rustc_hash::FxHashMap;

/// Capacité initiale par défaut pour éviter les premières réallocations.
const DEFAULT_SCRATCHPAD_CAPACITY: usize = 1024;

/// Zone de mémoire temporaire pour les opérations de transformation/simplification.
/// À réutiliser pour éviter les allocations répétées.
pub struct Scratchpad {
    /// Pile de travail pour le parcours non-récursif (DFS).
    /// Le booléen indique si le nœud a été visité (post-order traversal).
    stack: Vec<(ExprId, bool)>,

    /// Cache de transformation : associe l'ancien ExprId au nouveau ExprId simplifié.
    pub cache: FxHashMap<ExprId, ExprId>,

    /// Ensemble des nœuds déjà traités pour éviter les cycles ou les doubles traitements.
    pub visited: BitSet,
}

impl Scratchpad {
    /// Crée un nouveau scratchpad avec une capacité initiale par défaut.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_SCRATCHPAD_CAPACITY)
    }

    /// Crée un scratchpad avec des capacités initiales pour limiter les reallocs.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            stack: Vec::with_capacity(cap),
            cache: FxHashMap::default(),
            visited: BitSet::with_capacity(cap),
        }
    }

    /// Nettoie le scratchpad pour une nouvelle opération sans libérer la mémoire allouée.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.cache.clear();
        self.visited.clear();
    }

    /// On l'initialise avant chaque algorithme
    pub fn init(&mut self, root: ExprId) {
        self.clear();
        self.stack.push((root, false));
    }

    // --- ACCÈS À LA PILE ---

    /// Ajoute un nœud à explorer.
    /// 'processed' = false signifie qu'on descend,
    /// 'processed' = true signifie qu'on remonte (reconstruction).
    #[inline]
    pub fn push(&mut self, id: ExprId, processed: bool) {
        self.stack.push((id, processed));
    }

    /// Récupère le prochain nœud à traiter.
    #[inline]
    pub fn pop(&mut self) -> Option<(ExprId, bool)> {
        self.stack.pop()
    }

    /// Utilitaire pour empiler les enfants d'un nœud en vue d'un parcours DFS.
    /// On les empile à l'envers (.rev()) pour qu'ils soient traités dans l'ordre (0, 1, 2...).
    pub fn push_children(&mut self, children: &[ExprId]) {
        for &child in children.iter().rev() {
            self.push(child, false);
        }
    }

    // --- ACCÈS AU BITSET (VISITED) ---

    #[inline]
    pub fn mark_visited(&mut self, id: ExprId) {
        self.visited.insert(id.as_usize());
    }

    #[inline]
    pub fn is_visited(&self, id: ExprId) -> bool {
        self.visited.contains(id.as_usize())
    }

    // --- ACCÈS AU Cache ---

    /// Récupère une transformation (Optionnel).
    /// Si l'ID n'est pas dans le cache, renvoie None.
    #[inline]
    pub fn get(&self, id: ExprId) -> Option<ExprId> {
        self.cache.get(&id).copied()
    }

    /// Récupère la transformation de force (Panique si absent).
    /// À utiliser pour les enfants lors de la reconstruction.
    #[inline]
    pub fn fetch(&self, id: ExprId) -> ExprId {
        self.cache
            .get(&id)
            .copied()
            .expect("Logic error: attempted to fetch an ID that hasn't been processed yet")
    }

    /// Insère le résultat d'une transformation dans le cache.
    #[inline]
    pub fn insert(&mut self, old_id: ExprId, new_id: ExprId) {
        self.cache.insert(old_id, new_id);
    }
}
