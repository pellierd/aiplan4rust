use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

// Imports de ton projet
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

/// Trait pour tous les wrappers d'identifiants basés sur un index usize.
pub trait Id: Copy + Eq + Default + std::hash::Hash + Serialize + fmt::Display + From<usize> + Into<usize> {
    fn new(idx: usize) -> Self;
    fn as_usize(self) -> usize;
    fn is_valid(self) -> bool {
        self.as_usize() != usize::MAX
    }
}

// --- MACRO DE GÉNÉRATION UNIFORME ---

macro_rules! impl_id_type {
    ($id:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $id {
            pub value: usize,
        }

        impl Id for $id {
            fn new(value: usize) -> Self { Self { value } }
            fn as_usize(self) -> usize { self.value }
        }

        impl Default for $id {
            fn default() -> Self { Self { value: usize::MAX } }
        }

        impl $id {
            pub const fn new(value: usize) -> Self { Self { value } }
            pub fn as_usize(self) -> usize { self.value }
            pub fn is_valid(&self) -> bool { self.value != usize::MAX }
            pub fn invalid_value() -> usize { usize::MAX }
        }

        impl From<usize> for $id {
            fn from(value: usize) -> Self { Self::new(value) }
        }

        impl From<$id> for usize {
            fn from(id: $id) -> Self { id.value }
        }

        impl<T> Index<$id> for Vec<T> {
            type Output = T;
            fn index(&self, id: $id) -> &Self::Output { &self[id.value] }
        }

        impl<T> IndexMut<$id> for Vec<T> {
            fn index_mut(&mut self, id: $id) -> &mut Self::Output { &mut self[id.value] }
        }
    };
}

// --- DÉFINITIONS DES TYPES ---

impl_id_type!(StringID);
impl_id_type!(LiteralID);
impl_id_type!(TypeID);
impl_id_type!(PredicateID);
impl_id_type!(ParameterID);
impl_id_type!(VariableID);
impl_id_type!(FunctionID);
impl_id_type!(NumericFluentID);
impl_id_type!(ObjectID);
impl_id_type!(ObjectFluentID);

// --- LOGIQUE SPÉCIFIQUE (REMAP) ---

impl StringID {
    pub fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
        Ok(())
    }
}

impl LiteralID {
    pub fn remap_literal(&mut self, map: &HashMap<LiteralID, LiteralID>) {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
    }
}

// --- AFFICHAGE (DISPLAY AVEC PRÉFIXES) ---

macro_rules! impl_display_prefix {
    ($id:ident, $prefix:expr) => {
        impl fmt::Display for $id {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", $prefix, self.value)
            }
        }
    };
}

impl_display_prefix!(StringID, "S");
impl_display_prefix!(LiteralID, "@");
impl_display_prefix!(TypeID, "T");
impl_display_prefix!(PredicateID, "P");
impl_display_prefix!(ParameterID, "param");
impl_display_prefix!(VariableID, "var");
impl_display_prefix!(FunctionID, "F");
impl_display_prefix!(NumericFluentID, "NF");
impl_display_prefix!(ObjectID, "O");
impl_display_prefix!(ObjectFluentID, "OF");

// --- TRAITS INTERNER (RESOLUTION) ---

impl InternerDisplay for StringID {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(name) = interner.resolve_ident(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl InternerDisplay for LiteralID {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl SyntaxInternerDisplay for StringID {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(name) = interner.resolve_ident(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned_sym:{}>", self.value)
        }
    }
}

impl SyntaxInternerDisplay for LiteralID {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "<uninterned_lit:{}>", self.value)
        }
    }
}

// --- GROUNDING ---

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArgumentID {
    Object(ObjectID),
    ObjectFluent(ObjectFluentID),
}

impl fmt::Display for ArgumentID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentID::Object(id) => write!(f, "{}", id),
            ArgumentID::ObjectFluent(id) => write!(f, "{}", id),
        }
    }
}
