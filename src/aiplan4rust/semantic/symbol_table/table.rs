//! # Symbol Table for `aiplan4rust`
//!
//! This module defines the [`Table`] struct, which implements the symbol table used by the
//! `aiplan4rust` compiler pipeline. It provides facilities for storing and retrieving symbol entries,
//! performing scoped lookups, checking for declaration and usage conflicts, and preserving symbol
//! insertion order.
//!
//! The symbol table is used throughout parsing, semantic analysis, and linking phases.
//!
//! ## Key Features
//! - Maintains insertion order using `LinkedHashMap` for deterministic diagnostics and output.
//! - Supports filtering by symbol kind and scope.
//! - Tracks both declarations and usages.
//! - Keeps metadata about its construction origin (e.g., domain file, problem file, merged).
//!
//! ## Related Components
//! - [`SymbolEntry`]: Represents a symbol and its declarations/usages.
//! - [`SymbolKind`], [`Scope`], [`Declaration`], [`Usage`]: Metadata for symbols.
//!
//! ## Example
//! ```rust
//! use aiplan4rust::semantic::Table;
//!
//! let mut table = Table::default();
//! assert!(table.iter().count() == 0);
//! ```

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId};
use crate::aiplan4rust::semantic::signature_checker::MatchResult;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Filterable;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol_table::{SymbolTableError, SymbolTableOrigin};
use crate::aiplan4rust::semantic::type_checker::TypeHierarchy;
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::tree::NodeId;
use ahash::HashMapExt;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A symbol table used in `aiplan4rust` to store and manage symbols.
///
/// This structure manages a mapping between identifiers (`Ident`) and their corresponding
/// [`SymbolEntry`] metadata, which includes both declarations and usages of symbols.
/// It plays a key role in name resolution, scope tracking, and error reporting during semantic analysis.
///
/// The underlying data structure is a [`LinkedHashMap`] to preserve insertion order,
/// which is beneficial for deterministic behavior during compilation and debugging.
///
/// # Fields
/// - `symbols`: Ordered mapping from identifiers to `SymbolEntry` instances.
/// - `origin`: Indicates where this table was constructed from (e.g., domain file, problem file, merged).
/// - `root_id`: The root `NodeId` used to generate scope paths.
///
/// # Example
/// ```rust
/// let mut table = Table::default();
/// table.insert_symbol("my_symbol".into(), SymbolEntry::new(...));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    /// Map from symbol identifier to symbol metadata, preserving insertion order.
    pub(super) symbols: Vec<SymbolEntry>,

    /// Indicates the origin context of the symbol table (e.g., Domain, Problem, Merged).
    pub(super) origin: SymbolTableOrigin,

    /// Root node ID used to construct the root scope.
    pub(super) root_id: NodeId,

    /// Index de Cache pour les DÉCLARATIONS : NodeId -> (SymbolId, Position)
    /// Permet de savoir instantanément quel symbole est déclaré à tel endroit de l'AST.
    pub(super) declarations_index: FxHashMap<NodeId, SymbolRef>,

    /// Index de Cache pour les USAGES : NodeId -> (SymbolId, Position)
    /// Permet de retrouver l'usage d'un symbole à partir d'un nœud d'appel.
    pub(super) usages_index: FxHashMap<NodeId, SymbolRef>,
}

impl Default for Table {
    /// Creates a new, empty symbol table with default metadata.
    ///
    /// - The symbol map is initialized empty.
    /// - The origin is set to [`SymbolTableOrigin::None`].
    /// - The root node ID is set to the default [`NodeId`].
    ///
    /// # Returns
    /// A new default-initialized symbol table.
    ///
    /// # Example
    /// ```rust
    /// let table = Table::default();
    /// assert!(table.iter().next().is_none());
    /// ```
    fn default() -> Self {
        Self {
            /// La map ordonnée des symboles
            symbols: Vec::new(),

            /// Cache O(1) pour retrouver une déclaration par son NodeId
            /// On pré-alloue une petite capacité pour éviter les premières re-allocations
            declarations_index: FxHashMap::with_capacity(64),

            /// Cache O(1) pour retrouver un usage par son NodeId
            usages_index: FxHashMap::with_capacity(128),

            origin: SymbolTableOrigin::default(),
            root_id: NodeId::default(),
        }
    }
}

