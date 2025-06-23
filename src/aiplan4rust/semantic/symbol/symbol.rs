use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::interner::StringInterner;

use serde::Deserialize;
use serde::Serialize;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;


/// Represents a symbol in a given context, with its associated declarations and usages.
///
/// The `Symbol` struct is used to store the name, declarations, and usages of a symbol within a
/// domain. It provides functionality for adding declarations and usages, checking for uniqueness,
/// and formatting the symbol's name.
///
/// # Fields
///
/// - `name`: The unique name of the symbol. This name is used to identify the symbol in the system.
/// - `declarations`: A vector of declarations where the symbol is declared. A symbol can have
///   multiple declarations.
/// - `usages`: A vector of usages of the symbol in various parts of the system. A symbol can be
///   used in many places.
///
/// # Methods
///
/// - `new`: Creates a new `Symbol` instance with a given name.
/// - `name`: Returns the unique name of the symbol.
/// - `declarations`: Returns a reference to the list of declarations of the symbol.
/// - `usages`: Returns a reference to the list of usages of the symbol.
/// - `add_declaration`: Adds a new declaration for the symbol if it does not already exist.
/// - `add_usage`: Adds a new usage for the symbol if it does not already exist.
/// - `get_formatted_name`: Returns the part of the name before the first '/' character, if it
///   exists.
///
/// # Example
///
/// ```rust
/// let mut symbol = Symbol::new("exampleSymbol");
/// symbol.add_ (declaration);
/// symbol.add_usage(usage);
/// println!("{}", symbol);
/// ```

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    /// The unique name of the symbol.
    name: Ident,

    /// The AST node where the symbol is declared (only once).
    declarations: HashSet<Declaration>,

    /// The list of AST nodes where the symbol is used.
    usages: HashSet<Usage>,
}

// Manually implement the `Hash` trait for `Symbol`, using only the `name` field.
impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state); // Only hash the `name` field
    }
}

impl Symbol {
    /// Creates a new `Symbol` with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the name of the symbol.
    ///
    /// # Returns
    ///
    /// Returns a new `Symbol` instance with the specified name, and empty declarations and usages
    /// lists.
    pub fn new(name: Ident) -> Self {
        Symbol {
            name: name,
            declarations: HashSet::new(),
            usages: HashSet::new(),
        }
    }

    /// Returns the name of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `String` containing the symbol's name.
    pub fn name(&self) -> Ident {
        self.name
    }

    /// Returns the list of declarations for the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the list of `Declaration` objects.
    pub fn declarations(&self) -> &HashSet<Declaration> {
        &self.declarations
    }

    /// Returns a mutable reference to the list of declarations for the symbol.
    pub fn declarations_mut(&mut self) -> &mut HashSet<Declaration> {
        &mut self.declarations
    }

    /// Returns the list of usages for the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the list of `Usage` objects.
    pub fn usages(&self) -> &HashSet<Usage> {
        &self.usages
    }

    /// Adds a declaration for the symbol if it is not already present.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The `Declaration` to be added.
    ///
    /// # Returns
    ///
    /// `true` if the declaration was added, `false` if it was already present.
    pub fn add_declaration(&mut self, declaration: Declaration) -> bool {
        self.declarations.insert(declaration)
    }

    /// Adds a usage for the symbol if it is not already present.
    ///
    /// # Arguments
    ///
    /// * `usage` - The `Usage` to be added.
    ///
    /// # Returns
    ///
    /// `true` if the usage was added, `false` if it was already present.
    pub fn add_usage(&mut self, usage: Usage) -> bool {
        self.usages.insert(usage)
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Remap du nom principal
        if let Some(new_name) = map.get(&self.name) {
            self.name = new_name.clone();
        }

        // Extraire, modifier et reconstruire declarations
        self.declarations = self.declarations.drain()
            .map(|mut decl| {
                decl.remap_idents(map);
                decl
            })
            .collect();

        // Extraire, modifier et reconstruire usages
        self.usages = self.usages.drain()
            .map(|mut usage| {
                usage.remap_idents(map);
                usage
            })
            .collect();
    }

    /// Attempts to merge another symbol into this one by combining declarations and usages.
    ///
    /// Returns `true` if the symbols had the same name and were merged successfully.
    /// Returns `false` if the symbol names differ and the merge was not performed.
    pub fn merge_with(&mut self, other: Symbol) -> bool {
        if self.name != other.name {
            return false;
        }
        self.declarations.extend(other.declarations);
        self.usages.extend(other.usages);
        true
    }

    /// Retourne la représentation en `String` (prête pour `println!`)
    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    /// Formats the symbol information along with its declarations and usages,
    /// resolving interned identifiers via the provided `StringInterner`.
    ///
    /// This method writes a human-readable representation of the symbol, including
    /// its name, declarations, and usages, to the given formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - Formatter to write the output to.
    /// * `interner` - The `StringInterner` used to resolve identifier names within declarations and usages.
    ///
    /// # Example output
    ///
    /// ```text
    /// [Symbol: 'move']
    ///  - Declarations (2):
    ///    - Declaration details here...
    ///    - Declaration details here...
    ///  - Usages (3):
    ///    - Usage details here...
    ///    - Usage details here...
    ///    - Usage details here...
    /// ```
    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        // Display the symbol name
        match interner.resolve(self.name) {
            Some(name) => writeln!(w, "[Symbol: '{}']", name)?,
            None => writeln!(w, "[Symbol: <uninterned:{}>]", self.name)?,
        }

        // Display declarations
        writeln!(w, " - Declarations ({}):", self.declarations.len())?;
        for decl in &self.declarations {
            write!(w, "   - ")?;
            decl.fmt_with_interner(w, interner)?; // Appel direct à la méthode qui écrit dans `f`
            writeln!(w)?; // fin de ligne
        }

        // Display usages
        writeln!(w, " - Usages ({}):", self.usages.len())?;
        for usage in &self.usages {
            write!(w, "   - ")?;
            usage.fmt_with_interner(w, interner)?;
            writeln!(w)?;
        }

        Ok(())
    }

}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[Symbol: \'{}\']", self.name)?;

        // Display declarations
        writeln!(f, " - Declarations ({}):", self.declarations.len())?;
        for decl in &self.declarations {
            writeln!(f, "   - {}", decl)?;
        }

        // Display usages
        writeln!(f, " - Usages ({}):", self.usages.len())?;
        for usage in &self.usages {
            writeln!(f, "   - {}", usage)?;
        }

        Ok(())
    }
}
