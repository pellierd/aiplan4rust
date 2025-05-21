use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::parser::Span;
use crate::aiplan4rust::PDDLDisplay;

use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::hash::Hash;

/// Represents an Abstract Syntax Tree (AST) used to model elements of a program or PDDL
/// specification.
///
/// This structure derives the `Clone`, `Debug`, and `Deserialize` traits, allowing instances of
/// `Ast` to be cloned, displayed for debugging, serialiszed or deserialized from data formats such
/// as JSON or YAML.
///
/// # Fields (Private)
/// - `kind`: The type of the element represented by this AST node. It is an `AstKind`.
/// - `children`: A list of child `Ast` elements, represented as a `Vec<Box<Ast>>`.
/// - `start`, `end`, `begin_line`, `begin_column`, `end_line`, `end_column`: Positioning data in
/// the original input.
///
/// # Example
/// ```rust
/// use parser::Ast;
/// use parser::AstKind;
///
/// let ast = Ast::new(AstKind::Domain, 0, 10, 1, 1, 1, 10);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SyntaxNode {
    id: usize,
    kind: SyntaxNodeKind,
    children: Vec<Box<SyntaxNode>>,
    span: Span,
}

impl SyntaxNode {
    /// Creates a new AST node with the specified kind, children, start, and end positions.
    ///
    /// This function creates a new AST node with the provided kind, list of children nodes, start
    /// position, and end position.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the AST node.
    /// * `children` - The list of children nodes of the AST node.
    /// * `start` - The start position of the AST node.
    /// * `end` - The end position of the AST node.
    ///
    /// # Returns
    ///
    /// A new AST node initialized with the given parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// let kind = AstKind::PrimitiveType;
    /// let children = vec![]; // Initialize children nodes if any
    /// let start = 0;
    /// let end = 10;
    /// let ast_node = Ast::new(kind, children, start, end);
    /// ```
    pub fn new(
        kind: SyntaxNodeKind,
        children: Vec<Box<SyntaxNode>>,
        start: usize,
        end: usize,
    ) -> SyntaxNode {
        SyntaxNode {
            id: usize::MAX,
            kind,
            children,
            span: Span::new(start, end),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn set_id(&mut self, new_id: usize) {
        self.id = new_id;
    }

    /// Returns the kind of AST node.
    pub fn kind(&self) -> &SyntaxNodeKind {
        &self.kind
    }

    /// Modifies the kind of the AST node.
    ///
    /// This method updates the `kind` field of the `Ast` object to the provided
    /// `new_kind`. It allows changing the type of the AST node after its
    /// creation, enabling dynamic modification of the node's classification
    /// during the AST construction or traversal.
    ///
    /// # Parameters
    /// - `new_kind`: The new `AstKind` to set for the node.
    ///
    /// # Example
    /// ```
    /// let mut node = Ast::new(AstKind::Expression, vec![], 0, 10);
    /// node.set_kind(AstKind::Statement);
    /// ```
    pub fn set_kind(&mut self, new_kind: SyntaxNodeKind) {
        self.kind = new_kind;
    }

    /// Returns a reference to the vector of children of the AST node.
    ///
    /// This method provides access to the vector of children of an AST node.
    /// It returns an immutable reference to the vector, allowing you to read the children
    /// but not modify them directly.
    ///
    /// # Example
    /// ```
    /// let ast = Ast {
    ///     kind: AstKind::SomeKind,
    ///     children: vec![],
    ///     start: 0,
    ///     end: 10,
    /// };
    ///
    /// // Access the children of the AST (read-only)
    /// let children = ast.children();
    /// ```
    ///
    /// # Panics
    /// This method does not panic.
    ///
    /// # Returns
    /// Returns an immutable reference to `self.children` (the vector of `Box<Ast>`).
    pub fn children(&self) -> &Vec<Box<SyntaxNode>> {
        &self.children
    }

    /// Returns a reference to the `Span` of this `AstEntry`.
    ///
    /// # Returns
    /// * `&Span` - A reference to the span associated with this `AstEntry`.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Sets the children nodes of the current `Ast` node.
    ///
    /// This method replaces the current list of child nodes with a new list provided as an
    /// argument. The new children are specified as a `Vec<Box<Ast>>`, where each element represents
    /// a child node in the Abstract Syntax Tree (AST).
    ///
    /// # Arguments
    ///
    /// * `new_children` - A vector of `Box<Ast>` representing the new set of children for the
    /// current node.
    pub fn set_children(&mut self, new_children: Vec<Box<SyntaxNode>>) {
        self.children = new_children;
    }

    /// Returns a mutable reference to the vector of children of the AST node.
    ///
    /// This method allows direct modification of the vector of children of an AST node.
    /// It returns a mutable reference to the vector, enabling addition, removal, or modification
    /// of the elements in the vector.
    ///
    /// # Panics
    /// This method does not panic, but a mutable reference to `self` is required to access it.
    ///
    /// # Returns
    /// Returns a mutable reference to `self.children` (the vector of `Box<Ast>`).
    pub fn children_mut(&mut self) -> &mut Vec<Box<SyntaxNode>> {
        &mut self.children
    }

    /// Returns the starting offset of the AST node in the character stream.
    ///
    /// The offset represents the position (in number of characters)
    /// from the beginning of the input file or stream.
    pub fn start_offset(&self) -> usize {
        self.span.start()
    }

    /// Returns the ending offset of the AST node in the character stream.
    ///
    /// The offset represents the position (in number of characters)
    /// from the beginning of the input file or stream.
    pub fn end_offset(&self) -> usize {
        self.span.end()
    }

    /// Returns the starting location (line, column) of the AST node.
    ///
    /// If the location has not been initialized, both `line` and `column` will be set to `usize::MAX`.
    pub fn start_position(&self) -> (usize, usize) {
        self.span.start_position()
    }

    /// Returns the ending location (line, column) of the AST node.
    ///
    /// If the location has not been initialized, both `line` and `column` will be set to `usize::MAX`.
    pub fn end_location(&self) -> (usize, usize) {
        self.span.end_position()
    }

    /// Sets the starting location (line, column) of the AST node.
    ///
    /// # Parameters
    /// - `line`: The line number where the node starts (should be >= 0).
    /// - `column`: The column number where the node starts (should be >= 0).
    ///
    /// Both values are expected to be valid; no specific checks are performed.
    pub fn set_start_position(&mut self, line: usize, column: usize) {
        self.span.set_begin_line(line);
        self.span.set_begin_column(column);
    }

    /// Sets the ending location (line, column) of the AST node.
    ///
    /// # Parameters
    /// - `line`: The line number where the node ends (should be >= 0).
    /// - `column`: The column number where the node ends (should be >= 0).
    ///
    /// Both values are expected to be valid; no specific checks are performed.
    pub fn set_end_position(&mut self, line: usize, column: usize) {
        self.span.set_end_line(line);
        self.span.set_end_column(column);
    }

    /// Formats the AST node with indentation corresponding to its depth.
    ///
    /// This function formats the AST node with a given depth of indentation. Each level of depth
    /// increases the indentation by two spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `depth` - The depth of the AST node in the tree hierarchy.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt;
    ///
    /// let ast = Ast::new(/* Initialize AST node */);
    /// let mut formatter = fmt::Formatter::new();
    /// ast.fmt_with_depth(&mut formatter, 0).unwrap();
    /// println!("{}", formatter);
    /// ```
    ///
    fn fmt_with_depth(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        let indentation = "  ".repeat(depth); // Indentation par niveau de profondeur

        // Affichage avec l'indentation et le résultat formaté
        write!(f, "{}{} {}", indentation, self.kind, self.span)?;

        // Traitement des enfants s'il y en a
        if !self.children.is_empty() {
            write!(f, "\n")?;
            self.write_children_with_depth(f, depth + 1)?;
        }

        Ok(())
    }

    /// Writes the children of the AST node with indentation corresponding to their depth.
    ///
    /// This function writes the children of the AST node with a given depth of indentation. Each child
    /// is formatted with indentation two spaces greater than the depth of its parent node.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `depth` - The depth of the AST node in the tree hierarchy.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt;
    ///
    /// let ast = Ast::new(/* Initialize AST node */);
    /// let mut formatter = fmt::Formatter::new();
    /// ast.write_children_with_depth(&mut formatter, 1).unwrap();
    /// println!("{}", formatter);
    /// ```
    fn write_children_with_depth(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        let len = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            // Add newline only between children (not before the first child)
            if i > 0 {
                write!(f, "\n")?;
            }
            child.fmt_with_depth(f, depth)?;
            // Only add "End" if it's the last child
            if i == len - 1 {
                let indentation = "  ".repeat(depth - 1);
                write!(f, "\n{}End {}", indentation, self.kind)?;
            }
        }
        Ok(())
    }

    /// Converts the AST structure into a `HashMap` that maps references to AST nodes to their indices.
    ///
    /// This function generates a hash map where each entry associates a reference to an `Ast` node
    /// with its corresponding index. The indices are assigned recursively to all nodes in the AST
    /// structure, ensuring a unique mapping for each node. This map can be used for efficient
    /// lookups or referencing child nodes.
    ///
    /// # Returns
    /// A `HashMap<&Ast, usize>`, where:
    /// - The keys are references (`&Ast`) to the `Ast` nodes.
    /// - The values are the indices (`usize`) of those nodes in the AST structure.
    ///
    /// # Example
    /// ```rust
    /// let ast = Ast::new(...); // Create or load an Ast structure
    /// let map = ast.to_hash_map();
    /// // Now `map` contains a mapping of AST node references to their indices
    /// ```
    pub fn to_hash_map(&self) -> HashMap<&SyntaxNode, usize> {
        let mut map = HashMap::with_capacity(4096);
        let mut id_counter = 0;
        let mut stack = vec![self];

        while let Some(node) = stack.pop() {

            map.insert(node, id_counter);
            id_counter += 1;

            // Push children in reverse order to preserve left-to-right traversal
            for child in node.children.iter().rev() {
                stack.push(child);
            }
        }

        map
    }
}

impl fmt::Display for SyntaxNode {
    /// Formats the `Ast` with indentation based on its depth.
    ///
    /// This method implements the `Display` trait for the `Ast` struct, allowing it to be
    /// formatted as a string for user-friendly printing. The formatting is done with an
    /// indentation that reflects the depth of the node in the AST (Abstract Syntax Tree).
    ///
    /// # Parameters
    /// - `f`: A mutable reference to the `fmt::Formatter` used to format the output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

impl PDDLDisplay for SyntaxNode {
    /// Converts the current AST node into a PDDL string representation, starting from depth 0.
    ///
    /// This function is a convenience method that delegates the actual conversion to the
    /// `to_pddl_string_with_depth` function with an initial depth of 0. It is intended to be used
    /// when the user doesn't need to control the depth of the string representation.
    ///
    /// # Returns
    /// A `String` that represents the AST node in PDDL format starting at depth 0.
    fn to_pddl_string(&self) -> String {
        self.to_pddl_string_with_depth(0)
    }