impl Table {
    pub fn new(interner: &SymbolInterner) -> Self {
        let size = interner.symbol_len();
        let mut symbols = Vec::with_capacity(size);

        for i in 0..size {
            symbols.push(SymbolEntry::new(SymbolId::from(i)));
        }

        Self {
            symbols,
            declarations_index: FxHashMap::with_capacity(size),
            usages_index: FxHashMap::with_capacity(size),
            origin: SymbolTableOrigin::default(),
            root_id: NodeId::default(),
        }
    }

    /// Returns an iterator over all symbol identifiers present in the table.
    ///
    /// This provides a lightweight way to traverse the table when only the
    /// identity of symbols is required, avoiding direct access to the full
    /// symbol metadata.
    pub fn symbol_ids(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.symbols.iter().map(|entry| entry.id())
    }

    /// Returns the origin metadata of the symbol table.
    ///
    /// The origin describes the context from which the symbol table was built—
    /// for example, a domain file, a problem file, or the result of merging both.
    ///
    /// # Returns
    /// The `SymbolTableOrigin` associated with this symbol table.
    pub fn origin(&self) -> SymbolTableOrigin {
        self.origin
    }

    /// Updates the origin metadata of the symbol table.
    ///
    /// This is typically used during linking or transformation steps
    /// when the origin context of the table changes (e.g., from Domain to Merged).
    ///
    /// # Parameters
    /// - `origin`: The new `SymbolTableOrigin` to assign.
    pub fn set_origin(&mut self, origin: SymbolTableOrigin) {
        self.origin = origin;
    }

    /// Returns the root scope of the symbol table.
    ///
    /// # Returns
    /// A `Scope` constructed from the symbol table's internal root ID.
    pub fn root_scope(&self) -> Scope {
        Scope::new(self.root_id, None)
    }

    /// Sets the internal root ID used to build the root scope.
    ///
    /// # Parameters
    /// - `root_id`: A `NodeId` representing the new root for scope generation.
    pub fn set_root_id(&mut self, root_id: NodeId) {
        self.root_id = root_id;
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    #[inline]
    pub fn iter_declarations(&self, id: SymbolId) -> impl Iterator<Item = &Declaration> {
        self.symbols
            .get(id.as_usize())
            .into_iter()
            .flat_map(|entry| entry.declarations().iter())
    }

    #[inline]
    pub fn iter_usages(&self, id: SymbolId) -> impl Iterator<Item = &Usage> {
        self.symbols
            .get(id.as_usize())
            .into_iter()
            .flat_map(|entry| entry.usages().iter())
    }

    pub fn iter_primitive_types(&self) -> impl Iterator<Item = &Declaration> + '_ {
        let root = self.root_scope();
        self.symbols
            .iter()
            .flat_map(|entry| entry.declarations())
            .filter(move |d| d.scope() == &root && d.symbol_kind() == SymbolKind::PrimitiveType)
    }

    /// Retourne la déclaration du nom du domaine.
    ///
    /// Optimisé pour s'arrêter dès la première occurrence trouvée (Short-circuit).
    /// Étant donné que le nom du domaine est généralement l'un des premiers symboles
    /// rencontrés, cette opération est proche de O(1) en pratique.
    pub fn domain_name(&self) -> Option<&Declaration> {
        let root = self.root_scope();
        self.symbols
            .iter()
            .flat_map(|e| e.declarations())
            .find(|d| d.scope() == &root && d.symbol_kind() == SymbolKind::DomainName)
    }

    /// Retourne la déclaration du nom du problème.
    ///
    /// Effectue un scan linéaire rapide. Même si le nom du problème apparaît plus tard
    /// dans le fichier, l'absence d'allocations rend cette fonction très performante.
    pub fn problem_name(&self) -> Option<&Declaration> {
        let root = self.root_scope();
        self.symbols
            .iter()
            .flat_map(|e| e.declarations())
            .find(|d| d.scope() == &root && d.symbol_kind() == SymbolKind::ProblemName)
    }

