use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::ast::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::tree::TreeArena;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lang::Optimization;

/// Wrapper struct around `TreeArena<ExprNode>` representing an expression tree.
///
/// This struct provides a dedicated type for expressions, allowing
/// to add custom methods on top of the underlying `TreeArena`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Expr {
    tree: TreeArena<ExprNode>,
}

impl Expr {
    /// Creates a new, empty expression tree.
    ///
    /// # Examples
    ///
    /// ```
    /// let expr = Expr::new();
    /// assert!(expr.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            tree: TreeArena::<ExprNode>::new(),
        }
    }

    pub fn empty_or() -> Self {
        let mut empty_or = Expr::new();
        let root = ExprNode::new(ExprKind::Or, ExprContent::None, None);
        empty_or.alloc(root);
        empty_or
    }

    pub fn empty_and() -> Self {
        let mut empty_and = Expr::new();
        let root = ExprNode::new(ExprKind::And, ExprContent::None, None);
        empty_and.alloc(root);
        empty_and
    }

    pub fn metric_none() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Metric,
            ExprContent::Optimization(Optimization::None),
            None,
        );
        expr.alloc(root);
        expr
    }

    pub fn empty_length_spec() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Length,
            ExprContent::None,
            None,
        );
        expr.alloc(root);
        expr
    }
}

/// Implements the `FromAst` trait for `Expr`.
///
/// This allows constructing an `Expr` (expression tree) from an AST node subtree.
///
/// # Expected AST structure
///
/// The `node` is expected to be the root of an AST subtree representing a logical expression.
/// All descendants will be recursively converted into `ExprNode`s and stored in the `Expr` arena.
///
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - The `AstKind` cannot be converted into an `ExprKind`.
/// - The `AstContent` cannot be converted into an `ExprContent`.
/// - Any node lookup in the `TreeArena` fails.
///
/// # Example
///
/// ```rust
/// let expr = Expr::from_ast(ast_root_node, &arena)?;
/// ```
impl FromAst for Expr {
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        wrap(node, ast)
    }
}

/// Converts an AST node and its subtree into an Expr iteratively.
///
/// This function builds the entire Expr arena starting from the given node reference,
/// avoiding recursion by using an explicit stack.
fn wrap(
    node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>
) -> Result<Expr, ParserInternalError> {
    let mut expr = Expr::new();
    let mut stack = Vec::new();

    // (AST node, parent ExprNodeId)
    stack.push((node, None));

    while let Some((current_ast_node, parent_expr_id_opt)) = stack.pop() {
        // Create ExprNode
        let kind = ExprKind::try_from(current_ast_node.kind())?;
        let content = ExprContent::try_from(current_ast_node.content())?;
        let expr_node = ExprNode::new(kind, content, parent_expr_id_opt);
        let expr_node_id = expr.alloc(expr_node);

        // Attach to parent if needed
        if let Some(parent_id) = parent_expr_id_opt {
            expr.try_node_mut(parent_id)?.add_child(expr_node_id);
        }

        // Push children in reverse to preserve left-to-right order
        for &child_id in current_ast_node.children().iter().rev() {
            let child_node = ast.try_node(child_id)?;
            stack.push((child_node, Some(expr_node_id)));
        }
    }

    Ok(expr)
}


impl Deref for Expr {
    type Target = TreeArena<ExprNode>;

    /// Dereferences the `Expr` to the underlying `TreeArena<ExprNode>`.
    ///
    /// This allows transparent access to all methods of `TreeArena`.
    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for Expr {
    /// Mutable dereference to the underlying `TreeArena<ExprNode>`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Délègue au Display de TreeArena<ExprNode>
        fmt::Display::fmt(&self.tree, f)
    }
}

impl DisplayWithInterner for Expr {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.tree.fmt_with(f, interner)
    }
}

impl PlanningSyntaxDisplay for Expr {
    fn fmt_planning_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.tree.fmt_with(f, interner)
    }
}
