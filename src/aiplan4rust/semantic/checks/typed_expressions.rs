use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::AssignOp;
use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lang::Requirement::NumericFluents;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{Node, NodeId};
use crate::SymbolTable;

/// Checks the type compatibility of typed expressions in the syntax tree, including
/// comparisons, assignments, and arithmetic operations.
///
/// This function traverses the syntax tree to verify that expressions have compatible
/// types according to their operation kind. It specifically validates:
/// - Equality checks (`=`) and simple assignments (`assign`), ensuring operand compatibility.
/// - Numeric comparisons (`>`, `<`, `>=`, `<=`) and arithmetic assignments (`+=`, `-=`, etc.),
///   ensuring operands are numeric.
///
/// Type compatibility logic is delegated to specialized helper functions, and any
/// mismatches are reported as diagnostics through the provided manager.
///
/// # Parameters
///
/// - `context`: A reference to the [`CheckContext`] providing access to the syntax tree,
///   symbol table, and diagnostic metadata (provider, source ID).
/// - `type_checker`: A [`TypeChecker`] instance used for resolving type inheritance
///   and compatibility.
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] where
///   type errors are collected.
///
/// # Returns
///
/// - `Ok(true)`: All typed expressions are valid.
/// - `Ok(false)`: One or more type mismatches were detected and reported.
/// - `Err(SemanticError)`: An internal error occurred during tree traversal or type resolution.
///
/// # Example
///
/// ```rust
/// let check_ctx = context.as_check_context(Provider::Analyzer);
/// let is_valid = check_typed_expressions(&check_ctx, &type_checker, &mut diagnostic_manager)?;
/// ```
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`TypeChecker`]: crate::types::TypeChecker
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
pub fn check_typed_expressions(
    context: &CheckContext,
    symbol_table: &mut SymbolTable,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;

    for node in context.syntax_tree().preorder().values() {
        if is_equal_binary_comp(node) || is_assign(node) {
            let (ty1, ty2) = get_binary_operation_types(node, context, symbol_table)?;

            // Call check_equal_and_assign function to handle this case
            no_error &= check_equal_and_assignment_expression(
                context,
                type_checker,
                node,
                &ty1,
                &ty2,
                diagnostic_manager,
            )?;
        } else if is_numeric_expression(node) {
            let (ty1, ty2) = get_binary_operation_types(node, context, symbol_table)?;

            // Call check_other_cases function to handle these cases
            no_error &= check_numeric_expression(context, node, &ty1, &ty2, diagnostic_manager);
        }
    }

    Ok(no_error)
}

/// Returns `true` if the syntax represents an equality binary comparison (`BinaryComp::Equal`).
fn is_equal_binary_comp(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::Comparison) && node.as_compare_op() == Some(CompareOp::Equal)
}

/// Returns `true` if the syntax represents a simple assignment (`AssignOp::Assign`).
fn is_assign(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::Assignment) && node.as_assign_op() == Some(AssignOp::Assign)
}

/// Returns `true` if the syntax is a numeric comparison or a scale assignment logic.
///
/// This includes:
/// - Comparison operators: `Greater`, `GreaterEq`, `Less`, `LessEq`.
/// - Assignment operators: `ScaleUp`, `ScaleDown`, `Increase`, `Decrease`.
fn is_numeric_expression(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::Comparison)
        && matches!(
            node.as_compare_op(),
            Some(CompareOp::Greater)
                | Some(CompareOp::GreaterEq)
                | Some(CompareOp::Less)
                | Some(CompareOp::LessEq)
        )
        || matches!(node.kind(), AstKind::Assignment)
            && matches!(
                node.as_assign_op(),
                Some(AssignOp::ScaleUp)
                    | Some(AssignOp::ScaleDown)
                    | Some(AssignOp::Increase)
                    | Some(AssignOp::Decrease)
            )
}

