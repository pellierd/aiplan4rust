use crate::aiplan4rust::parser::syntax_tree::SyntaxNode;
use std::fmt;

/// A structure representing a syntax tree.
///
/// The `SyntaxTree` struct encapsulates an Abstract Syntax Tree (AST) and additional metadata,
/// such as the source filename and the timestamp of when the tree was generated. This structure
/// can be used to hold the parsed representation of a program, along with its associated
/// information.
///
/// # Fields
///
/// - `ast`: The Abstract Syntax Tree (AST) of the program, represented by an `Ast` type.
/// - `filename`: An optional filename from which the syntax tree was generated.
/// - `generated_at`: The time when the syntax tree was created.
///
/// # Example
///
/// ```rust
/// use std::time::SystemTime;
/// use crate::aiplan4rust::syntax::ast::Ast;
///
/// let ast = Box::new(Ast::new(...)); // Construct the AST
/// let syntax_tree = SyntaxTree::new(ast, Some("domain.pddl".to_string()), SystemTime::now());
///
/// // Access the AST and other metadata:
/// let ast_ref = syntax_tree.ast();
/// let file_name = syntax_tree.filename();
/// let generated_at = syntax_tree.generated_at();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxTree {
    /// The Abstract Syntax Tree (AST) representing the structure of the program.
    root: Box<SyntaxNode>,

    /// The optional filename from which the syntax tree was generated.
    filename: Option<String>,

    /// The time when the syntax tree was generated.
    generated_at: std::time::SystemTime,
}

impl SyntaxTree {
    /// Creates a new `SyntaxTree` instance.
    ///
    /// # Parameters
    ///
    /// - `ast`: A boxed `Ast` object representing the program's abstract syntax tree.
    /// - `filename`: An optional `String` holding the filename from which the AST was generated.
    /// - `generated_at`: A `SystemTime` representing the timestamp when the AST was generated.
    ///
    /// # Returns
    ///
    /// A new `SyntaxTree` instance with the provided values.
    pub fn new(
        ast: Box<SyntaxNode>,
        filename: Option<String>,
        generated_at: std::time::SystemTime,
    ) -> Self {
        SyntaxTree {
            root: ast,
            filename,
            generated_at,
        }
    }

    /// Accessor for the AST.
    ///
    /// # Returns
    ///
    /// A reference to the boxed `Ast` object.
    pub fn root(&self) -> &Box<SyntaxNode> {
        &self.root
    }

    /// Accessor for the filename.
    ///
    /// # Returns
    ///
    /// An `Option<&String>` that is `Some(filename)` if a filename is available, or `None` if not.
    pub fn filename(&self) -> Option<&String> {
        self.filename.as_ref()
    }
}

/// Implement the `fmt::Display` trait for `SyntaxTree`.
///
/// This implementation formats the `SyntaxTree` struct in a human-readable way, including
/// the `ast`, `filename`, and `generated_at` fields. The `Display` trait is used to
/// provide a custom string representation of the `SyntaxTree` when printed, for example,
/// using `println!`.
impl fmt::Display for SyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the AST (assuming `Ast` implements `Display`).
        writeln!(f, "SyntaxTree:")?;
        writeln!(f, " - AST: {}", self.root())?;

        // Display the filename if available.
        match &self.filename {
            Some(file) => writeln!(f, " - Filename: {}", file)?,
            None => writeln!(f, " - Filename: (not provided)")?,
        }

        // Display the generation time.
        writeln!(f, " - Generated at: {:?}", self.generated_at)
    }
}
