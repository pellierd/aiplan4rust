use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::{SymbolRef, SymbolOrigin};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::arena::NodeId;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::Span;

use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a specific usage of a symbol within the Abstract Syntax Tree (AST).
///
/// This struct encapsulates detailed information about where and how a symbol is referenced
/// during the compilation or analysis process. It tracks the symbol's reference, the
/// lexical scope of the usage, the origin of the symbol usage (such as the file or module),
/// the precise location in the source code, and the AST node associated with this usage.
///
/// # Fields
///
/// - `symbol_ref`: A reference to the symbol being used. This links the usage back to the symbol's
///   definition.
/// - `scope`: The lexical or logical scope (such as a function or block) in which the symbol is
///   used.
/// - `origin`: The origin or source of the usage, indicating the file, module, or context from
///   which this usage arises.
/// - `span`: The source code span that highlights the exact location of the usage in the source
///   code (e.g., line and column range).
/// - `ast`: The AST node index (NodeId) corresponding to this particular usage occurrence.
///
/// # Derives
///
/// This struct implements the following traits:
/// - `Debug`: Enables formatted printing useful for debugging.
/// - `Clone`: Allows creating deep copies of `Usage` instances.
/// - `Eq` and `PartialEq`: Support for equality comparisons.
/// - `Hash`: Enables use in hash-based collections like `HashMap` or `HashSet`.
/// - `Serialize` and `Deserialize`: Allow serializing to and deserializing from formats such as
///   JSON, enabling persistence or inter-process communication.
///
/// # Example
///
/// ```
/// # use your_crate::{Usage, SymbolRef, Scope, SymbolOrigin, Span, NodeId};
/// let usage = Usage {
///     symbol_ref: SymbolRef::new(...),
///     scope: Scope::Function,
///     origin: SymbolOrigin::File("src/main.rs".into()),
///     span: Span::new(10, 20),
///     ast: NodeId(42),
/// };
/// println!("{:?}", usage);
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Usage {
    /// Reference to the symbol being used.
    symbol_ref: SymbolRef,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The source of the usage (e.g., file or module).
    origin: SymbolOrigin,

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
    /// * `symbol_ref` - Reference to the symbol being used.
    /// * `scope` - The scope in which this symbol usage occurs.
    /// * `source` - The origin of this usage, indicating where the symbol comes from (e.g., domain or problem).
    /// * `span` - The span in the source code corresponding to this usage.
    /// * `ast` - The identifier of the AST node where this usage appears.
    ///
    /// # Returns
    ///
    /// A new `Usage` struct initialized with the given parameters.
    pub fn new(
        symbol_ref: SymbolRef,
        scope: Scope,
        source: SymbolOrigin,
        span: Span,
        ast: NodeId,
    ) -> Self {
        Usage {
            symbol_ref,
            scope,
            origin: source,
            span,
            ast,
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

    /// Returns a reference to the origin of the symbol usage.
    ///
    /// This indicates where the symbol was originally sourced from,
    /// such as the domain or problem context.
    ///
    /// # Returns
    ///
    /// A reference to the [`SymbolOrigin`] enum representing the symbol's provenance.
    pub fn origin(&self) -> SymbolOrigin {
        self.origin
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
            self.ast, self.symbol_kind(), symbol_str, self.scope, self.origin
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
            self.ast, self.symbol_kind(), self.symbol_ident(), self.scope, self.origin
        )?;
        Ok(())
    }
}