    /// Récupère la déclaration du nom du domaine ou renvoie une erreur si absente.
    pub fn try_domain_name(&self) -> Result<&Declaration, SymbolTableError> {
        self.domain_name()
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_kind(SymbolKind::DomainName))
    }

    /// Récupère la déclaration du nom du problème ou renvoie une erreur si absente.
    pub fn try_problem_name(&self) -> Result<&Declaration, SymbolTableError> {
        self.problem_name().ok_or_else(|| {
            SymbolTableError::declaration_not_found_for_kind(SymbolKind::ProblemName)
        })
    }

    /// Retrieves an immutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `id`: The identifier (`SymbolId`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a reference to the symbol, or `None` if not found.
    pub fn get_symbol(&self, id: SymbolId) -> Option<&SymbolEntry> {
        self.symbols.get(id.as_usize())
    }

    /// Retrieves an immutable reference to a symbol by its ID, or returns an error if not found.
    ///
    /// This is the fallible version of `get_symbol`. It is preferred when a missing
    /// symbol should stop the current semantic pass and provide a traced error.
    ///
    /// # Parameters
    /// - `id`: The internal identifier ([`SymbolId`持) of the symbol to look up.
    ///
    /// # Returns
    /// - `Ok(&SymbolEntry)`: A reference to the found symbol entry.
    /// - `Err(SymbolTableError::SymbolNotFound)`: If the ID does not exist in the table.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError::SymbolNotFound`] with a captured trace via `#[track_caller]`.
    pub fn try_get_symbol(&self, id: SymbolId) -> Result<&SymbolEntry, SymbolTableError> {
        // Avec un Vec, .get(index) est l'équivalent sûr.
        // On convertit le SymbolId en usize pour l'indexation.
        self.symbols
            .get(id.as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))
    }

    pub fn add_declaration(
        &mut self,
        id: SymbolId,
        decl: Declaration,
    ) -> Result<(), SymbolTableError> {
        let node_id = decl.source();

        // 1. GARDE D'IDEMPOTENCE
        // Le cache NodeId -> SymbolRef reste indispensable
        if self.declarations_index.contains_key(&node_id) {
            return Ok(());
        }

        // 2. ACCÈS DIRECT O(1)
        // On récupère l'entrée qui a été pré-initialisée à la création de la table.
        // L'indexation directe sur Vec est une instruction machine unique.
        let entry = self
            .symbols
            .get_mut(id.as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))?;

        // 3. INDEXATION & INSERTION
        // On utilise l'état actuel de l'entrée (vide ou déjà remplie par d'autres décl)
        let index = entry.declarations().len();
        entry.add_declaration(decl);

        // 4. MISE À JOUR DU CACHE
        // On lie le NodeId de l'AST à notre SymbolId et sa position dans le vecteur interne de l'entrée.
        self.declarations_index
            .insert(node_id, SymbolRef::new(id, index));

        Ok(())
    }

    /// Lie un usage à une déclaration de manière atomique et ultra-rapide.
    /// À utiliser dans apply_resolutions pour éviter les lookups manuels.
    pub fn link_resolution(&mut self, usage_id: NodeId, decl_id: NodeId, status: MatchResult) {
        // 1. On récupère les refs du cache (O(1))
        let u_ref = self.usages_index.get(&usage_id).copied();
        let d_ref = self.declarations_index.get(&decl_id).copied();

        // 2. Mise à jour de l'Usage (Lien Usage -> Déclaration)
        if let Some(r) = u_ref {
            // Accès direct au symbole via l'ID
            if let Some(entry) = self.symbols.get_mut(r.id().as_usize()) {
                // Accès direct à l'usage via l'index du vecteur
                // Plus besoin de map(|(_, v)| v), on est déjà sur la valeur !
                if let Some(u) = entry.usages_mut().get_mut(r.index()) {
                    u.set_declaration(decl_id);
                    u.set_resolution(status);
                }
            }
        }

        // 3. Mise à jour de la Déclaration (Lien Déclaration -> Usage)
        if let Some(r) = d_ref {
            if let Some(entry) = self.symbols.get_mut(r.id().as_usize()) {
                // Accès direct à la déclaration via l'index
                if let Some(d) = entry.declarations_mut().get_mut(r.index()) {
                    d.add_usage(usage_id);
                }
            }
        }
    }
    pub fn promote_declaration(
        &mut self,
        old_id: NodeId,
        new_id: NodeId,
    ) -> Result<&mut Declaration, SymbolTableError> {
        // 1. On extrait la référence (SymbolId + Index) via l'ancien NodeId
        // On retire l'ancienne clé du cache de recherche.
        let s_ref = self
            .declarations_index
            .remove(&old_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(old_id))?;

        // 2. On réinsère immédiatement la même référence (SymbolRef) sous le nouvel ID
        // L'index dans le vecteur interne du symbole reste identique.
        self.declarations_index.insert(new_id, s_ref);

        // 3. On récupère l'entrée du symbole via son ID (O(1) dans le Vec principal)
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 4. On récupère la déclaration via son index dans le vecteur
        // Plus besoin de .get_index_mut() ni de map(|(_, d)| d) !
        let decl = entry
            .declarations_mut()
            .get_mut(s_ref.index())
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(new_id))?;

        // 5. Mise à jour de l'ID source interne
        decl.set_source(new_id);

        Ok(decl)
    }

    pub fn get_declaration(&self, node_id: NodeId) -> Option<&Declaration> {
        // 1. On récupère la référence (SymbolId + Index) dans le cache (O(1))
        let s_ref = self.declarations_index.get(&node_id)?;

        // 2. On pointe vers l'entrée du symbole (O(1) dans le Vec principal)
        let entry = self.symbols.get(s_ref.id().as_usize())?;

        // 3. Accès direct à la déclaration via son index (O(1) dans le Vec interne)
        // On utilise .get() qui renvoie directement Option<&Declaration>
        entry.declarations().get(s_ref.index())
    }

    pub fn try_get_declaration(&self, node_id: NodeId) -> Result<&Declaration, SymbolTableError> {
        // 1. Recherche dans le cache global (NodeId -> SymbolRef) - O(1)
        let s_ref = self
            .declarations_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée du symbole (O(1) dans le Vec principal)
        let entry = self
            .symbols
            .get(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct à la déclaration spécifique via son index (O(1) dans le Vec interne)
        // .get() remplace avantageusement .get_index().map()
        entry
            .declarations()
            .get(s_ref.index())
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn get_declaration_mut(&mut self, node_id: NodeId) -> Option<&mut Declaration> {
        // 1. Accès au cache (O(1)) pour récupérer la référence
        // On déréférence ou on utilise .copied() pour ne pas garder un borrow sur self.declarations_index
        let s_ref = *self.declarations_index.get(&node_id)?;

        // 2. Récupération mutable de l'entrée (O(1))
        let entry = self.symbols.get_mut(s_ref.id().as_usize())?;

        // 3. Accès direct mutable par index dans le Vec interne
        // .get_mut(index) remplace .get_index_mut(index).map(...)
        entry.declarations_mut().get_mut(s_ref.index())
    }

    pub fn try_get_declaration_mut(
        &mut self,
        node_id: NodeId,
    ) -> Result<&mut Declaration, SymbolTableError> {
        // 1. Accès au cache (O(1))
        // On déréférence (*) pour copier le SymbolRef (Copy) et libérer l'emprunt sur self.declarations_index
        let s_ref = *self
            .declarations_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))?;

        // 2. Récupération mutable de l'entrée du symbole (O(1))
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct mutable par index (O(1))
        // .get_mut() sur le Vec est plus simple et plus rapide
        entry
            .declarations_mut()
            .get_mut(s_ref.index())
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn add_usage(&mut self, id: SymbolId, usage: Usage) -> Result<(), SymbolTableError> {
        let node_id = usage.source();

        // 1. GARDE D'IDEMPOTENCE
        // Si ce NodeId est déjà dans le cache, on ne fait rien.
        if self.usages_index.contains_key(&node_id) {
            return Ok(());
        }

        // 2. ACCÈS DIRECT O(1)
        // On récupère l'entrée via son index numérique.
        let entry = self
            .symbols
            .get_mut(id.as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))?;

        // 3. INDEXATION & INSERTION
        // On récupère la position actuelle dans la liste des usages du symbole.
        let index = entry.usages().len();
        entry.add_usage(usage);

        // 4. MISE À JOUR DU CACHE
        // On lie le NodeId de l'usage au SymbolId et à sa position dans l'entrée.
        self.usages_index.insert(node_id, SymbolRef::new(id, index));

        Ok(())
    }

    pub fn get_usage(&self, node_id: NodeId) -> Option<&Usage> {
        // 1. On interroge le cache global (NodeId -> SymbolRef)
        // C'est notre porte d'entrée en O(1).
        let s_ref = self.usages_index.get(&node_id)?;

        // 2. On récupère l'entrée du symbole par index direct (O(1))
        let entry = self.symbols.get(s_ref.id().as_usize())?;

        // 3. Accès direct par index dans le Vec interne de l'entrée
        // Plus de get_index, plus de map(|(_, u)| u), juste de l'accès mémoire pur.
        entry.usages().get(s_ref.index())
    }

    pub fn try_get_usage(&self, node_id: NodeId) -> Result<&Usage, SymbolTableError> {
        // 1. Recherche dans le cache global (O(1) via NodeId)
        let s_ref = self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée du symbole par son index numérique (O(1))
        let entry = self
            .symbols
            .get(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct à l'usage par son index (O(1))
        // .get() sur le Vec remplace .get_index().map()
        entry
            .usages()
            .get(s_ref.index())
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))
    }

    pub fn try_get_usage_mut(&mut self, node_id: NodeId) -> Result<&mut Usage, SymbolTableError> {
        // 1. Recherche dans le cache (O(1))
        // On déréférence (*) pour copier le SymbolRef et libérer l'emprunt sur self.usages_index
        let s_ref = *self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée mutable (O(1) dans le Vec principal)
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct mutable par index (O(1) dans le Vec interne)
        // get_mut() remplace get_index_mut().map()
        entry
            .usages_mut()
            .get_mut(s_ref.index())
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))
    }

    /// Resolves a symbol usage to its primary source declaration by traversing the
    /// bidirectional link (usage -> declaration).
    ///
    /// This provides O(1) access to the declaration that was linked to this specific
    /// usage during the semantic analysis phase.
    ///
    /// # Arguments
    /// * `symbol` - The identifier of the symbol to resolve.
    /// * `usage_node_id` - The AST node identifier where the symbol is used.
    ///
    /// # Returns
    /// * `Ok(&Declaration)` - A reference to the primary source declaration.
    /// * `Err(SymbolTableError)` - If the usage record is missing or if the link
    ///   to the declaration has not been established (unresolved).
    pub fn resolve_usage(&self, usage_node_id: NodeId) -> Result<&Declaration, SymbolTableError> {
        // 1. Accès direct à l'usage via le cache O(1) et l'index du Vec
        // Cette méthode utilise déjà le nouveau système d'indexation numérique.
        let usage = self.try_get_usage(usage_node_id)?;

        // 2. Récupération de l'ID du nœud de déclaration lié
        // Si l'usage n'est pas lié (None), on renvoie une erreur de résolution.
        let decl_node_id = usage
            .declaration()
            .ok_or_else(|| SymbolTableError::unresolved_usage(usage.symbol_id(), usage_node_id))?;

        // 3. Accès direct à la déclaration finale via le cache O(1)
        // On passe par le declarations_index puis par l'index du Vec.
        self.try_get_declaration(decl_node_id)
    }

    /// This method scans all symbols of kind [`SymbolKind::PrimitiveType`],
    /// collects their parent declarations, and packages them into a
    /// standalone [`TypeHierarchy`].
    ///
    /// The resulting object is independent of the table's lifetime,
    /// allowing it to be used by the `TypeChecker` without causing
    /// borrow checker conflicts.
    ///
    /// # Returns
    /// A [`TypeHierarchy`] reflecting the state of the types in this table.
    pub fn to_type_hierarchy(&self) -> TypeHierarchy {
        // On garde une capacité initiale raisonnable
        let mut hierarchy_map: HashMap<SymbolId, Vec<SymbolId>> =
            HashMap::with_capacity(self.symbols.len() / 4);

        // L'itérateur de Vec renvoie directement &SymbolEntry
        for entry in self.symbols.iter() {
            let id = entry.id(); // On récupère l'ID stocké dans l'entrée

            for decl in entry.declarations() {
                if decl.kind() == SymbolKind::PrimitiveType {
                    // Étape 1 : On récupère les parents (super-types) s'ils existent
                    if let Some(ty) = decl.ty() {
                        let parents = ty.members();

                        // Étape 2 : On enregistre les parents pour ce type
                        hierarchy_map
                            .entry(id)
                            .or_default()
                            .extend(parents.iter().cloned());

                        // Étape 3 : On s'assure que chaque parent est présent dans la map
                        // (Important pour les types racines qui ne sont jamais "enfants")
                        for &parent_id in parents {
                            hierarchy_map.entry(parent_id).or_default();
                        }
                    } else {
                        // Si le type n'a pas de parents, on l'inscrit comme noeud (potentiellement racine)
                        hierarchy_map.entry(id).or_default();
                    }
                }
            }
        }

        TypeHierarchy::new(hierarchy_map)
    }

    /// Reconstruit les index de recherche rapide à partir du vecteur de symboles actuel.
    pub fn rebuild_caches(&mut self) {
        // On vide mais on garde la capacité mémoire (évite des mallocs)
        self.declarations_index.clear();
        self.usages_index.clear();

        // Optionnel : si tu as vraiment beaucoup de données,
        // tu peux pré-réserver la place si tu as un compteur global.

        for entry in self.symbols.iter() {
            let symbol_id = entry.id();

            // On ne traite que les symboles qui ont des données
            if !entry.declarations().is_empty() {
                for (idx, decl) in entry.declarations().iter().enumerate() {
                    self.declarations_index
                        .insert(decl.source(), SymbolRef::new(symbol_id, idx));
                }
            }

            if !entry.usages().is_empty() {
                for (idx, usage) in entry.usages().iter().enumerate() {
                    self.usages_index
                        .insert(usage.source(), SymbolRef::new(symbol_id, idx));
                }
            }
        }
    }
}

