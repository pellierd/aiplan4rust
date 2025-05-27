use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::AssignOp;
use crate::aiplan4rust::parser::elements::BinaryComp;
use crate::aiplan4rust::parser::elements::Requirement::DurativeActions;
use crate::aiplan4rust::parser::elements::Requirement::NumericFluents;
use crate::aiplan4rust::parser::lexer::token::DURATION_VARIABLE;
use crate::aiplan4rust::parser::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::parser::lexer::token::TOTAL_TIME;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::checkers::TypeChecker;
use crate::aiplan4rust::semantic_analyser::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxNode;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

/// Verifies the type correctness of expressions in function calls and assignment operations.
///
/// This function traverses the abstract syntax tree (AST) to check that expression types are
/// valid for specific operations, including equality comparisons, assignments, and numerical
/// operations. It performs type checking for the following cases:
///
/// - **Equality checks** (`=`) and **simple assignments** (`assign`): Ensures that the operand
///   types are compatible.
/// - **Other comparisons** (`>`, `<`, `>=`, `<=`) and **arithmetic assignments** (`+=`, `-=`, `*=`,
///   `/=`):
///   Ensures that the operands are numeric or otherwise compatible with the operation.
///
/// The function delegates type compatibility checks to `AtomicExpressionChecker` and reports
/// any mismatches via the provided `ErrorManager`.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax tree containing both the AST and symbol table.
/// - `type_checker`: The `TypeChecker` instance used to perform type resolution and compatibility
///   checks.
/// - `errors`: A mutable reference to the `ErrorManager` for collecting any detected errors.
///
/// # Returns
/// - `Ok(true)` if no type mismatches are found.
/// - `Ok(false)` if one or more type mismatches are found (errors are logged).
/// - `Err(ParserInternalError)` if an internal error occurs during AST processing.
///
/// # Example
/// ```rust
/// let result = check(&annotated_syntax_tree, &type_checker, &mut error_manager)?;
/// if result {
///     println!("All expressions are type correct.");
/// }
/// ```

