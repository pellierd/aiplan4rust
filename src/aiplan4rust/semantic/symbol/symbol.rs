use std::collections::HashMap;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Usage;

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use indexmap::IndexSet;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::StringInterner;

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
    declarations: IndexSet<Declaration>,

    /// The list of AST nodes where the symbol is used.
    usages: IndexSet<Usage>,
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
            declarations: IndexSet::new(),
            usages: IndexSet::new(),
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
    pub fn declarations(&self) -> &IndexSet<Declaration> {
        &self.declarations
    }

    /// Returns a mutable reference to the list of declarations for the symbol.
    pub fn declarations_mut(&mut self) -> &mut IndexSet<Declaration> {
        &mut self.declarations
    }

    /// Returns the list of usages for the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the list of `Usage` objects.
    pub fn usages(&self) -> &IndexSet<Usage> {
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
        if let Some(new_name) = map.get(&self.name) {
            self.name = new_name.clone();
        }

        // On reconstruit la collection avec les éléments modifiés
        self.declarations = self.declarations.iter()
            .map(|decl| {
                let mut decl = decl.clone();
                decl.remap_idents(map);
                decl
            })
            .collect();

        self.usages = self.usages.iter()
            .map(|usage| {
                let mut usage = usage.clone();
                usage.remap_idents(map);
                usage
            })
            .collect();
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
        match interner.get_str(self.name) {
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