impl RemapSymbol for SymbolTable {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // 1. Calculer la taille finale une seule fois
        let max_id = map.values().map(|id| id.as_usize()).max().unwrap_or(0);

        // 2. Pré-allouer le nouveau vecteur
        let mut new_symbols = Vec::with_capacity(max_id + 1);
        for i in 0..=max_id {
            new_symbols.push(SymbolEntry::new(SymbolId::from(i)));
        }

        // 3. Déplacer et transformer en une seule passe
        let old_symbols = std::mem::take(&mut self.symbols);
        for mut entry in old_symbols {
            let old_id = entry.id();

            // Si le symbole a une correspondance dans la nouvelle map
            if let Some(&new_id) = map.get(&old_id) {
                // Mise à jour interne (ident, types, etc.)
                entry.remap_symbol(map)?;
                // On le place directement au bon index
                new_symbols[new_id.as_usize()] = entry;
            }
        }

        self.symbols = new_symbols;

        // 4. Reconstruction optimisée des caches
        self.rebuild_caches();

        Ok(())
    }
}

impl fmt::Display for Table {
    /// Formats the entire `Table` as a user-friendly string.
    ///
    /// This method implements the `Display` trait for the `Table` typing,
    /// allowing the table to be printed or logged conveniently. It
    /// formats each symbol stored in the table on its own line.
    ///
    /// # Parameters
    /// - `f`: The formatter to write the formatted string into.
    ///
    /// # Returns
    /// Returns a `fmt::Result` indicating success or failure during formatting.
    ///
    /// # Example
    /// ```
    /// let symbol_table = Table::new();
    /// println!("{}", symbol_table); // Prints all symbols line by line.
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On itère directement sur le vecteur de symboles.
        // Puisque la table est pré-initialisée, on parcourt tous les IDs
        // dans l'ordre de l'interner (0, 1, 2...).
        for symbol in self.symbols.iter() {
            // On n'affiche le symbole que s'il contient quelque chose
            // (optionnel, selon si tu veux voir les slots vides de l'interner)
            if !symbol.declarations().is_empty() || !symbol.usages().is_empty() {
                writeln!(f, "{}", symbol)?;
            }
        }

