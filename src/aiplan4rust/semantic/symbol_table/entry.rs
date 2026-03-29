//! This module defines the `SymbolEntry` struct, which represents a symbol in a syntax domain context.
//!
//! A `SymbolEntry` tracks the symbol's unique identifier (`Ident`), all its declarations, and usages within a domain or problem.
//! It provides methods to add declarations/usages, merge symbols, remap identifiers, and format output for debugging or display purposes.
//!
//! The module leverages hash sets to ensure uniqueness of declarations and usages, and supports serialization via Serde.
//! It integrates with a string interner for efficient symbol name handling.

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::tree::NodeId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Represents a symbol in a given context, with associated declarations and usages.
///
/// A symbol is identified by a unique identifier (`Ident`) and maintains collections of
/// declarations and usages in the domain. Declarations represent where the symbol
/// is introduced, and usages represent where it is referenced.
///
/// This struct supports adding declarations and usages (ensuring uniqueness), merging
/// with other symbols of the same identifier, remapping identifiers, and formatted display.
///
/// # Fields
///
/// - `ident`: The unique identifier of the symbol.
/// - `declarations`: The set of declarations for this symbol.
/// - `usages`: The set of usages of this symbol.
///
/// # Examples
///
/// ```rust
/// let mut symbol = SymbolEntry::new("example".into());
/// // add declarations and usages...
/// println!("{}", symbol);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// The unique identifier of the symbol.
    ident: SymbolId,

    /// The set of declarations where this symbol is introduced.
    declarations: IndexMap<NodeId, Declaration>,

    /// The set of usages where this symbol is referenced.
    usages: IndexMap<NodeId, Usage>,
}

impl PartialEq for SymbolEntry {
    /// Deux entrées sont égales si elles ont le même identifiant.
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}

impl Eq for SymbolEntry {} // Eq n'a pas de méthodes, c'est un marqueur

impl Hash for SymbolEntry {
    /// Hash basé uniquement sur l'identifiant pour rester cohérent avec PartialEq.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ident.hash(state);
    }
}
impl SymbolEntry {
    /// Creates a new `SymbolEntry` with the specified identifier.
    ///
    /// # Arguments
    ///
    /// * `ident` - The unique identifier for the symbol.
    ///
    /// # Returns
    ///
    /// A new `SymbolEntry` instance with empty declarations and usages.
    pub fn new(ident: SymbolId) -> Self {
        SymbolEntry {
            ident,
            declarations: IndexMap::new(),
            usages: IndexMap::new(),
        }
    }

    /// Returns the symbol's identifier.
    ///
    /// # Returns
    ///
    /// The unique identifier of the symbol.
    pub fn ident(&self) -> SymbolId {
        self.ident
    }

    /// Returns a reference to the set of declarations of this symbol.
    pub fn declarations(&self) -> &IndexMap<NodeId, Declaration> {
        &self.declarations
    }

    /// Returns a mutable reference to the set of declarations.
    pub fn declarations_mut(&mut self) -> &mut IndexMap<NodeId, Declaration> {
        &mut self.declarations
    }

