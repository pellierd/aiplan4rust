use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

// Imports de ton projet
use crate::aiplan4rust::compiler::syntax::{write_indent, SyntaxInternerDisplay};
use crate::aiplan4rust::support::interner::{InternerDisplay, InternerError, SymbolInterner};

// Masque sur les 60 bits de poids faible (Espace d'adressage utile)
pub const INDEX_MASK: usize = (1 << 60) - 1;
pub const DISCRIM_MASK: usize = !INDEX_MASK;

// Unique sentinelle globale du système (2^60 - 1)
pub const RAW_NONE: usize = INDEX_MASK;

pub trait Id:
    Copy + Eq + Ord + Default + std::hash::Hash + Serialize + fmt::Display + From<usize> + Into<usize>
{
    fn new(idx: usize) -> Self;
    fn as_usize(self) -> usize;
    fn is_none(self) -> bool;
    fn is_some(self) -> bool;
}

#[macro_export]
macro_rules! impl_id_type {
    ($id:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Ord,
            PartialOrd,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $id {
            pub value: usize,
        }
        $crate::impl_id_type_core!($id);
    };

    ($id:ident, custom_serde) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
        pub struct $id {
            pub value: usize,
        }
        $crate::impl_id_type_core!($id);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_id_type_core {
    ($id:ident) => {
        impl $crate::aiplan4rust::support::lang::id::Id for $id {
            #[inline(always)]
            fn new(value: usize) -> Self {
                assert!(
                    value <= $crate::aiplan4rust::support::lang::id::RAW_NONE,
                    "Index brut hors limite"
                );
                Self { value }
            }

            #[inline(always)]
            fn as_usize(self) -> usize {
                self.value & $crate::aiplan4rust::support::lang::id::INDEX_MASK
            }

            #[inline(always)]
            fn is_none(self) -> bool {
                let idx = self.value & $crate::aiplan4rust::support::lang::id::INDEX_MASK;
                idx == $crate::aiplan4rust::support::lang::id::RAW_NONE
            }

            #[inline(always)]
            fn is_some(self) -> bool {
                !self.is_none()
            }
        }

        impl Default for $id {
            #[inline(always)]
            fn default() -> Self {
                Self::NONE
            }
        }

        impl $id {
            pub const NONE: Self = Self {
                value: $crate::aiplan4rust::support::lang::id::RAW_NONE,
            };

            #[inline(always)]
            pub const fn new(value: usize) -> Self {
                Self { value }
            }

            #[inline(always)]
            pub fn as_usize(self) -> usize {
                self.value & $crate::aiplan4rust::support::lang::id::INDEX_MASK
            }

            #[inline(always)]
            pub fn is_none(self) -> bool {
                let idx = self.as_usize();
                idx == $crate::aiplan4rust::support::lang::id::RAW_NONE
            }

            #[inline(always)]
            pub fn is_some(self) -> bool {
                !self.is_none()
            }

            #[inline(always)]
            pub fn is_valid(&self) -> bool {
                // Valide si l'ID nettoyé de ses flags est strictement inférieur à la sentinelle
                (self.value & $crate::aiplan4rust::support::lang::id::INDEX_MASK) < $crate::aiplan4rust::support::lang::id::RAW_NONE
            }

            #[inline(always)]
            pub fn get_bit(self, bit_index: u8) -> bool {
                assert!(bit_index < 4, "L'index du bit doit être entre 0 et 3");
                let actual_shift = 60 + bit_index;
                ((self.value >> actual_shift) & 1) == 1
            }

            #[inline(always)]
            pub fn set_bit(self, bit_index: u8, value: bool) -> Self {
                assert!(bit_index < 4, "L'index du bit doit être entre 0 et 3");
                let actual_shift = 60 + bit_index;
                let bit_mask = 1 << actual_shift;

                let mut new_value = self.value;
                if value {
                    new_value |= bit_mask;
                } else {
                    new_value &= !bit_mask;
                }
                Self { value: new_value }
            }

            /// Retourne la valeur interne brute contenant l'index ET les bits de flags (ex: le bit 60 de négation).
            /// ⚠️ À utiliser avec précaution, principalement réservé à l'encodage opaque (Datalog, PNF).
            #[inline(always)]
            pub const fn as_raw_usize(self) -> usize {
                self.value
            }
        }

       impl From<usize> for $id {
            #[inline(always)]
            fn from(value: usize) -> Self {
                // 1. Isolation de l'index numérique pur (60 bits de poids faible)
                let pure_index = value & $crate::aiplan4rust::support::lang::id::INDEX_MASK;

                // 2. 🛡️ Sécurité préservée : On s'assure que l'index utile ne déborde pas sur la sentinelle (2^60 - 1)
                assert!(
                    pure_index < $crate::aiplan4rust::support::lang::id::RAW_NONE,
                    "L'index numérique pur donné écrase les sentinelles système !"
                );

                Self { value }
            }
        }

        impl From<&$id> for $id {
            #[inline(always)]
            fn from(id: &$id) -> Self {
                *id
            }
        }

        impl From<$id> for usize {
            #[inline(always)]
            fn from(id: $id) -> Self {
                // 🛡️ Zéro dette technique : Into<usize> renvoie TOUJOURS l'index pur sans les flags.
                // Cela évite de corrompre l'indexation standard des Vec à travers le reste du projet.
                id.as_usize()
            }
        }



        impl<T> ::std::ops::Index<$id> for Vec<T> {
            type Output = T;
            #[inline(always)]
            fn index(&self, id: $id) -> &T {
                assert!(
                    id.is_valid(),
                    "Tentative d'indexation avec un ID système invalide ou non initialisé !"
                );
                &self[id.as_usize()]
            }
        }

        impl<T> ::std::ops::IndexMut<$id> for Vec<T> {
            #[inline(always)]
            fn index_mut(&mut self, id: $id) -> &mut T {
                assert!(
                    id.is_valid(),
                    "Tentative d'indexation mutable avec un ID système invalide ou non initialisé !"
                );
                &mut self[id.as_usize()]
            }
        }
    };
}

// --- DÉFINITIONS DES TYPES CONCRETS ---
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
impl_id_type!(ActionDefId);
impl_id_type!(MethodSymbolId);
impl_id_type!(TaskLabelSymbolId);
impl_id_type!(DerivedPredicateDefId);
impl_id_type!(FunctionSkeletonId);
impl_id_type!(TaskSkeletonId);
impl_id_type!(AtomSkeletonId);
impl_id_type!(TypedListId);

// --- EXTENSIONS SPÉCIFIQUES À ATOM_SKELETON_ID ---

const ATOM_NEGATION_BIT_INDEX: u8 = 0;

impl AtomSkeletonId {
    #[inline(always)]
    pub fn is_negated(self) -> bool {
        self.get_bit(ATOM_NEGATION_BIT_INDEX)
    }

    #[inline(always)]
    pub fn set_negated(&mut self, negated: bool) {
        *self = self.set_bit(ATOM_NEGATION_BIT_INDEX, negated);
    }

    #[inline(always)]
    pub fn strip_negation(self) -> Self {
        self.set_bit(ATOM_NEGATION_BIT_INDEX, false)
    }
}

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

// --- AFFICHAGE UNIFORME ---

macro_rules! impl_display_prefix {
    ($id:ident, $prefix:expr) => {
        impl fmt::Display for $id {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", $prefix, self.as_usize())
            }
        }
    };
}