        Ok(())
    }
}

impl InternerDisplay for Table {
    /// Formats the symbol table using the provided string interner.
    ///
    /// This method iterates over all symbols stored in the table and writes their
    /// formatted representation to the given formatter, using the provided string interner
    /// to resolve symbol names or other interned strings.
    ///
    /// Each symbol's formatting is delegated to its own `fmt_with_interner` method.
    /// Symbols are separated by newlines in the output.
    ///
    /// # Parameters
    /// - `f`: The formatter to write the output to.
    /// - `interner`: A `StringInterner` used to resolve interned string IDs into strings.
    ///
    /// # Returns
    /// Returns `fmt::Result`, indicating success or failure during formatting.
    ///
    /// # Example
    /// ```rust
    /// use crate::aiplan4rust::semantic::symbol_table::Table;
    /// use crate::aiplan4rust::interner::{StringInterner, InternerDisplay};
    ///
    /// let table = Table::new();
    /// let interner = StringInterner::new();
    /// // ... populate table and interner ...
    /// println!("{}", table.fmt_with_interner(&mut std::fmt::Formatter::new(), &interner).unwrap());
    /// ```
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        // On itère sur le Vec de symboles.
        // L'ordre sera celui de l'interner (0, 1, 2...).
        for symbol in self.symbols.iter() {
            // On ne délègue l'affichage que si le symbole a du contenu
            // pour éviter des blocs vides dans les logs.
            if !symbol.declarations().is_empty() || !symbol.usages().is_empty() {
                symbol.fmt_with_interner(f, interner)?;
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

// Pour l'itération par référence : for entry in &table
impl<'a> IntoIterator for &'a Table {
    type Item = &'a SymbolEntry;
    type IntoIter = std::slice::Iter<'a, SymbolEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.iter()
    }
}

// Pour l'itération par référence mutuelle : for entry in &mut table
impl<'a> IntoIterator for &'a mut Table {
    type Item = &'a mut SymbolEntry;
    type IntoIter = std::slice::IterMut<'a, SymbolEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.iter_mut()
    }
}

/// Une référence légère vers une entrée de la table des symboles.
/// Elle combine l'identifiant unique du symbole et l'index de la donnée
/// (déclaration ou usage) dans les listes internes du SymbolEntry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolRef {
    /// L'identifiant unique du symbole (ex: s#14)
    id: SymbolId,

    /// L'index numérique dans la collection ordonnée (IndexMap ou Vec)
    /// à l'intérieur de SymbolEntry.
    index: usize,
}

impl SymbolRef {
    /// Crée une nouvelle référence vers un symbole à un index spécifique.
    pub fn new(id: SymbolId, index: usize) -> Self {
        Self { id, index }
    }

    /// Retourne l'identifiant du symbole.
    pub fn id(&self) -> SymbolId {
        self.id
    }

    /// Retourne l'index de la déclaration ou de l'usage.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Version spécialisée pour le remappage (plus propre que id_mut)
    pub fn remap_id(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        self.id.remap_idents(map)
    }
}
