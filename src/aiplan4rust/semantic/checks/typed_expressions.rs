use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::AssignOp;
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::Requirement::DurativeActions;
use crate::aiplan4rust::lang::Requirement::NumericFluents;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::{AstNode, AstKind};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode, NodeId};

/// Checks the type_checker correctness of typed expr in the syntax arena, including comparisons,
/// assignments, and arithmetic operations.
///
/// This function traverses the annotated syntax arena to verify that expr have compatible
/// types according to their operation kind. It supports:
/// - Equality checks (`=`) and simple assignments (`assign`), ensuring operand type_checker compatibility.
/// - Other comparisons (`>`, `<`, `>=`, `<=`) and arithmetic assignments (`+=`, `-=`, `*=`, `/=`),
///   ensuring operands are numeric or compatible.
///
/// Type compatibility checks are delegated to helper functions (e.g., `check_equal_and_assignment_expression`,
/// `check_numeric_expression`) and detailed errors are reported through the diagnostic manager.
///
/// # Parameters
/// - `ast_old`: The annotated syntax arena containing AST nodes and symbol information.
/// - `type_checker`: A `TypeChecker` instance used for type_checker resolution and compatibility validation.
/// - `source`: The diagnostic source context, indicating where diagnostics originate.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager for collecting errors.
///
/// # Returns
/// - `Ok(true)` if all typed expr are correct.
/// - `Ok(false)` if one or more type_checker mismatches were found and reported.
/// - `Err(ParserInternalError)` if an internal error occurred during processing.
///
/// # Example
/// ```rust
/// let result = check_typed_expressions(&ast_old, &type_checker, source, &mut diagnostic_manager)?;
/// if result {
///     println!("All typed expr are valid.");
/// }
/// ```
pub fn check_typed_expressions(
    context: &CheckContext,
    type_checker: &TypeChecker,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut no_error = true;

    for node in context.ast().preorder().values() {
        if is_equal_binary_comp(node) || is_assign(node) {
            let (ty1, ty2) = get_binary_operation_types(node, context)?;

            // Call check_equal_and_assign function to handle this case
            no_error &= check_equal_and_assignment_expression(
                context,
                type_checker,
                node,
                &ty1,
                &ty2,
                source,
                diagnostic_manager,
            )?;
        } else if is_numeric_expression(node) {
            let (ty1, ty2) = get_binary_operation_types(node, context)?;

            // Call check_other_cases function to handle these cases
            no_error &= check_numeric_expression(context, node, &ty1, &ty2, source, diagnostic_manager);
        }
    }

    Ok(no_error)
}

/// Returns `true` if the syntax represents an equality binary comparison (`BinaryComp::Equal`).
fn is_equal_binary_comp(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::FComp) && node.as_binary_comp() == Some(BinaryComp::Equal)
}

/// Returns `true` if the syntax represents a simple assignment (`AssignOp::Assign`).
fn is_assign(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::Assign) && node.as_assign_op() == Some(AssignOp::Assign)
}

/// Returns `true` if the syntax is a numeric comparison or a scale assignment expr.
///
/// This includes:
/// - Comparison operators: `Greater`, `GreaterEq`, `Less`, `LessEq`.
/// - Assignment operators: `ScaleUp`, `ScaleDown`, `Increase`, `Decrease`.
fn is_numeric_expression(node: &AstNode) -> bool {
    matches!(node.kind(), AstKind::FComp)
        && matches!(
            node.as_binary_comp(),
            Some(BinaryComp::Greater)
                | Some(BinaryComp::GreaterEq)
                | Some(BinaryComp::Less)
                | Some(BinaryComp::LessEq)
        )
        || matches!(node.kind(), AstKind::Assign)
        && matches!(
            node.as_assign_op(),
            Some(AssignOp::ScaleUp)
                | Some(AssignOp::ScaleDown)
                | Some(AssignOp::Increase)
                | Some(AssignOp::Decrease)
        )
}

/// Checks the type_checker compatibility of operands in equality (`=`) or assignment (`assign`) expr.
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
    ty1: &Type,
    ty2: &Type,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut no_error = true;

    if !type_checker.have_common_supertype(&ty1, &ty2)? {
        no_error = false;
        report_type_mismatch_in_expression(
            ty1,
            ty2,
            node.span().clone(),
            source.clone(),
            context,
            diagnostic_manager,
        );
    }

    Ok(no_error)
}

