//! Module defining the `Type` abstraction for the syntax problem intermediate representation (IR).
//!
//! This module provides a representation of PDDL types as non-empty lists of atomic identifiers,
//! supporting both primitive (atomic) types and union types (referred to as `either` in PDDL).
//!
//! A `Type` can represent:
//! - A single primitive type (e.g., `vehicle`)
//! - A union of multiple primitives (e.g., `either car truck`)
//!
//! Internally, the type stores a flat vector of `Ident`, simplifying processing
//! while preserving expressiveness for parsing, type checking, and semantic analysis.

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::error::LangError;
use crate::aiplan4rust::lang::{Ident, RemapIdents};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode, SyntaxSubtree};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// Represents a PDDL type in the syntax problem IR.
///
/// A `Type` is always a non-empty collection of atomic type identifiers.
/// - If it has exactly one member, it represents a primitive (atomic) type.
/// - If it has multiple members, it represents a union type (`either` in PDDL).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Type {
    /// Non-empty list of atomic type identifiers.
    members: Vec<Ident>,
}

impl Type {
    /// Creates a new empty `Type`.
    ///
    /// # Returns
    /// A `Type` instance with no members.
    pub fn new() -> Self {
        Self { members: Vec::new() }
    }

    /// Returns the members of this type.
    ///
    /// The returned slice contains the identifiers of all atomic types
    /// that compose this type.
    pub fn members(&self) -> &[Ident] {
        &self.members
    }

    /// Returns a mutable reference to the members of this type.
    ///
    /// This allows in-place modification of the atomic type identifiers
    /// that compose this `Type`.
    pub fn members_mut(&mut self) -> &mut Vec<Ident> {
        &mut self.members
    }

    /// Replaces the members of this type.
    ///
    /// The provided vector must be non-empty and should contain
    /// atomic type identifiers.
    pub fn set_members(&mut self, members: Vec<Ident>) {
        self.members = members;
    }

    /// Returns a reference to the canonical `object` type.
    ///
    /// # Example
    /// ```
    /// let obj_type = Type::object();
    /// assert!(obj_type.is_object());
    /// ```
    pub fn object() -> &'static Self {
        static OBJECT_TYPE: Lazy<Type> = Lazy::new(|| {
            let mut t = Type::new();
            t.add_type(StringInterner::IDENT_OBJECT);
            t
        });
        &OBJECT_TYPE
    }

    /// Returns `true` if this type is the `object` type.
    pub fn is_object(&self) -> bool {
        self == Type::object()
    }

    /// Returns a reference to the canonical `number` type.
    ///
    /// # Example
    /// ```
    /// let num_type = Type::number();
    /// assert!(num_type.is_number());
    /// ```
    pub fn number() -> &'static Self {
        static NUMBER_TYPE: Lazy<Type> = Lazy::new(|| {
            let mut t = Type::new();
            t.add_type(StringInterner::IDENT_NUMBER);
            t
        });
        &NUMBER_TYPE
    }

    /// Returns `true` if this type is the `number` type.
    pub fn is_number(&self) -> bool {
        self == Type::number()
    }

    /// Creates a new primitive (atomic) type from a single identifier.
    pub fn primitive(id: Ident) -> Self {
        Self { members: vec![id] }
    }

    /// Creates a new union type (`either`) from a non-empty list of identifiers.
    ///
    /// # Panics
    /// Panics if `ids` is empty.
    pub fn either(ids: Vec<Ident>) -> Self {
        Self { members: ids }
    }

    /// Adds a new atomic type identifier to this `Type`.
    pub fn add_type(&mut self, member: Ident) {
        self.members.push(member);
    }

    /// Returns `true` if the type is primitive (contains exactly one member).
    pub fn is_primitive(&self) -> bool {
        self.members.len() == 1 || self.is_number() || self.is_object()
    }

    /// Returns `true` if the type is a union (`either`) of atomic types.
    pub fn is_either(&self) -> bool {
        !self.is_number() && !self.is_object() && self.members.len() > 1
    }

    /// Returns `true` if the type has no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns the number of members in this type.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns a slice of all type members.
    pub fn as_slice(&self) -> &[Ident] {
        &self.members
    }

    /// Returns an iterator over the members.
    pub fn iter(&self) -> std::slice::Iter<'_, Ident> {
        self.members.iter()
    }

    /// Returns a mutable iterator over the members.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Ident> {
        self.members.iter_mut()
    }

    /// Consumes the type and returns an iterator over its members.
    pub fn into_iter(self) -> std::vec::IntoIter<Ident> {
        self.members.into_iter()
    }


}

