use std::fmt;
use std::fmt::Formatter;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::ast::{AstNode, FromAst};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode, SyntaxTree};

/// Represents a type in a planning problem IR.
///
/// A type is always represented as a non-empty list of atomic type identifiers.
/// If the list contains a single identifier, it represents an atomic (primitive) type.
/// If it contains multiple identifiers, it represents a union (called `either` in PDDL) of types.
///
/// This structure allows easy representation of both simple and union types
/// while keeping the internal model flat and efficient.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Type {
    /// Non-empty list of atomic type identifiers.
    members: Vec<Ident>,
}

impl Type {

    /// Creates a new empty `Type` with no members.
    ///
    /// # Returns
    ///
    /// A new `Type` instance with an empty list of members.
    pub fn new() -> Self {
        Self { members: Vec::new() }
    }

    /// Returns a static reference to the constant `OBJECT_TYPE`.
    ///
    /// This is a lazily initialized `Type` instance containing
    /// the identifier `StringInterner::IDENT_OBJECT`.
    ///
    /// The `Lazy` ensures that the initialization happens only once,
    /// on first access.
    ///
    /// # Example
    ///
    /// ```
    /// let obj_type = Type::object();
    /// // use obj_type here
    /// ```
    pub fn object() -> &'static Self {
        static OBJECT_TYPE: Lazy<Type> = Lazy::new(|| {
            let mut t = Type::new();
            t.add_type(StringInterner::IDENT_OBJECT);
            t
        });
        &OBJECT_TYPE
    }

    /// Returns a static reference to the constant `NUMBER_TYPE`.
    ///
    /// This is a lazily initialized `Type` instance containing
    /// the identifier `StringInterner::IDENT_NUMBER`.
    ///
    /// The `Lazy` ensures that the initialization happens only once,
    /// on first access.
    ///
    /// # Example
    ///
    /// ```
    /// let num_type = Type::number();
    /// // use num_type here
    /// ```
    pub fn number() -> &'static Self {
        static NUMBER_TYPE: Lazy<Type> = Lazy::new(|| {
            let mut t = Type::new();
            t.add_type(StringInterner::IDENT_NUMBER);
            t
        });
        &NUMBER_TYPE
    }

    /// Returns an empty instance of the type.
    ///
    /// This is a convenience method that creates a default (empty) value.
    /// It relies on the `Default` trait implementation for this type.
    ///
    /// # Examples
    ///
    /// ```
    /// let empty_list = TypedList::empty();
    /// assert!(empty_list.is_empty()); // supposant que is_empty() est défini
    /// ```
    pub fn empty() -> Self {
        Self::default()
    }

    /// Adds a new atomic or primitive type identifier to this type.
    ///
    /// # Arguments
    ///
    /// * `member` - The atomic or primitive type identifier to add.
    pub fn add_type(&mut self, member: Ident) {
        self.members.push(member);
    }

    /// Creates a new atomic type from a single atomic type identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the atomic type.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing an atomic type.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::ir::{Type, Ident};
    /// let t = Type::atomic_type(Ident(1));
    /// assert!(t.is_atomic_type());
    /// ```
    pub fn atomic_type(id: Ident) -> Self {
        Type { members: vec![id] }
    }

    /// Creates a new union type (either) from multiple atomic type identifiers.
    ///
    /// # Arguments
    ///
    /// * `ids` - A non-empty vector of atomic type identifiers to union.
    ///
    /// # Panics
    ///
    /// Panics if `ids` is empty.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing a union of atomic types.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::ir::{Type, Ident};
    /// let t = Type::either_type(vec![Ident(1), Ident(2)]);
    /// assert!(t.is_either_type());
    /// ```
    pub fn either_type(ids: Vec<Ident>) -> Self {
        assert!(!ids.is_empty(), "Either type must have at least one member");
        Type { members: ids }
    }

    /// Returns `true` if this type is atomic (contains exactly one member).
    pub fn is_atomic_type(&self) -> bool {
        self.members.len() == 1
    }

    /// Returns `true` if this type is a union (called `either` in PDDL) of atomic types.
    pub fn is_either_type(&self) -> bool {
        self.members.len() > 1
    }

    /// Returns `true` if the type has no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns the number of atomic type identifiers contained in this `Type`.
    ///
    /// # Returns
    ///
    /// The length of the `members` vector.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns an iterator over the members by reference.
    pub fn iter(&self) -> std::slice::Iter<'_, Ident> {
        self.members.iter()
    }

    /// Returns a mutable iterator over the members.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Ident> {
        self.members.iter_mut()
    }

    /// Consumes self and returns an iterator over the members by value.
    pub fn into_iter(self) -> std::vec::IntoIter<Ident> {
        self.members.into_iter()
    }

    /// Returns a slice containing all the atomic type identifiers in this `Type`.
    ///
    /// This provides a read-only view of the underlying members,
    /// allowing iteration and access without exposing the internal `Vec`.
    ///
    /// # Returns
    ///
    /// A slice of `Ident` representing the members of this `Type`.
    pub fn as_slice(&self) -> &[Ident] {
        &self.members
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_atomic_type() {
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
    /// Formats the type by resolving its member identifiers using the string interner.
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
                match interner.resolve(*ty) {
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
/// - If the type has no members, it produces no output.
/// - If the type has a single member, it prints the member name.
/// - If the type has multiple members, it prints `(either t1 t2 ...)`.
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
/// // Example 2: single type
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
impl SyntaxDisplay for Type {
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        match self.members.len() {
            0 => write!(f, "object"), // Pas de .to_string()
            1 => {
                let ty = self.members[0];
                match interner.resolve(ty) {
                    Some(type_name) => write!(f, "{}", type_name),
                    None => write!(f, "<uninterned:{}>", ty),
                }
            }
            _ => {
                write!(f, "(either")?;
                for ty in &self.members {
                    write!(f, " ")?;
                    match interner.resolve(*ty) {
                        Some(type_name) => write!(f, "{}", type_name)?,
                        None => write!(f, "<uninterned:{}>", ty)?,
                    }
                }
                write!(f, ")")
            }
        }
    }
}


impl FromAst for Type {
    /// Constructs a [`Type`] from an [`AstNode`] representing a collection of type identifiers.
    ///
    /// This implementation iterates over the children of the given AST syntax,
    /// retrieves each child syntax's identifier, and adds it to the list of type members.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let syntax: &AstArenaNode = ...;
    /// let ast: &Ast = ...;
    /// let ty = Type::from_ast(syntax, ast)?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`AiplanError`] if any of the child nodes cannot be retrieved.
    fn from_ast(node: &AstNode, ast: &SyntaxTree<AstNode>) -> Result<Self, AiplanError> {
        let mut ty = Type::new();
        for ty_id in node.children() {
            let child_node = ast.try_node(*ty_id)?;
            if !child_node.content().is_none() {
                ty.add_type(child_node.try_ident()?);
            }
        }
        Ok(ty)
    }
}
