use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::tree::{TreeArena, NodeId};
use crate::aiplan4rust::semantic::{AstArenaNode, SymbolTable};
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};
use crate::aiplan4rust::lang::Requirement;

/// Represents the semantic context resulting from the semantic analysis phase.
///
/// This structure contains the semantically enriched abstract syntax tree (AST),
/// a symbol table, semantic requirements, and metadata such as the source name
/// and generation timestamp.
///
/// It is the main output of the semantic analysis stage and acts as the interface
/// between parsing and later phases such as type checking, optimization, or code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// The AST stored in an tree for efficient indexing and traversal.
    ast: TreeArena<AstArenaNode>,

    /// The set of semantic requirements (e.g., domain-specific constraints or planner capabilities).
    requirements: HashSet<Requirement>,

    /// The symbol table constructed during semantic analysis.
    symbol_table: SymbolTable,

    interner: StringInterner,

    /// The name of the source file or input that was parsed.
    source_name: String,

    /// Timestamp indicating when semantic analysis was completed.
    generated_at: SystemTime,
}

impl Context {
    /// Constructs a new `SemanticContext` from its components.
    ///
    /// # Arguments
    /// * `ast` - The tree-based abstract syntax tree.
    /// * `source_name` - The name of the input source file.
    /// * `requirements` - A set of extracted semantic requirements.
    /// * `symbol_table` - The resulting symbol table from analysis.
    /// * `interner` - The string interner used for symbol resolution and deduplication.
    /// * `generated_at` - The timestamp marking when the context was built.
    pub fn new(
        ast: TreeArena<AstArenaNode>,
        source_name: String,
        requirements: HashSet<Requirement>,
        symbol_table: SymbolTable,
        interner: StringInterner,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            ast,
            source_name,
            requirements,
            symbol_table,
            interner,
            generated_at,
        }
    }

    /// Creates a new `AnnotatedSyntaxTree` from a `SyntaxTree`.
    ///
    /// # Arguments
    /// * `ast_old` - The original syntax tree to be annotated.
    ///
    /// # Returns
    /// * A new `AnnotatedSyntaxTree` created from the provided `ast_old`.
    pub fn from(ast: &mut Ast) -> Result<Self, ParserInternalError> {
        let arena = TreeArena::from_ast(ast);
        let requirements = Self::extract_requirements(&arena)?;
        let interner = ast.take_interner();
        let symbol_table = SymbolTable::from_ast(&arena)?;

        Ok(Context::new(
            arena,
            ast.source_name().to_string(),
            requirements,
            symbol_table,
            interner,
            SystemTime::now(),
        ))
    }

    /// Extracts all implied `Requirement` instances from an tree-based syntax tree,
    /// assuming all requirements are declared under a single parent node.
    ///
    /// Traverses the tree to find the first node of kind `Requirement`,
    /// collects it and all its children, then stops.
    ///
    /// # Arguments
    ///
    /// * `tree` - A reference to the tree-based syntax tree.
    ///
    /// # Returns
    ///
    /// A `HashSet` of all declared and implied `Requirement` instances.
    fn extract_requirements(arena: &TreeArena<AstArenaNode>) -> Result<HashSet<Requirement>, ParserInternalError> {
        let mut requirements = HashSet::new();

        // Step 1: Find the first `RequireDef` node in the AST
        let mut requirement_def_node = None;
        for node in arena.preorder() {
            if matches!(node.kind(), AstKind::RequireDef) {
                requirement_def_node = Some(node);
                break;
            }
        }

        // Step 2: If found, iterate over its children and collect all `Requirement` nodes
        if let Some(req_def) = requirement_def_node {
            for child in req_def.children() {
                let node = arena.try_node(*child)?;
                if matches!(node.kind(), AstKind::Requirement) {
                    if let Ok(req) = node.try_requirement() {
                        requirements.extend(req.imply());
                    }
                }
            }
        }

        Ok(requirements)
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

    /// Returns a reference to a node by its index, if it exists.
    pub fn get_node(&self, id: NodeId) -> Option<&AstArenaNode> {
        self.ast.get_node(id)
    }

    pub fn try_node(&self, id: NodeId) -> Result<&AstArenaNode, ParserInternalError> {
        self.ast.try_node(id)
    }

    /// Returns a reference to the internal AST tree.
    pub fn ast(&self) -> &TreeArena<AstArenaNode> {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut TreeArena<AstArenaNode> {
        &mut self.ast
    }

    pub fn take_ast(&mut self) -> TreeArena<AstArenaNode> {
        std::mem::take(&mut self.ast)
    }

    /// Returns the set of semantic requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    pub fn take_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.requirements)
    }

    /// Returns the symbol table used during semantic resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    pub fn take_symbol_table(&mut self) -> SymbolTable {
        std::mem::take(&mut self.symbol_table)
    }

    /// Returns the timestamp when this semantic context was created.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }

    /// Returns the source file name or input name associated with the AST.
    pub fn source_name(&self) -> &String {
        &self.source_name
    }

    pub fn interner(&self) -> &StringInterner {
         &self.interner
    }

     pub fn set_interner(&mut self, interner: StringInterner) {
        self.interner = interner;
    }

}

impl fmt::Display for Context {
    /// Formats the semantic context for display.
    ///
    /// The output includes metadata (timestamp and source), semantic requirements,
    /// the tree-based AST, and the symbol table.
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

        writeln!(f, "\nAbstract Syntax Tree:\n{:?}", self.ast)?;

        writeln!(f, "\nSymbol Table:\n{}", self.symbol_table)?;

        Ok(())
    }
}
