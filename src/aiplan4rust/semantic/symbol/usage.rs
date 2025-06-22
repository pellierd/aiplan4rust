use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::{SymbolRef, SymbolSource};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::arena::NodeId;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::Span;

use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

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
    /// Reference to the symbol being used.
    symbol_ref: SymbolRef,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The source of the usage (e.g., file or module).
    source: SymbolSource,

    /// The source code span corresponding to this usage.
    span: Span,

    /// The AST node index where the symbol usage occurs.
    ast: NodeId,
}

impl Usage {
    /// Constructs a new `Usage` instance.
    ///
    /// # Arguments
    ///
    /// * `symbol_ref` - The reference to the symbol being used.
    /// * `scope` - The scope in which the symbol usage occurs.
    /// * `source` - The origin/source of this usage (e.g., file or module).
    /// * `span` - The span in the source code corresponding to this usage.
    /// * `ast` - The AST node identifier where this usage appears.
    ///
    /// # Returns
    ///
    /// A new `Usage` struct initialized with the provided values.
    pub fn new(symbol_ref: SymbolRef, scope: Scope, source: SymbolSource, span: Span, ast: NodeId) -> Self {
        Usage {
            symbol_ref,
            scope,
            source,
            span,
            ast
        }
    }


    pub fn symbol_ref(&self) -> &SymbolRef {
        &self.symbol_ref
    }

    /// Returns the identifier (`Ident`) of the referenced symbol.
    pub fn symbol_ident(&self) -> Ident {
        self.symbol_ref.ident()
    }

    /// Returns the kind (`SymbolKind`) of the referenced symbol.
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol_ref.kind()
    }

    /// Returns a reference to the scope in which the symbol is used.
    ///
    /// # Returns
    ///
    /// A reference to the [`Scope`] where the symbol usage occurs.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns a reference to the source of the symbol usage.
    ///
    /// # Returns
    ///
    /// A reference to the [`SymbolSource`] indicating the origin of this usage.
    pub fn source(&self) -> &SymbolSource {
        &self.source
    }

    /// Returns a reference to the span in the source code for this usage.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the AST node identifier where the symbol is used.
    ///
    /// # Returns
    ///
    /// The [`NodeId`] corresponding to the AST node of this usage.
    pub fn node_id(&self) -> NodeId {
        self.ast
    }

    /// Remaps the identifiers in this declaration using the provided map.
    ///
    /// If the current symbol's identifier is found as a key in `map`,
    /// it is replaced by the corresponding value.
    ///
    /// # Arguments
    ///
    /// * `map` - A hash map from old `Ident` to new `Ident` to be applied.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Some(new_ident) = map.get(&self.symbol_ident()) {
            self.symbol_ref.set_ident(new_ident.clone());
        }
    }

    /// Converts the declaration to a `String` representation using the provided interner.
    ///
    /// # Arguments
    ///
    /// * `interner` - A `StringInterner` to resolve interned strings.
    ///
    /// # Returns
    ///
    /// A `String` representing this declaration, formatted using the interner.
    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    /// Formats the usage into the given writer, resolving interned strings via the interner.
    ///
    /// This method writes a human-readable representation of the usage, including its AST node index,
    /// symbol kind, identifier (resolved from the interner), scope, and source.
    ///
    /// # Arguments
    ///
    /// * `w` - A mutable reference to a type implementing `fmt::Write`, where the output is written.
    /// * `interner` - A `StringInterner` used to resolve the interned identifier string.
    ///
    /// # Returns
    ///
    /// Returns a `fmt::Result` indicating success or failure of the write operation.
    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        let symbol_str = match interner.resolve(self.symbol_ident()) {
            Some(name) => name,
            None => "<uninterned>",
        };

        write!(
            w,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.ast, self.symbol_kind(), symbol_str, self.scope, self.source
        )?;
        Ok(())
    }
}

impl fmt::Display for Usage {
    /// Formats the `Usage` for display.
    ///
    /// This implementation writes a human-readable string representing the usage,
    /// including the AST node index, symbol kind, identifier, scope, and source.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    ///
    /// # Returns
    ///
    /// Returns a `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[index: {}, kind: {}, ident: {}, scope: {}, usage: {}]",
            self.ast, self.symbol_kind(), self.symbol_ident(), self.scope, self.source
        )?;
        Ok(())
    }
}
