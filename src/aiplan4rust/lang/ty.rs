use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{Id, RemapSymbol, SymbolId, TypeId};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use core::borrow::Borrow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// Représente un typing PDDL générique (atomique ou union via `either`).
/// `ID` peut être un `StringID` (phase syntaxique) ou un `TypeID` (phase sémantique).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Type<ID: Id> {
    /// Liste non vide des identifiants atomiques composant ce typing.
    members: Vec<ID>,
}

impl<ID: Id> Default for Type<ID> {
    fn default() -> Self {
        Self {
            members: Vec::new(),
        }
    }
}

// --- Implémentation Générale (Commune à StringID et TypeID) ---

impl<ID: Id> Type<ID> {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
        }
    }

    pub fn root() -> Self {
        Type::new()
    }

    pub fn primitive(id: ID) -> Self {
        Self { members: vec![id] }
    }

    pub fn either(ids: Vec<ID>) -> Self {
        assert!(!ids.is_empty(), "Un type 'either' ne peut pas être vide.");
        Self { members: ids }
    }

    pub fn add_type(&mut self, member: ID) {
        self.members.push(member);
    }

    pub fn members(&self) -> &[ID] {
        &self.members
    }
    pub fn members_mut(&mut self) -> &mut Vec<ID> {
        &mut self.members
    }
    pub fn len(&self) -> usize {
        self.members.len()
    }
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn is_root(&self) -> bool {
        self.is_empty()
    }

    pub fn is_primitive(&self) -> bool {
        self.members.len() == 1
    }
    pub fn is_either(&self) -> bool {
        self.members.len() > 1
    }

    /// Retourne un itérateur sur les membres du typing.
    pub fn iter(&self) -> std::slice::Iter<'_, ID> {
        self.members.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, ID> {
        self.members.iter_mut()
    }
}

/// Permet d'utiliser `for types in &my_type` directement.
impl<'a, ID: Id> IntoIterator for &'a Type<ID> {
    type Item = &'a ID;
    type IntoIter = std::slice::Iter<'a, ID>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// --- 2. Bloc d'implémentation du Trait Borrow ---
// C'est ici que l'on permet la comparaison "Type <-> Slice" pour l'IndexMap
impl<ID: Id> Borrow<[ID]> for Type<ID> {
    #[inline(always)]
    fn borrow(&self) -> &[ID] {
        &self.members // ou self.members()
    }
}

impl<ID: Id> From<Vec<ID>> for Type<ID> {
    fn from(members: Vec<ID>) -> Self {
        Self { members }
    }
}

// --- Spécialisation pour StringID (Parsing / Syntaxe) ---

impl Type<SymbolId> {
    /* pub fn object() -> &'static Self {
        static OBJECT_TYPE: Lazy<Type<SymbolId>> = Lazy::new(|| {
            Type::primitive(SymbolInterner::OBJECT_SYMBOL_ID)
        });
        &OBJECT_TYPE
    }

    pub fn number() -> &'static Self {
        static NUMBER_TYPE: Lazy<Type<SymbolId>> = Lazy::new(|| {
            Type::primitive(SymbolInterner::NUMBER_SYMBOL_ID)
        });
        &NUMBER_TYPE
    }*/

    pub fn number() -> Self {
        Self::primitive(SymbolInterner::NUMBER_SYMBOL_ID)
    }

    pub fn object() -> Self {
        Self::primitive(SymbolInterner::OBJECT_SYMBOL_ID)
    }

    pub fn is_object(&self) -> bool {
        self.is_primitive() && self.members[0] == SymbolInterner::OBJECT_SYMBOL_ID
    }

    //pub fn is_object(&self) -> bool { self == Self::object() }

    //pub fn is_number(&self) -> bool { self == Self::number() }

    pub fn is_number(&self) -> bool {
        self.is_primitive() && self.members[0] == SymbolInterner::NUMBER_SYMBOL_ID
    }
}

impl Type<TypeId> {
    pub fn number() -> Self {
        Self::primitive(TypeId::NUMBER_TYPE_ID)
    }

    /// Returns true if this typing represents a numeric value.
    ///
    /// A typing is considered numeric if it is primitive and its
    /// single member is the reserved TypeId::NUMBER_TYPE_ID.
    pub fn is_number(&self) -> bool {
        self.is_primitive() && self.members[0].is_number()
    }
}

impl RemapSymbol for Type<SymbolId> {
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        for ident in &mut self.members {
            ident.remap_idents(map)?;
        }
        Ok(())
    }
}

// --- Affichage Spécialisé (Solution 1) ---

impl<ID: Id> fmt::Display for Type<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_primitive() {
            write!(f, "{}", self.members[0])
        } else {
            write!(f, "either(")?;
            for (i, id) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", id)?;
            }
            write!(f, ")")
        }
    }
}

// Spécialisation pour l'affichage via Interner pour StringID
impl InternerDisplay for Type<SymbolId> {
    fn fmt_with_interner(&self, w: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if self.members.is_empty() {
            return write!(w, "<empty>");
        }
        for (i, ty) in self.members.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            match interner.resolve_symbol(*ty) {
                Some(name) => write!(w, "{}", name)?,
                None => write!(w, "{}", ty)?,
            }
        }
        Ok(())
    }
}

// Spécialisation pour TypeID : On affiche l'ID technique (T#1) car l'interner ident ne le connaît pas
impl InternerDisplay for Type<TypeId> {
    fn fmt_with_interner(&self, w: &mut Formatter<'_>, _interner: &SymbolInterner) -> fmt::Result {
        if self.members.is_empty() {
            return write!(w, "<empty>");
        }
        for (i, ty) in self.members.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", ty)?;
        }
        Ok(())
    }
}

// --- Affichage Syntaxique Spécialisé ---

impl SyntaxInternerDisplay for Type<SymbolId> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        match self.members.len() {
            0 => write!(f, "object"),
            1 => {
                let ty = self.members[0];
                match interner.resolve_symbol(ty) {
                    Some(name) => write!(f, "{}", name),
                    None => write!(f, "{}", ty),
                }
            }
            _ => {
                write!(f, "(either")?;
                for ty in &self.members {
                    write!(f, " ")?;
                    match interner.resolve_symbol(*ty) {
                        Some(name) => write!(f, "{}", name)?,
                        None => write!(f, "{}", ty)?,
                    }
                }
                write!(f, ")")
            }
        }
    }
}

// Pour TypeID, la syntaxe PDDL n'est généralement plus requise (déjà compilé),
// mais on fournit un fallback cohérent.
impl SyntaxInternerDisplay for Type<TypeId> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        _interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        write!(f, "{}", self)
    }
}
