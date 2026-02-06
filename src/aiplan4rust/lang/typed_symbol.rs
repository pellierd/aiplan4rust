use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::{StringID, Id, Type, RemapIdents};
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

impl RemapIdents for TypedSymbol<StringID, StringID> {
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.ty.remap_idents(map)?;
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

        if !self.ty.is_empty() {
            write!(w, " - ")?;
            self.ty.fmt_with_interner(w, interner)?;
        }
        Ok(())
    }
}*/

impl InternerDisplay for TypedSymbol<StringID, StringID> {
    fn fmt_with_interner(&self, w: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        // Comme self.symbol est un StringID, il implémente InternerDisplay
        self.symbol.fmt_with_interner(w, interner)?;

        if !self.ty.is_empty() {
            write!(w, " - ")?;
            // Comme self.ty est un Type<StringID>, il implémente InternerDisplay
            self.ty.fmt_with_interner(w, interner)?;
        }
        Ok(())
    }
}

impl SyntaxInternerDisplay for TypedSymbol<StringID, StringID> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
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

        if !self.ty.is_empty() {
            write!(f, " - ")?;
            self.ty.fmt_syntax_with_interner(f, interner)?;
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
    ty: Type<ID>,
}

impl<ID: Id> TypedSymbol<ID> {
    pub fn new(symbol: StringID, types: Type<ID>) -> Self {
        TypedSymbol { symbol, ty: types }
    }

    pub fn symbol(&self) -> StringID { self.symbol }
    pub fn set_symbol(&mut self, symbol: StringID) { self.symbol = symbol; }

    pub fn ty(&self) -> &Type<ID> { &self.ty }
    pub fn ty_mut(&mut self) -> &mut Type<ID> { &mut self.ty }
    pub fn set_ty(&mut self, ty: Type<ID>) { self.ty = ty; }

}

// --- Remap (uniquement pour la phase StringID) ---

impl RemapIdents for TypedSymbol<StringID> {
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.ty.remap_idents(map)?;
        Ok(())
    }
}

// --- Affichage ---

impl<ID: Id> fmt::Display for TypedSymbol<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;
        if !self.ty.is_empty() {
            write!(f, " - {}", self.ty)?;
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

        if !self.ty.is_empty() {
            write!(w, " - ")?;
            self.ty.fmt_with_interner(w, interner)?;
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

        if !self.ty.is_empty() {
            write!(f, " - ")?;
            self.ty.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}*/
