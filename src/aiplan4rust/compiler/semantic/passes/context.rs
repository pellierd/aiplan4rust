use crate::aiplan4rust::compiler::syntax::ast::tree::Tree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;
use crate::aiplan4rust::support::diagnostic::Provider;
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::LiteralId;

/// Contextual information provided to semantic analysis passes.
///
/// `PassContext` bundles all immutable resources required during symbol resolution
/// and semantic checking. By separating these read-only resources from the mutable
/// [`SymbolTable`], we ensure thread-safety and adhere to Rust's borrowing rules.
///
/// # Lifetimes
/// * `'a`: Represents the lifetime of the source AST and the symbol interner.
pub struct PassContext<'a> {
    /// The immutable abstract syntax tree being analyzed.
    syntax_tree: &'a Tree<AstNode>,
    /// The global interner for symbol name resolution.
    interner: &'a SymbolInterner,
    /// Identifier for the current source file/input.
    source: LiteralId,
    /// The component currently performing the analysis.
    provider: Provider,
}

impl<'a> PassContext<'a> {
    /// Creates a new analysis context.
    pub fn new(
        ast: &'a Tree<AstNode>,
        interner: &'a SymbolInterner,
        source: LiteralId,
        provider: Provider,
    ) -> Self {
        Self {
            syntax_tree: ast,
            interner,
            source,
            provider,
        }
    }

    /// Returns a reference to the underlying syntax tree.
    #[inline]
    pub fn syntax_tree(&self) -> &Tree<AstNode> {
        self.syntax_tree
    }

    /// Returns a reference to the symbol interner.
    #[inline]
    pub fn interner(&self) -> &SymbolInterner {
        self.interner
    }

    /// Returns the source identifier for diagnostics.
    pub fn source(&self) -> LiteralId {
        self.source
    }

    /// Returns the provider responsible for the current pass.
    pub fn provider(&self) -> Provider {
        self.provider
    }
}