/// Checks the type_checker compatibility of operands in equality (`=`) or assignment (`assign`) logic.
///
/// This function verifies that the types of both operands involved in an equality or assignment
/// operation are compatible. Equality comparisons (`=`) require operands of the same type_checker,
/// while assignment operations (`assign`) may allow some flexibility depending on the domain,
/// such as assigning numeric values or specific user-defined types.
///
/// Type compatibility is checked using the provided `TypeChecker`. If a mismatch is found,
/// an error is reported through the `DiagnosticManager`.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax arena containing the AST and symbol information.
/// - `type_checker`: The type_checker checker used to validate type_checker compatibility.
/// - `syntax`: The syntax syntax representing the equality or assignment operation.
/// - `ty1`: The type_checker(s) of the left-hand side operand.
/// - `ty2`: The type_checker(s) of the right-hand side operand.
/// - `source`: The diagnostic source context indicating where diagnostics originate.
/// - `diagnostic_manager`: The diagnostic manager used to log any type_checker mismatches.
///
/// # Returns
/// - `Ok(true)` if the operand types are compatible.
/// - `Ok(false)` if the types are incompatible and an error is logged.
/// - `Err(ParserInternalError)` if an internal error occurs during the check.
///
/// # Example
/// ```rust
/// let ty1 = vec!["number".to_string()];
/// let ty2 = vec!["object".to_string()];
/// let result = check_equal_and_assignment_expression(
///     &annotated_syntax_tree,
///     &type_checker,
///     &syntax,
///     &ty1,
///     &ty2,
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn check_equal_and_assignment_expression(
    context: &CheckContext,
    type_checker: &TypeChecker,
    node: &AstNode,
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut no_error = true;

    if !type_checker.have_common_supertype(&ty1, &ty2)? {
        no_error = false;
        let error = Diagnostic::error_type_mismatch_in_expression(
            ty1.clone(),
            ty2.clone(),
            context.provider(),
            context.source(),
            node.span(),
        );

        diagnostic_manager.add_diagnostic(error);
    }

    Ok(no_error)
}

/// Checks whether the operand types in a numeric comparison or assignment logic
/// are compatible with numeric operations (i.e., of type_checker `number`).
///
/// This function is used specifically for logic involving numeric comparisons
/// (e.g., `greater`, `less`, `>=`, `<=`) and numeric assignment operations
/// (e.g., `increase`, `decrease`, `scale-up`, `scale-down`). For such logic
/// to be valid, both operands must have the `number` type_checker.
///
/// If either operand does not have the `number` type_checker, the function logs a
/// type_checker mismatch error.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax arena containing the AST and metadata.
/// - `syntax`: The syntax syntax representing the numeric logic.
/// - `ty1`: A reference to a vector of strings representing the type_checker of the left operand.
/// - `ty2`: A reference to a vector of strings representing the type_checker of the right operand.
/// - `source`: The diagnostic source indicating where this check is performed.
/// - `diagnostic_manager`: The error manager used to report type_checker errors.
///
/// # Returns
/// - `Ok(true)` if both operands have the `number` type_checker.
/// - `Ok(false)` if a type_checker mismatch is found and an error is logged.
/// - `Err(ParserInternalError)` if an internal error occurs during type_checker checking.
///
/// # Example
/// ```rust
/// let ty1 = vec!["number".to_string()];
/// let ty2 = vec!["number".to_string()];
/// let result = check_numeric_expression(
///     &annotated_syntax_tree,
///     &syntax,
///     &ty1,
///     &ty2,
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn check_numeric_expression(
    context: &CheckContext,
    node: &AstNode,
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    let mut no_error = true;

    // Handle Greater, Less, etc.
    if *ty1 != Type::<SymbolId>::number() || *ty2 != Type::<SymbolId>::number() {
        no_error = false;
        let error = Diagnostic::error_invalid_types_in_numeric_expression(
            ty1.clone(),
            ty2.clone(),
            context.provider(),
            context.source(),
            node.span(),
        );
        diagnostic_manager.add_diagnostic(error);
    }

    no_error
}