    /// Retourne une référence à la déclaration pour un NodeId donné,
    /// ou une erreur si elle n'existe pas.
    pub fn try_get_declaration(&self, node_id: NodeId) -> Result<&Declaration, SymbolTableError> {
        self.declarations
            .get(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    /// Retourne une Option sur la déclaration.
    pub fn get_declaration(&self, node_id: NodeId) -> Option<&Declaration> {
        self.declarations.get(&node_id)
    }

    /// Returns a reference to the set of usages of this symbol.
    pub fn usages(&self) -> &IndexMap<NodeId, Usage> {
        &self.usages
    }

    /// Returns a reference to the set of usages of this symbol.
    pub fn usages_mut(&mut self) -> &mut IndexMap<NodeId, Usage> {
        &mut self.usages
    }

    /// Adds a new declaration for this symbol, if it is not already present.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The declaration to add.
    ///
    /// # Returns
    ///
    /// `true` if the declaration was added (was not present before), otherwise `false`.
    pub fn add_declaration(&mut self, declaration: Declaration) -> bool {
        // Puisque la clé est le NodeId de la déclaration,
        // on l'extrait pour l'insertion dans l'IndexMap.
        let node_id = declaration.source();

        // insert() renvoie Some(old_value) si la clé existait déjà.
        // On retourne true seulement si le résultat est None (nouvelle insertion).
        self.declarations.insert(node_id, declaration).is_none()
    }

    /// Adds a new usage of this symbol, if it is not already present.
    ///
    /// # Arguments
    ///
    /// * `usage` - The usage to add.
    ///
    /// # Returns
    ///
    /// `true` if the usage was added (was not present before), otherwise `false`.
    pub fn add_usage(&mut self, usage: Usage) -> bool {
        // On utilise le NodeId comme clé unique pour l'usage dans la map
        let node_id = usage.source();

        // insert() renvoie Some(old_usage) si l'ID existait déjà.
        // On retourne true seulement si l'insertion est nouvelle (None).
        self.usages.insert(node_id, usage).is_none()
    }

    /// Merges another `SymbolEntry` into this one by combining declarations and usages.
    ///
    /// Only merges if both symbols have the same identifier.
    ///
    /// # Arguments
    ///
    /// * `other` - The other symbol to merge.
    ///
    /// # Returns
    ///
    /// `true` if merged successfully, `false` if identifiers differ and no merge was performed.
    pub fn merge_with(&mut self, other: SymbolEntry) -> bool {
        if self.ident != other.ident {
            return false;
        }
        self.declarations.extend(other.declarations);
        self.usages.extend(other.usages);
        true
    }
}

impl RemapSymbol for SymbolEntry {
    /// Remaps identifiers in this symbol entry, including the main symbol, its
    /// declarations, and all usages, according to the provided mapping.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    /// This is useful for renaming or aliasing symbols consistently, e.g.,
    /// after typing flattening or interner merging.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any declaration or usage cannot be remapped
    /// according to the given map.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // 1. Remap l'identifiant principal du symbole
        if let Some(new_ident) = map.get(&self.ident) {
            self.ident = *new_ident;
        }

        // 2. Remap les identifiants dans les déclarations (Modification en place)
        // On utilise values_mut() car le NodeId (la clé) ne change pas,
        // seul le contenu de la Declaration est modifié.
        for decl in self.declarations.values_mut() {
            decl.remap_symbol(map)?;
        }

        // 3. Remap les identifiants dans les usages (Modification en place)
        for usage in self.usages.values_mut() {
            usage.remap_symbol(map)?;
        }

        Ok(())
    }
}

impl fmt::Display for SymbolEntry {
    /// Formats the symbol entry, listing its ident, declarations, and usages.
    ///
    /// Example output:
    ///
    /// ```text
    /// [Symbol: 'move']
    ///  - Declarations (2):
    ///    - Declaration details...
    ///    - Declaration details...
    ///  - Usages (3):
    ///    - Usage details...
    ///    - Usage details...
    ///    - Usage details...
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[Symbol: '{}']", self.ident)?;

        writeln!(f, " - Declarations ({}):", self.declarations.len())?;
        // On utilise .values() pour ignorer la clé NodeId lors de l'affichage
        for decl in self.declarations.values() {
            writeln!(f, "   - {}", decl)?;
        }

        writeln!(f, " - Usages ({}):", self.usages.len())?;
        for usage in self.usages.values() {
            writeln!(f, "   - {}", usage)?;
        }

        Ok(())
    }
}

impl InternerDisplay for SymbolEntry {
    /// Formats the symbol entry using a `StringInterner` to resolve interned identifiers.
    ///
    /// This method provides a human-readable output of the symbol, its declarations, and usages,
    /// with symbol names resolved to their string forms via the interner.
    ///
    /// # Arguments
    ///
    /// * `w` - The formatter to write to.
    /// * `interner` - The `StringInterner` used to resolve symbol identifiers.
    fn fmt_with_interner(
        &self,
        w: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        // 1. Résolution du nom du symbole via l'interneur
        match interner.resolve_symbol(self.ident) {
            Some(name) => writeln!(w, "[Symbol: '{}']", name)?,
            None => writeln!(w, "[Symbol: <uninterned:{:?}>]", self.ident)?,
        }

        // 2. Affichage des déclarations (via .values() pour ignorer le NodeId)
        writeln!(w, " - Declarations ({}):", self.declarations.len())?;
        for decl in self.declarations.values() {
            write!(w, "   - ")?;
            decl.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        // 3. Affichage des usages (via .values())
        writeln!(w, " - Usages ({}):", self.usages.len())?;
        for usage in self.usages.values() {
            write!(w, "   - ")?;
            usage.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        Ok(())
    }
}
