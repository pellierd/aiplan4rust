use crate::aiplan4rust::parser::Source;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::symbol::TypedSymbol;

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a declaration in the abstract syntax tree (AST).
///
/// This struct holds information about a symbol declared in the program, including its
/// associated AST node, symbol type, scope, source, and optional types and arguments.
///
/// # Fields
///
/// * `ast` - The AST node index of the declaration.
/// * `kind` - The type or kind of the symbol declared (e.g., variable, function).
/// * `scope` - The scope in which the declaration is valid.
/// * `source` - The source from which the declaration originates (e.g., file or module).
/// * `types` - An optional list of types associated with the symbol, if any. This can be `None` if
///   not provided.
/// * `arguments` - An optional list of argument types, grouped in parameter lists, if applicable.
///
/// # Example
///
/// ```rust
/// let declaration = Declaration {
///     ast: 1,
///     kind: SymbolKind::Variable,
///     scope: Scope::Global,
///     source: Source::File("main.rs".into()),
///     types: Some(vec!["int".into()]),
///     arguments: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Declaration {
    /// The AST node of the declaration
    ast: usize,

    /// The kind of the symbol declared
    kind: SymbolKind,

    /// The scope of the declaration
    scope: Scope,

    /// The source of the declaration
    source: Source,

    /// Optional list of types associated with the symbol.
    types: Option<Vec<String>>,

    /// Optional list of argument types, grouped in parameter lists.
    arguments: Option<Vec<TypedSymbol<String>>>,
}

impl Declaration {
    /// Constructor to create a new `Declaration`
    pub fn new(
        ast: usize,
        kind: SymbolKind,
        scope: Scope,
        source: Source,
        types: Option<Vec<String>>,
        arguments: Option<Vec<TypedSymbol<String>>>,
    ) -> Self {
        Declaration {
            ast,
            kind,
            scope,
            source,
            types,
            arguments,
        }
    }

    /// Accessor for the AST node of the declaration.
    ///
    /// Returns the index of the AST node representing this declaration.
    ///
    /// # Returns
    ///
    /// * `usize` - The AST node index.
    pub fn ast(&self) -> usize {
        self.ast
    }

    /// Accessor for the kind of the symbol declared.
    ///
    /// Returns a reference to the symbol's kind (e.g., variable, function).
    ///
    /// # Returns
    ///
    /// * `&SymbolKind` - A reference to the kind of the symbol.
    pub fn kind(&self) -> &SymbolKind {
        &self.kind
    }

    /// Accessor for the scope of the declaration.
    ///
    /// Returns a reference to the scope in which the declaration is valid.
    ///
    /// # Returns
    ///
    /// * `&Scope` - A reference to the scope of the declaration.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Accessor for the source of the declaration.
    ///
    /// Returns a reference to the source from which the declaration originates
    /// (e.g., file or module).
    ///
    /// # Returns
    ///
    /// * `&Source` - A reference to the source of the declaration.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Accessor for the list of types associated with the symbol.
    ///
    /// Returns an optional reference to a vector of types, if available.
    ///
    /// # Returns
    ///
    /// * `Option<&Vec<String>>` - An optional reference to the list of types.
    pub fn types(&self) -> Option<&Vec<String>> {
        self.types.as_ref()
    }

    /// Accessor for the list of argument types associated with the symbol.
    ///
    /// Returns an optional reference to a vector of argument types, if available.
    ///
    /// # Returns
    ///
    /// * `Option<&Vec<TypedSymbol<String>>>` - An optional reference to the list of argument types.
    pub fn arguments(&self) -> Option<&Vec<TypedSymbol<String>>> {
        self.arguments.as_ref()
    }

    // Mutable accessors