impl RemapIdents for Type {
    /// Remaps all atomic type identifiers (`Ident`) contained in this `Type`
    /// according to the provided mapping table.
    ///
    /// Each member of the type is updated if a corresponding entry exists in `map`.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap` associating old `Ident` values with their new `Ident`s.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) -> Result<(), InternerError>{
        for ident in &mut self.members {
            ident.remap_idents(map)?;
        }
        Ok(())
    }
}



impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_primitive() {
            write!(f, "{}", self.members[0])
        } else {
            write!(f, "either(")?;
            for (i, id) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "t{}", id)?;
            }
            write!(f, ")")
        }
    }
}

impl InternerDisplay for Type {
    /// Formats the type_checker by resolving its member identifiers using the string interner.
    ///
    /// # Arguments
    /// * `w` - The formatter to write to.
    /// * `interner` - The string interner used to resolve identifiers.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(
        &self,
        w: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        if !self.members.is_empty() {
            for (i, ty) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(w, " ")?;
                }
                match interner.resolve_ident(*ty) {
                    Some(type_name) => write!(w, "{}", type_name)?,
                    None => write!(w, "<uninterned:{}>", ty)?,
                }
            }
            Ok(())
        } else {
            write!(w, "<empty>")?;
            Ok(())
        }
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for `Type`.
///
/// This trait formats a `Type` for PDDL-like syntax display.
///
/// - If the type_checker has no members, it produces no output.
/// - If the type_checker has a single member, it prints the member name.
/// - If the type_checker has multiple members, it prints `(either t1 t2 ...)`.
///
/// # Examples
///
/// ```
/// # use your_crate::{Type, StringInterner, PlanningDisplay};
/// # use std::fmt::Write;
///
/// let interner = StringInterner::new();
/// let mut t = Type::default();
///
/// // Example 1: empty
/// let mut s = String::new();
/// t.fmt_planning(&mut s, &interner).unwrap();
/// assert_eq!(s, "");
///
/// // Example 2: single type_checker
/// let id = interner.get_or_intern("robot");
/// t.members.push(id);
/// let mut s = String::new();
/// t.fmt_planning(&mut s, &interner).unwrap();
/// assert_eq!(s, "robot");
///
/// // Example 3: multiple types
/// t.members.push(interner.get_or_intern("vehicle"));
/// let mut s = String::new();
/// t.fmt_planning(&mut s, &interner).unwrap();
/// assert_eq!(s, "(either robot vehicle)");
/// ```
impl SyntaxInternerDisplay for Type {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        match self.members.len() {
            0 => write!(f, "object"), // Pas de .to_string()
            1 => {
                let ty = self.members[0];
                match interner.resolve_ident(ty) {
                    Some(type_name) => write!(f, "{}", type_name),
                    None => write!(f, "<uninterned:{}>", ty),
                }
            }
            _ => {
                write!(f, "(either")?;
                for ty in &self.members {
                    write!(f, " ")?;
                    match interner.resolve_ident(*ty) {
                        Some(type_name) => write!(f, "{}", type_name)?,
                        None => write!(f, "<uninterned:{}>", ty)?,
                    }
                }
                write!(f, ")")
            }
        }
    }
}
