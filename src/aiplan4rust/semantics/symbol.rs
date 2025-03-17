use crate::aiplan4rust::semantics::scope::Scope;
use crate::aiplan4rust::semantics::typed_symbol::TypedSymbol;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Source {
    #[default]
    Domain,
    Problem,
    Unknown,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Domain => write!(f, "Domain"),
            Source::Problem => write!(f, "Problem"),
            Source::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Enum representing the different kinds of symbols in the system.
///
/// This enum categorizes the symbols based on their role or type within a domain, problem, or plan.
/// It is used to distinguish between different symbol types when processing or analyzing a symbol.
///
/// # Variants
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Represents an action in the domain (e.g., a specific task or operation).
    Action,

    /// Represents a symbol used in the domain description (e.g., a symbol specific to a domain
    /// model).
    DASymbol,

    /// Represents a symbol associated with a method in the domain, used for defining methods
    /// that perform actions or tasks, often related to processes or operations in the domain.
    Method,

    /// Represents a symbol associated with a task in the domain, typically used to define a
    /// specific task or operation that can be planned and executed within the system.
    Task,

    /// Represents a unique identifier for a task in the domain, used for referencing tasks
    /// within the domain model.
    TaskID,

    /// Represents a constant value that does not change.
    Constant,

    /// Represents the name of a domain.
    DomainName,

    /// Represents a function symbol.
    Function,

    /// Represents a predicate symbol (typically used for logical conditions).
    Predicate,

    /// Represents a basic data type (e.g., integer, boolean).
    PrimitiveType,

    /// Represents the name of the problem being solved (e.g., a problem definition).
    ProblemName,

    /// Represents a requirement or constraint within a domain or problem context.
    Requirement,

    /// Represents a variable that can hold different values during execution.
    Variable,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Action => write!(f, "Action"),
            SymbolKind::DASymbol => write!(f, "Durative Action"),
            SymbolKind::PrimitiveType => write!(f, "Primitive Type"),
            SymbolKind::Predicate => write!(f, "Predicate"),
            SymbolKind::Variable => write!(f, "Variable"),
            SymbolKind::Constant => write!(f, "Constant"),
            SymbolKind::Function => write!(f, "Functor"),
            SymbolKind::DomainName => write!(f, "Domain Name"),
            SymbolKind::ProblemName => write!(f, "Problem Name"),
            SymbolKind::Requirement => write!(f, "Requirement"),
            // Add for HDDL
            SymbolKind::Method => write!(f, "Method"),
            SymbolKind::Task => write!(f, "Task"),
            SymbolKind::TaskID => write!(f, "TaskID"),
        }
    }
}

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

/// Represents the usage of a symbol in a specific context within the AST.
///
/// The `Usage` struct stores information about the reference to a symbol,
/// including its AST node, its kind, the scope in which it is used, and the
/// source of the usage.
///
/// The struct is derived with the following traits:
/// - `Debug`: To allow for easy debugging output.
/// - `Clone`: To allow for cloning of instances.
/// - `Eq`: To allow comparison for equality.
/// - `PartialEq`: To allow partial equality comparison.
/// - `Serialize`: To allow serialization for storage or transmission.
/// - `Deserialize`: To allow deserialization from serialized formats.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// The AST node index where the symbol is used.
    ast: usize,

    /// The kind of the symbol used (e.g., variable, function).
    kind: SymbolKind,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The source of the usage (e.g., file or module).
    source: Source,
}

impl Usage {
    /// Constructeur pour créer un nouveau `Usage`
    pub fn new(ast: usize, kind: SymbolKind, scope: Scope, source: Source) -> Self {
        Usage {
            ast,
            kind,
            scope,
            source,
        }
    }

    /// Accessor for the AST node of the usage.
    ///
    /// Returns the index of the AST node where the symbol is used.
    ///
    /// # Returns
    ///
    /// * `usize` - The index of the AST node where the symbol is used.
    pub fn ast(&self) -> usize {
        self.ast
    }

