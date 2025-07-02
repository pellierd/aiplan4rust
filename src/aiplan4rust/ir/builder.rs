use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::DisplayWithInterner;
use crate::aiplan4rust::ir::action::Action;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::tree::TreeNode;
use crate::aiplan4rust::ir::planning_problem::PlanningProblem;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};

#[derive(Debug)]
pub struct IRBuilder {
    name : String
}

impl IRBuilder {
    pub fn new() -> Self {
        IRBuilder {
            name: "IRBuilder".to_string(),
        }
    }
}

impl IRBuilder {

    pub fn build(
        &mut self,
        context: &LinkedSemanticContext
    ) -> Result<PlanningProblem, ParserInternalError> {
        let mut ir = PlanningProblem::new();
        let domain = context.domain();

        for node in domain.preorder() {
            match node.kind() {
                AstKind::DomainName => {
                    ir.set_domain_name(node.try_ident()?);
                }
                AstKind::PredicatesDef => {
                    
                }
                AstKind::ActionDef => {
                    let action = Action::from_ast(node, domain)?;
                    println!("{}", action.to_string_with_interner(context.interner()));
                    ir.add_action(action);
                }
                _ => {
                    // For now, ignore other kinds.
                    // You can add handling for MethodDef, FunctionDef, etc. here.
                }
            }
        }

        Ok(ir)
    }


}
