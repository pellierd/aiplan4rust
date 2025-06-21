use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::interner::StringInterner;

/// Represents a symbol identified by `Ident` with associated types,
/// also identified by `Ident`.
///
/// This structure models semantic symbols (variables, functions, etc.)
/// along with zero or more associated type identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol {
    symbol: Ident,
    types: Vec<Ident>,
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
    pub fn new(symbol: Ident, types: Vec<Ident>) -> Self {
        TypedSymbol { symbol, types }
    }

    /// Returns the symbol identifier.
    pub fn symbol(&self) -> Ident {
        self.symbol
    }

    /// Returns a reference to the vector of associated type identifiers.
    pub fn types(&self) -> &Vec<Ident> {
        &self.types
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Remap le symbol principal
        if let Some(new_symbol) = map.get(&self.symbol) {
            self.symbol = new_symbol.clone();
        }

        // Remap tous les types dans le vecteur
        for ty in self.types.iter_mut() {
            if let Some(new_ty) = map.get(ty) {
                *ty = new_ty.clone();
            }
        }
    }

    /// Returns a human-readable string representation of the symbol and types,
    /// using the provided string interner to resolve `Ident`s to their string names.
    ///
    /// # Arguments
    /// * `interner` - The string interner used to resolve identifiers.
    ///
    /// # Returns
    /// A `String` representing the symbol and its types.
    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    /// Formats the symbol and associated types using the provided formatter,
    /// resolving identifiers to their names through the given string interner.
    ///
    /// # Arguments
    /// * `w` - A formatter implementing `fmt::Write` (e.g., `String`, or a formatter).
    /// * `interner` - The string interner to resolve identifiers.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure.
    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        match interner.get_str(self.symbol) {
            Some(name) => write!(w, "{}", name)?,
            None => write!(w, "<uninterned:{}>", self.symbol)?,
        }

        if !self.types.is_empty() {
            write!(w, " - ")?;
            for (i, ty) in self.types.iter().enumerate() {
                if i > 0 {
                    write!(w, " ")?;
                }
                match interner.get_str(*ty) {
                    Some(type_name) => write!(w, "{}", type_name)?,
                    None => write!(w, "<uninterned:{}>", ty)?,
                }
            }
        }

        Ok(())
    }
}

impl fmt::Display for TypedSymbol {
    /// Displays the symbol and types by printing their raw `usize` identifiers.
    ///
    /// This does **not** resolve the identifiers via interner; use
    /// [`to_string_with_interner`] for human-readable output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;

        if !self.types.is_empty() {
            write!(f, " - ")?;
            for (i, ty) in self.types.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", ty)?;
            }
        }

        Ok(())
    }
}
