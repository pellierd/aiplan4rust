use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::AssignOp;
use crate::aiplan4rust::syntax::elements::BinaryComp;
use crate::aiplan4rust::syntax::elements::Requirement::DurativeActions;
use crate::aiplan4rust::syntax::elements::Requirement::NumericFluents;
use crate::aiplan4rust::syntax::lexer::token::DURATION_VARIABLE;
use crate::aiplan4rust::syntax::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::syntax::lexer::token::TOTAL_TIME;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::hir::HirNode;
use crate::aiplan4rust::semantic::hir::HirTree;

/// Checks the type correctness of typed expressions in the syntax tree, including comparisons,
/// assignments, and arithmetic operations.
///
/// This function traverses the annotated syntax tree to verify that expressions have compatible
/// types according to their operation kind. It supports:
/// - Equality checks (`=`) and simple assignments (`assign`), ensuring operand type compatibility.
/// - Other comparisons (`>`, `<`, `>=`, `<=`) and arithmetic assignments (`+=`, `-=`, `*=`, `/=`),
///   ensuring operands are numeric or compatible.
///
/// Type compatibility checks are delegated to helper functions (e.g., `check_equal_and_assignment_expression`,
/// `check_numeric_expression`) and detailed errors are reported through the diagnostic manager.
///
/// # Parameters
/// - `ast`: The annotated syntax tree containing AST nodes and symbol information.
/// - `type_checker`: A `TypeChecker` instance used for type resolution and compatibility validation.
/// - `source`: The diagnostic source context, indicating where diagnostics originate.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager for collecting errors.
///
/// # Returns
/// - `Ok(true)` if all typed expressions are correct.
/// - `Ok(false)` if one or more type mismatches were found and reported.
/// - `Err(ParserInternalError)` if an internal error occurred during processing.
///
/// # Example
/// ```rust
/// let result = check_typed_expressions(&ast, &type_checker, source, &mut diagnostic_manager)?;
/// if result {
///     println!("All typed expressions are valid.");
/// }
/// ```
pub fn check_typed_expressions(
    syntax_tree: &HirTree,
    type_checker: &TypeChecker,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    for node in syntax_tree.values() {
        match node.kind() {
            // Case for equality check (AssignOp::Assign and BinaryComp::Equal)
            AstKind::FComp(BinaryComp::Equal) | AstKind::Assign(AssignOp::Assign) => {
                let (ty1, ty2) = get_binary_operation_types(node, syntax_tree)?;

                // Call check_equal_and_assign function to handle this case
                no_error &= check_equal_and_assignment_expression(
                    syntax_tree,
                    type_checker,
                    node,
                    &ty1,
                    &ty2,
                    source,
                    diagnostic_manager,
                )?;
            }

            // Case for other comparison and assignment operations (Greater, Less, ScaleUp, etc.)
            AstKind::FComp(BinaryComp::Greater)
            | AstKind::FComp(BinaryComp::GreaterEq)
            | AstKind::FComp(BinaryComp::Less)
            | AstKind::FComp(BinaryComp::LessEq)
            | AstKind::Assign(AssignOp::ScaleUp)
            | AstKind::Assign(AssignOp::ScaleDown)
            | AstKind::Assign(AssignOp::Increase)
            | AstKind::Assign(AssignOp::Decrease) => {
                let (ty1, ty2) = get_binary_operation_types(node, syntax_tree)?;

                // Call check_other_cases function to handle these cases
                no_error &= check_numeric_expression(syntax_tree, node, &ty1, &ty2, source, diagnostic_manager)?;
            }

            _ => {}
        }
    }

    Ok(no_error)
}

