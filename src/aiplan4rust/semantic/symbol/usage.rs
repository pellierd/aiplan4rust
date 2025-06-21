use std::collections::HashMap;
use crate::aiplan4rust::syntax::{Span, StringInterner};
use crate::aiplan4rust::semantic::symbol::SymbolSource;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use crate::aiplan4rust::syntax::elements::Ident;

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
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Usage {

    symbol: Ident,

    /// The kind of the symbol used (e.g., variable, function).
    kind: SymbolKind,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    span: Span,

    /// The source of the usage (e.g., file or module).
    source: SymbolSource,

    /// The AST node index where the symbol is used.
    ast: usize,

}

impl Usage {
    /// Constructeur pour créer un nouveau `Usage`
    pub fn new(symbol: Ident, kind: SymbolKind, scope: Scope, source: SymbolSource, span: Span, ast: usize) -> Self {
        Usage {
            symbol,
            kind,
            scope,
            source,
            span,
            ast
        }
    }

    pub fn symbol(&self) -> Ident {
        self.symbol
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
    pub fn source(&self) -> &SymbolSource {
        &self.source
    }

    pub fn span(&self) -> &Span {
        &self.span
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
    pub fn source_mut(&mut self) -> &mut SymbolSource {
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
    pub fn set_source(&mut self, source: SymbolSource) {
        self.source = source;
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Some(new_ident) = map.get(&self.symbol) {
            self.symbol = new_ident.clone();
        }
    }


    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        let symbol_str = match interner.get_str(self.symbol) {
            Some(name) => name,
            None => "<uninterned>",
        };

        write!(
            w,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.ast, self.kind, symbol_str, self.scope, self.source
        )?;
        Ok(())
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.ast, self.kind, self.symbol, self.scope, self.source
        )?;
        Ok(())
    }
}
