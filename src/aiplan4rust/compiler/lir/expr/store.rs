use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::iter::postorder::PostorderIter;
use crate::aiplan4rust::compiler::lir::expr::iter::preorder::PreorderIter;
use crate::aiplan4rust::compiler::lir::expr::iter::tree_preorder::TreePreorderIter;
use crate::aiplan4rust::compiler::lir::expr::{Expr, ExprEntry, ExprId, ExprKind, ExprNode};
use crate::aiplan4rust::support::lang::{TypeId, TypedList, TypedListId, VariableId};
use fxhash::FxBuildHasher;
use hashbrown::HashMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::hash::Hash;

/// Structure de recherche temporaire pour le Zero-Copy.
/// Elle permet de chercher dans la HashMap avec des références sans allouer de Vec.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ExprLookup<'a> {
    kind: &'a ExprKind,
    children: &'a [ExprId],
}

/// On indique à hashbrown comment comparer notre structure temporaire
/// avec les entrées stockées.
impl<'a> hashbrown::Equivalent<ExprEntry> for ExprLookup<'a> {
    fn equivalent(&self, key: &ExprEntry) -> bool {
        // On compare les types (Enums)
        // Puis on compare les slices d'IDs.
        // Note : SmallVec implémente la comparaison avec les slices de manière très efficace.
        key.kind() == self.kind && key.children() == self.children
    }
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct ExprStore {
    /// Stockage contigu des données (Arène)
    entries: Vec<ExprEntry>,

    /// Index de recherche pour le Hash-Consing.
    /// Utilise FxHash pour des performances maximales sur les petits types.
    #[serde(skip)]
    lookup: HashMap<ExprEntry, ExprId, FxBuildHasher>,

    /// Cache des variables libres, synchronisé avec `entries`.
    /// free_vars[i] contient les variables libres de entries[i].
    free_vars: Vec<VariableSet>,

    // --- Cache des constantes fréquentes ---
    const_true: ExprId,  // Représente ExprEntryKind::And avec 0 enfants
    const_false: ExprId, // Représente ExprEntryKind::Or avec 0 enfants

    /// L'arène unique pour stocker à plat toutes les listes de variables (TypedList)
    quantifier_lists: Vec<TypedList<VariableId, TypeId>>,

    /// Index de déduplication pour le Hash-Consing local des listes de variables.
    /// Ignoré par Serde car entièrement reconstruit au chargement.
    #[serde(skip)]
    list_lookup: HashMap<TypedList<VariableId, TypeId>, usize, FxBuildHasher>,
}

impl Default for ExprStore {
    fn default() -> Self {
        let mut store = Self {
            entries: Vec::with_capacity(1024),
            lookup: hashbrown::HashMap::with_capacity_and_hasher(
                1024,
                core::hash::BuildHasherDefault::<fxhash::FxHasher>::default(),
            ),
            free_vars: Vec::with_capacity(1024),
            // Initialisés temporairement, seront fixés par intern_initial_constants
            const_true: ExprId::default(),
            const_false: ExprId::default(),

            // --- NOUVEAU : Allocation initiale de l'arène de listes ---
            // 64 est une bonne capacité de départ pour éviter les premières réallocations
            quantifier_lists: Vec::with_capacity(64),

            // --- NOUVEAU : Allocation de l'index de déduplication (Hash-Consing) ---
            list_lookup: hashbrown::HashMap::with_capacity_and_hasher(
                64,
                core::hash::BuildHasherDefault::<fxhash::FxHasher>::default(),
            ),
        };

        // --- NOUVEAU : Garantir l'index 0 pour la liste vide ---
        let empty_list = TypedList::new();
        store
            .list_lookup
            .insert(empty_list.clone(), TypedListId::EMPTY.as_usize());
        store.quantifier_lists.push(empty_list);

        // On interne immédiatement les constantes pour fixer leurs IDs (souvent 0 et 1)
        store.const_true = store.intern(ExprKind::And, &[]);
        store.const_false = store.intern(ExprKind::Or, &[]);

        store
    }
}

impl ExprStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// La méthode centrale : récupère l'ID existant ou crée une nouvelle entrée.
    /// Garanti Zero-Allocation si l'expression existe déjà.
    pub fn intern(&mut self, kind: ExprKind, children: &[ExprId]) -> ExprId {
        // 1. RECHERCHE ZERO-COPY
        // On ne crée rien, on regarde juste si ça existe
        let query = ExprLookup {
            kind: &kind,
            children,
        };

        if let Some(&id) = self.lookup.get(&query) {
            return id;
        }

        // 2. CRÉATION DE L'ID
        let id = ExprId::new(self.entries.len());

        // 3. STOCKAGE DÉFINITIF
        // On crée l'entry (Zéro-alloc si <= 4 enfants)
        let entry = ExprEntry::new(kind, children);

        let fv = self.compute_free_variables(&entry);
        self.free_vars.push(fv);

        // On insère dans le lookup en premier (on doit cloner ici car la table de hash
        // a besoin de posséder sa propre clé pour rester valide)
        self.lookup.insert(entry.clone(), id);

        // On déplace l'entrée originale dans le vecteur (Zéro-copie, juste un move)
        self.entries.push(entry);

        id
    }

