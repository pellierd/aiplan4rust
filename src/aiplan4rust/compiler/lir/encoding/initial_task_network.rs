use crate::aiplan4rust::compiler::lir::encoding::{
    task_network, typed_list, EncodingError, EncodingRegistry,
};
use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
use crate::aiplan4rust::compiler::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::support::lang::TypedList;

pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder, // Ajout indispensable du builder
) -> Result<InitialTaskNetwork, EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();

    registry.clear_variables();
    registry.clear_task_labels();

    let mut child_index = 0;

    // 1. Encodage des paramètres (Variables de l'ITN)
    let parameters = if let Ok(parameters_def_id) = node.try_child(child_index) {
        let parameters_def_node = ast.try_node(parameters_def_id)?;

        if parameters_def_node.kind() == AstKind::ParametersDef {
            let param_node_id = parameters_def_node.try_child(0)?;
            let param_node = ast.try_node(param_node_id)?;
            child_index += 1;

            typed_list::encode_variable_list(
                &SyntaxSubtree::new(param_node, param_node_id, ast),
                registry,
                builder.store_mut(),
            )?
        } else {
            builder.store_mut().intern_typed_list(TypedList::empty())
        }
    } else {
        builder.store_mut().intern_typed_list(TypedList::empty())
    };

    // 2. Encodage du corps (subtasks, ordering, etc.)
    let tw_node_id = node.try_child(child_index)?;
    let tw_node = ast.try_node(tw_node_id)?;

    // On délègue au module task_network avec le builder
    let tw = task_network::encode(
        &SyntaxSubtree::new(tw_node, tw_node_id, ast),
        registry,
        builder, // Le builder transmet les ExprId
    )?;

    let variable_symbols = registry.get_variable_symbols();
    let task_label_symbols = registry.get_task_label_symbols();

    let init_tw = InitialTaskNetwork::new(parameters, tw)
        .with_variable_symbols(variable_symbols)
        .with_task_label_symbols(task_label_symbols);

    Ok(init_tw)
}
