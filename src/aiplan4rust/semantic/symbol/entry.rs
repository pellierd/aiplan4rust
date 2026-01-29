//! This module defines the `SymbolEntry` struct, which represents a symbol in a syntax domain context.
//!
//! A `SymbolEntry` tracks the symbol's unique identifier (`Ident`), all its declarations, and usages within a domain or problem.
//! It provides methods to add declarations/usages, merge symbols, remap identifiers, and format output for debugging or display purposes.
//!
//! The module leverages hash sets to ensure uniqueness of declarations and usages, and supports serialization via Serde.
//! It integrates with a string interner for efficient symbol name handling.

use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::lang::{StringID, RemapIdents};
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// The unique identifier of the symbol.
    ident: StringID,

    /// The set of declarations where this symbol is introduced.
    declarations: HashSet<Declaration>,

    /// The set of usages where this symbol is referenced.
    usages: HashSet<Usage>,
}

impl Hash for SymbolEntry {
    /// Computes the hash of the symbol based solely on its `ident`.
    ///
    /// This ensures that symbols with the same identifier hash identically,
    /// regardless of their declarations or usages.
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
    pub fn new(ident: StringID) -> Self {
        SymbolEntry {
            ident,
            declarations: HashSet::new(),
            usages: HashSet::new(),
        }
    }

    /// Returns the symbol's identifier.
    ///
    /// # Returns
    ///
    /// The unique identifier of the symbol.
    pub fn ident(&self) -> StringID {
        self.ident
    }

    /// Returns a reference to the set of declarations of this symbol.
    pub fn declarations(&self) -> &HashSet<Declaration> {
        &self.declarations
    }

    /// Returns a mutable reference to the set of declarations.
    pub fn declarations_mut(&mut self) -> &mut HashSet<Declaration> {
        &mut self.declarations
    }

    /// Returns a reference to the set of usages of this symbol.
    pub fn usages(&self) -> &HashSet<Usage> {
        &self.usages
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
        self.declarations.insert(declaration)
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
        self.usages.insert(usage)
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

impl RemapIdents for SymbolEntry {
    /// Remaps identifiers in this symbol entry, including the main symbol, its
    /// declarations, and all usages, according to the provided mapping.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    /// This is useful for renaming or aliasing symbols consistently, e.g.,
    /// after type flattening or interner merging.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any declaration or usage cannot be remapped
    /// according to the given map.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        // Remap main symbol identifier
        if let Some(new_ident) = map.get(&self.ident) {
            self.ident = *new_ident;
        }

        // Remap identifiers in declarations
        let mut new_declarations = HashSet::with_capacity(self.declarations.len());
        for mut decl in self.declarations.drain() {
            decl.remap_idents(map)?;
            new_declarations.insert(decl);
        }
        self.declarations = new_declarations;

        // Remap identifiers in usages
        let mut new_usages = HashSet::with_capacity(self.usages.len());
        for mut usage in self.usages.drain() {
            usage.remap_idents(map)?;
            new_usages.insert(usage);
        }
        self.usages = new_usages;

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
        for decl in &self.declarations {
            writeln!(f, "   - {}", decl)?;
        }

        writeln!(f, " - Usages ({}):", self.usages.len())?;
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
        interner: &StringInterner,
    ) -> fmt::Result {
        match interner.resolve_ident(self.ident) {
            Some(name) => writeln!(w, "[Symbol: '{}']", name)?,
            None => writeln!(w, "[Symbol: <uninterned:{}>]", self.ident)?,
        }

        writeln!(w, " - Declarations ({}):", self.declarations.len())?;
        for decl in &self.declarations {
            write!(w, "   - ")?;
            decl.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        writeln!(w, " - Usages ({}):", self.usages.len())?;
        for usage in &self.usages {
            write!(w, "   - ")?;
            usage.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        Ok(())
    }
}
