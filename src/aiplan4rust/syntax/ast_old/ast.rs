use crate::aiplan4rust::syntax::ast_old::AstNodeOld;
use crate::aiplan4rust::syntax::ast_old::iterators::{PostorderIter, PreorderIter};

use std::io::Write;
use std::fmt;
use serde::{Deserialize, Serialize};

/// A structure representing an abstract syntax tree (AST) and its metadata.
///
/// The `Ast` struct encapsulates the root of an abstract syntax tree,
/// along with metadata such as the associated source name and the time
/// the tree was generated. It is typically used to store the result of
/// parsing a source file or input string.
///
/// # Fields
///
/// - `root`: The root node of the abstract syntax tree, represented by an `AstNode`.
/// - `source_name`: A `String` identifying the source of the AST (e.g., a filename or label).
/// - `generated_at`: A `SystemTime` indicating when the AST was generated.
///
/// # Example
///
/// ```rust
/// use std::time::SystemTime;
/// use crate::aiplan4rust::syntax::ast::{Ast, AstNode};
///
/// let root = Box::new(AstNode::new(...)); // Construct the root AST node
/// let ast = Ast::new(root, "domain.pddl".to_string(), SystemTime::now());
///
/// // Access the AST and metadata:
/// let root_ref = ast.root();
/// let source = ast.source_name();
/// let timestamp = ast.generated_at();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AstOld {
    /// The Abstract Syntax Tree (AST) representing the structure of the program.
    root: Box<AstNodeOld>,

    /// The optional filename from which the syntax tree was generated.
    source_name: String,

    /// The time when the syntax tree was generated.
    generated_at: std::time::SystemTime,
}

impl AstOld {
    /// Creates a new `Ast` instance.
    ///
    /// # Parameters
    ///
    /// - `root`: A boxed `AstNode` representing the root of the abstract syntax tree.
    /// - `source_name`: A `String` representing the source name associated with the AST (e.g., a
    ///   filename or other identifier).
    /// - `generated_at`: A `SystemTime` indicating when the AST was generated.
    ///
    /// # Returns
    ///
    /// A new `Ast` instance initialized with the provided values.
    ///
    /// # Example
    ///
    /// ```
    /// let root = Box::new(AstNode::new());
    /// let source_name = String::from("example_source");
    /// let generated_at = std::time::SystemTime::now();
    /// let ast = Ast::new(root, source_name, generated_at);
    /// ```
    pub fn new(
        root: Box<AstNodeOld>,
        source_name: String,
        generated_at: std::time::SystemTime,
    ) -> Self {
        AstOld {
            root,
            source_name,
            generated_at,
        }
    }

    /// Accessor for the AST.
    ///
    /// # Returns
    ///
    /// A reference to the boxed `Ast` object.
    pub fn root(&self) -> &Box<AstNodeOld> {
        &self.root
    }

    /// Accessor for the AST.
    ///
    /// # Returns
    ///
    /// A reference to the boxed `Ast` object.
    pub fn root_mut(&mut self) -> &mut Box<AstNodeOld> {
        &mut self.root
    }

    /// Returns a reference to the source name.
    ///
    /// # Returns
    ///
    /// A reference to the source name as a `&String`.
    ///
    /// # Example
    ///
    /// ```
    /// let obj = MyStruct { source_name: String::from("example_source") };
    /// assert_eq!(obj.source_name(), "example_source");
    /// ```
    pub fn source_name(&self) -> &String {
        &self.source_name
    }

    /// Returns the timestamp at which the AST was generated.
    ///
    /// This method provides access to the creation time of the AST,
    /// which can be useful for tracking, debugging, or caching purposes.
    ///
    /// # Returns
    /// A [`SystemTime`] value indicating when the AST was built.
    ///
    /// # Example
    /// ```rust
    /// let ast = ...; // An instance of `Ast`
    /// let timestamp = ast.generated_at();
    /// println!("AST generated at: {:?}", timestamp);
    /// ```
    ///
    /// # Note
    /// This timestamp is typically set during AST construction and
    /// represents the system time at that moment.
    ///
    /// [`SystemTime`]: std::time::SystemTime
    pub fn generated_at(&self) -> std::time::SystemTime {
        self.generated_at
    }

    /// Returns an iterator over the tree in preorder (depth-first).
    ///
    /// # Example
    /// ```rust
    /// for node in root.preorder() {
    ///     println!("{:?}", node);
    /// }
    /// ```
    pub fn preorder(&self) -> PreorderIter<'_> {
        PreorderIter::new(self.root())
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
        PostorderIter::new(self.root())
    }

    /// Assign unique consecutive IDs to all nodes in the AST starting from `start_id`.
    ///
    /// This method delegates to the root node's `assign_unique_ids` method.
    ///
    /// # Arguments
    ///
    /// * `start_id` - The initial ID to assign to the root node.
    pub fn assign_unique_ids(&mut self, start_id: usize) {
        self.root.assign_unique_ids(start_id);
    }

    /// Checks whether all node IDs in the AST are unique.
    ///
    /// This method delegates to the root node's `check_ids_unique` method.
    ///
    /// # Returns
    ///
    /// `true` if all node IDs are unique, `false` otherwise.
    pub fn check_ids_unique(&self) -> bool {
        self.root.check_ids_unique()
    }

}

/// Implements the `fmt::Display` trait for `Ast`.
///
/// This implementation provides a human-readable string representation of the `Ast`
/// structure. It includes details such as the source name, generation timestamp,
/// and the root node of the abstract syntax tree.
///
/// This is useful for debugging or logging, as it allows instances of `Ast`
/// to be printed using macros like `println!`.
impl fmt::Display for AstOld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Abstract Syntax Tree:")?;
        writeln!(f, " - Source: {}", self.source_name())?;
        writeln!(f, " - Generated at: {:?}", self.generated_at)?;
        writeln!(f, " - Nodes:\n{}", self.root())?;
        Ok(())
    }
}
