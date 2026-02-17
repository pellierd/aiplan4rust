use std::collections::HashMap;
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId, Id, TypedSymbol};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};


/// Une liste de symboles typés.
/// SID: ID du symbole (ex: StringID, VariableID)
/// TID: ID du type (ex: StringID, TypeID)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList<SID: Id, TID: Id> {
    symbols: Vec<TypedSymbol<SID, TID>>,
}

impl<SID: Id, TID: Id> TypedList<SID, TID> {
    pub fn new() -> Self {
        Self { symbols: Vec::new() }
    }

    pub fn from_symbols(symbols: Vec<TypedSymbol<SID, TID>>) -> Self {
        Self { symbols }
    }

    pub fn empty() -> Self {
        Self::default()
    }
}

// --- Implémentation de Remap (uniquement pour la phase StringID complète) ---

impl RemapSymbol for TypedList<SymbolId, SymbolId> {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        for ts in &mut self.symbols {
            ts.remap_symbol(map)?;
        }
        Ok(())
    }
}

// --- Delegation (Deref) ---

impl<SID: Id, TID: Id> Deref for TypedList<SID, TID> {
    type Target = Vec<TypedSymbol<SID, TID>>;
    fn deref(&self) -> &Self::Target { &self.symbols }
}

impl<SID: Id, TID: Id> DerefMut for TypedList<SID, TID> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.symbols }
}

// --- Iteration ---

impl<SID: Id, TID: Id> IntoIterator for TypedList<SID, TID> {
    type Item = TypedSymbol<SID, TID>;
    type IntoIter = std::vec::IntoIter<TypedSymbol<SID, TID>>;
    fn into_iter(self) -> Self::IntoIter { self.symbols.into_iter() }
}

impl<'a, SID: Id, TID: Id> IntoIterator for &'a TypedList<SID, TID> {
    type Item = &'a TypedSymbol<SID, TID>;
    type IntoIter = std::slice::Iter<'a, TypedSymbol<SID, TID>>;
    fn into_iter(self) -> Self::IntoIter { self.symbols.iter() }
}

// --- Affichage ---

impl<SID: Id, TID: Id> fmt::Display for TypedList<SID, TID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            write!(f, "{sym}")?;
        }
        write!(f, ")")
    }
}

impl<SID: Id, TID: Id> InternerDisplay for TypedList<SID, TID>
where TypedSymbol<SID, TID>: InternerDisplay
{
    fn fmt_with_interner(&self, f: &mut fmt::Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            sym.fmt_with_interner(f, interner)?;
        }
        write!(f, ")")
    }
}

impl<SID: Id, TID: Id> SyntaxInternerDisplay for TypedList<SID, TID>
where TypedSymbol<SID, TID>: SyntaxInternerDisplay
{
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut fmt::Formatter<'_>, interner: &SymbolInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            sym.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}

/*/// Une liste de symboles typés.
/// ID peut être StringID (syntaxe) ou TypeID (grounding).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList<ID: Id> {
    symbols: Vec<TypedSymbol<ID>>,
}

impl<ID: Id> TypedList<ID> {
    pub fn new() -> Self {
        Self { symbols: Vec::new() }
    }

    pub fn from_symbols(symbols: Vec<TypedSymbol<ID>>) -> Self {
        Self { symbols }
    }

    pub fn empty() -> Self {
        Self::default()
    }
}

// --- Implémentation de Remap (uniquement pour la phase StringID) ---

impl RemapIdents for TypedList<StringID> {
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        for ts in &mut self.symbols {
            ts.remap_idents(map)?;
        }
        Ok(())
    }
}

// --- Delegation (Deref) ---

impl<ID: Id> Deref for TypedList<ID> {
    type Target = Vec<TypedSymbol<ID>>;
    fn deref(&self) -> &Self::Target { &self.symbols }
}

impl<ID: Id> DerefMut for TypedList<ID> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.symbols }
}

// --- Iteration ---

impl<ID: Id> IntoIterator for TypedList<ID> {
    type Item = TypedSymbol<ID>;
    type IntoIter = std::vec::IntoIter<TypedSymbol<ID>>;
    fn into_iter(self) -> Self::IntoIter { self.symbols.into_iter() }
}

impl<'a, ID: Id> IntoIterator for &'a TypedList<ID> {
    type Item = &'a TypedSymbol<ID>;
    type IntoIter = std::slice::Iter<'a, TypedSymbol<ID>>;
    fn into_iter(self) -> Self::IntoIter { self.symbols.iter() }
}

// --- Affichage ---

impl<ID: Id> fmt::Display for TypedList<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            write!(f, "{sym}")?;
        }
        write!(f, ")")
    }
}

impl<ID: Id> InternerDisplay for TypedList<ID>
where TypedSymbol<ID>: InternerDisplay
{
    fn fmt_with_interner(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            sym.fmt_with_interner(f, interner)?;
        }
        write!(f, ")")
    }
}

impl<ID: Id> SyntaxInternerDisplay for TypedList<ID>
where TypedSymbol<ID>: SyntaxInternerDisplay
{
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        // Note: La syntaxe PDDL des listes n'utilise pas toujours des parenthèses 
        // selon l'endroit (ex: paramètres d'action), mais on suit ton implémentation actuelle.
        for (i, sym) in self.symbols.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            sym.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}*/
