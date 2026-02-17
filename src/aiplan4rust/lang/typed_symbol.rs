use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{SymbolId, Id, Type, RemapSymbol};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;



#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol<SID: Id, TID: Id> {
    /// SID est l'ID du symbole (StringID, VariableID, etc.)
    symbol: SID,
    /// TID est l'ID utilisé pour le type
    ty: Type<TID>,
}

impl<SID: Id, TID: Id> TypedSymbol<SID, TID> {
    pub fn new(symbol: SID, ty: Type<TID>) -> Self {
        TypedSymbol { symbol, ty }
    }

    pub fn symbol(&self) -> SID { self.symbol }
    pub fn set_symbol(&mut self, symbol: SID) { self.symbol = symbol; }

    pub fn ty(&self) -> &Type<TID> { &self.ty }
    pub fn ty_mut(&mut self) -> &mut Type<TID> { &mut self.ty }
    pub fn set_ty(&mut self, ty: Type<TID>) { self.ty = ty; }
}

impl<SID, TID> fmt::Display for TypedSymbol<SID, TID>
where
    SID: Id + fmt::Display,
    TID: Id + fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Affiche "symbole - type"
        write!(f, "{} - {}", self.symbol, self.ty)
    }
}

impl RemapSymbol for TypedSymbol<SymbolId, SymbolId> {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.ty.remap_symbol(map)?;
        Ok(())
    }
}

/*impl<SID: Id, TID: Id> InternerDisplay for TypedSymbol<SID, TID>
where
    SID: InternerDisplay, // SID doit savoir s'afficher avec l'interner
    Type<TID>: InternerDisplay
{
    fn fmt_with_interner(&self, w: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        // On délègue l'affichage du symbole à son propre type SID
        self.symbol.fmt_with_interner(w, interner)?;

        if !self.types.is_empty() {
            write!(w, " - ")?;
            self.types.fmt_with_interner(w, interner)?;
        }
        Ok(())
    }
}*/

impl InternerDisplay for TypedSymbol<SymbolId, SymbolId> {
    fn fmt_with_interner(&self, w: &mut fmt::Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        // Comme self.symbol est un StringID, il implémente InternerDisplay
        self.symbol.fmt_with_interner(w, interner)?;

        if !self.ty.is_empty() {
            write!(w, " - ")?;
            // Comme self.types est un Type<StringID>, il implémente InternerDisplay
            self.ty.fmt_with_interner(w, interner)?;
        }
        Ok(())
    }
}

impl SyntaxInternerDisplay for TypedSymbol<SymbolId, SymbolId> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize
    ) -> fmt::Result {
        write_indent(f, indent)?;

        // Utilise directement l'implémentation de StringID pour le symbole
        self.symbol.fmt_syntax_with_interner(f, interner)?;

        if !self.ty.is_empty() {
            write!(f, " - ")?;
            // Utilise l'implémentation de Type<StringID>
            self.ty.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}

/*impl<SID: Id, TID: Id> SyntaxInternerDisplay for TypedSymbol<SID, TID>
where
    SID: SyntaxInternerDisplay,
    Type<TID>: SyntaxInternerDisplay
{
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        self.symbol.fmt_syntax_with_interner(f, interner)?;

        if !self.types.is_empty() {
            write!(f, " - ")?;
            self.types.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}*/


/*/// Représente un symbole typé.
/// ID peut être StringID (syntaxe) ou TypeID (grounding/sémantique pour les types).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedSymbol<ID: Id> {
    /// Le symbole reste généralement un StringID (le nom de la variable/constante)
    /// mais on pourrait aussi le rendre générique si besoin. 
    /// Ici, on garde StringID pour le nom et ID pour le type.
    symbol: StringID,
    types: Type<ID>,
}

impl<ID: Id> TypedSymbol<ID> {
    pub fn new(symbol: StringID, types: Type<ID>) -> Self {
        TypedSymbol { symbol, types: types }
    }

    pub fn symbol(&self) -> StringID { self.symbol }
    pub fn set_symbol(&mut self, symbol: StringID) { self.symbol = symbol; }

    pub fn types(&self) -> &Type<ID> { &self.types }
    pub fn ty_mut(&mut self) -> &mut Type<ID> { &mut self.types }
    pub fn set_ty(&mut self, types: Type<ID>) { self.types = types; }

}

// --- Remap (uniquement pour la phase StringID) ---

impl RemapIdents for TypedSymbol<StringID> {
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.types.remap_idents(map)?;
        Ok(())
    }
}

// --- Affichage ---

impl<ID: Id> fmt::Display for TypedSymbol<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;
        if !self.types.is_empty() {
            write!(f, " - {}", self.types)?;
        }
        Ok(())
    }
}

// Spécialisation de l'InternerDisplay
impl<ID: Id> InternerDisplay for TypedSymbol<ID>
where Type<ID>: InternerDisplay
{
    fn fmt_with_interner(&self, w: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        match interner.resolve_ident(self.symbol) {
            Some(name) => write!(w, "{}", name)?,
            None => write!(w, "{}", self.symbol)?,
        }

        if !self.types.is_empty() {
            write!(w, " - ")?;
            self.types.fmt_with_interner(w, interner)?;
        }
        Ok(())
    }
}

impl<ID: Id> SyntaxInternerDisplay for TypedSymbol<ID>
where Type<ID>: SyntaxInternerDisplay
{
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        match interner.resolve_ident(self.symbol) {
            Some(name) => write!(f, "{}", name)?,
            None => write!(f, "{}", self.symbol)?,
        }

        if !self.types.is_empty() {
            write!(f, " - ")?;
            self.types.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}*/
