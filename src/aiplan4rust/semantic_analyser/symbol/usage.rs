use crate::aiplan4rust::semantic_analyser::symbol::{Scope, Source, SymbolKind};
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// The AST node index where the symbol is used.
    ast: usize,

    /// The kind of the symbol used (e.g., variable, function).
    kind: SymbolKind,

    /// The scope where the symbol is used (e.g., function, block).
    scope: Scope,

    /// The source of the usage (e.g., file or module).
    source: Source,
}

impl Usage {
    /// Constructeur pour créer un nouveau `Usage`
    pub fn new(ast: usize, kind: SymbolKind, scope: Scope, source: Source) -> Self {
        Usage {
            ast,
            kind,
            scope,
            source,
        }
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
    pub fn source(&self) -> &Source {
        &self.source
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
    pub fn source_mut(&mut self) -> &mut Source {
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
    pub fn set_source(&mut self, source: Source) {
        self.source = source;
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[index: {}, kind: {}, scope: {}, usage: {}]",
            self.ast, self.kind, self.scope, self.source
        )?;
        Ok(())
    }
}
