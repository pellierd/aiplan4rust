use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::tree::TreeArena;

/// A list of `TypedSymbol` items.
///
/// This struct wraps a `Vec<TypedSymbol>` and provides
/// convenient access to the underlying slice via dereferencing.
///
/// # Examples
///
/// ```
/// let mut list = TypedList::new();
/// list.push(typed_symbol);
/// assert_eq!(list.len(), 1);
/// for symbol in list.iter() {
///     // use symbol
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList {
    symbols: Vec<TypedSymbol>,
}

impl TypedList {
    /// Creates a new, empty `TypedSymbolList`.
    ///
    /// # Returns
    ///
    /// A fresh `TypedSymbolList` containing no elements.
    pub fn new() -> Self {
        Self { symbols: Vec::new() }
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
}

impl Deref for TypedList {
    type Target = Vec<TypedSymbol>;

    /// Dereferences the `TypedList` to a `Vec<TypedSymbol>`.
    ///
    /// This allows using all methods of `Vec<TypedSymbol>` directly on
    /// `TypedList` instances, including methods like `push`, `pop`, `len`, and more.
    fn deref(&self) -> &Self::Target {
        &self.symbols
    }
}
/// Allows consuming iteration over `TypedList`, yielding owned `TypedSymbol`s.
///
/// This implementation enables using `TypedList` in a `for` loop or
/// passing it to functions expecting an iterator of `TypedSymbol`,
/// consuming the list in the process.
///
/// # Example
///
/// ```
/// let typed_list: TypedList = ...;
/// for symbol in typed_list {
///     // symbol is a TypedSymbol
/// }
/// ```
impl IntoIterator for TypedList {
    type Item = TypedSymbol;
    type IntoIter = std::vec::IntoIter<TypedSymbol>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.into_iter()
    }
}

/// Allows borrowed iteration over `TypedList`, yielding references to `TypedSymbol`s.
///
/// This implementation enables iterating over a borrowed `TypedList` without consuming it.
///
/// # Example
///
/// ```
/// let typed_list: &TypedList = ...;
/// for symbol in typed_list {
///     // symbol is a &TypedSymbol
/// }
/// ```
impl<'a> IntoIterator for &'a TypedList {
    type Item = &'a TypedSymbol;
    type IntoIter = std::slice::Iter<'a, TypedSymbol>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.iter()
    }
}

impl DerefMut for TypedList {
    /// Dereferences the `TypedList` mutably to a `Vec<TypedSymbol>`.
    ///
    /// This allows mutating the underlying vector, enabling calls to
    /// mutable methods like `push`, `clear`, `sort`, etc.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.symbols
    }
}

impl FromAst for TypedList {
    /// Constructs a `TypedList` from an AST node.
    ///
    /// This function expects the given `node` to be of type `TypedList`,
    /// where its children are nodes of type `TypedItem`.
    ///
    /// It iterates over the children of the `TypedList` node, converts each child
    /// `TypedItem` node into a `TypedSymbol` using its `from_ast` method,
    /// and collects them into a new `TypedList`.
    ///
    /// # Parameters
    /// - `node`: Reference to the AST node representing a `TypedList`.
    /// - `ast`: Reference to the entire AST arena for node lookups.
    ///
    /// # Returns
    /// - `Ok(TypedList)` containing all parsed `TypedSymbol` instances from the children.
    /// - `Err(ParserInternalError)` if any child node fails to convert.
    fn from_ast(
        node: &AstNode,
        ast: &TreeArena<AstNode>
    ) -> Result<Self, ParserInternalError> {
        let mut typed_list = TypedList::new();
        for id in node.children() {
            let child = ast.try_node(*id)?;
            let ty = TypedSymbol::from_ast(&child, ast)?;
            typed_list.push(ty);
        }
        Ok(typed_list)
    }
}

/// Implements `Display` for `TypedList`.
///
/// This implementation formats the `TypedList` as a space-separated list of symbols
/// enclosed in parentheses.
///
/// Each symbol is displayed using its own `Display` implementation, without
/// any additional processing or name resolution.
///
/// # Example
///
/// ```text
/// (sym1 sym2 sym3)
/// ```
///
/// # Behavior
///
/// - Starts by writing an opening parenthesis `(`.
/// - Iterates over all symbols in the list.
/// - Writes each symbol separated by a space.
/// - Ends with a closing parenthesis `)`.
impl fmt::Display for TypedList {
    /// Formats the `TypedList` for display.
    ///
    /// Writes the list of symbols in parentheses, separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{sym}")?;
            first = false;
        }
        write!(f, ")")
    }
}

/// Implements `DisplayWithInterner` for `TypedList`.
///
/// A `TypedList` is a collection of `TypedSymbol`s, each potentially associated
/// with a type. The output is a parenthesized, space-separated list of symbols
/// with their types, suitable for debugging or display.
///
/// # Example
///
/// ```text
/// (?x - location ?y - (either robot vehicle))
/// ```
impl DisplayWithInterner for TypedList {
    /// Formats the `TypedList` using the provided `StringInterner`.
    ///
    /// This writes the list in PDDL-style syntax:
    /// - Starts with an opening parenthesis `(`.
    /// - Symbols are separated by spaces.
    /// - Each symbol is formatted via `TypedSymbol::fmt_with`.
    /// - Ends with a closing parenthesis `)`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The interner to resolve symbol names.
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            sym.fmt_with(f, interner)?;
            first = false;
        }
        write!(f, ")")
    }
}

/// Displays a `TypedList` in PDDL syntax.
///
/// A `TypedList` is a list of `TypedSymbol`s, each with optional types.
/// The output is formatted as a parenthesized, space-separated list of typed symbols.
///
/// This implementation is intended for generating PDDL-compatible representations
/// of variables or objects with their declared types.
///
/// # Example
///
/// ```text
/// (?x - location ?y - (either robot vehicle))
/// ```
impl PlanningSyntaxDisplay for TypedList {
    /// Formats the `TypedList` in PDDL syntax.
    ///
    /// This function writes:
    /// - An opening parenthesis `(`.
    /// - Each `TypedSymbol`, separated by spaces.
    /// - A closing parenthesis `)`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    /// * `interner` - The `StringInterner` used to resolve symbol and type names.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting succeeded.
    ///
    /// # Example Output
    ///
    /// ```text
    /// (?x - location ?y - robot)
    /// ```
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize
    ) -> std::fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            sym.fmt_planning_syntax(f, interner)?;
            first = false;
        }
        Ok(())
    }
}
