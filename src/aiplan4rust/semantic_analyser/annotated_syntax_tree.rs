use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement;
use crate::aiplan4rust::parser::syntax_tree::{SyntaxNodeKind, SyntaxTree};
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::semantic_analyser::SymbolTable;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Represents an annotated syntax tree that includes both an Abstract Syntax Tree (AST) and a
/// symbol table, alongside metadata such as the file from which it was derived and the time it was
/// generated.
///
/// The `AnnotatedSyntaxTree` is used to store parsed information for further semantic analysis and
/// manipulation. It holds an optional AST and symbol table that are necessary for performing
/// operations such as linking and semantic analysis. The struct also stores metadata, including the
/// filename and the time of creation.
///
/// This struct is commonly used as part of the lifting process, where it represents a higher-level,
/// annotated version of the problem and domain described in the PDDL files.
///
/// The following aliases are provided:
/// - `LiftedProblem`: A type alias for `AnnotatedSyntaxTree`, representing an annotated syntax tree
///   for a problem.
/// - `LiftedDomain`: A type alias for `AnnotatedSyntaxTree`, representing an annotated syntax tree
///   for a domain.
pub type LiftedProblem = AnnotatedSyntaxTree;
pub type LiftedDomain = AnnotatedSyntaxTree;

/// A structure that encapsulates the annotated abstract syntax tree (AST),
/// along with associated semantic information.
///
/// The `AnnotatedSyntaxTree` combines the parsed syntax tree with additional
/// context such as declared requirements, the symbol table, and metadata
/// like the source filename and generation timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedSyntaxTree {
    /// The abstract syntax tree (AST) of the domain or problem.
    syntax_tree: HeapSyntaxTree,

    /// The set of declared `Requirement`s extracted from the AST.
    ///
    /// These influence the semantics of the problem and control which
    /// constructs are permitted in the syntax tree.
    requirements: HashSet<Requirement>,

    /// The symbol table constructed from the AST.
    ///
    /// Contains all symbols (e.g., types, constants, functions) defined in the input.
    symbol_table: SymbolTable,

    /// The name of the file from which the AST was parsed.
    filename: String,

    /// The timestamp indicating when the AST was generated.
    generated_at: SystemTime,
}

impl Default for AnnotatedSyntaxTree {
    fn default() -> Self {
        AnnotatedSyntaxTree {
            syntax_tree: Default::default(),
            symbol_table: Default::default(),
            requirements: Default::default(),
            filename: String::new(),
            generated_at: SystemTime::now(),
        }
    }
}

impl AnnotatedSyntaxTree {
    /// Creates a new `AnnotatedSyntaxTree` with the provided components.
    ///
    /// # Arguments
    /// * `ast` - An `AstTable` representing the abstract syntax tree.
    /// * `symbol_table` - A `SymbolTable` containing the symbols from the syntax tree.
    /// * `filename` - A `String` representing the filename of the source.
    /// * `generated_at` - A `SystemTime` representing when the AST was generated.
    ///
    /// # Returns
    /// A new `AnnotatedSyntaxTree` instance initialized with the given values.
    fn new(
        ast: HeapSyntaxTree,
        symbol_table: SymbolTable,
        requirements: HashSet<Requirement>,
        filename: String,
    ) -> Self {
        AnnotatedSyntaxTree {
            syntax_tree: ast,
            symbol_table,
            requirements,
            filename,
            generated_at: SystemTime::now(),
        }
    }

    /// Creates a new `AnnotatedSyntaxTree` from a `SyntaxTree`.
    ///
    /// # Arguments
    /// * `syntax_tree` - The original syntax tree to be annotated.
    ///
    /// # Returns
    /// * A new `AnnotatedSyntaxTree` created from the provided `syntax_tree`.
    pub fn from(syntax_tree: &SyntaxTree) -> Result<Self, ParserInternalError> {
        // Check if the AST exists in the syntax_tree
        let root = syntax_tree.root();

        // Convert the syntax tree into a heap syntax tree
        let heap_syntax_tree = HeapSyntaxTree::from(&root)?;

        // Extract the requirements from the syntax tree
        let requirements = Self::extract_requirements(&heap_syntax_tree);

        // Create the SymbolTable with the AST and the Bimap
        let mut symbol_table = SymbolTable::new();
        symbol_table.initialize_from_ast(0, &heap_syntax_tree)?;

        // Create and return the annotated_syntax_tree
        Ok(AnnotatedSyntaxTree::new(
            heap_syntax_tree,
            symbol_table,
            requirements,
            syntax_tree.filename().unwrap().clone(),
        ))
    }

    /// Extracts all `Requirement` nodes from the given syntax tree.
    ///
    /// This function iterates over the nodes of the provided `HeapSyntaxTree`
    /// and collects all nodes of kind `SyntaxNodeKind::Requirement`.
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - A reference to the syntax tree from which to extract requirements.
    ///
    /// # Returns
    ///
    /// A `HashSet` containing all unique `Requirement` instances found in the syntax tree.
    ///
    fn extract_requirements(syntax_tree: &HeapSyntaxTree) -> HashSet<Requirement> {
        let mut requirements = HashSet::new();
        for node in syntax_tree.values() {
            if let SyntaxNodeKind::Requirement(req) = node.kind() {
                requirements.insert(req.clone());
            }
        }
        requirements
    }

    /// Checks whether a specific `Requirement` is declared in the syntax tree.
    ///
    /// This method returns `true` if the given `requirement` is present in the
    /// set of declared requirements, meaning the corresponding feature is enabled
    /// and may be used in the domain or problem description.
    ///
    /// # Arguments
    ///
    /// * `requirement` - A reference to the `Requirement` to check for.
    ///
    /// # Returns
    ///
    /// `true` if the requirement is declared; `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if annotated_syntax_tree.has_requirement(&Requirement::Fluent) {
    ///     println!("Fluent support is enabled.");
    /// }
    /// ```
    pub fn has_requirement(&self, requirement: &Requirement) -> bool {
        self.requirements.contains(requirement)
    }

    /// Returns a reference to the `AstTable` if available.
    ///
    /// # Returns
    /// * `&AstTable` representing the AST.
    pub fn syntax_tree(&self) -> &HeapSyntaxTree {
        &self.syntax_tree
    }

    /// Returns a reference to the `SymbolTable` if available.
    ///
    /// # Returns
    /// * `&SymbolTable` representing the symbol table.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the `SymbolTable` if available.
    ///
    /// # Returns
    /// * `&mut SymbolTable` for modifying the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Returns the filename from which the AST was parsed.
    ///
    /// # Returns
    /// * `&String` representing the filename of the source.
    pub fn filename(&self) -> &String {
        &self.filename
    }
}

impl fmt::Display for AnnotatedSyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display general information about the AST and symbol table
        write!(f, "Annotated Syntax Tree Information:\n")?;

        // Display the source file name
        write!(f, "Source File: {}\n", self.filename)?;

        // Display the generation timestamp
        let duration_since_epoch = self
            .generated_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::new(0, 0));
        let timestamp = duration_since_epoch.as_secs();
        write!(f, "Generated at: {} seconds since UNIX epoch\n", timestamp)?;

        // Display the AST if available
        write!(f, "AST: \n{}", self.syntax_tree)?;

        // Display the symbol table if available
        write!(f, "Symbol Table: \n{}", self.symbol_table)?;

        Ok(())
    }
}
