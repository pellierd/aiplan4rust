use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::encoding::typed_list;
use crate::aiplan4rust::lir::encoding::{expr, task_network, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::expr::ExprBuilder;
use crate::aiplan4rust::lir::problem::{MethodDef, NewLiftedProblem};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut NewLiftedProblem,
    builder: &mut ExprBuilder, // Injection du builder central
) -> Result<(), EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // Nettoyage du registre pour le nouveau scope de la méthode
    registry.clear_variables();
    registry.clear_task_labels();

    // --- ÉTAPE 1 : Identité de la Méthode ---
    let method_name_node_id = node.try_child(0)?;
    let method_name_node = ast.try_node(method_name_node_id)?;
    let method_name_str_id = method_name_node.try_ident()?;

    let method_symbol_id = ir.add_method_symbol(method_name_str_id);

    // --- ÉTAPE 2 : Encodage de la Signature (Paramètres) ---
    let parameters_def_id = node.try_child(1)?;
    let parameters_def_node = ast.try_node(parameters_def_id)?;
    let vars_node_id = parameters_def_node.try_child(0)?;
    let vars_node = ast.try_node(vars_node_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(vars_node, vars_node_id, ast),
        registry,
    )?;

    // --- ÉTAPE 3 : Encodage du Corps (Task, Precondition, Network) ---
    let def_body_node = ast.try_node(node.try_child(2)?)?;
    let children = def_body_node.children();
    let mut child_index = 0;

    // A. La tâche abstraite (Achieved Task) - Retourne un ExprId
    let task_node_id = children[child_index];
    let task_node = ast.try_node(task_node_id)?;
    let achieved_task = expr::encode(
        &SyntaxSubtree::new(task_node, task_node_id, ast),
        registry,
        builder,
    )?;
    child_index += 1;

    // B. Précondition optionnelle - Retourne un ExprId
    let precondition = if children.len() > child_index {
        let pre_node_def = ast.try_node(children[child_index])?;
        if pre_node_def.kind() == AstKind::MethodPreconditionDef {
            let pre_node_id = pre_node_def.try_child(0)?;
            let pre_node = ast.try_node(pre_node_id)?;
            child_index += 1;
            expr::encode(
                &SyntaxSubtree::new(pre_node, pre_node_id, ast),
                registry,
                builder,
            )?
        } else {
            builder.empty_or() // Utilisation du builder au lieu de Expr::empty_or()
        }
    } else {
        builder.empty_or()
    };

    // C. Réseau de tâches (Decomposition) - Retourne un TaskNetwork (contenant des ExprId)
    let tw_node_def_id = children[child_index];
    let tw_node_def = ast.try_node(tw_node_def_id)?;
    let task_network = task_network::encode(
        &SyntaxSubtree::new(tw_node_def, tw_node_def_id, ast),
        registry,
        builder,
    )?;

    // --- ÉTAPE 4 : Stockage ---
    let variable_symbols = registry.get_variable_symbols();
    let task_label_symbols = registry.get_task_label_symbols();

    // Construction de l'objet Method (S'assurer que Method::new accepte ExprId)
    let method_skeleton = MethodDef::new(
        method_symbol_id,
        parameters,
        achieved_task,
        precondition,
        task_network,
    )
    .with_variable_symbols(variable_symbols)
    .with_task_label_symbols(task_label_symbols);

    ir.add_method_def(method_skeleton);

    Ok(())
}
