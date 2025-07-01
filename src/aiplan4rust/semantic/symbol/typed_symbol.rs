use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::semantic::symbol::Type;
use crate::aiplan4rust::syntax::ast::{Ast, FromAst};
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Represents a symbol identified by `Ident` with associated types,
/// also identified by `Ident`.
///
/// This structure models semantic symbols (variables, functions, etc.)
/// along with zero or more associated type identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol {
    symbol: Ident,
    ty: Type,
}

impl TypedSymbol {
    /// Creates a new `TypedSymbol` from a symbol and its associated types.
    ///
    /// # Arguments
    /// * `symbol` - The main symbol identifier.
    /// * `types` - A vector of associated type identifiers.
    ///
    /// # Returns
    /// A new `TypedSymbol` instance.
    pub fn new(symbol: Ident, types: Type) -> Self {
        TypedSymbol { symbol, ty: types }
    }

    /// Returns the symbol identifier.
    pub fn symbol(&self) -> Ident {
        self.symbol
    }

    /// Returns a reference to the vector of associated type identifiers.
    pub fn types(&self) -> &Type {
        &self.ty
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Remap le symbol principal
        if let Some(new_symbol) = map.get(&self.symbol) {
            self.symbol = new_symbol.clone();
        }

        // Remap tous les types dans le vecteur
        for ty in self.ty.iter_mut() {
            if let Some(new_ty) = map.get(ty) {
                *ty = new_ty.clone();
            }
        }
    }
}

impl fmt::Display for TypedSymbol {
    /// Displays the symbol and types by printing their raw `usize` identifiers.
    ///
    /// This does **not** resolve the identifiers via interner; use
    /// [`to_string_with_interner`] for human-readable output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;

        if !self.ty.is_empty() {
            write!(f, " - ")?;
            for (i, ty) in self.ty.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", ty)?;
            }
        }

        Ok(())
    }
}

impl DisplayWithInterner for TypedSymbol {
    /// Formats the symbol and its associated type using the string interner.
    ///
    /// # Arguments
    /// * `w` - The formatter to write to.
    /// * `interner` - The string interner used to resolve identifiers.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with(
        &self,
        w: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        // Format the symbol name
        match interner.resolve(self.symbol) {
            Some(name) => write!(w, "{}", name)?,
            None => write!(w, "<uninterned:{}>", self.symbol)?,
        }

        // If type is not empty, format it after a separator
        if !self.ty.is_empty() {
            write!(w, " - ")?;
            self.ty.fmt_with(w, interner)?;
        }

        Ok(())
    }
}

impl FromAst for TypedSymbol {
    /// Constructs a `TypedSymbol` from an AST node.
    ///
    /// # Expectations
    /// - The provided `node` **must** be of kind `TypedItem`.
    /// - The node has **at most two children**:
    ///   - The **first child** is the symbol (mandatory).
    ///   - The **second child** is the type (optional).
    /// - If the second child (type) is absent, returns a `TypedSymbol` with an empty `Type`.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if accessing the children or parsing fails.
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        let children = node.children();
        let symbol_node = ast.try_node(children[0])?;

        let ty = if children.len() > 1 {
            let ty_node = ast.try_node(children[1])?;
            Type::from_ast(ty_node, ast)?
        } else {
            Type::new()
        };

        Ok(TypedSymbol::new(symbol_node.try_ident()?, ty))
    }
}