    /// Retourne l'ID de l'expression (and), représentant la constante TRUE.
    #[inline]
    pub fn empty_and(&self) -> ExprId {
        self.const_true
    }

    /// Retourne l'ID de l'expression (or), représentant la constante FALSE.
    #[inline]
    pub fn empty_or(&self) -> ExprId {
        self.const_false
    }

    /// Retourne l'identifiant fort de la liste typée vide garantie globale (Index 0).
    #[inline]
    pub fn empty_typed_list(&self) -> TypedListId {
        TypedListId::EMPTY // Renvoie l'ID qui encapsule la valeur 0
    }

    /// Accès direct au masque (utile pour les unions dans intern)
    #[inline]
    pub fn get_free_vars(&self, id: ExprId) -> &VariableSet {
        &self.free_vars[id.as_usize()]
    }

    /// La fonction que tu as écrite, simplifiée en utilisant VariableSet
    #[inline]
    pub fn is_variable_free(&self, expr_id: ExprId, var_id: VariableId) -> bool {
        // On récupère le set, puis on délègue la vérification du bit
        self.get_free_vars(expr_id).contains(var_id)
    }

    #[inline]
    pub fn get(&self, id: ExprId) -> Option<ExprNode<'_>> {
        self.entries
            .get(id.as_usize())
            .map(|entry| ExprNode::new(id, entry))
    }

    pub fn fetch(&self, id: ExprId) -> Result<ExprNode<'_>, StorerError> {
        self.get(id).ok_or_else(|| StorerError::expr_not_found(id))
    }

    /// Wraps a given [`ExprId`] in an [`Expr`] proxy container bound to this store.
    ///
    /// This method simplifies expression tree traversal and evaluation by providing
    /// an ergonomic view over the raw node data.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique [`ExprId`] of the root node to wrap.
    ///
    /// # Returns
    ///
    /// * `Some(Expr<'_>)` - A structural expression proxy if the `id` exists within the store.
    /// * `None` - If the `id` is out of bounds or invalid for this specific store instance.
    ///
    /// # Performance
    ///
    /// This operation is extremely cheap and marked `#[inline]` as it performs a constant-time
    /// bounds check before wrapping the reference, preventing the creation of invalid proxies.
    #[inline]
    pub fn get_expr(&self, id: ExprId) -> Option<Expr<'_>> {
        // Safety bounds check: verify that the id actually maps to an existing slot in the arena
        if self.contains(id) {
            Some(Expr::new(id, self))
        } else {
            None
        }
    }

    /// Fetches a given [`ExprId`] and wraps it in an [`Expr`] proxy container bound to this store.
    ///
    /// Unlike [`get_expr`], this method returns a structured error if the identifier is invalid,
    /// making it ideal for propagation inside the compiler pipeline.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique [`ExprId`] of the root node to wrap.
    ///
    /// # Errors
    ///
    /// * [`StorerError::expr_not_found`] - If the `id` is out of bounds or unregistered.
    #[inline]
    pub fn fetch_expr(&self, id: ExprId) -> Result<Expr<'_>, StorerError> {
        if self.contains(id) {
            Ok(Expr::new(id, self))
        } else {
            Err(StorerError::expr_not_found(id))
        }
    }

    /// Helper method to verify if an [`ExprId`] is registered in this store.
    /// (Adjust the internal logic below based on how your store indexes its arena,
    /// e.g., checking against a vector length `id.index() < self.nodes.len()`)
    #[inline]
    pub fn contains(&self, id: ExprId) -> bool {
        // Replace with your actual internal representation check
        id.as_usize() < self.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn preorder(&self, root: ExprId) -> PreorderIter<'_> {
        PreorderIter::new(self, root)
    }

    pub fn tree_preorder(&self, root: ExprId) -> TreePreorderIter<'_> {
        TreePreorderIter::new(self, root)
    }

    pub fn postorder(&self, root: ExprId) -> PostorderIter<'_> {
        PostorderIter::new(self, root)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    // =========================================================================
    //  API TYPED_LIST (Stockage brut et résolution)
    // =========================================================================

    /// Interne une liste brute dans l'arène globale et retourne son ID fort (`TypedListId`).
    /// Garanti sans doublon (Hash-Consing). Idéal pour les Tasks, Actions et l'Encoder.
    pub fn intern_typed_list(&mut self, list: TypedList<VariableId, TypeId>) -> TypedListId {
        if let Some(&idx) = self.list_lookup.get(&list) {
            TypedListId::new(idx) // Utilise ton impl_id_type!(TypedListId)
        } else {
            let idx = self.quantifier_lists.len();
            self.list_lookup.insert(list.clone(), idx);
            self.quantifier_lists.push(list);
            TypedListId::new(idx) // Utilise ton impl_id_type!(TypedListId)
        }
    }

    /// Récupère une référence sur une liste via son `TypedListId`.
    #[inline]
    pub fn get_typed_list(&self, id: TypedListId) -> Option<&TypedList<VariableId, TypeId>> {
        self.quantifier_lists.get(id.as_usize())
    }

    /// Récupère une liste de manière stricte ou renvoie une erreur.
    pub fn fetch_typed_list(
        &self,
        id: TypedListId,
    ) -> Result<&TypedList<VariableId, TypeId>, StorerError> {
        self.get_typed_list(id)
            .ok_or_else(|| StorerError::typed_list_not_found(id))
    }

    /// Retourne le nombre d'éléments (paramètres) d'une liste typée à partir de son identifiant unique.
    ///
    /// Propage une `StorerError` si le `TypedListId` fourni n'existe pas dans l'arène.
    #[inline]
    pub fn typed_list_len(&self, id: TypedListId) -> Result<usize, StorerError> {
        let list = self.fetch_typed_list(id)?;
        Ok(list.len())
    }

    /// Vérifie si une liste typée spécifique ne contient aucun paramètre.
    ///
    /// Propage une `StorerError` si le `TypedListId` fourni n'existe pas dans l'arène.
    #[inline]
    pub fn is_typed_list_empty(&self, id: TypedListId) -> Result<bool, StorerError> {
        let list = self.fetch_typed_list(id)?;
        Ok(list.is_empty())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookup.clear();
        // --- À AJOUTER : Vider l'arène des listes ---
        self.quantifier_lists.clear();
        self.list_lookup.clear();

        let empty_list = TypedList::new();
        self.list_lookup
            .insert(empty_list.clone(), TypedListId::EMPTY.as_usize());
        self.quantifier_lists.push(empty_list);

        self.const_true = self.intern(ExprKind::And, &[]);
        self.const_false = self.intern(ExprKind::Or, &[]);
    }

    pub fn rebuild_caches(&mut self) {
        let count = self.entries.len();

        // 1. On prépare le lookup (comme tu le faisais)
        self.lookup.clear();
        self.lookup.reserve(count);

        // 2. On prépare le cache des variables libres
        self.free_vars.clear();
        self.free_vars.reserve(count);

        // --- À AJOUTER : Reconstruire l'index de déduplication des listes ---
        if self.quantifier_lists.is_empty() {
            let empty_list = TypedList::new();
            self.quantifier_lists.push(empty_list);
        }

        self.list_lookup.clear();
        self.list_lookup.reserve(self.quantifier_lists.len());
        for (idx, list) in self.quantifier_lists.iter().enumerate() {
            if idx == TypedListId::EMPTY.as_usize() {
                assert!(
                    list.is_empty(),
                    "L'index 0 de l'arène doit impérativement être la liste vide !"
                );
            }
            self.list_lookup.insert(list.clone(), idx);
        }

        // 3. On reconstruit tout linéairement
        for i in 0..count {
            let entry = &self.entries[i];
            let id = ExprId::new(i);

            // On remet l'entrée dans la table de hachage
            self.lookup.insert(entry.clone(), id);

            // On calcule et on stocke les variables libres pour cet index
            let fv = self.compute_free_variables(entry);
            self.free_vars.push(fv);
        }
    }

    #[inline]
    fn compute_free_variables(&self, entry: &ExprEntry) -> VariableSet {
        let mut fv = VariableSet::new();

        match entry.kind() {
            // CAS A : La source du signal (La variable elle-même)
            ExprKind::Variable(v_id) => {
                fv.insert(*v_id);
            }

            // --- CORRIGÉ : CAS B2 propre avec le TypedListId extrait du variant ---
            ExprKind::Forall(list_id) | ExprKind::Exists(list_id) => {
                // Dans ce nouveau modèle, le corps (body) est le premier enfant direct
                if let Some(&body_id) = entry.children().first() {
                    // 1. On récupère les variables libres du corps
                    fv = *self.get_free_vars(body_id);

                    // 2. On résout la liste via ton store/registre (ici visiblement `quantifier_lists`)
                    // Si quantifier_lists attend un usize, on utilise .as_usize(),
                    // mais si get_typed_list fonctionne, préfère-le.
                    if let Some(vars) = self.quantifier_lists.get(list_id.as_usize()) {
                        for v in vars {
                            fv.remove(v.symbol());
                        }
                    }
                }
            }

            // CAS C : Tout le reste
            _ => {
                for &child_id in entry.children() {
                    fv.union_with(self.get_free_vars(child_id));
                }
            }
        }
        fv
    }
}