/// Checks the type compatibility of operands in equality (`=`) or assignment (`assign`) expressions.
///
/// This function verifies that the types of both operands involved in an equality or assignment
/// operation are compatible. Equality comparisons (`=`) require operands of the same type,
/// while assignment operations (`assign`) may allow some flexibility depending on the domain,
/// such as assigning numeric values or specific user-defined types.
///
/// Type compatibility is checked using the provided `TypeChecker`. If a mismatch is found,
/// an error is reported through the `DiagnosticManager`.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax tree containing the AST and symbol information.
/// - `type_checker`: The type checker used to validate type compatibility.
/// - `node`: The syntax node representing the equality or assignment operation.
/// - `ty1`: The type(s) of the left-hand side operand.
/// - `ty2`: The type(s) of the right-hand side operand.
/// - `source`: The diagnostic source context indicating where diagnostics originate.
/// - `diagnostic_manager`: The diagnostic manager used to log any type mismatches.
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
///     &node,
///     &ty1,
///     &ty2,
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn check_equal_and_assignment_expression(
    annotated_syntax_tree: &HirTree,
    type_checker: &TypeChecker,
    node: &HirNode,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;
    if !type_checker.have_common_supertype(ty1, ty2)? {
        no_error = false;
        report_type_mismatch_in_expression(
            ty1,
            ty2,
            source.clone(),
            annotated_syntax_tree.filename(),
            node.span().clone(),
            diagnostic_manager,
        );
    }

    Ok(no_error)
}

/// Reports a type mismatch error for an expression involving two type lists.
///
/// This function creates and adds a diagnostic error indicating that the two sets
/// of types involved in an expression are incompatible.
///
/// # Parameters
/// - `ty1`: The first type list involved in the expression.
/// - `ty2`: The second type list involved in the expression.
/// - `source`: The diagnostic source context indicating where diagnostics originate.
/// - `filename`: The filename where the error occurred.
/// - `span`: The span of the syntax node causing the error.
/// - `diagnostic_manager`: The diagnostic manager to which the error will be added.
fn report_type_mismatch_in_expression(
    ty1: &[String],
    ty2: &[String],
    source: Provider,
    filename: &str,
    span: Span,
    diagnostic_manager: &mut DiagnosticManager,
) {
    let error = Diagnostic::new(
        DiagnosticKind::TypeMismatchInExpression {
            ty1: ty1.to_vec(),
            ty2: ty2.to_vec(),
        },
        source,
        filename.to_string(),
        span,
    );
    diagnostic_manager.add_diagnostic(error);
}

/// Checks whether the operand types in a numeric comparison or assignment expression
/// are compatible with numeric operations (i.e., of type `number`).
///
/// This function is used specifically for expressions involving numeric comparisons
/// (e.g., `greater`, `less`, `>=`, `<=`) and numeric assignment operations
/// (e.g., `increase`, `decrease`, `scale-up`, `scale-down`). For such expressions
/// to be valid, both operands must have the `number` type.
///
/// If either operand does not have the `number` type, the function logs a
/// type mismatch error.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax tree containing the AST and metadata.
/// - `node`: The syntax node representing the numeric expression.
/// - `ty1`: A reference to a vector of strings representing the type of the left operand.
/// - `ty2`: A reference to a vector of strings representing the type of the right operand.
/// - `source`: The diagnostic source indicating where this check is performed.
/// - `diagnostic_manager`: The error manager used to report type errors.
///
/// # Returns
/// - `Ok(true)` if both operands have the `number` type.
/// - `Ok(false)` if a type mismatch is found and an error is logged.
/// - `Err(ParserInternalError)` if an internal error occurs during type checking.
///
/// # Example
/// ```rust
/// let ty1 = vec!["number".to_string()];
/// let ty2 = vec!["number".to_string()];
/// let result = check_numeric_expression(
///     &annotated_syntax_tree,
///     &node,
///     &ty1,
///     &ty2,
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn check_numeric_expression(
    annotated_syntax_tree: &HirTree,
    node: &HirNode,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    source: Provider,
    diagnostic_manager:&mut DiagnosticManager
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    // Handle Greater, Less, etc.
    let number = vec![NUMBER_TYPE.to_string()];
    if ty1 != &number || ty2 != &number {
        no_error = false;
        report_invalid_types_in_numeric_expression(
            source,
            annotated_syntax_tree.filename(),
            node.span().clone(),
            ty1.clone(),
            ty2.clone(),
            diagnostic_manager,
        );
    }

    Ok(no_error)
}