    /// Converts the current AST node into a PDDL string representation with the specified depth.
    ///
    /// This function recursively generates the PDDL string representation of the AST node and its
    /// children, considering the depth of the node in the tree. The `depth` parameter helps control
    /// the indentation or the level of nesting for the PDDL string representation. It is intended
    /// for more advanced use cases where control over the depth is required (e.g., formatting the
    /// output).
    ///
    /// # Parameters
    /// - `depth`: A `usize` that represents the depth of the current node in the AST tree. This can
    /// be used to adjust the indentation or nesting of the resulting PDDL string.
    ///
    /// # Returns
    /// A `String` that represents the AST node and its children in PDDL format, taking into account
    /// the depth.
    fn to_pddl_string_with_depth(&self, depth: usize) -> String {
        let offset = " ".repeat(depth * 2);
        let mut pddl = String::new();

        match &self.kind() {
            SyntaxNodeKind::Domain => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(
                        pddl,
                        "\n{}{}",
                        offset,
                        child.to_pddl_string_with_depth(depth + 1)
                    )
                    .unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::DomainName(_) => {
                write!(pddl, "{}({})", offset, self.kind.to_pddl_string(),).unwrap();
            }
            SyntaxNodeKind::RequireDef => {
                write!(pddl, "{}{}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Requirement(requirement) => {
                write!(pddl, "{}", requirement.to_pddl_string()).unwrap();
            }
            SyntaxNodeKind::TypesDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::TypedList => {
                write!(pddl, "{}", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0
                        && !(child.kind == SyntaxNodeKind::TypedList && child.children().is_empty())
                    {
                        write!(pddl, " ").unwrap();
                    }
                    if matches!(child.kind(), SyntaxNodeKind::Type) {
                        write!(pddl, "- ").unwrap();
                    }
                    write!(pddl, "{}", child.to_pddl_string()).unwrap();
                }
            }
            SyntaxNodeKind::PrimitiveType(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            SyntaxNodeKind::Type => {
                write!(pddl, "{}", offset).unwrap();
                match self.children().as_slice() {
                    [single_child] => {
                        write!(pddl, "{}", single_child.to_pddl_string()).unwrap();
                    }
                    multiple_children if multiple_children.len() > 1 => {
                        write!(pddl, "(either").unwrap();
                        for child in multiple_children {
                            write!(pddl, " {}", child.to_pddl_string()).unwrap();
                        }
                        write!(pddl, ")").unwrap();
                    }
                    _ => unreachable!("AstKind::Type with with no child encountered"),
                }
            }
            SyntaxNodeKind::ConstantsDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::Constant(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            SyntaxNodeKind::PredicatesDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::AtomicFormulaSkeleton
            | SyntaxNodeKind::AtomicFunctionSkeleton
            | SyntaxNodeKind::AtomicFormula
            | SyntaxNodeKind::FunctionTerm => {
                write!(pddl, "{}(", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0 {
                        write!(pddl, " ").unwrap();
                    }
                    write!(pddl, "{}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Predicate(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            SyntaxNodeKind::FunctionsDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::FunctionSymbol(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            SyntaxNodeKind::ActionDef => {
                let children = self.children();
                write!(
                    pddl,
                    "{}({} {} ",
                    offset,
                    self.kind.to_pddl_string(),
                    children[0].to_pddl_string()
                )
                .unwrap();
                write!(
                    pddl,
                    "\n{}",
                    children[1].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
                write!(pddl, "{}", children[2].to_pddl_string_with_depth(depth + 1)).unwrap();
                write!(pddl, "\n{})", offset).unwrap();
            }
            SyntaxNodeKind::ActionSymbol(symbol) => {
                write!(pddl, "{}", symbol).unwrap();
            }
            SyntaxNodeKind::ActionDefBody => {
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth)).unwrap();
                }
            }
            SyntaxNodeKind::PreconditionDef => {
                write!(
                    pddl,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children()[0].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            SyntaxNodeKind::EffectDef => {
                write!(
                    pddl,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children()[0].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            SyntaxNodeKind::Or => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::And => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Not => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::FComp(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Assign(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Operation(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            SyntaxNodeKind::Forall => {
                write!(
                    pddl,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            SyntaxNodeKind::Exists => {
                write!(
                    pddl,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            SyntaxNodeKind::Imply => {
                write!(
                    pddl,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            SyntaxNodeKind::When => {
                write!(
                    pddl,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            _ => pddl = self.kind.to_pddl_string(),
        }
        pddl
    }
}
