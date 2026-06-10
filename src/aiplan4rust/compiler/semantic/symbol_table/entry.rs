//! This module defines the `SymbolEntry` struct, which represents a symbol in a syntax domain context.
//!
//! A `SymbolEntry` tracks the symbol's unique identifier (`Ident`), all its declarations, and usages within a domain or problem.
//! It provides methods to add declarations/usages, merge symbols, remap identifiers, and format output for debugging or display purposes.
//!
//! The module leverages hash sets to ensure uniqueness of declarations and usages, and supports serialization via Serde.
//! It integrates with a string interner for efficient symbol name handling.

use crate::aiplan4rust::compiler::semantic::symbol::Declaration;
use crate::aiplan4rust::compiler::semantic::symbol::Usage;
use crate::aiplan4rust::support::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::support::lang::{RemapSymbol, SymbolId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// The unique identifier of the symbol.
    ident: SymbolId,

    /// The set of declarations where this symbol is introduced.
    declarations: Vec<Declaration>,

    /// The set of usages where this symbol is referenced.
    usages: Vec<Usage>,
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
            declarations: Vec::with_capacity(16),
            usages: Vec::with_capacity(64),
        }
    }

    /// Returns the symbol's identifier.
    ///
    /// # Returns
    ///
    /// The unique identifier of the symbol.
    pub fn id(&self) -> SymbolId {
        self.ident
    }

    /// Returns a reference to the set of declarations of this symbol.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Returns a mutable reference to the set of declarations.
    pub fn declarations_mut(&mut self) -> &mut Vec<Declaration> {
        &mut self.declarations
    }

    /// Returns a reference to the set of usages of this symbol.
    pub fn usages(&self) -> &[Usage] {
        &self.usages
    }

    /// Returns a reference to the set of usages of this symbol.
    pub fn usages_mut(&mut self) -> &mut Vec<Usage> {
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
    pub fn add_declaration(&mut self, declaration: Declaration) -> usize {
        let index = self.declarations.len();
        self.declarations.push(declaration);
        index
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
    pub fn add_usage(&mut self, usage: Usage) -> usize {
        let index = self.usages.len();
        self.usages.push(usage);

        index
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
    /*pub fn merge_with(&mut self, other: SymbolEntry) -> bool {
        // 1. Vérification d'identité
        if self.ident != other.ident {
            return false;
        }

        // 2. Fusion des déclarations
        for decl in other.declarations {
            if !self
                .declarations
                .iter()
                .any(|d| d.source() == decl.source())
            {
                self.declarations.push(decl);
            }
        }

        // 3. Fusion des usages
        for usage in other.usages {
            if !self.usages.iter().any(|u| u.source() == usage.source()) {
                self.usages.push(usage);
            }
        }

        true
    }*/

    pub fn merge_with(&mut self, mut other: SymbolEntry) -> bool {
        if self.ident != other.ident {
            return false;
        }

        // 1. Fusion des déclarations : On déplace tout d'un coup (0 allocation supplémentaire)
        self.declarations.append(&mut other.declarations);
        // On trie par source et on supprime les doublons consécutifs
        self.declarations.sort_unstable_by_key(|d| d.source());
        self.declarations.dedup_by(|a, b| a.source() == b.source());

        // 2. Fusion des usages : Même chose, traitement de masse en bloc
        self.usages.append(&mut other.usages);
        self.usages.sort_unstable_by_key(|u| u.source());
        self.usages.dedup_by(|a, b| a.source() == b.source());

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

        // 2. Remap les identifiants dans les déclarations
        // On itère simplement sur le Vec en mode mutable
        for decl in &mut self.declarations {
            decl.remap_symbol(map)?;
        }

        // 3. Remap les identifiants dans les usages
        for usage in &mut self.usages {
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
        // On itère directement sur le Vec<Declaration>
        for decl in &self.declarations {
            writeln!(f, "   - {}", decl)?;
        }

        writeln!(f, " - Usages ({}):", self.usages.len())?;
        // On itère directement sur le Vec<Usage>
        for usage in &self.usages {
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

        // 2. Affichage des déclarations
        // On itère directement sur le Vec<Declaration>
        writeln!(w, " - Declarations ({}):", self.declarations.len())?;
        for decl in &self.declarations {
            write!(w, "   - ")?;
            decl.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        // 3. Affichage des usages
        // On itère directement sur le Vec<Usage>
        writeln!(w, " - Usages ({}):", self.usages.len())?;
        for usage in &self.usages {
            write!(w, "   - ")?;
            usage.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        Ok(())
    }
}
