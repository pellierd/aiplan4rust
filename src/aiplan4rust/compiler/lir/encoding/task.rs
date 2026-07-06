//! HTN Abstract Task Encoding
//!
//! This module handles the encoding and registration of abstract (compound) tasks.
//!
//! In HTN planning, an abstract task represents a high-level objective. It
//! acts as a contract: any method claiming to decompose this task must
//! match its signature.
//!
//! This module specializes a generic [`NamedTypedList`] into a [`Task`]
//! skeleton, stores it in the LIR, and updates the evaluator to map the
//! task's symbol node to its internal ID.

use crate::aiplan4rust::compiler::lir::encoding::EncodingRegistry;
use crate::aiplan4rust::compiler::lir::encoding::{typed_list, EncodingError};
use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
use crate::aiplan4rust::compiler::lir::problem::skeleton::task::Task;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::{AstKind, AstNode};

/// Encodes an HTN abstract task signature and registers it within the LIR context.
///
/// This function translates an AST task definition into its Lifted representation by:
/// 1. **Parsing**: Extracting the task name and its typed parameters using [`named_typed_list`].
/// 2. **LIR Storage**: Adding the resulting [`Task`] skeleton to the [`LiftedProblem`].
/// 3. **Registry Binding**: Mapping the task's name `NodeId` to both its [`TaskID`]
///    and [`TaskSkeletonID`] in the [`EncodingRegistry`].
///
/// This dual registration allows the encoder to resolve task references in methods
/// by linking the specific AST symbol to its resolved LIR counterparts.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the task (e.g., `(transport ?p - package)`).
/// * `evaluator` - The mutable evaluator for symbol resolution and ID mapping.
/// * `ir` - The mutable [`LiftedProblem`] where the task is stored.
///
/// # Errors
///
/// Returns [`LirError`] if the signature is malformed, types cannot be resolved,
/// or mandatory components are missing.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    builder: &mut ExprBuilder,
) -> Result<(), EncodingError> {
    let node = subtree.node();
    let tree = subtree.tree();

    // 1. IDENTITÉ : On extrait le nom et on crée l'ID sémantique immédiatement
    let task_symbol_node_id = node.try_child(0)?;
    let name_node = tree.try_node(task_symbol_node_id)?;
    let name_str_id = name_node.try_ident()?;

    // On demande au Problem de nous donner l'ID officiel pour ce nom
    let task_symbol_id = ir.add_task_symbol(name_str_id);

    // 2. STRUCTURE : Extraction des paramètres (Second fils, forcément ParametersDef)
    registry.clear_variables();

    let parameters_def_id = node.try_child(1)?;
    let parameters_def_node = tree.try_node(parameters_def_id)?;
    debug_assert!(parameters_def_node.kind() == AstKind::ParametersDef);

    let parameters_id = parameters_def_node.try_child(0)?;
    let parameters_node = tree.try_node(parameters_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(parameters_node, parameters_id, tree),
        registry,
        builder.store_mut(),
    )?;

    // 3. CONSTRUCTION : On utilise task_symbol_id au lieu de name_str_id
    // La signature contient maintenant l'ID typé
    let variable_symbols = registry.get_variable_symbols();
    let task_skeleton =
        Task::new(task_symbol_id, parameters).with_variable_symbols(variable_symbols);

    // 4. STOCKAGE : On enregistre le squelette
    // Note: add_task_def ne renvoie plus que le skeleton_id puisque le symbol_id est déjà connu
    let task_skeleton_id = ir.add_task_def(task_skeleton);

    // 5. MAPPING : On lie le nœud AST aux deux types d'IDs
    registry.register_task_skeleton(task_symbol_node_id, task_skeleton_id);
    registry.register_task_symbol(task_symbol_node_id, task_symbol_id);

    Ok(())
}
