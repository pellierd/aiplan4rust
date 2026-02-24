//! # LIR Expression Encoder
//!
//! This module provides the ops to transform a PDDL-based Abstract Syntax Tree (AST)
//! into a Lifted Intermediate Representation (LIR) [`Expr`].
//!
//! ## Overview
//!
//! The encoding process takes a [`SyntaxSubtree<AstNode>`] and produces a flat,
//! index-based expression tree. Unlike the source AST, the LIR expression is optimized
//! for planning tasks, with symbols already resolved to internal identifiers
//! (Predicates, Functions, Variables) via an [`EncodingRegistry`].
//!
//! ## Architecture
//!
//! The encoder is built around three main pillars:
//!
//! 1.  **Iterative Traversal**: To handle potentially deep PDDL expr without
//!     risking stack overflows, the encoder uses an explicit [`Vec`]-based stack
//!     instead of recursion.
//! 2.  **Symbol Resolution**: During encoding, every identifier in the AST is
//!     resolved against the [`SymbolTable`] to ensure semantic correctness and
//!     link usages to their declarations.
//! 3.  **Flat Storage**: The resulting [`Expr`] stores nodes in a contiguous vector.
//!     Parent-child relationships are maintained using stable indices ([`NodeId`]).
//!
//! ## Key Components
//!
//! * [`encode`]: The entry point that orchestrates the full tree transformation.
//! * [`alloc_node`]: Manages the dual task of translating AST data and persisting it
//!   into the LIR storage.
//! * [`encode_content`]: The semantic core that resolves symbols, constants, and
//!   quantifier scopes.
//!
//! ## Scoping and Variables
//!
//! When encountering quantifiers (e.g., `forall`, `exists`), this module registers
//! local variables within the [`EncodingRegistry`]. These variables are then
//! available for resolution by child nodes (the quantifier's body) during the
//! traversal.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{typed_list, EncodingRegistry};
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encodes an AST subtree into a LIR Expression.
/// This is the "free function" version of the previous TryFrom.
/// Encodes an AST subtree into a Lifted Intermediate Representation (LIR) [`Expr`].
///
/// This function serves as the primary orchestrator for the expression encoding process.
/// It performs a non-recursive, stack-based traversal of the provided AST subtree,
/// transforming each [`AstNode`] into a LIR-compatible [`ExprNode`].
///
/// # Arguments
///
/// * `subtree` - The source [`SyntaxSubtree`] containing the AST nodes to be encoded.
/// * `registry` - The mutable [`EncodingRegistry`] used for symbol resolution,
///   skeleton lookups, and managing local variable scopes.
///
/// # Returns
///
/// * `Ok(Expr)` - A complete LIR expression containing allocated nodes and a valid root ID.
/// * `Err(LirError)` - If any part of the encoding or symbol resolution fails.
///
/// # Process Flow
///
/// 1. **Initialization**: Creates a new, empty [`Expr`] container.
/// 2. **Root Allocation**: Encodes and allocates the root AST node. This ID is set as the
///    entry point of the LIR expression.
/// 3. **Iterative Traversal**: Uses a manual stack to visit every child node. This avoids
///    stack overflow issues associated with deep recursion in complex expr.
/// 4. **Incremental Building**: For each node popped from the stack:
///     - It is encoded and allocated via [`alloc_node`].
///     - Its valid children are pushed back onto the stack for subsequent processing.
///
/// # Example Logic
///
/// During traversal, the function maintains a mapping between the current AST node and
/// its parent in the LIR. This ensures that even though the storage is a flat vector,
/// the tree hierarchy is perfectly preserved.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<Expr, LirError> {
    let mut expr = Expr::new();
    let root_ast = subtree.node();
    let root_ast_id = subtree.node_id();

    // 1. Process and allocate the root of the expression tree
    let root_id = alloc_node(&mut expr, root_ast, root_ast_id, subtree, None, registry)?;
    expr.set_root_id(root_id)?;

    // 2. Initialize the traversal stack: (AST Node, AST ID, LIR Parent ID)
    let mut stack: Vec<(&AstNode, NodeId, NodeId)> = Vec::new();
    push_children_to_stack(&mut stack, subtree, root_ast, root_id)?;

    // 3. Process remaining nodes until the stack is empty
    while let Some((current_ast_node, current_ast_id, parent_id)) = stack.pop() {
        let node_id = alloc_node(
            &mut expr,
            current_ast_node,
            current_ast_id,
            subtree,
            Some(parent_id),
            registry,
        )?;

        // Schedule children of the current node to be processed next
        push_children_to_stack(&mut stack, subtree, current_ast_node, node_id)?;
    }

    Ok(expr)
}

