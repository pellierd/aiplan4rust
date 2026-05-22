use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::store::encoding_old::registry::EncodingRegistry;
use crate::aiplan4rust::lir::store::encoding_old::{expr, task_network, typed_list};
use crate::aiplan4rust::lir::store::expr_old::Expr;
use crate::aiplan4rust::lir::store::problem_old::method::Method;
use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes an HTN method from the syntax tree into the LIR.
///
/// Processes the method's signature, the abstract task it implements,
/// its optional preconditions, and the resulting task network decomposition.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the method definition.
/// * `evaluator` - The evaluator for symbol and parameter resolution.
/// * `ir` - The mutable `LiftedProblem` where the method is registered.
///
/// # Returns
///
/// * `Ok(Method)` - The encoded HTN method.
/// * `Err(LirError)` - If the structure is invalid or any component fails to encoding_old.
///
/// # Errors
///
/// This function returns an error if:
/// * The method header or the implemented task expression is malformed.
/// * The mandatory task network definition is missing.
/// * Symbol resolution fails within the precondition or the task network.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    registry.clear_variables();
    registry.clear_task_labels();

    // --- ÉTAPE 1 : Identité de la Méthode ---
    let method_name_node_id = node.try_child(0)?;
    let method_name_node = ast.try_node(method_name_node_id)?;
    let method_name_str_id = method_name_node.try_ident()?;

    // Réservation de l'ID sémantique dans le Problem
    let method_symbol_id = ir.add_method_symbol(method_name_str_id);

    // --- ÉTAPE 2 : Encodage de la Signature ---
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

    // A. La tâche abstraite (Achieved Task)
    let task_node_id = children[child_index];
    let task_node = ast.try_node(task_node_id)?;
    let achieved_task = expr::encode(&SyntaxSubtree::new(task_node, task_node_id, ast), registry)?;
    child_index += 1;

    // B. Précondition optionnelle (Utilisation de ta logique if children.len())
    let precondition = if children.len() > child_index {
        let pre_node_def = ast.try_node(children[child_index])?;
        if pre_node_def.kind() == AstKind::MethodPreconditionDef {
            let pre_node_id = pre_node_def.try_child(0)?;
            let pre_node = ast.try_node(pre_node_id)?;
            child_index += 1;
            expr::encode(&SyntaxSubtree::new(pre_node, pre_node_id, ast), registry)?
        } else {
            Expr::empty_or() // On garde empty_or comme demandé
        }
    } else {
        Expr::empty_or()
    };

    // C. Réseau de tâches (Decomposition)
    let tw_node_def_id = children[child_index];
    let tw_node_def = ast.try_node(tw_node_def_id)?;
    let task_network = task_network::encode(
        &SyntaxSubtree::new(tw_node_def, tw_node_def_id, ast),
        registry,
    )?;

    // --- ÉTAPE 4 : Stockage ---
    let variable_symbols = registry.get_variable_symbols();
    let task_label_symbols = registry.get_task_label_symbols();

    // On passe 'parameters' directement : il est déplacé (move) dans Method sans clone.
    let method_skeleton = Method::new(
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
