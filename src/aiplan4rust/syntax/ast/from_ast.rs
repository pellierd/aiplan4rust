use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::AstArenaNode;
use crate::aiplan4rust::tree::TreeArena;

/// Trait for constructing an instance of a type from an AST node.
///
/// # Overview
/// Types implementing this trait define how to build themselves from
/// an AST node (`AstArenaNode`) within a given AST context (`Ast`).
///
/// This is typically used during semantic analysis or IR construction,
/// where domain-specific structures are created by traversing and interpreting the AST.
///
/// # Method
/// - `from_ast` takes:
///    - a reference to the AST node representing the element to construct,
///    - a reference to the full AST context, which can be used to access
///      related nodes or additional information.
///
/// # Errors
/// The method returns a `Result<Self, ParserInternalError>`,
/// allowing to propagate errors encountered during parsing or validation.
///
/// # Requirements
/// Implementors must be sized (`Self: Sized`) to allow returning `Self`.
///
/// # Example
/// ```ignore
/// impl FromAst for Action {
///     fn from_ast(node: &AstArenaNode, ast: &Ast) -> Result<Self, ParserInternalError> {
///         // Implementation to parse Action from AST node
///     }
/// }
/// ```
pub trait FromAst {
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError>
    where
        Self: Sized;
}
