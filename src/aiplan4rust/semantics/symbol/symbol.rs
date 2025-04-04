use crate::aiplan4rust::semantics::symbol::{Declaration, Usage};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

/// Represents a symbol in a given context, with its associated declarations and usages.
///
/// The `Symbol` struct is used to store the name, declarations, and usages of a symbol within a domain.
/// It provides functionality for adding declarations and usages, checking for uniqueness,
/// and formatting the symbol's name.
///
/// # Fields
///
/// - `name`: The unique name of the symbol. This name is used to identify the symbol in the system.
/// - `declarations`: A vector of declarations where the symbol is declared. A symbol can have multiple declarations.
/// - `usages`: A vector of usages of the symbol in various parts of the system. A symbol can be used in many places.
///
/// # Methods
///
/// - `new`: Creates a new `Symbol` instance with a given name.
/// - `name`: Returns the unique name of the symbol.
/// - `declarations`: Returns a reference to the list of declarations of the symbol.
/// - `usages`: Returns a reference to the list of usages of the symbol.
/// - `add_declaration`: Adds a new declaration for the symbol if it does not already exist.
/// - `add_usage`: Adds a new usage for the symbol if it does not already exist.
/// - `get_formatted_name`: Returns the part of the name before the first '/' character, if it exists.
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
    name: String,

    /// The AST node where the symbol is declared (only once).
    declarations: Vec<Declaration>,

    /// The list of AST nodes where the symbol is used.
    usages: Vec<Usage>,
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
    /// Returns a new `Symbol` instance with the specified name, and empty declarations and usages lists.
    pub fn new(name: &str) -> Self {
        Symbol {
            name: name.to_string(),
            declarations: Vec::new(),
            usages: Vec::new(),
        }
    }

    /// Returns the name of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `String` containing the symbol's name.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns the list of declarations for the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the list of `Declaration` objects.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Returns the list of usages for the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the list of `Usage` objects.
    pub fn usages(&self) -> &[Usage] {
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
        let mut added = false;
        if !self.declarations.contains(&declaration) {
            self.declarations.push(declaration);
            added = true;
        }
        added
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
        let mut added = false;
        if !self.usages.contains(&usage) {
            self.usages.push(usage);
            added = true;
        }
        added
    }

    /// Returns the part of the name before the first '/' character, if it exists.
    ///
    /// # Returns
    ///
    /// A string slice containing the part of the name before the first '/' character,
    /// or the full name if no '/' is present.
    pub fn get_formatted_name(&self) -> &str {
        self.name
            .split_once('/')
            .map(|(before, _)| before)
            .unwrap_or(&self.name)
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
