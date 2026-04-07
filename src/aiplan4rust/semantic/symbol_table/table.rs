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
    pub(super) symbols: LinkedHashMap<SymbolId, SymbolEntry>,

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
            symbols: LinkedHashMap::new(),

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
    /// Creates a new, empty `SymbolTable` with default values.
    ///
    /// # Returns
    /// A new `SymbolTable` instance with no symbols and a default origin.
    pub fn new() -> Self {
        Table::default()
    }

    pub fn keys(&self) -> impl Iterator<Item = &SymbolId> {
        self.symbols.keys()
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

    /// Returns an iter over all symbols in the table as immutable references.
    ///
    /// # Returns
    /// An iter yielding (`&Ident`, `&SymbolEntry`) pairs for all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&SymbolId, &SymbolEntry)> {
        self.symbols.iter()
    }

    /// Retrieves an immutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `name`: The identifier (`Ident`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a reference to the symbol, or `None` if not found.
    pub fn get_symbol(&self, name: SymbolId) -> Option<&SymbolEntry> {
        self.symbols.get(&name)
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
        self.symbols
            .get(&id)
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))
    }

    /*/// Retrieves a mutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `name`: The identifier (`Ident`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a mutable reference to the symbol, or `None` if not found.
    pub fn get_symbol_mut(&mut self, name: SymbolId) -> Option<&mut SymbolEntry> {
        self.symbols.get_mut(&name)
    }*/

    pub fn add_declaration(
        &mut self,
        id: SymbolId,
        decl: Declaration,
    ) -> Result<(), SymbolTableError> {
        let node_id = decl.source();

        // 1. GARDE D'IDEMPOTENCE
        // Si ce NodeId est déjà dans le cache, on ne fait rien (évite E2011).
        if self.declarations_index.contains_key(&node_id) {
            return Ok(());
        }

        // 2. RÉCUPÉRATION OU CRÉATION (Entry API pour la performance)
        let entry = self
            .symbols
            .entry(id)
            .or_insert_with(|| SymbolEntry::new(id));

        // 3. INDEXATION & INSERTION
        // L'index est la taille actuelle avant l'ajout
        let index = entry.declarations().len();
        entry.add_declaration(decl);

        // 4. MISE À JOUR DU CACHE
        self.declarations_index
            .insert(node_id, SymbolRef::new(id, index));

        Ok(())
    }

    /// Lie un usage à une déclaration de manière atomique et ultra-rapide.
    /// À utiliser dans apply_resolutions pour éviter les lookups manuels.
    pub fn link_resolution(&mut self, usage_id: NodeId, decl_id: NodeId, status: MatchResult) {
        // 1. On récupère les refs du cache (C'est du O(1) pur)
        let u_ref = self.usages_index.get(&usage_id).copied();
        let d_ref = self.declarations_index.get(&decl_id).copied();

        // 2. Mise à jour de l'Usage (Usage -> Decl)
        if let Some(r) = u_ref {
            if let Some(entry) = self.symbols.get_mut(&r.id()) {
                if let Some(u) = entry.usages_mut().get_index_mut(r.index()).map(|(_, v)| v) {
                    u.set_declaration(decl_id);
                    u.set_resolution(status);
                }
            }
        }

        // 3. Mise à jour de la Déclaration (Decl -> Usage)
        // On ne le fait que si decl_id n'est pas un ID virtuel (ex: built-ins)
        if let Some(r) = d_ref {
            if let Some(entry) = self.symbols.get_mut(&r.id()) {
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
        // 1. On extrait la référence (SymbolId + Index) via l'ancien ID
        // On utilise .remove() car l'ancien ID (la coquille vide) n'existera plus
        let s_ref = self
            .declarations_index
            .remove(&old_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(old_id))?;

        // 2. On réinsère immédiatement la même référence sous le nouvel ID
        // L'index reste identique, donc les autres SymbolRef ne sont pas impactés
        self.declarations_index.insert(new_id, s_ref);

        // 3. On récupère la déclaration mutable pour la retourner
        let entry = self
            .symbols
            .get_mut(&s_ref.id())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        let decl = entry
            .declarations_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, d)| d)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(new_id))?;

        // 4. Mise à jour de l'ID source interne à la déclaration
        decl.set_source(new_id);

        Ok(decl)
    }

    pub fn get_declaration(&self, node_id: NodeId) -> Option<&Declaration> {
        // 1. Accès direct au cache via le NodeId
        let s_ref = self.declarations_index.get(&node_id)?;

        // 2. Récupération de l'entrée du symbole (O(1))
        let entry = self.symbols.get(&s_ref.id())?;

        // 3. Accès direct à la déclaration par son index (O(1))
        // On utilise get_index de IndexMap (ou l'équivalent sur ta collection)
        entry
            .declarations()
            .get_index(s_ref.index())
            .map(|(_, d)| d)
    }

    pub fn try_get_declaration(&self, node_id: NodeId) -> Result<&Declaration, SymbolTableError> {
        // 1. Recherche dans le cache global
        let s_ref = self
            .declarations_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée (devrait toujours exister si le cache est cohérent)
        let entry = self
            .symbols
            .get(&s_ref.id())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct par index
        entry
            .declarations()
            .get_index(s_ref.index())
            .map(|(_, d)| d)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn get_declaration_mut(&mut self, node_id: NodeId) -> Option<&mut Declaration> {
        // 1. Accès au cache (lecture seule ici, donc .get suffit)
        let s_ref = *self.declarations_index.get(&node_id)?;

        // 2. Récupération mutable de l'entrée du symbole
        let entry = self.symbols.get_mut(&s_ref.id())?;

        // 3. Accès direct mutable par index
        // Note : get_index_mut est la méthode correspondante dans IndexMap
        entry
            .declarations_mut() // Assure-toi que cette méthode existe et renvoie &mut IndexMap
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

        // 2. Récupération mutable de l'entrée du symbole
        let entry = self
            .symbols
            .get_mut(&s_ref.id())
            .ok_or(SymbolTableError::SymbolNotFound(s_ref.id()))?;

        // 3. Accès direct mutable par index
        entry
            .declarations_mut()
            .get_index_mut(s_ref.index())
            .map(|(_, d)| d)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn add_usage(&mut self, id: SymbolId, usage: Usage) -> Result<(), SymbolTableError> {
        let node_id = usage.source();

        // 1. GARDE D'IDEMPOTENCE
        if self.usages_index.contains_key(&node_id) {
            return Ok(());
        }

        // 2. RÉCUPÉRATION OU CRÉATION
        let entry = self
            .symbols
            .entry(id)
            .or_insert_with(|| SymbolEntry::new(id));

        // 3. INDEXATION & INSERTION
        let index = entry.usages().len();
        entry.add_usage(usage);

        // 4. MISE À JOUR DU CACHE
        self.usages_index.insert(node_id, SymbolRef::new(id, index));

        Ok(())
    }

    pub fn get_usage(&self, node_id: NodeId) -> Option<&Usage> {
        // 1. On interroge le cache global des usages via le NodeId
        let s_ref = self.usages_index.get(&node_id)?;

        // 2. On récupère l'entrée du symbole correspondante
        let entry = self.symbols.get(&s_ref.id())?;

        // 3. Accès direct par index (Performance maximale)
        // .get_index() provient de IndexMap, c'est une opération en temps constant.
        entry.usages().get_index(s_ref.index()).map(|(_, u)| u)
    }

    pub fn try_get_usage(&self, node_id: NodeId) -> Result<&Usage, SymbolTableError> {
        // 1. Recherche dans le cache global (O(1))
        let s_ref = self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée du symbole
        let entry = self
            .symbols
            .get(&s_ref.id())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès direct à l'usage par son index
        entry
            .usages()
            .get_index(s_ref.index())
            .map(|(_, u)| u)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))
    }

    pub fn try_get_usage_mut(&mut self, node_id: NodeId) -> Result<&mut Usage, SymbolTableError> {
        // 1. Recherche dans le cache avec erreur spécifique
        let s_ref = *self
            .usages_index
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::uage_not_found_for_node(node_id))?;

        // 2. Récupération de l'entrée mutable
        let entry = self
            .symbols
            .get_mut(&s_ref.id())
            .ok_or_else(|| SymbolTableError::symbol_not_found(s_ref.id()))?;

        // 3. Accès indexé mutable
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
        self.symbols.values()
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
        // 1. Accès direct à l'usage via le cache O(1)
        // On ne cherche plus l'Entry, on va direct à l'usage
        let usage = self.try_get_usage(usage_node_id)?;

        // 2. Récupération de l'ID de la déclaration liée
        let decl_node_id = usage
            .declaration()
            .ok_or_else(|| SymbolTableError::unresolved_usage(usage.symbol_id(), usage_node_id))?;

        // 3. Accès direct à la déclaration finale via le cache O(1)
        // Zéro recherche linéaire, zéro unwrap risqué
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
        let mut hierarchy_map: HashMap<SymbolId, Vec<SymbolId>> =
            HashMap::with_capacity(self.symbols.len() / 4);

        for (&id, entry) in self.symbols.iter() {
            for decl in entry.declarations().values() {
                if decl.kind() == SymbolKind::PrimitiveType {
                    // Étape 1 : On récupère les parents s'ils existent
                    if let Some(ty) = decl.ty() {
                        let parents = ty.members();

                        // Étape 2 : On insère les parents pour l'enfant actuel
                        // On ne stocke pas le résultat de .entry() dans une variable longue durée
                        hierarchy_map
                            .entry(id)
                            .or_default()
                            .extend(parents.iter().cloned());

                        // Étape 3 : On s'assure que chaque parent a sa propre entrée
                        for &parent_id in parents {
                            hierarchy_map.entry(parent_id).or_default();
                        }
                    } else {
                        // Si pas de parents, on s'assure quand même que le type existe (root)
                        hierarchy_map.entry(id).or_default();
                    }
                }
            }
        }
        TypeHierarchy::new(hierarchy_map)
    }
}

