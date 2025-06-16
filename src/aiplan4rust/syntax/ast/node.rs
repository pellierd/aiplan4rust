use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::{AstNode, Span};
use crate::aiplan4rust::syntax::ast::iterators::{PostorderIter, PreorderIter};
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::syntax::ast::serialize::SerializableNode;

use std::collections::HashSet;
use std::fmt;
use std::fmt::Write;

use serde::{Serialize, Serializer, Deserialize, Deserializer};


/// Represents a node in the Abstract Syntax Tree (AST).
///
/// This struct models a syntactic element of a program or a PDDL specification.
/// Each node contains:
/// - a type (`AstKind`) indicating its syntactic category,
/// - a list of child nodes,
/// - a span (`Span`) representing its position in the source file,
/// - and a unique identifier that can be assigned.
///
/// # Derived Traits
/// - `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`
///
/// # Example
/// ```rust
/// use parser::Node;
/// use parser::AstKind;
///
/// let node = Node::new(AstKind::Domain, vec![], 0, 10);
/// println!("Created node: {:?}", node);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Node {
    id: usize,
    kind: AstKind,
    children: Vec<Box<Node>>,
    span: Span,
}

impl Node {
    // === Constructors ===

    /// Creates a new AST node with the specified type, children, and source positions.
    ///
    /// # Arguments
    /// - `kind`: The node type (`AstKind`).
    /// - `children`: Vector of child nodes (`Box<Node>`).
    /// - `start`: Start position in the character stream (offset).
    /// - `end`: End position in the character stream (offset).
    ///
    /// # Returns
    /// A new `Node` instance.
    ///
    /// # Example
    /// ```rust
    /// let node = Node::new(AstKind::PrimitiveType, vec![], 0, 10);
    /// ```
    pub fn new(kind: AstKind, children: Vec<Box<Node>>, start: usize, end: usize) -> Node {
        Node {
            id: usize::MAX,
            kind,
            children,
            span: Span::new(start, end),
        }
    }

    /// Creates a new AST node with an explicit `Span`.
    ///
    /// # Arguments
    /// - `kind`: The node type.
    /// - `children`: Child nodes.
    /// - `span`: Source interval (`Span`).
    ///
    /// # Returns
    /// A new `Node` instance.
    pub fn new_with_span(kind: AstKind, children: Vec<Box<Node>>, span: Span) -> Node {
        Node {
            id: usize::MAX,
            kind,
            children,
            span,
        }
    }


    // === Accessors & Mutators ===

