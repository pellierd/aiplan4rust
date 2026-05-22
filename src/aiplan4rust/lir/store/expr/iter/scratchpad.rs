use crate::aiplan4rust::lir::store::expr::ExprId;
use bit_set::BitSet;
use rustc_hash::FxHashMap;
use std::ops::Range;

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
    /// Unique buffer continu contenant tous les littéraux de tous les groupes AND mis bout à bout.
    flat_groups: Vec<ExprId>,
    /// Les délimitations (début..fin) de chaque groupe AND au sein du `flat_groups`.
    group_boundaries: Vec<Range<usize>>,
    /// Buffer de swap à plat pour réorganiser les `Range<usize>` sans toucher aux données physiques.
    swap_boundaries: Vec<Range<usize>>,
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
            // --- Buffers de factorisation (FNF) OPTIMISÉS ---
            // On prévoit de la place pour stocker environ 64 littéraux au total mis à plat
            flat_groups: Vec::with_capacity(64),
            // On prévoit une capacité initiale pour 16 groupes AND distincts
            group_boundaries: Vec::with_capacity(16),
            swap_boundaries: Vec::with_capacity(16),
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

        // --- FNF OPTIMISÉS ---
        // On vide le tableau continu et les tables d'index sans libérer leur mémoire allouée
        self.flat_groups.clear();
        self.group_boundaries.clear();
        self.swap_boundaries.clear();

        // Reste de la FNF
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
    pub fn flat_groups(&self) -> &[ExprId] {
        &self.flat_groups
    }

    #[inline]
    pub fn flat_groups_mut(&mut self) -> &mut Vec<ExprId> {
        &mut self.flat_groups
    }

    #[inline]
    pub fn group_boundaries(&self) -> &[Range<usize>] {
        &self.group_boundaries
    }

    #[inline]
    pub fn group_boundaries_mut(&mut self) -> &mut Vec<Range<usize>> {
        &mut self.group_boundaries
    }

    #[inline]
    pub fn swap_boundaries_mut(&mut self) -> &mut Vec<Range<usize>> {
        &mut self.swap_boundaries
    }

    /// Extrait une vue en lecture seule (slice) d'un groupe AND spécifique via son index.
    /// Remplace avantageusement l'ancien `scratch.group_buffer()[idx]`
    #[inline]
    pub fn get_group(&self, group_idx: usize) -> &[ExprId] {
        let range = &self.group_boundaries[group_idx];
        &self.flat_groups[range.start..range.end] // <--- Accès direct par champs usize
    }

    /// Extrait une vue mutable d'un groupe AND spécifique si tu as besoin de le modifier localement.
    #[inline]
    pub fn get_group_mut(&mut self, group_idx: usize) -> &mut [ExprId] {
        let range = &self.group_boundaries[group_idx];
        &mut self.flat_groups[range.start..range.end] // <--- Accès direct par champs usize
    }

    #[inline]
    pub fn other_kids_mut(&mut self) -> &mut Vec<ExprId> {
        &mut self.other_kids
    }

    /// MODIFICATION : Centralise le comptage des fréquences pour une tranche spécifique [start..end].
    pub fn compute_frequencies_for_slice(&mut self, start: usize, end: usize) {
        self.freq_map.clear();

        // On se base désormais sur le nombre de segments (groupes)
        let end = std::cmp::min(end, self.group_boundaries.len());
        if start >= end {
            return;
        }

        // On parcourt les index des groupes de la tranche courante
        for i in start..end {
            // 1. Extraction directe des bornes du groupe courant sans .clone()
            let group_range = &self.group_boundaries[i];

            // 2. Accès direct en mémoire contiguë via le slice indexé
            let group = &self.flat_groups[group_range.start..group_range.end];

            for &id in group {
                // 3. Incrémentation sécurisée dans la freq_map
                *self.freq_map.entry(id).or_insert(0) += 1;
            }
        }
    }

    /// MODIFICATION : Partitionne la portion [start..end] des groupes selon le facteur `f`.
    /// Les segments contenant `f` sont placés en premier. Le facteur y est retiré.
    /// Retourne le nombre d'éléments qui contenaient le facteur `f`.
    pub fn partition_slice_by_factor(&mut self, start: usize, end: usize, f: ExprId) -> usize {
        self.swap_boundaries.clear();
        let mut count_with_f = 0;

        for i in start..end {
            // 1. On prend une référence sur la Range (pas de move, pas de clone)
            let range = &self.group_boundaries[i];

            // 2. On extrait les bornes qui sont des usize (Copy)
            let g_start = range.start;
            let g_end = range.end;

            // 3. On passe les bornes explicites à la tranche mutable
            // Rust comprend parfaitement les emprunts disjoints ici
            let group_mut = &mut self.flat_groups[g_start..g_end];

            // --- Reste de ton code (recherche dichotomique et rotation) ---
            if let Ok(found_idx) = group_mut.binary_search(&f) {
                group_mut[found_idx..].rotate_left(1);

                // On recrée une nouvelle Range ajustée (le groupe a rétréci de 1)
                let updated_range = g_start..(g_end - 1);
                self.swap_boundaries.insert(count_with_f, updated_range);
                count_with_f += 1;
            } else {
                // Le groupe n'a pas f, on récrée une Range identique à l'originale
                self.swap_boundaries.push(g_start..g_end);
            }
        }

        // Étape B : Réinjection des Ranges réorganisés dans le buffer principal
        // On remplace l'ancienne portion de délimitations par celle triée dans swap_boundaries.
        self.group_boundaries
            .splice(start..end, self.swap_boundaries.drain(..));

        count_with_f
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
