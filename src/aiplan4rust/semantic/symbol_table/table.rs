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
use linked_hash_map::LinkedHashMap;
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
    pub(super) declarations_index: HashMap<NodeId, SymbolRef>,

    /// Index de Cache pour les USAGES : NodeId -> (SymbolId, Position)
    /// Permet de retrouver l'usage d'un symbole à partir d'un nœud d'appel.
    pub(super) usages_index: HashMap<NodeId, SymbolRef>,
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
            declarations_index: HashMap::with_capacity(64),

            /// Cache O(1) pour retrouver un usage par son NodeId
            usages_index: HashMap::with_capacity(128),

            origin: SymbolTableOrigin::default(),
            root_id: NodeId::default(),
        }
    }
}

impl Table {
    pub fn new(interner: &SymbolInterner) -> Self {
        println!("{}", interner);
        let size = interner.symbol_len();
        let mut symbols = Vec::with_capacity(size);

        // On pré-remplit le Vec avec des entrées vides pour chaque ID
        for i in 0..size {
            symbols.push(SymbolEntry::new(SymbolId::from(i)));
        }

        SymbolTable {
            symbols,
            declarations_index: HashMap::with_capacity(size),
            usages_index: HashMap::with_capacity(size),
            origin: SymbolTableOrigin::default(),
            root_id: NodeId::default(),
        }
    }

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