/// Encodes an AST node and allocates it within the LIR expression storage.
///
/// This function serves as the primary bridge between the AST and the LIR storage.
/// It orchestrates the transformation of a single node and manages the pointers
/// required to maintain tree integrity within the flat vector storage of the [`Expr`].
///
/// # Arguments
///
/// * `expr` - The mutable LIR [`Expr`] container where the node will be persisted.
/// * `ast_node` - A reference to the source node from the AST.
/// * `ast_node_id` - The unique identifier of the node in the source AST, essential for
///   resolving symbol declarations.
/// * `subtree` - The context of the current AST subtree, used for recursive lookups.
/// * `parent_id` - The identifier of the parent node in the **LIR** (not the AST).
///   If `None`, this node is treated as the root.
/// * `registry` - The registry used for symbol resolution and tracking local scopes.
///
/// # Returns
///
/// * `Ok(NodeId)` - The new identifier of the allocated node within the LIR expression.
/// * `Err(LirError)` - If the node fails to encoding or if a parent-child link cannot be established.
///
/// # Process
///
/// 1. **Encoding**: Calls [`encode_node`] to resolve symbols and map AST kinds to LIR kinds.
/// 2. **Storage**: Allocates a slot in the `Expr` vector and stores the resulting [`ExprNode`].
/// 3. **Linking**: If a `parent_id` is provided, the function mutates the parent node in
///    the LIR to add the new node's ID to its list of children.
fn alloc_node(
    expr: &mut Expr,
    ast_node: &AstNode,
    ast_node_id: NodeId,
    subtree: &SyntaxSubtree<AstNode>,
    parent_id: Option<NodeId>,
    registry: &mut EncodingRegistry,
) -> Result<NodeId, LirError> {
    // Transform AST data into LIR data structure
    let expr_node = encode_node(ast_node, ast_node_id, parent_id, subtree, registry)?;

    // Persist the node into the expression's internal buffer
    let expr_node_id = expr.alloc(expr_node);

    // Maintain tree integrity by registering this node with its parent
    if let Some(pid) = parent_id {
        expr.try_node_mut(pid)?.add_child(expr_node_id);
    }

    Ok(expr_node_id)
}

