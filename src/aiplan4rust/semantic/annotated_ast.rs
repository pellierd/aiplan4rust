use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::elements::Requirement;

/// A semantically enriched version of the raw AST.
pub struct AnnotatedAst {
    /// The original parsed AST.
    ast: Ast,

    /// The set of semantic requirements extracted during analysis.
    requirements: HashSet<Requirement>,

    /// The symbol table constructed during semantic analysis.
    symbol_table: SymbolTable,

    /// Timestamp indicating when semantic annotation occurred.
    annotated_at: SystemTime,
}

impl AnnotatedAst {
    /// Creates a new `AnnotatedAst` from its components.
    pub fn new(
        ast: Ast,
        requirements: HashSet<Requirement>,
        symbol_table: SymbolTable,
        annotated_at: SystemTime,
    ) -> Self {
        Self {
            ast,
            requirements,
            symbol_table,
            annotated_at,
        }
    }

    /// Returns a reference to the inner AST.
    pub fn ast(&self) -> &Ast {
        &self.ast
    }

    /// Returns a mutable reference to the inner AST.
    pub fn ast_mut(&mut self) -> &mut Ast {
        &mut self.ast
    }

    /// Returns the set of requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Returns a reference to the symbol table.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Returns the timestamp of annotation.
    pub fn annotated_at(&self) -> SystemTime {
        self.annotated_at
    }

    /// Delegates access to the source name stored in the underlying AST.
    pub fn source_name(&self) -> &str {
        &self.ast.source_name()
    }
}

impl fmt::Display for AnnotatedAst {
fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "Annotated Syntax Tree Information:\n")?;

    // Display the source file name
    writeln!(f, "Source File: {}", self.ast.source_name())?;

    // Display the annotation timestamp
    let duration_since_epoch = self
        .annotated_at
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::new(0, 0));
    let timestamp = duration_since_epoch.as_secs();
    writeln!(f, "Annotated at: {} seconds since UNIX epoch\n", timestamp)?;

    // Display requirements
    writeln!(f, "\nRequirements:")?;
    for req in &self.requirements {
        writeln!(f, "  - {}", req)?;
    }

    // Display the AST structure
    writeln!(f, "Abstract Syntax Tree:\n{}", self.ast())?;

    // Display symbol table
    writeln!(f, "\nSymbol Table:\n{}", self.symbol_table)?;

    Ok(())
}
}
