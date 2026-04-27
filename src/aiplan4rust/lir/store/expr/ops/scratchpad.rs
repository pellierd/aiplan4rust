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
    visited: BitSet,

    // --- Buffers de factorisation (Logique pure) ---
    group_buffer: Vec<Vec<ExprId>>,
    other_kids: Vec<ExprId>,

    // --- Buffer de statistiques (Pour ne pas réécrire plus tard) ---
    freq_map: FxHashMap<ExprId, u32>,

    temporal_cache: FxHashMap<usize, (ExprId, ExprId, ExprId)>,

    /// Buffers pour la reconstruction des connecteurs (And/Or)
    /// pour éviter de réallouer des Vec à chaque nœud parent.
    starts_buffer: Vec<ExprId>,
    ends_buffer: Vec<ExprId>,
    overalls_buffer: Vec<ExprId>,

    /// Buffer pour extraire les enfants du store sans allocation.
    children_buffer: Vec<ExprId>,
    build_buffer: Vec<ExprId>,
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

            // --- Buffers de factorisation (FNF) ---
            group_buffer: Vec::with_capacity(16),
            other_kids: Vec::with_capacity(16),
            freq_map: FxHashMap::default(),

            // --- Nouveaux champs pour la TNF ---
            // Le cache utilise le PackedId (usize)
            temporal_cache: FxHashMap::default(),

            // Buffers dédiés à la reconstruction des triplets
            starts_buffer: Vec::with_capacity(64),
            ends_buffer: Vec::with_capacity(64),
            overalls_buffer: Vec::with_capacity(64),

            // NNF
            children_buffer: Vec::with_capacity(64),
            build_buffer: Vec::with_capacity(64),
        }
    }

    /// Nettoie le scratchpad pour une nouvelle opération sans libérer la mémoire allouée.
    /// Nettoie TOUS les buffers et caches du scratchpad.
    /// Indispensable avant de commencer une nouvelle transformation.
    pub fn clear(&mut self) {
        // Bases
        self.stack.clear();
        self.cache.clear();
        self.visited.clear();

        // FNF
        self.group_buffer.clear();
        self.other_kids.clear();
        self.freq_map.clear();

        // TNF
        self.temporal_cache.clear();
        self.starts_buffer.clear();
        self.ends_buffer.clear();
        self.overalls_buffer.clear();

        // NNF
        self.children_buffer.clear();
        self.build_buffer.clear();
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

    // --- Gestion du Cache TNF ---

    /// Enregistre la décomposition temporelle (triplet) pour un identifiant donné.
    #[inline]
    pub fn save_temporal_decomposition(
        &mut self,
        packed_id: usize,
        triplet: (ExprId, ExprId, ExprId),
    ) {
        self.temporal_cache.insert(packed_id, triplet);
    }

    /// Récupère la décomposition temporelle déjà calculée.
    #[inline]
    pub fn get_temporal_decomposition(&self, packed_id: usize) -> (ExprId, ExprId, ExprId) {
        *self
            .temporal_cache
            .get(&packed_id)
            .expect("Logic error: temporal components missing")
    }

    // --- Collecte des spécificateurs ---

    /// Prépare les accumulateurs pour une nouvelle agrégation de spécificateurs temporels.
    #[inline]
    pub fn clear_time_specifier_buffers(&mut self) {
        self.starts_buffer.clear();
        self.ends_buffer.clear();
        self.overalls_buffer.clear();
    }

    /// Ventile les composants d'une expression vers les spécificateurs temporels correspondants.
    #[inline]
    pub fn push_time_specifier(&mut self, s: ExprId, e: ExprId, o: ExprId, empty: ExprId) {
        if s != empty {
            self.starts_buffer.push(s);
        }
        if e != empty {
            self.ends_buffer.push(e);
        }
        if o != empty {
            self.overalls_buffer.push(o);
        }
    }

    // Accesseurs pour la phase de reconstruction
    #[inline]
    pub fn at_start_buffer(&self) -> &[ExprId] {
        &self.starts_buffer
    }
    #[inline]
    pub fn at_end_buffer(&self) -> &[ExprId] {
        &self.ends_buffer
    }
    #[inline]
    pub fn overall_buffer(&self) -> &[ExprId] {
        &self.overalls_buffer
    }

    // --- Accesseurs pour FNF (Factorisation) ---

    #[inline]
    pub fn group_buffer(&self) -> &[Vec<ExprId>] {
        &self.group_buffer
    }

    #[inline]
    pub fn group_buffer_mut(&mut self) -> &mut Vec<Vec<ExprId>> {
        &mut self.group_buffer
    }

    #[inline]
    pub fn other_kids_mut(&mut self) -> &mut Vec<ExprId> {
        &mut self.other_kids
    }

    /// Centralise le comptage des fréquences pour la factorisation.
    pub fn compute_frequencies(&mut self) {
        self.freq_map.clear();
        for group in &self.group_buffer {
            for &id in group {
                *self.freq_map.entry(id).or_insert(0) += 1;
            }
        }
    }

    /// Recherche le meilleur facteur commun.
    pub fn find_best_factor(&self) -> Option<ExprId> {
        self.freq_map
            .iter()
            .filter(|&(_, &count)| count >= 2)
            .max_by_key(|&(&id, count)| (count, id))
            .map(|(&id, _)| id)
    }

    #[inline]
    pub fn children_buffer(&self) -> &[ExprId] {
        self.children_buffer.as_slice()
    }
    #[inline]
    pub fn build_buffer(&self) -> &[ExprId] {
        self.build_buffer.as_slice()
    }

    #[inline]
    pub fn children_buffer_mut(&mut self) -> &mut Vec<ExprId> {
        &mut self.children_buffer
    }

    #[inline]
    pub fn build_buffer_mut(&mut self) -> &mut Vec<ExprId> {
        &mut self.build_buffer
    }

    /// Prépare un segment d'enfants dans le buffer et retourne ses indices.
    #[inline(always)]
    pub fn prepare_children_segment(&mut self, children: &[ExprId]) -> (usize, usize) {
        let start = self.children_buffer().len();
        self.children_buffer_mut().extend_from_slice(children);
        let end = self.children_buffer().len();
        (start, end)
    }

    /// Récupère les indices du dernier segment de `count` éléments.
    #[inline(always)]
    pub(crate) fn last_segment_indices(&self, count: usize) -> (usize, usize) {
        let end = self.children_buffer().len();
        (end - count, end)
    }
}