    /// Returns the total size of the subtree (number of nodes).
    ///
    /// # Example
    /// ```rust
    /// let size = node.size();
    /// ```
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(|c| c.size()).sum::<usize>()
    }

    /// Returns a reference to the node's identifier.
    pub fn id(&self) -> &usize {
        &self.id
    }

    /// Sets the unique identifier for this node.
    ///
    /// # Example
    /// ```rust
    /// node.set_id(42);
    /// ```
    pub fn set_id(&mut self, new_id: usize) {
        self.id = new_id;
    }

    /// Returns a reference to the node's type (`AstKind`).
    pub fn kind(&self) -> &AstKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut AstKind {
        &mut self.kind
    }

    /// Changes the node type.
    ///
    /// # Example
    /// ```rust
    /// node.set_kind(AstKind::Statement);
    /// ```
    pub fn set_kind(&mut self, new_kind: AstKind) {
        self.kind = new_kind;
    }

    /// Returns an immutable reference to the child nodes.
    pub fn children(&self) -> &Vec<Box<Node>> {
        &self.children
    }

    /// Returns a mutable reference to the child nodes.
    pub fn children_mut(&mut self) -> &mut Vec<Box<Node>> {
        &mut self.children
    }

    /// Replaces the child nodes with a new vector.
    pub fn set_children(&mut self, new_children: Vec<Box<Node>>) {
        self.children = new_children;
    }

    /// Returns a reference to the node's span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Returns the start offset in the character stream.
    pub fn start_offset(&self) -> usize {
        self.span.start()
    }

    /// Returns the end offset in the character stream.
    pub fn end_offset(&self) -> usize {
        self.span.end()
    }

    /// Returns the start position as (line, column).
    ///
    /// Returns `(usize::MAX, usize::MAX)` if not initialized.
    pub fn start_position(&self) -> (usize, usize) {
        self.span.start_position()
    }

    /// Returns the end position as (line, column).
    ///
    /// Returns `(usize::MAX, usize::MAX)` if not initialized.
    pub fn end_position(&self) -> (usize, usize) {
        self.span.end_position()
    }

    /// Sets the start position (line, column).
    ///
    /// # Arguments
    /// - `line`: line number.
    /// - `column`: column number.
    pub fn set_start_position(&mut self, line: usize, column: usize) {
        self.span.set_begin_line(line);
        self.span.set_begin_column(column);
    }

    /// Sets the end position (line, column).
    ///
    /// # Arguments
    /// - `line`: line number.
    /// - `column`: column number.
    pub fn set_end_position(&mut self, line: usize, column: usize) {
        self.span.set_end_line(line);
        self.span.set_end_column(column);
    }

    // === ID Management ===

    /// Recursively assigns unique and consecutive identifiers to all nodes
    /// in the tree, traversing in preorder.
    ///
    /// # Arguments
    /// - `start_id`: starting identifier (usually 0).
    ///
    /// # Example
    /// ```rust
    /// root.assign_unique_ids(0);
    /// ```
    pub fn assign_unique_ids(&mut self, start_id: usize) {
        fn helper(node: &mut Node, counter: &mut usize) {
            node.id = *counter;
            *counter += 1;
            for child in &mut node.children {
                helper(child, counter);
            }
        }

        let mut counter = start_id;
        helper(self, &mut counter);
    }

    /// Checks whether all IDs in the tree are unique.
    ///
    /// # Returns
    /// `true` if all IDs are unique, otherwise `false`.
    ///
    /// # Example
    /// ```rust
    /// assert!(root.check_ids_unique());
    /// ```
    pub fn check_ids_unique(&self) -> bool {
        fn helper(node: &Node, seen: &mut HashSet<usize>) -> bool {
            if !seen.insert(node.id) {
                return false; // Duplicate ID found
            }
            node.children.iter().all(|child| helper(child, seen))
        }

        let mut seen = HashSet::new();
        helper(self, &mut seen)
    }

    // === Iterators ===

    /// Returns an iterator over the tree in preorder (depth-first).
    ///
    /// # Example
    /// ```rust
    /// for node in root.preorder() {
    ///     println!("{:?}", node);
    /// }
    /// ```
    pub fn preorder(&self) -> PreorderIter<'_> {
        PreorderIter::new(self)
    }

    /// Returns an iterator over the tree in postorder (depth-first).
    ///
    /// # Example
    /// ```rust
    /// for node in root.postorder() {
    ///     println!("{:?}", node);
    /// }
    /// ```
    pub fn postorder(&self) -> PostorderIter<'_> {
        PostorderIter::new(self)
    }

    // === Formatting ===

    /// Formats the AST node with indentation proportional to depth.
    ///
    /// Each depth level adds two spaces of indentation.
    ///
    /// # Arguments
    /// - `f`: formatter to write to.
    /// - `depth`: current depth in the tree.
    ///
    /// # Example
    /// ```rust
    /// use std::fmt;
    /// node.fmt_with_depth(&mut formatter, 0)?;
    /// ```
    fn fmt_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let indentation = "  ".repeat(depth);
        write!(f, "{}{} {} {}", indentation, self.kind, self.span, self.id)?;

        if !self.children.is_empty() {
            write!(f, "\n")?;
            self.write_children_with_depth(f, depth + 1)?;
        }
        Ok(())
    }

    /// Recursively formats children with indentation.
    ///
    /// # Arguments
    /// - `f`: formatter.
    /// - `depth`: current depth.
    fn write_children_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let len = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            if i > 0 {
                write!(f, "\n")?;
            }
            child.fmt_with_depth(f, depth)?;
            if i == len - 1 {
                let indentation = "  ".repeat(depth - 1);
                write!(f, "\n{}End {}", indentation, self.kind)?;
            }
        }
        Ok(())
    }
}


impl Serialize for AstNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convertit AstNode en SerializableNode puis sérialise
        let serializable: SerializableNode = self.into(); // clone car tu as que &self
        serializable.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AstNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Désérialise d'abord en SerializableNode
        let serializable = SerializableNode::deserialize(deserializer)?;
        // Convertit en AstNode
        Ok(serializable.into_ast_node())
    }
}

impl fmt::Display for Node {
    /// Formats the entire tree starting from the root node (depth 0).
    ///
    /// Allows printing a `Node` via `println!` or `{}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

impl SyntaxDisplay for Node {
    /// Converts the current AST node into its syntax string representation, starting from depth 0.
    ///
    /// This function is a convenience method that delegates the actual conversion to the
    /// `to_syntax_string_with_depth` function with an initial depth of 0. It is intended to be used
    /// when the user doesn't need to control the depth or formatting of the string representation.
    ///
    /// # Returns
    /// A `String` representing the AST node in the target syntax format, starting at depth 0.
    fn to_syntax_string(&self) -> String {
        self.to_syntax_string_with_depth(0)
    }