/// Schedules the children of an AST node to be processed by pushing them onto the traversal stack.
///
/// This helper manages the iterative tree traversal by converting the AST parent-child
/// relationship into stack operations. It specifically filters out structural "meta-nodes"
/// (like [`AstKind::TypedList`]) that should not appear as independent nodes in the LIR.
///
/// # Arguments
///
/// * `stack` - The mutable traversal stack holding a triplet:
///     1. `&'a AstNode`: A reference to the next AST node to process.
///     2. `NodeId`: The unique identifier of the node in the source AST (used for symbol lookup).
///     3. `NodeId`: The ID of the already-allocated parent in the **LIR** storage.
/// * `subtree` - The syntax context used to look up child nodes by their IDs.
/// * `ast_node` - The current AST node whose children are being scheduled.
/// * `parent_id` - The LIR identifier of the node currently being processed, which will
///   act as the parent for these children.
///
/// # Returns
///
/// * `Ok(())` - If all valid children were successfully pushed onto the stack.
/// * `Err(LirError)` - If a child ID reference in the AST is dangling or invalid.
///
/// # Technical Details
///
/// * **Stack Order (LIFO):** Children are pushed in **reverse order**. This ensures that when
///   popped, they are processed in the original left-to-right order found in the PDDL source.
/// * **Filtered Nodes:** Nodes of type [`AstKind::TypedList`] are ignored here. Because they
///   represent structural groupings (like variable declarations), they are typically
///   collapsed or handled by the parent's `encode_content` ops.
fn push_children_to_stack<'a>(
    stack: &mut Vec<(&'a AstNode, NodeId, NodeId)>,
    subtree: &'a SyntaxSubtree<'a, AstNode>,
    ast_node: &'a AstNode,
    parent_id: NodeId,
) -> Result<(), LirError> {
    // We iterate in reverse to maintain left-to-right processing order in the stack (LIFO)
    for &child_id in ast_node.children().iter().rev() {
        let child_node = subtree.tree().try_node(child_id)?;

        // Skip TypedList meta-nodes as they are handled during parent content resolution
        if child_node.kind() == AstKind::TypedList {
            continue;
        }

        // Schedule the node for the next iteration of the encoder loop
        stack.push((child_node, child_id, parent_id));
    }
    Ok(())
}