impl std::ops::Index<ExprId> for ExprStore {
    type Output = ExprEntry;

    fn index(&self, id: ExprId) -> &Self::Output {
        &self.entries[id.as_usize()]
    }
}

impl<'de> Deserialize<'de> for ExprStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 1. Définition de la structure de transport intermédiaire
        #[derive(Deserialize)]
        struct ExprStoreData {
            entries: Vec<ExprEntry>,
            // Le helper #[serde(default)] permet de ne pas crasher si tu lis un ancien
            // fichier JSON/Bincode qui n'avait pas encore l'arène des listes.
            #[serde(default)]
            quantifier_lists: Vec<TypedList<VariableId, TypeId>>,
        }

        let data = ExprStoreData::deserialize(deserializer)?;
        let count = data.entries.len();
        let lists_count = data.quantifier_lists.len();

        // 2. Reconstruction de l'instance avec ses structures à plat
        let mut store = Self {
            entries: data.entries,
            free_vars: Vec::with_capacity(count),
            lookup: HashMap::with_capacity_and_hasher(count, FxBuildHasher::default()),
            const_true: ExprId::default(),
            const_false: ExprId::default(),

            // --- NOUVEAU : Restauration de l'arène des listes ---
            quantifier_lists: data.quantifier_lists,
            // L'index de hachage associé est initialisé vide, prêt à être rebâti
            list_lookup: HashMap::with_capacity_and_hasher(lists_count, FxBuildHasher::default()),
        };

        // 3. IMPORTANT : Reconstruit TOUS les index à chaud (lookup, free_vars ET list_lookup !)
        store.rebuild_caches();

        Ok(store)
    }
}

const BITSET_WORDS: usize = 4;

/// Un ensemble de variables représenté par un bitset de 256 bits.
/// Placé en haut du fichier du old.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct VariableSet([u64; BITSET_WORDS]);

impl VariableSet {
    pub fn new() -> Self {
        Self([0; BITSET_WORDS])
    }

    #[inline]
    pub fn insert(&mut self, id: VariableId) {
        let idx = id.as_usize();
        if idx < BITSET_WORDS * 64 {
            self.0[idx / 64] |= 1 << (idx % 64);
        }
    }

    #[inline]
    pub fn remove(&mut self, id: VariableId) {
        let idx = id.as_usize();
        if idx < BITSET_WORDS * 64 {
            self.0[idx / 64] &= !(1 << (idx % 64));
        }
    }

    #[inline]
    pub fn contains(&self, id: VariableId) -> bool {
        let idx = id.as_usize();
        idx < BITSET_WORDS * 64 && (self.0[idx / 64] & (1 << (idx % 64))) != 0
    }

    pub fn union_with(&mut self, other: &Self) {
        for i in 0..BITSET_WORDS {
            self.0[i] |= other.0[i];
        }
    }
}
