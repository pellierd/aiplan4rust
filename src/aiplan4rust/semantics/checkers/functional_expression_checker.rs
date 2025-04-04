use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::ast_table::{AstEntry, AstTable};
use crate::aiplan4rust::semantics::checkers::TypeChecker;
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::ast::Requirement::{DurativeActions, NumericFluents};
use crate::aiplan4rust::syntax::ast::{AssignOp, AstKind, BinaryComp};
use crate::aiplan4rust::syntax::token::{DURATION_VARIABLE, NUMBER_TYPE, TOTAL_TIME};

/// Verifies the types of expressions used in function calls and assignment operations.
///
/// This function checks the types of expressions in the abstract syntax tree (AST) to ensure
/// that they are compatible for their respective operations, specifically focusing on equality
/// checks, assignment operations, and other comparisons and assignments. The function processes
/// the following cases:
///
/// - Equality check (`=`) and assignment (`assign`): Verifies that the types of the operands
///   match.
/// - Other comparisons (greater than, less than, etc.) and assignments (scale up, scale down,
///   etc.): Verifies that both operands are numeric or compatible for the operation.
///
/// The function uses `AtomicExpressionChecker` to validate type compatibility for each
/// operation and logs errors  if any type mismatches are found.
///
/// # Parameters
/// - `symbol_table`: A reference to the `SymbolTable` used to resolve variable types.
/// - `ast_table`: A reference to the `AstTable` that contains the abstract syntax tree entries.
///
/// # Returns
/// - `Ok(true)` if no type mismatches were found for the expressions.
/// - `Ok(false)` if one or more type mismatches were found, and errors were logged.
/// - `Err(ParserInternalError)` if an internal error occurs while processing the AST.
///
/// # Example
/// ```rust
/// let symbol_table = ...;
/// let ast_table = ...;
/// let result = check(&symbol_table, &ast_table);
/// ```
pub fn check(
    tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    let ast_table = tree.ast();
    let symbol_table = tree.symbol_table();

    for ast in ast_table.values() {
        match ast.kind() {
            // Case for equality check (AssignOp::Assign and BinaryComp::Equal)
            AstKind::FComp(BinaryComp::Equal) | AstKind::Assign(AssignOp::Assign) => {
                let (ty1, ty2) = get_binary_operation_types(ast, symbol_table, ast_table)?;

                // Call check_equal_and_assign function to handle this case
                no_error &= check_equal_and_assignment_expression(
                    tree,
                    type_checker,
                    ast,
                    &ty1,
                    &ty2,
                    errors,
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
                let (ty1, ty2) = get_binary_operation_types(ast, symbol_table, ast_table)?;

                // Call check_other_cases function to handle these cases
                no_error &= check_numeric_expression(tree, ast, &ty1, &ty2, errors)?;
            }

            _ => {}
        }
    }

    Ok(no_error)
}

/// Verifies that the types of the operands in an equality (`=`) or assignment (`assign`)
/// expression are compatible.
///
/// This function handles both equality (`=`) and assignment (`assign`) operations. The types of
/// the left and right operands must be compatible for the expression to be valid. While
/// equality (`=`) typically involves comparing operands of the same type, assignments
/// (`assign`) can involve various types depending on the PDDL domain, including numbers, or
/// other user-defined types.
///
/// # Parameters
/// - `ast`: A reference to the `AstEntry` representing the expression to check.
/// - `symbol_table`: A reference to the `SymbolTable` used to resolve type information.
/// - `ty1`: A reference to a vector of strings representing the type of the first operand.
/// - `ty2`: A reference to a vector of strings representing the type of the second operand.
///
/// # Returns
/// Returns a `Result<bool, ParserInternalError>`.
/// - `Ok(true)` if the types are compatible (i.e., the left and right operands match).
/// - `Ok(false)` if there is a type incompatibility, and an error is logged.
/// - `Err` if there is an internal parsing error.
///
/// # Example
/// ```rust
/// let ast_entry = ...;
/// let ty1 = vec!["object".to_string()]; // An object type in PDDL
/// let ty2 = vec!["object".to_string()]; // Another object type in PDDL
/// let result = check_equal_and_assignment_expression(&ast_entry, &symbol_table, &ty1, &ty2);
/// ```
fn check_equal_and_assignment_expression(
    tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    ast: &AstEntry,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    if !type_checker.match_type(ty1, ty2)? {
        no_error = false;
        let (line, column) = ast.span().start_position();
        let content = format!("Type incompatibility in expression {}: ", ast);
        let error = ParsingError::new(
            ParserErrorKind::ParseError,
            Some(tree.filename().clone()),
            line,
            column,
            content,
        );
        errors.add_error(error);
    }

    Ok(no_error)
}

/// Verifies that the types of the operands in a numeric comparison or assignment expression
/// are compatible with the numeric type (i.e., `number`).
///
/// This function is specifically for handling operations like Greater, Less, etc., where the
/// operands must be of the numeric type.
///
/// # Parameters
/// - `ast`: A reference to the `AstEntry` representing the expression to check.
/// - `ty1`: A reference to a vector of strings representing the types of the first operand.
/// - `ty2`: A reference to a vector of strings representing the types of the second operand.
///
/// # Returns
/// Returns a `Result<bool, ParserInternalError>`.
/// - `Ok(true)` if the types are compatible with numeric operations (`number`).
/// - `Ok(false)` if there is a type incompatibility, and an error is logged.
/// - `Err` if there is an internal parsing error.
///
/// # Example
/// ```rust
/// let ast_entry = ...;
/// let ty1 = vec!["number".to_string()];
/// let ty2 = vec!["number".to_string()];
/// let result = check_numeric_expression(&ast_entry, &ty1, &ty2);
/// ```
fn check_numeric_expression(
    tree: &AnnotatedSyntaxTree,
    ast: &AstEntry,
    ty1: &Vec<String>,
    ty2: &Vec<String>,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    // Handle Greater, Less, etc.
    let number = vec![NUMBER_TYPE.to_string()];
    if ty1 != &number || ty2 != &number {
        no_error = false;
        let (line, column) = ast.span().start_position();
        let content = format!("Type incompatibility in expression {}: ", ast);
        let error = ParsingError::new(
            ParserErrorKind::ParseError,
            Some(tree.filename().clone()),
            line,
            column,
            content,
        );
        errors.add_error(error);
    }

    Ok(no_error)
}

/// Retrieves the types of the two operands involved in a binary operation.
///
/// This function is designed to validate that the given abstract syntax tree (AST) entry
/// represents a binary operation with exactly two children. It then retrieves the types
/// of both operands (the children) by consulting the provided symbol table and AST table.
/// If any issues arise, such as missing children, undeclared types, or incorrect numbers
/// of children, an error is returned.
///
/// # Arguments
///
/// * `ast` - A reference to the `AstEntry` representing the binary operation in the AST.
/// * `symbol_table` - A reference to the `SymbolTable` that holds the variable types for the
///   scope.
/// * `ast_table` - A reference to the `AstTable` that holds the full set of AST entries.
///
/// # Returns
///
/// This function returns a `Result` containing a tuple of two `Vec<String>` values representing
/// the types of the two operands if successful. The types are derived from the symbol table and
/// AST table based on the respective operands' positions in the AST. If an error occurs, a
/// `ParserInternalError` is returned.
///
/// # Errors
///
/// The function may return an error in the following cases:
/// - If the `ast` entry does not have exactly two children, a `ParserInternalError` is returned
///   with the message "Binary operations must have exactly two children."
/// - If either of the two operands is missing from the `ast_table`, a `ParserInternalError`
///   with the message "Missing first argument." or "Missing second argument." will be returned
///   accordingly.
/// - If either of the two operands does not have a declared type in the symbol table, a
///   `ParserInternalError` will be returned with the message "No type declared for the first
///   argument." or "No type declared for the second argument."
///
/// # Example
/// ```rust
/// let (ty1, ty2) = get_binary_operation_types(&ast, &symbol_table, &ast_table)?;
/// ```
fn get_binary_operation_types(
    ast: &AstEntry,
    symbol_table: &SymbolTable,
    ast_table: &AstTable,
) -> Result<(Vec<String>, Vec<String>), ParserInternalError> {
    // Validate that there are exactly 2 children
    if ast.children().len() != 2 {
        return Err(ParserInternalError::new(
            "Binary operations must have exactly two children.".to_string(),
        ));
    }

    let arg1 = ast_table
        .get_entry(ast.children()[0])
        .ok_or_else(|| ParserInternalError::new("Missing first argument.".to_string()))?;
    let arg2 = ast_table
        .get_entry(ast.children()[1])
        .ok_or_else(|| ParserInternalError::new("Missing second argument.".to_string()))?;

    let ty1 = get_type(ast.children()[0], arg1, symbol_table, ast_table)?.ok_or_else(|| {
        ParserInternalError::new("No type declared for the first argument.".to_string())
    })?;
    let ty2 = get_type(ast.children()[1], arg2, symbol_table, ast_table)?.ok_or_else(|| {
        ParserInternalError::new("No type declared for the second argument.".to_string())
    })?;

    Ok((ty1, ty2))
}

/// Retrieves the type of an AST node based on its kind.
///
/// This function handles several AST node types, including numbers, variables, constants,
/// and function terms. It delegates the actual type retrieval to specific helper functions
/// for each type of AST node.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table.
/// - `ast`: A reference to the AST entry to analyze.
/// - `symbol_table`: A reference to the symbol table.
/// - `ast_table`: A reference to the AST table.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>`, which represents the type of
/// the AST node, or an error if the node type is unexpected.
///
/// # Errors
/// Returns a `ParserInternalError` if the AST node kind is not one of the expected types.
pub fn get_type(
    index: usize,
    ast: &AstEntry,
    symbol_table: &SymbolTable,
    ast_table: &AstTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    match ast.kind() {
        // Case 1: Directly a number -> Type is NUMBER_TYPE
        AstKind::Number(_) => get_number_type(),

        // Case 2: Variable
        AstKind::Variable(symbol) => get_variable_type(index, symbol, symbol_table, ast_table),

        // Case 3: Constant
        AstKind::Constant(symbol) => get_constant_type(index, symbol, symbol_table),

        // Case 4: Function Term
        AstKind::FunctionTerm => get_function_term_type(index, ast, symbol_table, ast_table),

        // Default case: Unexpected AST node
        _ => Err(ParserInternalError::new(format!(
            "Unexpected AST node kind found: {}",
            ast.kind()
        ))),
    }
}

/// Helper to return a type `NUMBER_TYPE`.
///
/// This function returns the type for a number, which is predefined as `NUMBER_TYPE`.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>`, with a single element `NUMBER_TYPE`.
fn get_number_type() -> Result<Option<Vec<String>>, ParserInternalError> {
    Ok(Some(vec![NUMBER_TYPE.to_string()]))
}

/// Helper to retrieve the type of a variable.
///
/// This function handles the special case of a `DURATION_VARIABLE` and delegates to
/// `get_declaration_type` for other variables.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table.
/// - `symbol`: The name of the variable symbol.
/// - `symbol_table`: A reference to the symbol table.
/// - `ast_table`: A reference to the AST table.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the variable,
/// or an error if the variable has multiple declarations.
fn get_variable_type(
    index: usize,
    symbol: &str,
    symbol_table: &SymbolTable,
    ast_table: &AstTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    if symbol == DURATION_VARIABLE && ast_table.requirements().contains(&DurativeActions) {
        return get_number_type();
    }
    get_declaration_type(index, symbol, symbol_table)
}

/// Helper to retrieve the type of a constant.
///
/// This function delegates to `get_declaration_type` to retrieve the type of a constant.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table.
/// - `symbol`: The name of the constant symbol.
/// - `symbol_table`: A reference to the symbol table.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the constant,
/// or an error if the constant has multiple declarations.
fn get_constant_type(
    index: usize,
    symbol: &str,
    symbol_table: &SymbolTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    get_declaration_type(index, symbol, symbol_table)
}

/// Helper function to retrieve the type of a symbol from the symbol table.
///
/// This function looks up a symbol in the symbol table and returns its associated type.
/// If the symbol has multiple declarations, it returns an error.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table.
/// - `symbol`: The name of the symbol.
/// - `symbol_table`: A reference to the symbol table.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>` representing the symbol's type,
/// or an error if the symbol has multiple declarations.
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
/// This function checks if the `FunctionTerm` has a valid functor and determines
/// the type based on the associated function symbol.
///
/// # Parameters
/// - `index`: The index of the symbol in the symbol table.
/// - `ast`: A reference to the AST entry representing the function term.
/// - `symbol_table`: A reference to the symbol table.
/// - `ast_table`: A reference to the AST table.
///
/// # Returns
/// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the
/// function term, or an error if the functor is invalid or the term has no functor.
fn get_function_term_type(
    index: usize,
    ast: &AstEntry,
    symbol_table: &SymbolTable,
    ast_table: &AstTable,
) -> Result<Option<Vec<String>>, ParserInternalError> {
    let children = ast.children();
    if children.is_empty() {
        return Err(ParserInternalError::new(
            "Function term has no functor (empty children).".to_string(),
        ));
    }

    let functor_index = children[0];
    let functor_entry = ast_table.get_entry(functor_index).ok_or_else(|| {
        ParserInternalError::new(format!("No AST entry found for index {}.", functor_index))
    })?;

    if let AstKind::FunctionSymbol(symbol) = functor_entry.kind() {
        if symbol == TOTAL_TIME && ast_table.requirements().contains(&NumericFluents) {
            return get_number_type();
        }
        return get_declaration_type(index, symbol, symbol_table);
    }

    Err(ParserInternalError::new(
        "First child of function term is not a FunctionSymbol.".to_string(),
    ))
}