/// Reports a type_checker mismatch error for an expr involving two type_checker lists.
///
/// This function creates and adds a diagnostic error indicating that the two sets
/// of types involved in an expr are incompatible.
///
/// # Parameters
/// - `ty1`: The first type_checker list involved in the expr.
/// - `ty2`: The second type_checker list involved in the expr.
/// - `source`: The diagnostic source context indicating where diagnostics originate.
/// - `filename`: The filename where the error occurred.
/// - `span`: The span of the syntax syntax causing the error.
/// - `diagnostic_manager`: The diagnostic manager to which the error will be added.
fn report_type_mismatch_in_expression(
    ty1: &Type,
    ty2: &Type,
    span: Span,
    source: Provider,
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) {

    let interner = context.interner(); // Ou ajuster selon ton accès à l'interner

    let ty1: Vec<String> = ty1
        .iter()
        .map(|id| interner.try_resolve(*id).unwrap_or("<invalid>").to_string())
        .collect();

    let ty2: Vec<String> = ty2
        .iter()
        .map(|id| interner.try_resolve(*id).unwrap_or("<invalid>").to_string())
        .collect();

    let error = Diagnostic::new(
        DiagnosticKind::TypeMismatchInExpression {
            ty1,
            ty2,
        },
        source,
        context.source_name().to_string(),
        span,
    );
    diagnostic_manager.add_diagnostic(error);
}

/// Checks whether the operand types in a numeric comparison or assignment expr
/// are compatible with numeric operations (i.e., of type_checker `number`).
///
/// This function is used specifically for expr involving numeric comparisons
/// (e.g., `greater`, `less`, `>=`, `<=`) and numeric assignment operations
/// (e.g., `increase`, `decrease`, `scale-up`, `scale-down`). For such expr
/// to be valid, both operands must have the `number` type_checker.
///
/// If either operand does not have the `number` type_checker, the function logs a
/// type_checker mismatch error.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax arena containing the AST and metadata.
/// - `syntax`: The syntax syntax representing the numeric expr.
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
    ty1: &Type,
    ty2: &Type,
    source: Provider,
    diagnostic_manager:&mut DiagnosticManager
) -> bool {
    let mut no_error = true;

    // Handle Greater, Less, etc.
    if ty1 != Type::number() || ty2 != Type::number() {
        no_error = false;
        report_invalid_types_in_numeric_expression(
            ty1,
            ty2,
            node.span().clone(),
            source,
            context,
            diagnostic_manager,
        );
    }

    no_error
}

/// Reports a diagnostic error when numeric expr have invalid operand types.
///
/// This helper function creates and adds a diagnostic indicating that the operand types
/// in a numeric expr are invalid (i.e., not of type_checker `number`).
///
/// # Parameters
/// - `source`: The diagnostic source indicating where the error arises.
/// - `filename`: The filename where the error occurs.
/// - `span`: The span (location) in the source code for the error.
/// - `ty1`: The types of the left operand.
/// - `ty2`: The types of the right operand.
/// - `diagnostic_manager`: The diagnostic manager to which the error is added.
fn report_invalid_types_in_numeric_expression(
    ty1: &Type,
    ty2: &Type,
    span: Span,
    source: Provider,
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) {

    // Shoudl be removed when symbol table refactoring will be done
    let interner = context.interner(); // Ou ajuster selon ton accès à l'interner

    let ty1: Vec<String> = ty1
        .iter()
        .map(|id| interner.try_resolve(*id).unwrap_or("<invalid>").to_string())
        .collect();

    let ty2: Vec<String> = ty2
        .iter()
        .map(|id| interner.try_resolve(*id).unwrap_or("<invalid>").to_string())
        .collect();

    let error = Diagnostic::new(
        DiagnosticKind::InvalidTypesInNumericExpression { ty1, ty2 },
        source,
        context.source_name().to_string(),
        span,
    );
    diagnostic_manager.add_diagnostic(error);
}

/// Retrieves and returns the types of both operands in a binary expr.
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
) -> Result<(Type, Type), SemanticCheckError> {

    let ast = context.ast();

    // Try to get the first child node index and node
    let arg1_id = node.try_child(0)?;
    let arg1 = ast.try_node(arg1_id)?;

    // Try to get the second child node index and node
    let arg2_id = node.try_child(1)?;
    let arg2 = ast.try_node(arg2_id)?;

    // Get the type of the first operand, or return a specific error if missing
    let ty1 = get_type(arg1_id, arg1, context)?.ok_or_else(|| {
        SemanticCheckError::missing_operand_type(arg1_id, 0)
    })?;

    // Get the type of the second operand, or return a specific error if missing
    let ty2 = get_type(arg2_id, arg2, context)?.ok_or_else(|| {
        SemanticCheckError::missing_operand_type(arg2_id, 1)
    })?;

    Ok((ty1, ty2))
}