impl RemapSymbol for SymbolTable {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // --- ÉTAPE 1 : Remap interne des SymbolEntry (Déclarations/Usages internes) ---
        for (_key, symbol) in self.symbols.iter_mut() {
            symbol.remap_symbol(map)?;
        }

        // --- ÉTAPE 2 : Reconstruction de la map principale (symbols) ---
        let mut new_symbols = LinkedHashMap::with_capacity(self.symbols.len());

        for (key, symbol) in std::mem::take(&mut self.symbols) {
            let new_key = map
                .get(&key)
                .cloned()
                .ok_or_else(|| InternerError::missing_ident(key))?;

            if new_symbols.contains_key(&new_key) {
                return Err(InternerError::conflict(new_key))?;
            }
            new_symbols.insert(new_key, symbol);
        }
        self.symbols = new_symbols;

        // --- ÉTAPE 3 : Synchronisation des Caches O(1) ---
        // On met à jour le SymbolId stocké dans chaque SymbolRef

        // Mise à jour des Déclarations
        for s_ref in self.declarations_index.values_mut() {
            // s_ref.id est un SymbolId, on peut appeler ta méthode dessus
            // (Assure-toi que le champ 'id' est accessible ou via un getter mut)
            s_ref.remap_id(map)?;
        }

        // Mise à jour des Usages
        for s_ref in self.usages_index.values_mut() {
            s_ref.remap_id(map)?;
        }

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
        // Iterate over all symbols stored in the symbol table.
        for symbol in self.symbols.values() {
            // Write each symbol followed by a newline into the formatter.
            // The `?` operator propagates any error that might occur.
            writeln!(f, "{}", symbol)?;
        }
        // Return Ok to indicate successful formatting.
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
        for symbol in self.symbols.values() {
            symbol.fmt_with_interner(f, interner)?;
            writeln!(f)?;
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