/// Reports a diagnostic error when numeric expressions have invalid operand types.
///
/// This helper function creates and adds a diagnostic indicating that the operand types
/// in a numeric expression are invalid (i.e., not of type `number`).
///
/// # Parameters
/// - `source`: The diagnostic source indicating where the error arises.
/// - `filename`: The filename where the error occurs.
/// - `span`: The span (location) in the source code for the error.
/// - `ty1`: The types of the left operand.
/// - `ty2`: The types of the right operand.
/// - `diagnostic_manager`: The diagnostic manager to which the error is added.
fn report_invalid_types_in_numeric_expression(
    source: Provider,
    filename: &str,
    span: Span,
    ty1: Vec<String>,
    ty2: Vec<String>,
    diagnostic_manager: &mut DiagnosticManager,
) {
    let error = Diagnostic::new(
        DiagnosticKind::InvalidTypesInNumericExpression { ty1, ty2 },
        source,
        filename.to_string(),
        span,
    );
    diagnostic_manager.add_diagnostic(error);
}

/// Retrieves and returns the types of both operands in a binary expression.
///
/// This function ensures that the given syntax node represents a binary operation
/// with exactly two children. It then looks up the types of both operand nodes
/// using the annotated syntax tree and associated symbol table.
///
/// # Parameters
/// - `node`: The syntax node representing the binary operation.
/// - `annotated_syntax_tree`: The annotated syntax tree containing the full AST and symbol
///   information.
///
/// # Returns
/// A `Result` containing a pair of vectors of strings:
/// - The first vector represents the type(s) of the left operand.
/// - The second vector represents the type(s) of the right operand.
///
/// # Errors
/// This function returns a `ParserInternalError` in the following cases:
/// - The node does not have exactly two children (binary operations must have two).
/// - One of the children is missing in the syntax tree.
/// - One of the operands has no associated type in the symbol table.
///
/// # Example
/// ```rust
/// let (ty1, ty2) = get_binary_operation_types(&node, &annotated_syntax_tree)?;
/// ```
fn get_binary_operation_types(
    node: &HirNode,
    syntax_tree: &HirTree,
) -> Result<(Vec<String>, Vec<String>), ParserInternalError> {
    // Validate that there are exactly 2 children
    if node.children().len() != 2 {
        return Err(ParserInternalError::new(
            "Binary operations must have exactly two children.".to_string(),
        ));
    }

    let arg1 = syntax_tree
        .get_entry(node.children()[0])
        .ok_or_else(|| ParserInternalError::new("Missing first argument.".to_string()))?;
    let arg2 = syntax_tree
        .get_entry(node.children()[1])
        .ok_or_else(|| ParserInternalError::new("Missing second argument.".to_string()))?;

    let ty1 = get_type(node.children()[0], arg1, syntax_tree)?.ok_or_else(|| {
        ParserInternalError::new("No type declared for the first argument.".to_string())
    })?;
    let ty2 = get_type(node.children()[1], arg2, syntax_tree)?.ok_or_else(|| {
        ParserInternalError::new("No type declared for the second argument.".to_string())
    })?;

    Ok((ty1, ty2))
}

