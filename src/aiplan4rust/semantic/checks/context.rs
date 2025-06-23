use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::arena::ArenaAst;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};

/// A lightweight wrapper to pass semantic context components to verification functions.
///
/// `CheckContext` allows verification functions to be reused in both normal semantic passes
/// and in linking, by providing a flexible structure containing only the AST, symbol table,
/// interner, and source name — without requiring a full `SemanticContext`.
///
/// # Why This Wrapper?
/// In standard semantic analysis, all data (AST, symbols, interner) comes from a single file
/// and is contained in a `SemanticContext`. But during **linking**, data must be verified
/// using components that come from **different sources** — for example, the domain file and
/// the problem file might each have their own interner or symbol table.
///
/// `CheckContext` allows you to pass a **custom combination of AST, symbols and interner**
/// to verification functions. This enables code reuse: the same checks can run seamlessly
/// whether you're analyzing a single file or linking two files together.
///
/// # When to Use
/// - During semantic verification of a single parsed file (via `SemanticContext`)
/// - During linking, when combining and verifying symbols or AST nodes from multiple files
/// - When reusing verification passes on renamed ASTs or merged symbol tables
///
/// # Benefits
/// - Enables **reuse** of semantic check functions during the linking phase
/// - Avoids building temporary `SemanticContext` structs just for verification
/// - Keeps function signatures clean and consistent
///
/// # Example
/// ```rust
/// fn check_task_ordering(ctx: CheckContext) {
///     let ast = ctx.ast();
///     let symbols = ctx.symbol_table();
///     // Use them directly for analysis...
/// }
/// ```
///
/// # Fields
/// - `ast`: The abstract syntax tree (AST) to check.
/// - `symbols`: The symbol table used to resolve identifiers.
/// - `interner`: The interner for resolving strings to internal symbols.
/// - `source_name`: The name of the source file (used in diagnostics).

pub struct Context<'a> {
    ast:         &'a ArenaAst,
    symbols:     &'a SymbolTable,
    interner:    &'a StringInterner,
    source_name: &'a str,
}

impl<'a> Context<'a> {
    /// Creates a new `CheckContext` from separate components.
    pub fn new(
        ast: &'a ArenaAst,
        symbols: &'a SymbolTable,
        interner: &'a StringInterner,
        source_name: &'a str,
    ) -> Self {
        Context { ast, symbols, interner, source_name }
    }

    /// Creates a `CheckContext` from a full `SemanticContext`.
    pub fn from_semantic_context(ctx: &'a SemanticContext) -> Self {
        Context {
            ast: &ctx.ast(),
            symbols: &ctx.symbol_table(),
            interner: &ctx.interner(),
            source_name: &ctx.source_name(),
        }
    }

    /// Returns the AST.
    pub fn ast(&self) -> &'a ArenaAst {
        self.ast
    }

    /// Returns the symbol table.
    pub fn symbol_table(&self) -> &'a SymbolTable {
        self.symbols
    }

    /// Returns the string interner.
    pub fn interner(&self) -> &'a StringInterner {
        self.interner
    }

    /// Returns the name of the source file.
    pub fn source_name(&self) -> &'a str {
        self.source_name
    }
}