/// Encodes a single AST node into its LIR representation ([`ExprNode`]).
///
/// This function acts as a coordinator that:
/// 1.  Translates the raw [`AstKind`] into a LIR-compatible [`ExprKind`].
/// 2.  Resolves the node's semantic payload (identifiers, constants, or structural skeletons)
///     via [`encode_content`].
/// 3.  Wraps the results into a new [`ExprNode`], preserving the hierarchical link to the parent.
///
/// # Arguments
///
/// * `ast_node` - The source node from the Abstract Syntax Tree.
/// * `ast_node_id` - The unique identifier of the node in the source AST. This is required
///   to resolve symbol usages against the global symbol table.
/// * `parent_id` - The identifier of the parent node in the **LIR** expression (not the AST).
/// * `subtree` - The syntax subtree context, used for navigating children or sibling data.
/// * `registry` - The central registry used for symbol resolution and state management.
///
/// # Returns
///
/// * `Ok(ExprNode)` - A fully initialized LIR node ready for allocation.
/// * `Err(LirError)` - If the node kind is invalid for an expression or if symbol resolution fails.
///
/// # Technical Note
///
/// This function is "pure" in the sense that it does not modify the `Expr` container itself;
/// it only produces the data structure. The actual insertion into the expression's
/// internal storage is handled by the caller (typically via [`alloc_node`]).
fn encode_node(
    ast_node: &AstNode,
    ast_node_id: NodeId,
    parent_id: Option<NodeId>,
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<ExprNode, LirError> {
    // 1. Map the AST kind to a LIR kind (e.g., AstKind::And -> ExprKind::And)
    let kind = encode_kind(ast_node.kind())?;

    // 2. Resolve IDs, Skeletons, or Literals based on the node's category
    let content = encode_content(ast_node, ast_node_id, subtree, registry)?;

    // 3. Assemble the node with its parent reference
    Ok(ExprNode::new(kind, content, parent_id))
}

/// Resolves and encodes the semantic content of an AST node into LIR [`ExprContent`].
///
/// This function is the semantic heart of the encoder. It performs symbol resolution by
/// bridging the gap between raw AST identifiers and the internal LIR registry.
///
/// # Arguments
///
/// * `ast_node` - The current node being processed from the AST.
/// * `ast_node_id` - The unique identifier of the node in the source AST (used for symbol lookups).
/// * `subtree` - The context of the current AST subtree for navigating children (e.g., signatures).
/// * `registry` - The mutable encoding registry used for symbol table lookups and variable registration.
///
/// # Returns
///
/// * `Ok(ExprContent)` - The resolved LIR content, which can be:
///     - A **Skeleton ID** for complex terms (Atomic Formulas, Function Terms, Tasks).
///     - A **Logical ID** for atomic symbols (Predicates, Functors, Constants, Variables).
///     - A **Literal Value** or **Operator** for leaf nodes.
/// * `Err(LirError)` - If a symbol cannot be resolved or if the content is semantically invalid.
///
/// # Resolution Logic
///
/// 1. **Complex Terms**: For nodes like `AtomicFormula`, it resolves the first child (the predicate)
///    to find its corresponding structural skeleton in the registry.
/// 2. **Quantifiers**: It extracts variable signatures, encodes them via [`typed_list`],
///    and registers them in the local scope of the [`EncodingRegistry`].
/// 3. **Atomic Symbols**: It uses the `ast_node_id` to query the symbol table and retrieve
///    the unique internal ID (e.g., a specific `VariableId` or `PredicateId`).
/// 4. **Primitives**: Maps raw `AstContent` (Floats, Operators) directly to `ExprContent`.
fn encode_content(
    ast_node: &AstNode,
    ast_node_id: NodeId,
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<ExprContent, LirError> {

    match ast_node.kind() {
        // --- Complex Terms (Signatures / Skeletons) ---
        // These nodes represent "calls" (e.g., p(x, y)). We resolve the structural
        // skeleton which contains the symbol ID and the expected argument types.
        AstKind::AtomicFormula => {
            let predicate_id = ast_node.children()[0];
            let atom_skeleton_declaration = registry.symbol_table()
                .try_resolve_declaration_by_usage(predicate_id, SymbolKind::Predicate)?;
            let atom_skeleton_id = registry.try_resolve_atom_skeleton(atom_skeleton_declaration.node_id())?;
            Ok(ExprContent::AtomSkeleton(atom_skeleton_id))
        },
        AstKind::FunctionTerm => {
            let function_id = ast_node.children()[0];
            let function_skeleton_declaration = registry.symbol_table()
                .try_resolve_declaration_by_usage(function_id, SymbolKind::Function)?;
            let function_skeleton_id = registry.try_resolve_function_skeleton(function_skeleton_declaration.node_id())?;
            Ok(ExprContent::FunctionSkeleton(function_skeleton_id))
        },
        AstKind::Task => {
            // The task identifier is the first child of the Task node
            let task_id = ast_node.children()[0];

            // 1. Attempt "soft" resolution for a Compound Task skeleton.
            let declaration = match registry.symbol_table().resolve_declaration_by_usage(task_id, SymbolKind::Task)? {
                // Successfully resolved as a Compound Task
                Some(decl) => decl,

                // 2. Fallback to "strict" resolution for a Primitive Action.
                // If it's not a compound task, it must be an action.
                None => registry.symbol_table()
                    .try_resolve_declaration_by_usage(task_id, SymbolKind::Action)?
            };

            // 3. Retrieve the unique Skeleton ID from the registry.
            // This ID was generated during the first pass (Collection Phase).
            let task_skeleton_id = registry.try_resolve_task_skeleton(declaration.node_id())?;

            Ok(ExprContent::TaskSkeleton(task_skeleton_id))
        },

        // --- Quantifiers (Scope Management) ---
        AstKind::Forall | AstKind::Exists => {
            let children = ast_node.children();
            let typed_list_node = subtree.tree().try_node(children[0])?;
            let typed_list_tree = SyntaxSubtree::new(typed_list_node, children[0], subtree.tree());

            // 1. Encode the variable signatures (names and types)
            let vars = typed_list::encode_variable_list(&typed_list_tree, registry)?;

            // 2. Register variables in the local scope.
            // Since we use an iterative traversal, variables are registered in the
            // registry using their unique NodeId. This ensures children nodes
            // can resolve these variables even without a recursive call stack.
            for &typed_variable_node_id in typed_list_node.children() {
                let typed_variable_node = subtree.tree().try_node(typed_variable_node_id)?;
                let variable_node_id = typed_variable_node.try_child(0)?;
                let variable_node = subtree.tree().try_node(variable_node_id)?;
                let variable_symbol = variable_node.try_ident()?;
                registry.register_variable(variable_node_id, variable_symbol);
            }

            Ok(ExprContent::QuantifierVariables(vars))
        },

        // --- Atomic Symbols (Identities) ---
        // These nodes represent the symbols themselves. We resolve their
        // logical ID from the registry based on their declaration NodeId.
        AstKind::Predicate => {
            let predicate_declaration = registry.symbol_table().try_resolve_declaration_by_usage(ast_node_id, SymbolKind::Predicate)?;
            let predicate_id = registry.try_resolve_predicate(predicate_declaration.node_id())?;
            Ok(ExprContent::PredicateSymbol(predicate_id))
        },
        AstKind::FunctionSymbol => {
            let functor_declaration = registry.symbol_table().try_resolve_declaration_by_usage(ast_node_id, SymbolKind::Function)?;
            let functor_id = registry.try_resolve_functor(functor_declaration.node_id())?;
            Ok(ExprContent::FunctionSymbol(functor_id))
        },
        AstKind::Constant => {
            let constant_declaration = registry.symbol_table().try_resolve_declaration_by_usage(ast_node_id, SymbolKind::Constant)?;
            let constant_id = registry.try_resolve_object(constant_declaration.node_id())?;
            Ok(ExprContent::Constant(constant_id))
        },
        AstKind::Variable => {
            let variable_declaration = registry.symbol_table().try_resolve_declaration_by_usage(ast_node_id, SymbolKind::Variable)?;
            let variable_id = registry.try_resolve_variable(variable_declaration.node_id())?;
            Ok(ExprContent::Variable(variable_id))
        },
        AstKind::TaskSymbol => {
            // 1. Attempt "soft" resolution for a Compound Task.
            let declaration = match registry.symbol_table().resolve_declaration_by_usage(ast_node_id, SymbolKind::Task)? {
                // Successfully resolved as a Compound Task.
                Some(decl) => decl,

                // 2. Fallback to "strict" resolution for a Primitive Action.
                // If it's not a Task, we try to resolve as an Action.
                None => registry.symbol_table()
                    .try_resolve_declaration_by_usage(ast_node_id, SymbolKind::Action)?
            };

            // 3. Final ID Retrieval from the Registry (Pass 1).
            let task_symbol_id = registry.try_resolve_task_symbol(declaration.node_id())?;

            Ok(ExprContent::TaskSymbol(task_symbol_id))
        }
        AstKind::TaskID => {
            let label_symbol_id = ast_node.try_ident()?;
            let task_label_id = registry.try_resolve_task_label(label_symbol_id)?;
            Ok(ExprContent::TaskLabelSymbol(task_label_id))
        }

        // --- Leaf Nodes and Operators ---
        // If the Kind is not a complex symbol, we extract the raw primitive
        // value or the operator stored within the AST content.
        _ => match ast_node.content() {
            AstContent::Float(f) => Ok(ExprContent::Number(*f)),
            AstContent::BinaryComp(op) => Ok(ExprContent::BinaryComp(*op)),
            AstContent::AssignOp(op) => Ok(ExprContent::AssignOp(*op)),
            AstContent::ArithmeticOp(op) => Ok(ExprContent::ArithmeticOp(*op)),
            AstContent::Optimization(op) => Ok(ExprContent::Optimization(*op)),
            AstContent::None => Ok(ExprContent::None),
            _ => {
                println!("{}", ast_node);
                Err(ExprError::unsupported_content(ast_node.content().clone()).into())
            },
        }
    }
}

/// Maps a raw [`AstKind`] to its corresponding [`ExprKind`] in the LIR.
///
/// This function acts as a semantic filter during the encoding process. It ensures that
/// only AST nodes that are valid within the context of an expression (e.g., logical
/// operators, quantifiers, fluents) are translated.
///
/// # Arguments
///
/// * `kind` - The raw [`AstKind`] extracted from the AST node.
///
/// # Returns
///
/// * `Ok(ExprKind)` - The equivalent expression kind used by the Lifted Intermediate Representation.
/// * `Err(ExprError)` - If the `AstKind` does not belong in an expression (e.g., a domain or problem definition node).
///
/// # Errors
///
/// Returns [`ExprError::InvalidAstNode`] if the provided `AstKind` cannot be mapped
/// to an expression, preventing malformed AST structures from entering the LIR.
fn encode_kind(kind: AstKind) -> Result<ExprKind, ExprError> {
    match kind {
        AstKind::And => Ok(ExprKind::And),
        AstKind::Or => Ok(ExprKind::Or),
        AstKind::Not => Ok(ExprKind::Not),
        AstKind::Imply => Ok(ExprKind::Imply),
        AstKind::Forall => Ok(ExprKind::Forall),
        AstKind::Exists => Ok(ExprKind::Exists),
        AstKind::Predicate => Ok(ExprKind::Predicate),
        AstKind::Variable => Ok(ExprKind::Variable),
        AstKind::Constant => Ok(ExprKind::Constant),
        AstKind::FunctionSymbol => Ok(ExprKind::FunctionSymbol),
        AstKind::TaskSymbol => Ok(ExprKind::TaskSymbol),
        AstKind::PrefName => Ok(ExprKind::PrefName),
        AstKind::FunctionTerm => Ok(ExprKind::FunctionTerm),
        AstKind::Number => Ok(ExprKind::Number),
        AstKind::AtomicFormula => Ok(ExprKind::AtomicFormula),
        AstKind::FComp => Ok(ExprKind::FComp),
        AstKind::Assign => Ok(ExprKind::Assign),
        AstKind::Operation => Ok(ExprKind::Operation),
        AstKind::AtStart => Ok(ExprKind::AtStart),
        AstKind::AtEnd => Ok(ExprKind::AtEnd),
        AstKind::Overall => Ok(ExprKind::Overall),
        AstKind::Always => Ok(ExprKind::Always),
        AstKind::Sometime => Ok(ExprKind::Sometime),
        AstKind::Within => Ok(ExprKind::Within),
        AstKind::AtMostOnce => Ok(ExprKind::AtMostOnce),
        AstKind::SometimeAfter => Ok(ExprKind::SometimeAfter),
        AstKind::SometimeBefore => Ok(ExprKind::SometimeBefore),
        AstKind::AlwaysWithin => Ok(ExprKind::AlwaysWithin),
        AstKind::HoldDuring => Ok(ExprKind::HoldDuring),
        AstKind::HoldAfter => Ok(ExprKind::HoldAfter),
        AstKind::TimedInitialLiteral => Ok(ExprKind::TimedInitialLiteral),
        AstKind::Metric => Ok(ExprKind::Metric),
        AstKind::TotalTime => Ok(ExprKind::TotalTime),
        AstKind::IsViolated => Ok(ExprKind::IsViolated),
        AstKind::Length => Ok(ExprKind::Length),
        AstKind::Serial => Ok(ExprKind::Serial),
        AstKind::Parallel => Ok(ExprKind::Parallel),
        AstKind::Task => Ok(ExprKind::Task),
        AstKind::TaskID => Ok(ExprKind::TaskID),
        AstKind::TaggedTask => Ok(ExprKind::TaggedTask),
        AstKind::TaskOrderingConstraint => Ok(ExprKind::TaskOrderingConstraint),
        other => Err(ExprError::invalid_ast_node(other)),
    }
}