/// Determines the type of a syntax node based on its kind.
///
/// This function supports several kinds of nodes: numbers, variables, constants,
/// and function terms. It delegates type resolution to specialized helper functions
/// depending on the node kind. The function is used during type checking to retrieve
/// the declared or inferred type of an expression or symbol.
///
/// # Parameters
/// - `index`: The index of the current node in the syntax tree.
/// - `node`: A reference to the `HeapSyntaxNode` representing the AST node to analyze.
/// - `annotated_syntax_tree`: A reference to the annotated syntax tree that provides access
///   to both the symbol table and the full syntax structure.
///
/// # Returns
/// A `Result` containing:
/// - `Some(Vec<String>)` if the node has an associated type.
/// - `None` if the type is undefined but not erroneous (e.g., optional typing).
/// - `Err(ParserInternalError)` if the node kind is invalid or cannot be typed.
///
/// # Errors
/// - Returns an error if the node kind is not one of the expected kinds (`Number`, `Variable`,
///   `Constant`, `FunctionTerm`).
///
/// # Example
/// ```rust
/// let ty = get_type(index, &node, &annotated_syntax_tree)?;
/// ```
pub fn get_type(
    index: usize,
    node: &HirNode,
    annotated_syntax_tree: &HirTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    match node.kind() {
        // Case 1: Directly a number -> Type is NUMBER_TYPE
        AstKind::Number(_) => get_number_type(),

        // Case 2: Variable
        AstKind::Variable(symbol) => get_variable_type(index, symbol, annotated_syntax_tree),

        // Case 3: Constant
        AstKind::Constant(symbol) => get_constant_type(index, symbol, annotated_syntax_tree),

        // Case 4: Function Term
        AstKind::FunctionTerm => get_function_term_type(index, node, annotated_syntax_tree),

        // Default case: Unexpected AST node
        _ => Err(ParserInternalError::new(format!(
            "Unexpected AST node kind found: {}",
            node.kind()
        ))),
    }
}

/// Returns the predefined type for numeric values.
///
/// This helper function is used when an AST node represents a numeric literal.
/// It returns the predefined type associated with numbers (i.e., `NUMBER_TYPE`),
/// wrapped in a `Vec<String>` to be consistent with other type representations
/// in the type checking system.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(vec!["number"]))` if the type resolution is successful.
/// - `Err(ParserInternalError)` is not expected in this implementation, but
///   the return type remains consistent with other type-checking helpers.
///
/// # Example
/// ```rust
/// let ty = get_number_type()?; // Returns Some(["number".to_string()])
/// ```
fn get_number_type() -> Result<Option<Vec<String>>, ParserInternalError> {
    Ok(Some(vec![NUMBER_TYPE.to_string()]))
}

/// Retrieves the type of a variable symbol from the symbol table.
///
/// This function resolves the type of a variable used in the AST by consulting the
/// symbol table. If the variable is the special `DURATION_VARIABLE` and the domain
/// declares the `:durative-actions` requirement, the type is directly inferred as
/// `number`. Otherwise, it delegates the lookup to `get_declaration_type`.
///
/// # Parameters
/// - `index`: The index of the AST node, used for error tracking.
/// - `symbol`: The name of the variable (e.g., `"?x"`).
/// - `annotated_syntax_tree`: A reference to the annotated syntax tree containing the
///   symbol table and domain requirements.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(types))`: A vector of type names if the variable was successfully resolved.
/// - `Ok(Some(types))`: A vector of type names if the variable was successfully resolved.
/// - `Ok(None)`: If the variable is declared but without a type (unusual).
/// - `Err(ParserInternalError)`: If the variable has conflicting declarations or is undeclared.
///
/// # Special Case
/// - If the symbol is `?duration` and the domain has the `:durative-actions` requirement,
///   the function directly returns `Some(["number"])` as its type.
///
/// # Example
/// ```rust
/// let ty = get_variable_type(42, "?x", &annotated_syntax_tree)?;
/// ```
fn get_variable_type(
    index: usize,
    symbol: &str,
    annotated_syntax_tree: &HirTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    if symbol == DURATION_VARIABLE && annotated_syntax_tree.has_requirement(&DurativeActions) {
        return get_number_type();
    }
    get_declaration_type(index, annotated_syntax_tree.symbol_table())
}