    /// Converts the current AST node into its syntax string representation with the specified depth.
    ///
    /// This function recursively generates the string representation of the AST node and its
    /// children according to the target syntax, considering the depth of the node in the tree.
    /// The `depth` parameter can be used to control indentation or the level of nesting in the output.
    /// It is intended for advanced use cases where control over formatting is required.
    ///
    /// # Parameters
    /// - `depth`: A `usize` representing the depth of the current node in the AST tree. This can
    /// be used to adjust indentation or nesting in the resulting syntax string.
    ///
    /// # Returns
    /// A `String` representing the AST node and its children in the target syntax format, respecting
    /// the specified depth.
    fn to_syntax_string_with_depth(&self, depth: usize) -> String {
        let offset = " ".repeat(depth * 2);
        let mut str = String::new();

        match &self.kind() {
            AstKind::Domain => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(
                        str,
                        "\n{}{}",
                        offset,
                        child.to_syntax_string_with_depth(depth + 1)
                    )
                    .unwrap();
                }
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::DomainName(_) => {
                write!(str, "{}({})", offset, self.kind.to_syntax_string(),).unwrap();
            }
            AstKind::RequireDef => {
                write!(str, "{}{}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Requirement(requirement) => {
                write!(str, "{}", requirement.to_syntax_string()).unwrap();
            }
            AstKind::TypesDef => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, "\n{}", child.to_syntax_string_with_depth(depth + 1)).unwrap();
                }
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::TypedList => {
                write!(str, "{}", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0
                        && !(child.kind == AstKind::TypedList && child.children().is_empty())
                    {
                        write!(str, " ").unwrap();
                    }
                    if matches!(child.kind(), AstKind::Type) {
                        write!(str, "- ").unwrap();
                    }
                    write!(str, "{}", child.to_syntax_string()).unwrap();
                }
            }
            AstKind::PrimitiveType(symbol) => {
                write!(str, "{}{}", offset, symbol).unwrap();
            }
            AstKind::Type => {
                write!(str, "{}", offset).unwrap();
                match self.children().as_slice() {
                    [single_child] => {
                        write!(str, "{}", single_child.to_syntax_string()).unwrap();
                    }
                    multiple_children if multiple_children.len() > 1 => {
                        write!(str, "(either").unwrap();
                        for child in multiple_children {
                            write!(str, " {}", child.to_syntax_string()).unwrap();
                        }
                        write!(str, ")").unwrap();
                    }
                    _ => unreachable!("AstKind::Type with with no child encountered"),
                }
            }
            AstKind::ConstantsDef => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, "\n{}", child.to_syntax_string_with_depth(depth + 1)).unwrap();
                }
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::Constant(symbol) => {
                write!(str, "{}{}", offset, symbol).unwrap();
            }
            AstKind::PredicatesDef => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, "\n{}", child.to_syntax_string_with_depth(depth + 1)).unwrap();
                }
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::AtomicFormulaSkeleton
            | AstKind::AtomicFunctionSkeleton
            | AstKind::AtomicFormula
            | AstKind::FunctionTerm => {
                write!(str, "{}(", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0 {
                        write!(str, " ").unwrap();
                    }
                    write!(str, "{}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Predicate(symbol) => {
                write!(str, "{}{}", offset, symbol).unwrap();
            }
            AstKind::FunctionsDef => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, "\n{}", child.to_syntax_string_with_depth(depth + 1)).unwrap();
                }
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::FunctionSymbol(symbol) => {
                write!(str, "{}{}", offset, symbol).unwrap();
            }
            AstKind::ActionDef => {
                let children = self.children();
                write!(
                    str,
                    "{}({} {} ",
                    offset,
                    self.kind.to_syntax_string(),
                    children[0].to_syntax_string()
                )
                .unwrap();
                write!(
                    str,
                    "\n{}",
                    children[1].to_syntax_string_with_depth(depth + 1)
                )
                .unwrap();
                write!(str, "{}", children[2].to_syntax_string_with_depth(depth + 1)).unwrap();
                write!(str, "\n{})", offset).unwrap();
            }
            AstKind::ActionSymbol(symbol) => {
                write!(str, "{}", symbol).unwrap();
            }
            AstKind::ActionDefBody => {
                for child in self.children() {
                    write!(str, "\n{}", child.to_syntax_string_with_depth(depth)).unwrap();
                }
            }
            AstKind::PreconditionDef => {
                write!(
                    str,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children()[0].to_syntax_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            AstKind::EffectDef => {
                write!(
                    str,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children()[0].to_syntax_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            AstKind::Or => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::And => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Not => {
                write!(str, "{}({}", offset, self.kind.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::FComp(op) => {
                write!(str, "{}({}", offset, op.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Assign(op) => {
                write!(str, "{}({}", offset, op.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Operation(op) => {
                write!(str, "{}({}", offset, op.to_syntax_string()).unwrap();
                for child in self.children() {
                    write!(str, " {}", child.to_syntax_string()).unwrap();
                }
                write!(str, ")").unwrap();
            }
            AstKind::Forall => {
                write!(
                    str,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children[0].to_syntax_string(),
                    self.children[1].to_syntax_string()
                )
                .unwrap();
            }
            AstKind::Exists => {
                write!(
                    str,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children[0].to_syntax_string(),
                    self.children[1].to_syntax_string()
                )
                .unwrap();
            }
            AstKind::Imply => {
                write!(
                    str,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children[0].to_syntax_string(),
                    self.children[1].to_syntax_string()
                )
                .unwrap();
            }
            AstKind::When => {
                write!(
                    str,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_syntax_string(),
                    self.children[0].to_syntax_string(),
                    self.children[1].to_syntax_string()
                )
                .unwrap();
            }
            _ => str = self.kind.to_syntax_string(),
        }
        str
    }
}