    /// Accessor for the kind of the symbol used.
    ///
    /// Returns a reference to the symbol's kind.
    ///
    /// # Returns
    ///
    /// * `&SymbolKind` - A reference to the kind of the symbol.
    pub fn kind(&self) -> &SymbolKind {
        &self.kind
    }

    /// Accessor for the scope in which the symbol is used.
    ///
    /// Returns a reference to the scope of the usage.
    ///
    /// # Returns
    ///
    /// * `&Scope` - A reference to the scope where the symbol is used.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Accessor for the source of the usage.
    ///
    /// Returns a reference to the source from which the usage originates.
    ///
    /// # Returns
    ///
    /// * `&Source` - A reference to the source of the usage.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Mutable accessor for the scope of the usage.
    ///
    /// Allows modification of the scope where the symbol is used.
    ///
    /// # Returns
    ///
    /// * `&mut Scope` - A mutable reference to the scope of the usage.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut self.scope
    }

    /// Mutable accessor for the source of the usage.
    ///
    /// Allows modification of the source from which the usage originates.
    ///
    /// # Returns
    ///
    /// * `&mut Source` - A mutable reference to the source of the usage.
    pub fn source_mut(&mut self) -> &mut Source {
        &mut self.source
    }

    /// Setter for the scope of the usage.
    ///
    /// Sets the scope where the symbol is used to the provided value.
    ///
    /// # Arguments
    ///
    /// * `scope` - The new scope to set for the usage.
    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    /// Setter for the source of the usage.
    ///
    /// Sets the source of the usage to the provided value.
    ///
    /// # Arguments
    ///
    /// * `source` - The new source to set for the usage.
    pub fn set_source(&mut self, source: Source) {
        self.source = source;
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[index: {}, kind: {}, scope: {}, usage: {}]",
            self.ast, self.kind, self.scope, self.source
        )?;
        Ok(())
    }
}

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
/// symbol.add_declaration(declaration);
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

/// A trait that defines methods to access the `kind` and `scope` of a symbol.
///
/// This trait allows a unified interface for both `Declaration` and `Usage` types,
/// providing access to the kind of the symbol and its scope. It is useful for
/// grouping declarations and usages in a manner that allows filtering based on
/// symbol attributes such as type (`SymbolKind`) and scope (`Scope`).
///
/// # Required Methods
///
/// - `kind`: Returns a reference to the symbol's `SymbolKind`.
/// - `scope`: Returns a reference to the symbol's `Scope`.
///
/// This trait is implemented for both `Declaration` and `Usage` types, enabling
/// filtering or grouping based on symbol properties.
///
/// # Example
///
/// ```rust
/// let declaration = Declaration::new(...);
/// let usage = Usage::new(...);
///
/// // Both can be treated as FilterableSymbol:
/// let kind = declaration.kind();
/// let scope = usage.scope();
/// ```
pub trait FilterableSymbol: Debug + Display {
    /// Returns the `SymbolKind` of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the symbol.
    fn kind(&self) -> &SymbolKind;

    /// Returns the `Scope` of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the symbol.
    fn scope(&self) -> &Scope;
}

// Implementing `FilterableSymbol` for the `Declaration` struct.
impl FilterableSymbol for Declaration {
    /// Returns the `SymbolKind` of the declaration.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the declaration.
    fn kind(&self) -> &SymbolKind {
        self.kind() // Delegates to the `kind` method in `Declaration`.
    }

    /// Returns the `Scope` of the declaration.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the declaration.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to the `scope` method in `Declaration`.
    }
}

// Implementing `FilterableSymbol` for the `Usage` struct.
impl FilterableSymbol for Usage {
    /// Returns the `SymbolKind` of the usage.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the usage.
    fn kind(&self) -> &SymbolKind {
        self.kind() // Delegates to the `kind` method in `Usage`.
    }

    /// Returns the `Scope` of the usage.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the usage.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to the `scope` method in `Usage`.
    }
}
