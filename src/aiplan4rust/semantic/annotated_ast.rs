use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::elements::Requirement;

/// Represents the semantic context resulting from the semantic analysis phase.
///
/// This structure contains the semantically enriched abstract syntax tree (AST),
/// a symbol table, semantic requirements, and metadata such as the source name
/// and generation timestamp.
///
/// It is the main output of the semantic analysis stage and acts as the interface
/// between parsing and later phases such as type checking, optimization, or code generation.
pub struct SemanticContext {
    /// The AST stored in an arena for efficient indexing and traversal.
    ast: ArenaAst,

    /// The set of semantic requirements (e.g., domain-specific constraints or planner capabilities).
    requirements: HashSet<Requirement>,

    /// The symbol table constructed during semantic analysis.
    symbol_table: SymbolTable,

    /// The name of the source file or input that was parsed.
    source_name: String,

    /// Timestamp indicating when semantic analysis was completed.
    generated_at: SystemTime,
}

impl SemanticContext {
    /// Constructs a new `SemanticContext` from its components.
    ///
    /// # Arguments
    /// * `ast` - The arena-based abstract syntax tree.
    /// * `source_name` - The name of the input source file.
    /// * `requirements` - A set of extracted semantic requirements.
    /// * `symbol_table` - The resulting symbol table from analysis.
    /// * `generated_at` - The timestamp marking when the context was built.
    pub fn new(
        ast: ArenaAst,
        source_name: String,
        requirements: HashSet<Requirement>,
        symbol_table: SymbolTable,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            ast,
            source_name,
            requirements,
            symbol_table,
            generated_at,
        }
    }

    /// Returns a reference to a node by its index, if it exists.
    pub fn get(&self, id: usize) -> Option<&ArenaAstNode> {
        self.ast.get(id)
    }

    /// Returns a reference to the internal AST arena.
    pub fn ast(&self) -> &ArenaAst {
        &self.ast
    }

    /// Returns the set of semantic requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Returns the symbol table used during semantic resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Returns the timestamp when this semantic context was created.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }

    /// Returns the source file name or input name associated with the AST.
    pub fn source_name(&self) -> &String {
        &self.source_name
    }
}

impl fmt::Display for SemanticContext {
    /// Formats the semantic context for display.
    ///
    /// The output includes metadata (timestamp and source), semantic requirements,
    /// the arena-based AST, and the symbol table.
    ///
    /// # Example
    /// ```text
    /// Annotated Syntax Tree Information:
    /// Source: domain.pddl
    /// Annotated at: 1718523096 seconds since UNIX epoch
    ///
    /// Requirements:
    ///   - :strips
    ///   - :typing
    ///
    /// Abstract Syntax Tree:
    /// Node(kind=Domain, span=..., children=[...])
    ///
    /// Symbol Table:
    /// ...
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Semantic Context Report:\n")?;

        writeln!(f, "Source: {}", self.source_name)?;

        let duration_since_epoch = self
            .generated_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::new(0, 0));
        let timestamp = duration_since_epoch.as_secs();
        writeln!(f, "Generated at: {} seconds since UNIX epoch\n", timestamp)?;

        writeln!(f, "Requirements:")?;
        for req in &self.requirements {
            writeln!(f, "  - {}", req)?;
        }

        writeln!(f, "\nAbstract Syntax Tree:\n{}", self.ast)?;

        writeln!(f, "\nSymbol Table:\n{}", self.symbol_table)?;

        Ok(())
    }
}
