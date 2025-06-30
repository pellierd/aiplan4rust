
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::ir::action::Action;
use crate::aiplan4rust::ir::expr::wrapper::wrap;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::tree::TreeNode;
use crate::aiplan4rust::interner::display::DisplayWithInterner;

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

    pub fn build(&mut self, context: &LinkedSemanticContext) -> Result<(), ParserInternalError> {

        println!("building IRBuilder");
        Self::build_actions(&context);

        // Return the built arena or any other structure as needed
        Ok(())
    }


    fn build_actions(context: &LinkedSemanticContext) -> Result<Vec<Action>, ParserInternalError> {
        let mut actions = Vec::new();

        let ast = context.domain();
        println!("AST ------>  {}", ast.to_string_with_interner(context.interner()));
        //println!("AST ------>  {}", ast);
        let entries = context.symbol_table().collect_declarations(None, Some(&SymbolKind::Action), None);
        for entry in entries {
            let action_symbol_node = ast.try_node(entry.node_id())?;
            let action_node = ast.try_node(action_symbol_node.try_parent()?)?;
            let action_def_body_node = ast.try_node(action_node.try_child(2)?)?;
            let pre_def_node = ast.try_node(action_def_body_node.try_child(0)?)?;
            let eff_def_node = ast.try_node(action_def_body_node.try_child(1)?)?;
            let pre = wrap(pre_def_node.try_child(0)?, ast)?;
            let eff = wrap(eff_def_node.try_child(0)?, ast)?;


            let action = Action::new(
                entry.symbol_ref().ident(),
                entry.arguments().unwrap().clone(),
                pre,
                eff,
            );

            println!("{}", action.to_string_with_interner(context.interner()));

            actions.push(action);
        }

        Ok(actions)
    }



}