/// Retrieves the type of a constant symbol from the symbol table.
///
/// This function resolves the type of a constant declared in the domain or problem file.
/// It delegates the actual lookup to `get_declaration_type`, which handles symbol table
/// access and conflict resolution.
///
/// # Parameters
/// - `index`: The index of the AST node, used for error reporting.
/// - `symbol`: The name of the constant (e.g., `"loc1"`).
/// - `annotated_syntax_tree`: A reference to the annotated syntax tree containing the
///   symbol table and other context.
///
/// # Returns
/// A `Result` containing:
/// - `Ok(Some(types))`: A vector of type names if the constant was successfully resolved.
/// - `Ok(None)`: If the constant exists but has no declared type.
/// - `Err(ParserInternalError)`: If the constant is not declared or declared inconsistently.
///
/// # Example
/// ```rust
/// let ty = get_constant_type(12, "loc1", &annotated_syntax_tree)?;
/// ```
fn get_constant_type(
    index: usize,
    _symbol: &str,
    annotated_syntax_tree: &HirTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    get_declaration_type(index, annotated_syntax_tree.symbol_table())
}

/// Helper function to retrieve the types associated with a symbol usage from the symbol table.
///
/// This function looks up a declaration corresponding to a usage identified by its AST index in the
/// symbol table. It returns the types declared for that symbol usage, if any. If multiple
/// declarations are found for the same usage, it returns an error to enforce uniqueness. If no
/// declaration is found for the usage, it returns `Ok(None)`.
///
/// # Parameters
/// - `index`: The AST index representing the usage of the symbol in the syntax tree.
/// - `symbol_table`: A reference to the symbol table containing symbol declarations and usages.
///
/// # Returns
/// Returns a `Result` containing:
/// - `Ok(Some(types))`: A vector of type names if a single declaration with types is found.
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
    index: usize,
    symbol_table: &SymbolTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    match symbol_table.resolve_declaration_by_usage(index)? {
        None => Ok(None),
        Some(decl) => Ok(decl.types().cloned()),
    }
}

/// Helper to handle `FunctionTerm` and retrieve its type.
///
/// This function checks if the first child of the `FunctionTerm` node is a valid functor,
/// retrieves its symbol, and determines the type associated with the function term.
/// Specifically, it handles the special case where the functor is a `TOTAL_TIME` symbol and
/// ensures the presence of the `NumericFluents` requirement for the `number` type.
/// If the functor is invalid or missing, an error is returned.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table. This is used for symbol lookup.
/// - `node`: A reference to the AST entry representing the function term to analyze.
/// - `annotated_syntax_tree`: A reference to the annotated syntax tree, providing access to the
///   syntax tree and symbol table.
///
/// # Returns
/// This function returns a `Result` containing:
/// - `Ok(Some(types))`: A vector of type names if the functor is valid, and its type is determined.
/// - `Ok(None)`: If the function term has no functor, or no type is declared for it.
/// - `Err(ParserInternalError)`: If the functor is missing, invalid, or the child is not a
///  `FunctionSymbol`.
///
/// # Example
/// ```rust
/// let ty = get_function_term_type(10, &node, &annotated_syntax_tree)?;
/// ```
fn get_function_term_type(
    index: usize,
    node: &HirNode,
    syntax_tree: &HirTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    let children = node.children();
    if children.is_empty() {
        return Err(ParserInternalError::new(
            "Function term has no functor (empty children).".to_string(),
        ));
    }

    let functor_index = children[0];
    let functor_entry = syntax_tree.get_entry(functor_index).ok_or_else(|| {
        ParserInternalError::new(format!("No AST entry found for index {}.", functor_index))
    })?;

    if let AstKind::FunctionSymbol(symbol) = functor_entry.kind() {
        if symbol == TOTAL_TIME && syntax_tree.has_requirement(&NumericFluents) {
            return get_number_type();
        }
        return get_declaration_type(index, syntax_tree.symbol_table());
    }

    Err(ParserInternalError::new(
        "First child of function term is not a FunctionSymbol.".to_string(),
    ))
}