impl_display_prefix!(SymbolId, "s");
impl_display_prefix!(LiteralId, "L");
impl_display_prefix!(TypeId, "T");
impl_display_prefix!(VariableId, "v");
impl_display_prefix!(ObjectId, "o");
impl_display_prefix!(PredicateSymbolId, "p");
impl_display_prefix!(FunctionSymbolId, "f");
impl_display_prefix!(TaskSymbolId, "tk");
impl_display_prefix!(ActionSymbolId, "a");
impl_display_prefix!(ActionDefId, "ad");
impl_display_prefix!(DerivedPredicateDefId, "dp");
impl_display_prefix!(MethodSymbolId, "m");
impl_display_prefix!(PreferenceSymbolId, "pr");
impl_display_prefix!(TaskLabelSymbolId, "TL");
impl_display_prefix!(FluentId, "fl");
impl_display_prefix!(NumericFluentId, "nf");
impl_display_prefix!(FunctionSkeletonId, "FS");
impl_display_prefix!(TaskSkeletonId, "TS");
impl_display_prefix!(TypedListId, "tl");

impl fmt::Display for AtomSkeletonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_negated() {
            write!(f, "not_AS#{}", self.as_usize())
        } else {
            write!(f, "AS#{}", self.as_usize())
        }
    }
}

// --- DETAILS DU TYPEID ---

impl TypeId {
    // La racine prend l'index 0 (Null-Object Pattern)
    pub const ROOT_TYPE_ID: Self = Self::new(0);
    // Le type "number" prend l'index 1 (ou une autre valeur standard)
    pub const NUMBER_TYPE_ID: Self = Self::new(1);

    #[inline]
    pub const fn root() -> Self {
        Self::ROOT_TYPE_ID
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.as_usize() == 0
    }

    #[inline]
    pub const fn number() -> Self {
        Self::NUMBER_TYPE_ID
    }

    #[inline]
    pub fn is_number(&self) -> bool {
        self.as_usize() == Self::NUMBER_TYPE_ID.as_usize()
    }
}

// --- DETAILS DU TYPEDLISTID ---

impl TypedListId {
    /// Constante globale représentant une liste typée vide (Index 0).
    pub const EMPTY: Self = Self::new(0);

    /// Retourne l'identifiant d'une liste vide.
    #[inline(always)]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Indique si la liste référencée est la liste vide par défaut.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.as_usize() == 0
    }
}

// --- TRAITS INTERNER (RESOLUTION) ---

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
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(name) = interner.resolve_symbol(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned_sym:{}>", self.value)
        }
    }
}

impl SyntaxInternerDisplay for LiteralId {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "<uninterned_lit:{}>", self.value)
        }
    }
}