pub fn check(
    syntax_tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    for node in syntax_tree.values() {
        match node.kind() {
            // Case for equality check (AssignOp::Assign and BinaryComp::Equal)
            SyntaxNodeKind::FComp(BinaryComp::Equal) | SyntaxNodeKind::Assign(AssignOp::Assign) => {
                let (ty1, ty2) = get_binary_operation_types(node, syntax_tree)?;

                // Call check_equal_and_assign function to handle this case
                no_error &= check_equal_and_assignment_expression(
                    syntax_tree,
                    type_checker,
                    node,
                    &ty1,
                    &ty2,
                    diagnostic_manager,
                )?;
            }

            // Case for other comparison and assignment operations (Greater, Less, ScaleUp, etc.)
            SyntaxNodeKind::FComp(BinaryComp::Greater)
            | SyntaxNodeKind::FComp(BinaryComp::GreaterEq)
            | SyntaxNodeKind::FComp(BinaryComp::Less)
            | SyntaxNodeKind::FComp(BinaryComp::LessEq)
            | SyntaxNodeKind::Assign(AssignOp::ScaleUp)
            | SyntaxNodeKind::Assign(AssignOp::ScaleDown)
            | SyntaxNodeKind::Assign(AssignOp::Increase)
            | SyntaxNodeKind::Assign(AssignOp::Decrease) => {
                let (ty1, ty2) = get_binary_operation_types(node, syntax_tree)?;

                // Call check_other_cases function to handle these cases
                no_error &= check_numeric_expression(syntax_tree, node, &ty1, &ty2, diagnostic_manager)?;
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
/// an error is reported through the `ErrorManager`.
///
/// # Parameters
/// - `annotated_syntax_tree`: The annotated syntax tree containing the AST and symbol information.
/// - `type_checker`: The type checker used to validate type compatibility.
/// - `node`: The syntax node representing the equality or assignment operation.
/// - `ty1`: The type(s) of the left-hand side operand.
/// - `ty2`: The type(s) of the right-hand side operand.
/// - `errors`: The error manager used to log any type mismatches.
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
///     &mut errors,
/// );
/// ```

fn check_equal_and_assignment_expression(
    annotated_syntax_tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    node: &AnnotatedSyntaxNode,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;
    if !type_checker.have_common_supertype(ty1, ty2)? {
        no_error = false;
        let error = Diagnostic::new(
            DiagnosticKind::TypeMismatchInExpression { ty1: ty1.clone(), ty2 : ty2.clone() },
            DiagnosticSource::SemanticAnalyzer,
            annotated_syntax_tree.filename().clone(),
            node.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
    }

    Ok(no_error)
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
/// - `errors`: The error manager used to report type errors.
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
/// let result = check_numeric_expression(&annotated_syntax_tree, &node, &ty1, &ty2, &mut errors);
/// ```

fn check_numeric_expression(
    annotated_syntax_tree: &AnnotatedSyntaxTree,
    node: &AnnotatedSyntaxNode,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    diagnostic_manager:&mut DiagnosticManager
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    // Handle Greater, Less, etc.
    let number = vec![NUMBER_TYPE.to_string()];
    if ty1 != &number || ty2 != &number {
        no_error = false;
        let error = Diagnostic::new(
            DiagnosticKind::InvalidTypesInNumericExpression { ty1: ty1.clone(), ty2 : ty2.clone() },
            DiagnosticSource::SemanticAnalyzer,
            annotated_syntax_tree.filename().clone(),
            node.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
    }

    Ok(no_error)
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
    node: &AnnotatedSyntaxNode,
    syntax_tree: &AnnotatedSyntaxTree,
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
    node: &AnnotatedSyntaxNode,
    annotated_syntax_tree: &AnnotatedSyntaxTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    match node.kind() {
        // Case 1: Directly a number -> Type is NUMBER_TYPE
        SyntaxNodeKind::Number(_) => get_number_type(),

        // Case 2: Variable
        SyntaxNodeKind::Variable(symbol) => get_variable_type(index, symbol, annotated_syntax_tree),

        // Case 3: Constant
        SyntaxNodeKind::Constant(symbol) => get_constant_type(index, symbol, annotated_syntax_tree),

        // Case 4: Function Term
        SyntaxNodeKind::FunctionTerm => get_function_term_type(index, node, annotated_syntax_tree),

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
    annotated_syntax_tree: &AnnotatedSyntaxTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    if symbol == DURATION_VARIABLE && annotated_syntax_tree.has_requirement(&DurativeActions) {
        return get_number_type();
    }
    get_declaration_type(index, symbol, annotated_syntax_tree.symbol_table())
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
    symbol: &str,
    annotated_syntax_tree: &AnnotatedSyntaxTree,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    get_declaration_type(index, symbol, annotated_syntax_tree.symbol_table())
}

/// Helper function to retrieve the type of a symbol from the symbol table.
///
/// This function looks up a symbol in the symbol table using its index and returns its associated
/// type. If the symbol has multiple declarations, it raises an error to ensure that the symbol is
/// declared only once. It accesses the symbol's type through the `declarations` and handles cases
/// where there are no types declared for the symbol.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table. This is used to retrieve the correct
///   declaration.
/// - `symbol`: The name of the symbol being looked up.
/// - `symbol_table`: A reference to the symbol table, which holds all symbol declarations and their
///   associated types.
///
/// # Returns
/// This function returns a `Result` containing:
/// - `Ok(Some(types))`: A vector of type names if the symbol has a single declaration with types.
/// - `Ok(None)`: If the symbol exists but does not have any declared types.
/// - `Err(ParserInternalError)`: If the symbol has multiple declarations, an error is returned
///   indicating a conflict.
///
/// # Example
/// ```rust
/// let ty = get_declaration_type(10, "varX", &symbol_table)?;
/// ```
fn get_declaration_type(
    index: usize,
    symbol: &str,
    symbol_table: &SymbolTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    let declarations = symbol_table.get_declaration_by_usage(index)?;
    if declarations.len() > 1 {
        return Err(ParserInternalError::new(format!(
            "Symbol '{}' has multiple declarations.",
            symbol
        )));
    }
    Ok(declarations
        .get(0)
        .map(|decl| decl.types().cloned())
        .unwrap_or(None))
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
    node: &AnnotatedSyntaxNode,
    syntax_tree: &AnnotatedSyntaxTree,
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

    if let SyntaxNodeKind::FunctionSymbol(symbol) = functor_entry.kind() {
        if symbol == TOTAL_TIME && syntax_tree.has_requirement(&NumericFluents) {
            return get_number_type();
        }
        return get_declaration_type(index, symbol, syntax_tree.symbol_table());
    }

    Err(ParserInternalError::new(
        "First child of function term is not a FunctionSymbol.".to_string(),
    ))
}