/// Retrieves and returns the types of both operands in a binary logic.
///
/// This function ensures that the given syntax syntax represents a binary operation
/// with exactly two children. It then looks up the types of both operand nodes
/// using the annotated syntax arena and associated symbol table.
///
/// # Parameters
/// - `syntax`: The syntax syntax representing the binary operation.
/// - `annotated_syntax_tree`: The annotated syntax arena containing the full AST and symbol
///   information.
///
/// # Returns
/// A `Result` containing a pair of vectors of strings:
/// - The first vector represents the type_checker(s) of the left operand.
/// - The second vector represents the type_checker(s) of the right operand.
///
/// # Errors
/// This function returns a `SemanticCheckError` in the following cases:
/// - The syntax does not have exactly two children (binary operations must have two).
/// - One of the children is missing in the syntax arena.
/// - One of the operands has no associated type_checker in the symbol table.
///
/// # Example
/// ```rust
/// let (ty1, ty2) = get_binary_operation_types(&syntax, &annotated_syntax_tree)?;
/// ```
fn get_binary_operation_types(
    node: &AstNode,
    context: &CheckContext,
    symbol_table: &SymbolTable,
) -> Result<(Type<SymbolId>, Type<SymbolId>), SemanticError> {
    let ast = context.syntax_tree();

    // Try to get the first child node index and node
    let arg1_id = node.try_child(0)?;
    let arg1 = ast.try_node(arg1_id)?;

    // Try to get the second child node index and node
    let arg2_id = node.try_child(1)?;
    let arg2 = ast.try_node(arg2_id)?;

    // Get the typing of the first operand, or return a specific error if missing
    let ty1 = get_type(arg1_id, arg1, context, symbol_table)?
        .ok_or_else(|| SemanticCheckError::missing_operand_type(arg1_id, 0))?;

    // Get the typing of the second operand, or return a specific error if missing
    let ty2 = get_type(arg2_id, arg2, context, symbol_table)?
        .ok_or_else(|| SemanticCheckError::missing_operand_type(arg2_id, 1))?;

    Ok((ty1, ty2))
}

/// Determines the type_checker of a syntax syntax based on its kind.
///
/// This function supports several kinds of nodes: numbers, variables, constants,
/// and function terms. It delegates type_checker resolution to specialized helper functions
/// depending on the syntax kind. The function is used during type_checker checking to retrieve
/// the declared or inferred type_checker of an logic or symbol.
///
/// # Parameters
/// - `index`: The index of the current syntax in the syntax arena.
/// - `syntax`: A reference to the `HeapSyntaxNode` representing the AST syntax to analyze.
/// - `annotated_syntax_tree`: A reference to the annotated syntax arena that provides access
///   to both the symbol table and the full syntax structure.
///
/// # Returns
/// A `Result` containing:
/// - `Some(Vec<String>)` if the syntax has an associated type_checker.
/// - `None` if the type_checker is undefined but not erroneous (e.g., optional typing).
/// - `Err(ParserInternalError)` if the syntax kind is invalid or cannot be typed.
///
/// # Errors
/// - Returns an error if the syntax kind is not one of the expected kinds (`Number`, `Variable`,
///   `Constant`, `FunctionTerm`).
///
/// # Example
/// ```rust
/// let ty = get_type(index, &syntax, &annotated_syntax_tree)?;
/// ```
pub fn get_type(
    index: NodeId,
    node: &AstNode,
    context: &CheckContext,
    symbol_table: &SymbolTable,
) -> Result<Option<Type<SymbolId>>, SemanticError> {
    match node.kind() {
        // Case 1: Directly a number -> Type is NUMBER_TYPE
        AstKind::Number => get_number_type(),

        // Case 2: Variable
        AstKind::Variable => {
            get_variable_type(index, node.content().try_ident()?, context, symbol_table)
        }

        // Case 3: Constant
        AstKind::Object => get_constant_type(index, node.content().try_ident()?, symbol_table),

        // Case 4: Function Term
        AstKind::Function => get_function_term_type(node, context, symbol_table),

        // Case 5: Arithmetic Operation
        AstKind::Arithmetic => get_number_type(),

        // Default case: Unexpected AST syntax kind
        found_kind => Err(SemanticError::unexpected_node_kind(
            index,
            vec![
                AstKind::Number,
                AstKind::Variable,
                AstKind::Object,
                AstKind::Function,
                AstKind::Arithmetic,
            ],
            found_kind,
        )),
    }
}

