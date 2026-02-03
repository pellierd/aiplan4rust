use std::fmt;
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::ExprNode;
use crate::aiplan4rust::lir::problem::renderers::render_context::RenderContext;
use crate::aiplan4rust::lir::problem::renderers::syntax::expr_content;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    node: &ExprNode,
    ctx: &RenderContext,
) -> std::fmt::Result  {
    let kind_name = format!("{:?}", node.kind()).to_lowercase();
    write!(f, "({}", kind_name)?;

    if !matches!(node.content(), Content::None) {
        write!(f, " ")?;
        expr_content::render(f, node.content(), ctx)?;
    }

    write!(f, ")")
}