    /// Mutable accessor for the scope of the declaration.
    ///
    /// Returns a mutable reference to the scope, allowing modification.
    ///
    /// # Returns
    ///
    /// * `&mut Scope` - A mutable reference to the scope of the declaration.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut self.scope
    }

    /// Mutable accessor for the list of types associated with the symbol.
    ///
    /// Returns a mutable reference to the vector of types, allowing modification.
    ///
    /// # Returns
    ///
    /// * `Option<&mut Vec<String>>` - A mutable reference to the list of types.
    pub fn types_mut(&mut self) -> Option<&mut Vec<String>> {
        self.types.as_mut()
    }

    /// Mutable accessor for the list of argument types associated with the symbol.
    ///
    /// Returns a mutable reference to the vector of argument types, allowing modification.
    ///
    /// # Returns
    ///
    /// * `Option<&mut Vec<TypedSymbol<String>>>` - A mutable reference to the list of argument types.
    pub fn arguments_mut(&mut self) -> Option<&mut Vec<TypedSymbol<String>>> {
        self.arguments.as_mut()
    }

    // Setters

    /// Setter for the source of the declaration.
    ///
    /// Sets the source of the declaration to the provided value.
    ///
    /// # Arguments
    ///
    /// * `source` - The new source to set for the declaration.
    pub fn set_source(&mut self, source: Source) {
        self.source = source;
    }

    /// Setter for the list of types associated with the symbol.
    ///
    /// Sets the types of the symbol to the provided list of types.
    ///
    /// # Arguments
    ///
    /// * `types` - The list of types to set for the symbol.
    pub fn set_types(&mut self, types: Option<Vec<String>>) {
        self.types = types;
    }

    /// Setter for the list of argument types associated with the symbol.
    ///
    /// Sets the argument types of the symbol to the provided list of argument types.
    ///
    /// # Arguments
    ///
    /// * `arguments` - The list of argument types to set for the symbol.
    pub fn set_arguments(&mut self, arguments: Option<Vec<TypedSymbol<String>>>) {
        self.arguments = arguments;
    }

    /// Setter for the scope of the declaration.
    ///
    /// Sets the scope of the declaration to the provided value.
    ///
    /// # Arguments
    ///
    /// * `scope` - The new scope to set for the declaration.
    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    /// Formats the types of the declaration for display.
    ///
    /// If the declaration has a list of types, this function formats them and writes them
    /// to the formatter. The types are displayed in a parenthesized list. If there is only
    /// one type, it is directly displayed. If there are multiple types, they are prefixed
    /// with the word "either" and separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter where the types will be written.
    ///
    /// # Returns
    ///
    /// This function returns a `fmt::Result`, which indicates success or failure
    /// in formatting.
    ///
    /// # Example
    ///
    /// ```rust
    /// let declaration = Declaration { ... };
    /// println!("{}", declaration.format_types());
    /// ```
    fn format_types(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(types) = &self.types {
            write!(f, ", types: (")?;
            if types.is_empty() {
                write!(f, ")")?;
            } else if types.len() == 1 {
                // Directly format the single type
                if let Some(single_type) = types.iter().next() {
                    write!(f, "{})", single_type)?;
                }
            } else {
                // Format multiple types with 'either'
                write!(f, "either")?;
                for ty in types {
                    write!(f, " {}", ty)?;
                }
                write!(f, ")")?;
            }
        }
        Ok(())
    }

    /// Formats the arguments of the declaration for display.
    ///
    /// If the declaration has a list of arguments, this function formats them
    /// and writes them to the formatter. The arguments are displayed as a comma-separated
    /// list enclosed in parentheses. Each argument is printed with its name, separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter where the arguments will be written.
    ///
    /// # Returns
    ///
    /// This function returns a `fmt::Result`, which indicates success or failure
    /// in formatting.
    ///
    /// # Example
    ///
    /// ```rust
    /// let declaration = Declaration { ... };
    /// println!("{}", declaration.format_arguments());
    /// ```
    fn format_arguments(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(arguments) = &self.arguments {
            write!(f, ", arguments: (")?;
            for (i, argument) in arguments.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                // Affiche le nom de l'argument avec son index
                write!(f, "{}", argument)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the main elements: ast, kind, scope, and source
        write!(f, "[index: {}, kind: {}", self.ast(), self.kind())?;

        // Add scope and source at the end
        write!(f, ", scope: {}, source: {}", self.scope(), self.source())?;

        // Call the format_types function to format the types
        self.format_types(f)?;

        // Call the format_arguments function to format the arguments
        self.format_arguments(f)?;

        // Close the bracket
        write!(f, "]")
    }
}