/// Returns the predefined type_checker for numeric values.
///
/// This helper function is used when an AST syntax represents a numeric literal.
/// It returns the predefined type_checker associated with numbers (i.e., `NUMBER_TYPE`),
/// wrapped in a `Vec<String>` to be consistent with other type_checker representations
/// in the type_checker checking system.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(vec!["number"]))` if the type_checker resolution is successful.
/// - `Err(ParserInternalError)` is not expected in this implementation, but
///   the return type_checker remains consistent with other type_checker-checking helpers.
///
/// # Example
/// ```rust
/// let ty = get_number_type()?; // Returns Some(["number".to_string()])
/// ```
fn get_number_type() -> Result<Option<Type<SymbolId>>, SemanticError> {
    Ok(Some(Type::<SymbolId>::number().clone()))
}

/// Retrieves the type of a variable symbol, handling both explicit declarations and implicit built-ins.
///
/// This function resolves the type of a variable used in the AST using a priority-based logic:
///
/// 1. **Explicit Declaration**: It first consults the symbol table. If the user has explicitly
///    declared the variable (e.g., in `:parameters`), that type is returned. This allows
///    users to override or "shadow" built-in variables (the "tordu" case).
/// 2. **Implicit Built-in**: If no explicit declaration is found and the symbol matches
///    `?duration`, it is automatically inferred as a `number`.
/// 3. **Fallback**: Otherwise, it attempts a standard lookup via `get_declaration_type`.
///
/// ### Permissive Design
/// Unlike strict PDDL, this function does not verify the `:durative-actions` requirement
/// here to avoid blocking semantic analysis on domains with missing or late-parsed requirements.
///
/// # Parameters
/// - `index`: The `NodeId` of the AST node where the variable is used.
/// - `symbol`: The `SymbolId` of the variable (e.g., the ID for `?x` or `?duration`).
/// - `context`: The semantic context providing access to the symbol table and syntax tree.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(Type))`: The resolved type (e.g., `number` or a user-defined type).
/// - `Ok(None)`: If the variable exists but has no associated type.
/// - `Err(SemanticError)`: If an internal error occurs or resolution fails.
///
/// # Example
/// ```rust
/// // If ?duration is used but not declared in parameters, returns 'number'
/// let ty = get_variable_type(node_id, DURATION_ID, &context)?;
/// ```
fn get_variable_type(
    index: NodeId,
    symbol: SymbolId,
    _context: &CheckContext,
    symbol_table: &SymbolTable,
) -> Result<Option<Type<SymbolId>>, SemanticError> {
    // 2. Implicit Case: If no explicit declaration exists, check for reserved symbols.
    // ?duration is implicitly a 'number' in durative actions.
    if symbol == SymbolInterner::DURATION_VARIABLE_SYMBOL_ID {
        // We return 'number' without strict requirement checks to remain
        // robust against IPC benchmarks with missing :durative-actions tags.
        return get_number_type();
    }

    // 3. Fallback: Standard declaration lookup.
    get_declaration_type(index, symbol_table)
}

/// Retrieves the type_checker of a constant symbol from the symbol table.
///
/// This function resolves the type_checker of a constant declared in the domain or problem file.
/// It delegates the actual lookup to `get_declaration_type`, which handles symbol table
/// access and conflict resolution.
///
/// # Parameters
/// - `index`: The index of the AST syntax, used for error reporting.
/// - `symbol`: The name of the constant (e.g., `"loc1"`).
/// - `annotated_syntax_tree`: A reference to the annotated syntax arena containing the
///   symbol table and other context.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(types))`: A vector of type_checker names if the constant was successfully resolved.
/// - `Ok(None)`: If the constant exists but has no declared type_checker.
/// - `Err(ParserInternalError)`: If the constant is not declared or declared inconsistently.
///
/// # Example
/// ```rust
/// let ty = get_constant_type(12, "loc1", &annotated_syntax_tree)?;
/// ```
fn get_constant_type(
    index: NodeId,
    _symbol: SymbolId,
    symbol_table: &SymbolTable,
) -> Result<Option<Type<SymbolId>>, SemanticError> {
    get_declaration_type(index, symbol_table)
}