/// Determines the type_checker of a syntax syntax based on its kind.
///
/// This function supports several kinds of nodes: numbers, variables, constants,
/// and function terms. It delegates type_checker resolution to specialized helper functions
/// depending on the syntax kind. The function is used during type_checker checking to retrieve
/// the declared or inferred type_checker of an expr or symbol.
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
) -> Result<Option<Type>, SemanticCheckError> {
    match node.kind() {
        // Case 1: Directly a number -> Type is NUMBER_TYPE
        AstKind::Number => get_number_type(),

        // Case 2: Variable
        AstKind::Variable => get_variable_type(index, node.content().try_ident()?, context),

        // Case 3: Constant
        AstKind::Constant => get_constant_type(index, node.content().try_ident()?, context),

        // Case 4: Function Term
        AstKind::FunctionTerm => get_function_term_type(index, node, context),

        // Default case: Unexpected AST syntax kind
        found_kind => Err(SemanticCheckError::unexpected_ast_kind(
            AstKind::Number,
            found_kind,
            index,
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
fn get_number_type() -> Result<Option<Type>, SemanticCheckError> {
    Ok(Some(Type::number().clone()))
}

/// Retrieves the type_checker of a variable symbol from the symbol table.
///
/// This function resolves the type_checker of a variable used in the AST by consulting the
/// symbol table. If the variable is the special `DURATION_VARIABLE` and the domain
/// declares the `:durative-actions` requirement, the type_checker is directly inferred as
/// `number`. Otherwise, it delegates the lookup to `get_declaration_type`.
///
/// # Parameters
/// - `index`: The index of the AST syntax, used for error tracking.
/// - `symbol`: The name of the variable (e.g., `"?x"`).
/// - `annotated_syntax_tree`: A reference to the annotated syntax arena containing the
///   symbol table and domain requirements.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(types))`: A vector of type_checker names if the variable was successfully resolved.
/// - `Ok(Some(types))`: A vector of type_checker names if the variable was successfully resolved.
/// - `Ok(None)`: If the variable is declared but without a type_checker (unusual).
/// - `Err(ParserInternalError)`: If the variable has conflicting declarations or is undeclared.
///
/// # Special Case
/// - If the symbol is `?duration` and the domain has the `:durative-actions` requirement,
///   the function directly returns `Some(["number"])` as its type_checker.
///
/// # Example
/// ```rust
/// let ty = get_variable_type(42, "?x", &annotated_syntax_tree)?;
/// ```
fn get_variable_type(
    index: NodeId,
    symbol: Ident,
    context: &CheckContext,
) -> Result<Option<Type>, SemanticCheckError> {
    if symbol == StringInterner::IDENT_DURATION_VARIABLE && context.requirements().contains(&DurativeActions) {
        return get_number_type();
    }
    get_declaration_type(index, context)
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
    _symbol: Ident,
    context: &CheckContext,
) -> Result<Option<Type>, SemanticCheckError> {
    get_declaration_type(index, context)
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
    context: &CheckContext,
) -> Result<Option<Type>, SemanticCheckError> {
    match context.symbol_table().resolve_declaration_by_usage(node_id)? {
        Some(decl) => Ok(decl.types().cloned()), // Clone not necessary
        None => Ok(None),
    }
}

/// Helper to handle a `FunctionTerm` node and retrieve its type.
///
/// This function checks if the first child of the `FunctionTerm` AST node is a valid functor,
/// retrieves its corresponding AST entry, and determines the type associated with the function term.
/// It specifically handles the special case where the functor is the `TOTAL_TIME` symbol and
/// ensures the presence of the `NumericFluents` requirement before returning the number type.
///
/// If the functor is missing, invalid, or not of kind `FunctionSymbol`, an error is returned.
///
/// # Parameters
/// - `index`: The AST node ID of the `FunctionTerm`.
/// - `node`: Reference to the `FunctionTerm` AST node.
/// - `context`: Semantic checking context, providing access to the AST, symbol table, and requirements.
///
/// # Returns
/// - `Ok(Some(type))`: The type of the function term if determined successfully.
/// - `Ok(None)`: If the function term has no functor or no type could be inferred.
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
    index: NodeId,
    node: &AstNode,
    context: &CheckContext,
) -> Result<Option<Type>, SemanticCheckError> {
    let functor_index = node.try_child(0)?;
    let functor_entry = context.ast().try_node(functor_index)?;

    if let AstKind::FunctionSymbol = functor_entry.kind() {
        if functor_entry.try_ident()? == StringInterner::IDENT_TOTAL_TIME
            && context.requirements().contains(&NumericFluents)
        {
            return get_number_type();
        }
        return get_declaration_type(index, context);
    }

    Err(SemanticCheckError::unexpected_ast_kind(
        AstKind::FunctionSymbol,
        functor_entry.kind(),
        index,
    ))
}
