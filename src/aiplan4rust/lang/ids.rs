use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

// Imports de ton projet
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

/// Trait pour tous les wrappers d'identifiants basés sur un index usize.
pub trait Id: Copy + Eq + Ord + Default + std::hash::Hash + Serialize + fmt::Display + From<usize> + Into<usize> {
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

impl_id_type!(SymbolId);
impl_id_type!(LiteralId);

impl_id_type!(TypeId);
impl_id_type!(VariableId);
impl_id_type!(ObjectId);
impl_id_type!(FluentId);
impl_id_type!(NumericFluentId);

impl_id_type!(PredicateSymbolId);
impl_id_type!(FunctionSymbolId);
impl_id_type!(PreferenceSymbolId);
impl_id_type!(TaskSymbolId);
impl_id_type!(ActionSymbolId);
impl_id_type!(MethodSymbolId);
impl_id_type!(TaskLabelSymbolId);

impl_id_type!(AtomSkeletonId);
impl_id_type!(FunctionSkeletonId);
impl_id_type!(TaskSkeletonId);

// --- LOGIQUE SPÉCIFIQUE (REMAP) ---

impl SymbolId {
    pub fn remap_idents(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
        Ok(())
    }
}

impl LiteralId {
    pub fn remap_literal(&mut self, map: &HashMap<LiteralId, LiteralId>) {
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

// --- Symbols & Interning ---
// s1, s42 (Generic internal symbol)
impl_display_prefix!(SymbolId, "s");
// L10 (Literal value, usually a constant or a boolean)
impl_display_prefix!(LiteralId, "L");

// --- Domain & Typing ---
// T1, T_robot (Type identifier)
impl_display_prefix!(TypeId, "T");
// v0, v1 (Variable identifier)
impl_display_prefix!(VariableId, "v");
// o12 (Instance of an object/constant)
impl_display_prefix!(ObjectId, "o");

// --- Domain Model Symbols (Names) ---
// p5 (Predicate name/symbol)
impl_display_prefix!(PredicateSymbolId, "p");
// f2 (Function name/symbol - lower case to distinguish from skeleton)
impl_display_prefix!(FunctionSymbolId, "f");
// tk4 (Task symbol - specific to HTN/Planning)
impl_display_prefix!(TaskSymbolId, "tk");
// a3 (Action symbol/operator name)
impl_display_prefix!(ActionSymbolId, "a");
// m7 (Method symbol in HTN planning)
impl_display_prefix!(MethodSymbolId, "m");
// pr1 (Preference symbol for soft constraints)
impl_display_prefix!(PreferenceSymbolId, "pr");
// TL9 (Task Label instance - Upper case to distinguish from Task Symbol)
impl_display_prefix!(TaskLabelSymbolId, "TL");

// --- Fluents (State Variables) ---
// fl8 (Generic fluent - used for state tracking)
impl_display_prefix!(FluentId, "fl");
// nf4 (Numeric fluent - e.g., battery levels, distances)
impl_display_prefix!(NumericFluentId, "nf");
// of2 (Object fluent - a state variable returning an ObjectId)

// --- Skeletons (Structural Instances) ---
// AS9 (Atom Skeleton: Predicate + Arguments)
impl_display_prefix!(AtomSkeletonId, "AS");
// FS2 (Function Skeleton: Function + Arguments)
impl_display_prefix!(FunctionSkeletonId, "FS");
// TS7 (Task Skeleton: Task + Arguments)
impl_display_prefix!(TaskSkeletonId, "TS");
// --- TRAITS INTERNER (RESOLUTION) ---

impl TypeId {

    /// Identifiant réservé pour le type "number" (fonctions numériques).
    /// Correspond au SymbolId(1) dans le SymbolInterner.
    pub const NUMBER_TYPE_ID: Self = Self::new(1);

    /// Checks if this type represents a numeric value.
    ///
    /// In our system, the numeric type is globally identified
    /// by the constant TypeId::NUMBER_TYPE_ID (index 1).
    pub fn is_number(&self) -> bool {
        self.as_usize() == Self::NUMBER_TYPE_ID.as_usize()
    }
}

impl InternerDisplay for SymbolId {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if let Some(name) = interner.resolve_symbol(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", SymbolInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl InternerDisplay for LiteralId {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "{}", SymbolInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl SyntaxInternerDisplay for SymbolId {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &SymbolInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(name) = interner.resolve_symbol(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned_sym:{}>", self.value)
        }
    }
}

impl SyntaxInternerDisplay for LiteralId {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &SymbolInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "<uninterned_lit:{}>", self.value)
        }
    }
}
