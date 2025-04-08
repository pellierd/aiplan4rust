use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::semantic_analyser::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::tree::SyntaxTree;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedSyntaxTree {
    /// The Abstract Syntax Tree (AST) of the domain/problem (if available).
    syntax_tree: HeapSyntaxTree,
    /// The symbol table related to the AST (if available).
    symbol_table: SymbolTable,
    /// The filename where the AST was parsed from.
    filename: String,
    /// The timestamp of when the AST was generated.
    generated_at: SystemTime,
}

impl Default for AnnotatedSyntaxTree {
    fn default() -> Self {
        AnnotatedSyntaxTree {
            syntax_tree: Default::default(),
            symbol_table: Default::default(),
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
    pub fn new(ast: HeapSyntaxTree, symbol_table: SymbolTable, filename: String) -> Self {
        AnnotatedSyntaxTree {
            syntax_tree: ast,
            symbol_table,
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
        let ast = syntax_tree.root();

        // Convert the AST into a hash map
        let ast_table = HeapSyntaxTree::from(&ast)?;

        // Create the SymbolTable with the AST and the Bimap
        let mut symbol_table = SymbolTable::new();
        symbol_table.initialize_from_ast(0, &ast_table)?;

        // Create and return the annotated_syntax_tree
        Ok(AnnotatedSyntaxTree::new(
            ast_table,
            symbol_table,
            syntax_tree.filename().unwrap().clone(),
        ))
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
