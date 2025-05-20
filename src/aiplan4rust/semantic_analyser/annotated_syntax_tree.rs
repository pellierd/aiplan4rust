use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::parser::syntax_tree::SyntaxTree;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxNode;
use crate::aiplan4rust::semantic_analyser::SymbolTable;
use crate::aiplan4rust::parser::SymbolOrigin;

use linked_hash_map::LinkedHashMap;
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
    syntax_tree: LinkedHashMap<usize, AnnotatedSyntaxNode>,

    /// The set of declared `Requirement`s extracted from the AST.
    requirements: HashSet<Requirement>,

    /// The symbol table constructed from the AST.
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
    pub fn get_parent(&self, id: usize) -> Option<&AnnotatedSyntaxNode> {
        for node in self.syntax_tree.values() {
            if node.children().contains(&id) {
                return Some(node);
            }
        }
        None
    }
    pub fn source(&self) -> SymbolOrigin {
        if let Some(first_node) = self.syntax_tree.values().next() {
            match first_node.kind() {
                SyntaxNodeKind::Domain => SymbolOrigin::Domain,
                SyntaxNodeKind::Problem => SymbolOrigin::Problem,
                _ => SymbolOrigin::Unknown,
            }
        } else {
            SymbolOrigin::Unknown
        }
    }
    pub fn contains_kind(&self, kind: SyntaxNodeKind) -> bool {
        self.syntax_tree
            .iter()
            .any(|(_, entry)| *entry.kind() == kind)
    }

    pub fn insert(
        &mut self,
        index: usize,
        entry: AnnotatedSyntaxNode,
    ) -> Option<AnnotatedSyntaxNode> {
        self.syntax_tree.insert(index, entry)
    }

    pub fn get_entry(&self, id: usize) -> Option<&AnnotatedSyntaxNode> {
        self.syntax_tree.get(&id)
    }

    pub fn add_entry(&mut self, id: usize, entry: AnnotatedSyntaxNode) {
        self.syntax_tree.insert(id, entry);
    }
    pub fn contains_entry(&self, id: usize) -> bool {
        self.syntax_tree.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &AnnotatedSyntaxNode)> {
        self.syntax_tree.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut AnnotatedSyntaxNode)> {
        self.syntax_tree.iter_mut()
    }
    pub fn keys(&self) -> impl Iterator<Item = &usize> {
        self.syntax_tree.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &AnnotatedSyntaxNode> {
        self.syntax_tree.values()
    }

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
        syntax_tree: LinkedHashMap<usize, AnnotatedSyntaxNode>,
        symbol_table: SymbolTable,
        requirements: HashSet<Requirement>,
        filename: String,
    ) -> Self {
        AnnotatedSyntaxTree {
            syntax_tree,
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
        // Convert the syntax tree into a linked map of nodes
        let nodes = syntax_tree.flatten()?;

        // Extract the requirements from the syntax tree
        let requirements = Self::extract_requirements(&nodes);

        // Create the symbol table from the syntax tree
        let mut symbol_table = SymbolTable::new(syntax_tree.source());
        symbol_table.initialize_from_ast(0, &nodes)?;

        // Create and return the annotated syntax tree
        Ok(AnnotatedSyntaxTree::new(
            nodes,
            symbol_table,
            requirements,
            syntax_tree.filename().unwrap().clone(),
        ))
    }

    /// Extracts all implied `Requirement` instances from the given annotated syntax tree.
    ///
    /// This function iterates over the nodes of the provided `HeapSyntaxTree` (represented as a
    /// `LinkedHashMap`) and collects all requirements explicitly declared via nodes of kind
    /// `SyntaxNodeKind::Requirement`. For each such node, the function expands it to include
    /// all implied requirements (e.g., `:adl` implies `:strips`, `:typing`, etc.).
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - A reference to the annotated syntax tree containing all parsed nodes.
    ///
    /// # Returns
    ///
    /// A `HashSet` containing all unique `Requirement` instances, including those implied by
    /// the declared ones.
    fn extract_requirements(
        syntax_tree: &LinkedHashMap<usize, AnnotatedSyntaxNode>,
    ) -> HashSet<Requirement> {
        let mut requirements = HashSet::new();
        for node in syntax_tree.values() {
            if let SyntaxNodeKind::Requirement(req) = node.kind() {
                requirements.extend(req.imply());
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

    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
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
        writeln!(f, "Abstract Syntax Tree : {{")?;
        for (index, entry) in &self.syntax_tree {
            writeln!(f, "  {}: {}", index, entry)?;
        }
        write!(f, "}}")?;

        // Display the symbol table if available
        write!(f, "Symbol Table: \n{}", self.symbol_table)?;

        Ok(())
    }
}
