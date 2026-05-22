use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::expr::iter::postorder::PostorderIter;
use crate::aiplan4rust::lir::expr::iter::preorder::PreorderIter;
use crate::aiplan4rust::lir::expr::iter::tree_preorder::TreePreorderIter;
use crate::aiplan4rust::lir::expr::{ExprEntry, ExprEntryKind, ExprId, ExprNodeRef};
use fxhash::FxBuildHasher;
use hashbrown::HashMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::hash::{Hash, Hasher};

/// Structure de recherche temporaire pour le Zero-Copy.
/// Elle permet de chercher dans la HashMap avec des références sans allouer de Vec.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ExprLookup<'a> {
    kind: &'a ExprEntryKind,
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
        };

        // On interne immédiatement les constantes pour fixer leurs IDs (souvent 0 et 1)
        store.const_true = store.intern(ExprEntryKind::And, &[]);
        store.const_false = store.intern(ExprEntryKind::Or, &[]);

        store
    }
}

impl ExprStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// La méthode centrale : récupère l'ID existant ou crée une nouvelle entrée.
    /// Garanti Zero-Allocation si l'expression existe déjà.
    pub fn intern(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
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
    pub fn get(&self, id: ExprId) -> Option<ExprNodeRef<'_>> {
        self.entries
            .get(id.as_usize())
            .map(|entry| ExprNodeRef::new(id, entry))
    }

    pub fn fetch(&self, id: ExprId) -> Result<ExprNodeRef<'_>, StorerError> {
        self.get(id).ok_or_else(|| StorerError::expr_not_found(id))
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

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookup.clear();
        self.const_true = self.intern(ExprEntryKind::And, &[]);
        self.const_false = self.intern(ExprEntryKind::Or, &[]);
    }

    pub fn rebuild_caches(&mut self) {
        let count = self.entries.len();

        // 1. On prépare le lookup (comme tu le faisais)
        self.lookup.clear();
        self.lookup.reserve(count);

        // 2. On prépare le cache des variables libres
        self.free_vars.clear();
        self.free_vars.reserve(count);

        // 3. On reconstruit tout linéairement
        // L'ordre 0..count est vital car les variables libres d'un parent
        // dépendent de celles de ses enfants (déjà traitées car ID_enfant < ID_parent).
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
            ExprEntryKind::Variable(v_id) => {
                fv.insert(*v_id);
            }

            // CAS B : Le filtre (Quantificateurs)
            ExprEntryKind::Forall(vars) | ExprEntryKind::Exists(vars) => {
                if let Some(&body_id) = entry.children().first() {
                    fv = *self.get_free_vars(body_id);
                    for v in vars {
                        fv.remove(v.symbol());
                    }
                }
            }

            // CAS C : Tout le reste (Atomes, And, Or, Not, Opérateurs temporels...)
            // On fait l'union de TOUS les enfants.
            // Si un enfant est un PredicateSymbol, son bitset est vide -> Union neutre.
            // Si un enfant est une Variable, on récupère son bit -> Union utile.
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
        #[derive(Deserialize)]
        struct ExprStoreData {
            entries: Vec<ExprEntry>,
        }

        let data = ExprStoreData::deserialize(deserializer)?;
        let count = data.entries.len();

        let mut store = Self {
            entries: data.entries,
            // On initialise le cache avec la même capacité que les entrées
            free_vars: Vec::with_capacity(count),
            lookup: HashMap::with_capacity_and_hasher(count, FxBuildHasher::default()),
            const_true: ExprId::default(),
            const_false: ExprId::default(),
        };

        // Important : Reconstruire à la fois le Hash-Consing (lookup)
        // ET le cache des variables libres (free_vars)
        store.rebuild_caches();

        Ok(store)
    }
}

const BITSET_WORDS: usize = 4;

/// Un ensemble de variables représenté par un bitset de 256 bits.
/// Placé en haut du fichier du store.
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