/// Helper function to retrieve the types associated with a symbol usage from the symbol table.
///
/// This function looks up a declaration corresponding to a usage identified by its AST index in the
/// symbol table. It returns the types declared for that symbol usage, if any. If multiple
/// declarations are found for the same usage, it returns an error to enforce uniqueness. If no
/// declaration is found for the usage, it returns `Ok(None)`.
///
/// # Parameters
/// - `index`: The AST index representing the usage of the symbol in the syntax arena.
/// - `symbol_table`: A reference to the symbol table containing symbol declarations and usages.
///
/// # Returns
/// Returns a `Result` containing:
/// - `Ok(Some(types))`: A vector of type_checker names if a single declaration with types is found.
/// - `Ok(None)`: If no declaration is found or the declaration has no types.
/// - `Err(ParserInternalError)`: If multiple declarations are found for the same usage, indicating
///   a conflict.
///
/// # Example
/// ```rust
/// let types = get_declaration_type(10, &symbol_table)?;
/// if let Some(types_vec) = types {
///     println!("Types found: {:?}", types_vec);
/// } else {
///     println!("No types declared for this usage.");
/// }
/// ```
fn get_declaration_type(
    node_id: NodeId,
    symbol_table: &SymbolTable,
) -> Result<Option<Type<SymbolId>>, SemanticError> {
    match symbol_table.resolve_usage(node_id) {
        Ok(decl) => Ok(decl.ty().cloned()),
        Err(_) => Ok(None),
    }
}

/// Helper to handle a `FunctionTerm` node and retrieve its typing.
///
/// This function checks if the first child of the `FunctionTerm` AST node is a valid functor,
/// retrieves its corresponding AST entry, and determines the typing associated with the function term.
/// It specifically handles the special case where the functor is the `TOTAL_TIME` symbol and
/// ensures the presence of the `NumericFluents` requirement before returning the number typing.
///
/// If the functor is missing, invalid, or not of kind `FunctionSymbol`, an error is returned.
///
/// # Parameters
/// - `node`: Reference to the `FunctionTerm` AST node.
/// - `context`: Semantic checking context, providing access to the AST, symbol table, and requirements.
///
/// # Returns
/// - `Ok(Some(typing))`: The typing of the function term if determined successfully.
/// - `Ok(None)`: If the function term has no functor or no typing could be inferred.
/// - `Err(SemanticCheckError)`: If the functor is missing, invalid, or of an unexpected kind.
///
/// # Errors
/// Returns `SemanticCheckError::unexpected_ast_kind` if the functor's AST node kind is not `FunctionSymbol`.
///
/// # Example
/// ```rust
/// let ty = get_function_term_type(node_id, &function_term_node, &context)?;
/// ```
fn get_function_term_type(
    node: &AstNode,
    context: &CheckContext,
    symbol_table: &SymbolTable,
) -> Result<Option<Type<SymbolId>>, SemanticError> {
    let functor_index = node.try_child(0)?;
    let functor_entry = context.syntax_tree().try_node(functor_index)?;

    if let AstKind::FunctionSymbol = functor_entry.kind() {
        let function_symbol = functor_entry.try_ident()?;
        if function_symbol == SymbolInterner::TOTAL_TIME_SYMBOL_ID
            && context.declared_requirements().contains(&NumericFluents)
        {
            return get_number_type();
        }
        return get_declaration_type(functor_index, symbol_table);
    }

    Err(SemanticError::unexpected_node_kind(
        functor_index,
        vec![AstKind::FunctionSymbol],
        functor_entry.kind(),
    ))
}