    /// Retourne un itérateur sur les paires (SymbolId, &SymbolEntry).
    /// C'est l'équivalent de l'ancien iter() de ta HashMap.
    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, &SymbolEntry)> {
        self.symbols
            .iter()
            .enumerate()
            .map(|(idx, entry)| (SymbolId::from(idx), entry))
    }

    /// Retrieves an immutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `name`: The identifier (`Ident`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a reference to the symbol, or `None` if not found.
    pub fn get_symbol(&self, name: SymbolId) -> Option<&SymbolEntry> {
        let entry = self.symbols.get(name.as_usize());
        if entry.is_none() {
            println!(
                "!!! CRITICAL: ID {} non trouvé dans le vecteur de taille {}",
                name.as_usize(),
                self.symbols.len()
            );
        } else if entry.unwrap().declarations().is_empty() {
            println!(
                "!!! WARNING: ID {} trouvé mais declarations est VIDE",
                name.as_usize()
            );
        }
        entry
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
        // 1. On récupère les refs du cache (C'est du O(1) pur)
        // SymbolRef contient le SymbolId et l'index dans le vecteur interne de l'entrée.
        let u_ref = self.usages_index.get(&usage_id).copied();
        let d_ref = self.declarations_index.get(&decl_id).copied();

        // 2. Mise à jour de l'Usage (Lien Usage -> Déclaration)
        if let Some(r) = u_ref {
            // Accès direct par index dans le Vec<SymbolEntry>
            if let Some(entry) = self.symbols.get_mut(r.id().as_usize()) {
                // Accès direct par index dans le IndexMap/Vec de l'entrée
                if let Some(u) = entry.usages_mut().get_index_mut(r.index()).map(|(_, v)| v) {
                    u.set_declaration(decl_id);
                    u.set_resolution(status);
                }
            }
        }

        // 3. Mise à jour de la Déclaration (Lien Déclaration -> Usage)
        // d_ref peut être None si decl_id est un symbole virtuel (ex: built-ins, types réservés)
        if let Some(r) = d_ref {
            // Accès direct par index dans le Vec<SymbolEntry>
            if let Some(entry) = self.symbols.get_mut(r.id().as_usize()) {
                if let Some(d) = entry
                    .declarations_mut()
                    .get_index_mut(r.index())
                    .map(|(_, v)| v)
                {
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
        // On utilise .remove() car l'ancien ID (la "coquille vide" de l'AST) est remplacé.
        let s_ref = self
            .declarations_index
            .remove(&old_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(old_id))?;

        // 2. On réinsère immédiatement la même référence (SymbolRef) sous le nouvel ID de nœud
        // L'index dans le vecteur interne du symbole ne change pas.
        self.declarations_index.insert(new_id, s_ref);

        // 3. On récupère l'entrée du symbole via son index numérique (O(1))
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 4. On récupère la déclaration spécifique au sein de ce symbole
        let decl = entry
            .declarations_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, d)| d)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(new_id))?;

        // 5. Mise à jour de l'ID source interne à la déclaration pour refléter le nouveau nœud
        decl.set_source(new_id);

        Ok(decl)
    }

    pub fn get_declaration(&self, node_id: NodeId) -> Option<&Declaration> {
        // 1. Accès direct au cache via le NodeId (HashMap interne)
        let s_ref = self.declarations_index.get(&node_id)?;

        // 2. Récupération de l'entrée du symbole via son index numérique (O(1))
        // On convertit le SymbolId contenu dans s_ref en usize
        let entry = self.symbols.get(s_ref.id().as_usize())?;

        // 3. Accès direct à la déclaration par son index dans l'entrée
        entry
            .declarations()
            .get_index(s_ref.index())
            .map(|(_, d)| d)
    }

    pub fn try_get_declaration(&self, node_id: NodeId) -> Result<&Declaration, SymbolTableError> {
        // 1. Recherche dans le cache global (NodeId -> SymbolRef)
        let s_ref = self
            .declarations_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée via l'index numérique du SymbolId (O(1))
        // On convertit le SymbolId en usize pour l'accès au Vec.
        let entry = self
            .symbols
            .get(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct à la déclaration spécifique par son index dans l'entrée
        entry
            .declarations()
            .get_index(s_ref.index())
            .map(|(_, d)| d)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn get_declaration_mut(&mut self, node_id: NodeId) -> Option<&mut Declaration> {
        // 1. Accès au cache (O(1) via HashMap) pour récupérer la référence
        let s_ref = *self.declarations_index.get(&node_id)?;

        // 2. Récupération mutable de l'entrée du symbole via son index numérique (O(1))
        // On convertit le SymbolId en usize pour l'accès direct au Vec
        let entry = self.symbols.get_mut(s_ref.id().as_usize())?;

        // 3. Accès direct mutable par index dans la collection interne de l'entrée
        entry
            .declarations_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, d)| d)
    }

    pub fn try_get_declaration_mut(
        &mut self,
        node_id: NodeId,
    ) -> Result<&mut Declaration, SymbolTableError> {
        // 1. Accès au cache (on copie le SymbolRef pour libérer l'emprunt sur self.declarations_index)
        let s_ref = *self
            .declarations_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))?;

        // 2. Récupération mutable de l'entrée du symbole par son index numérique (O(1))
        // On utilise le usize issu du SymbolId pour un accès direct au vecteur.
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct mutable par index au sein de l'entrée
        entry
            .declarations_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, d)| d)
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
        // 1. On interroge le cache global des usages (NodeId -> SymbolRef)
        // C'est la seule opération de hachage restante.
        let s_ref = self.usages_index.get(&node_id)?;

        // 2. On récupère l'entrée du symbole correspondante par index direct (O(1))
        // On convertit le SymbolId en usize pour l'accès au Vec.
        let entry = self.symbols.get(s_ref.id().as_usize())?;

        // 3. Accès direct par index dans la collection interne de l'entrée
        // .get_index() sur une IndexMap (ou un Vec) est une opération en temps constant.
        entry.usages().get_index(s_ref.index()).map(|(_, u)| u)
    }

    pub fn try_get_usage(&self, node_id: NodeId) -> Result<&Usage, SymbolTableError> {
        // 1. Recherche dans le cache global (O(1) via NodeId)
        let s_ref = self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée du symbole par son index numérique (O(1))
        // L'ID est directement converti en usize pour pointer dans le Vec.
        let entry = self
            .symbols
            .get(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct à l'usage par son index au sein de l'entrée
        entry
            .usages()
            .get_index(s_ref.index())
            .map(|(_, u)| u)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))
    }

    pub fn try_get_usage_mut(&mut self, node_id: NodeId) -> Result<&mut Usage, SymbolTableError> {
        // 1. Recherche dans le cache (on déréférence s_ref pour libérer l'emprunt sur le cache)
        let s_ref = *self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée mutable par index direct (O(1))
        let entry = self
            .symbols
            .get_mut(s_ref.id().as_usize())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès indexé mutable au sein de l'entrée
        entry
            .usages_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, u)| u)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))
    }

    /// Returns an iter over all symbols in the table as immutable references.
    ///
    /// # Returns
    /// An iter yielding references to each `SymbolEntry` in the table.
    pub fn values(&self) -> impl Iterator<Item = &SymbolEntry> {
        // Sur un Vec, .iter() remplace .values() des Maps.
        // C'est un passage en revue linéaire de la mémoire, très rapide.
        self.symbols.iter()
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

            for decl in entry.declarations().values() {
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
    fn rebuild_caches(&mut self) {
        self.declarations_index.clear();
        self.usages_index.clear();

        for entry in self.symbols.iter() {
            let symbol_id = entry.id();

            // Ré-indexation des déclarations
            for (idx, (node_id, _)) in entry.declarations().iter().enumerate() {
                self.declarations_index
                    .insert(*node_id, SymbolRef::new(symbol_id, idx));
            }

            // Ré-indexation des usages
            for (idx, (node_id, _)) in entry.usages().iter().enumerate() {
                self.usages_index
                    .insert(*node_id, SymbolRef::new(symbol_id, idx));
            }
        }
    }
}

impl RemapSymbol for SymbolTable {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // --- ÉTAPE 1 : Remap récursif (Utilise ta fonction SymbolEntry::remap_symbol) ---
        // On modifie les données "internes" (ident, types dans decls, etc.)
        // AVANT de déplacer les entrées dans le nouveau vecteur.
        for entry in self.symbols.iter_mut() {
            entry.remap_symbol(map)?;
        }

        // --- ÉTAPE 2 : Reconstruction du stockage (Vec) ---
        let max_id = map.values().map(|id| id.as_usize()).max().unwrap_or(0);
        let mut new_symbols = Vec::with_capacity(max_id + 1);

        for i in 0..=max_id {
            new_symbols.push(SymbolEntry::new(SymbolId::from(i)));
        }

        let old_symbols = std::mem::take(&mut self.symbols);
        for old_entry in old_symbols {
            // Note : old_entry.ident est déjà mis à jour par l'étape 1 !
            let new_id = old_entry.id();

            // On vérifie si ce symbole est conservé dans le nouvel interner
            // (Si l'ID a changé ou est resté le même, il doit être dans la map)
            if map.contains_key(&new_id) || map.values().any(|&v| v == new_id) {
                new_symbols[new_id.as_usize()] = old_entry;
            }
        }
        self.symbols = new_symbols;

        // --- ÉTAPE 3 : Reconstruction des index (Caches O(1)) ---
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
