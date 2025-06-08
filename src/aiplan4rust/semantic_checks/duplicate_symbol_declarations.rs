use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::SymbolTable;

use crate::aiplan4rust::parser::SymbolOrigin;
use crate::aiplan4rust::semantic_checks::Checker;

/// Checks for conflicting symbol declarations between the problem and domain syntax trees.
///
/// This function verifies that no symbol declared in the problem conflicts with
/// existing declarations in the domain. Specifically, it checks for symbols
/// declared in the problem that have the same name as symbols declared in the domain,
/// but differ in kind. If such conflicts are found, diagnostic errors are reported.
///
/// # Parameters
///
/// - `domain`: Reference to the domain's annotated syntax tree, containing its symbol table.
/// - `problem`: Reference to the problem's annotated syntax tree, containing its symbol table.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where conflicts will be
///   recorded.
/// - `context`: The checker context indicating the source of diagnostics.
///
/// # Returns
///
/// Returns `Ok(true)` if no conflicts were found between the problem and domain declarations.
/// Returns `Ok(false)` if conflicts were detected and reported.
/// Returns `Err(ParserInternalError)` if an internal error occurs during the check process (e.g.,
///   missing AST entry).
///
/// # Behavior
///
/// The function iterates over all symbols declared in the problem's symbol table.
/// For each declaration originating from the problem (and not exempt from checks),
/// it verifies whether the domain declares symbols with the same name.
/// If the domain declares symbols of a different kind than the problem's declaration,
/// a conflict diagnostic is generated and added to the diagnostic manager.
///
/// # Steps
///
/// 1. Initialize a success flag.
/// 2. Retrieve symbol tables from both domain and problem.
/// 3. Iterate over all problem symbols and their declarations.
/// 4. Skip declarations exempt from conflict checks or not from the problem source.
/// 5. Check if relevant domain declarations exist for the symbol.
/// 6. Collect the kinds of relevant domain declarations.
/// 7. Determine if the problem declaration kind matches any domain kind.
/// 8. If no match, report a conflict error.
/// 9. Update the success flag accordingly.
/// 10. Return the overall success result.
///
/// # Example
///
/// ```rust
/// let result = check_cross_duplicate_symbol_declarations(&domain_ast, &problem_ast, &mut diag_manager, context);
/// match result {
///     Ok(true) => println!("No conflicts found."),
///     Ok(false) => println!("Conflicting declarations detected."),
///     Err(e) => eprintln!("Internal error: {}", e),
/// }
/// ```
pub fn check_cross_duplicate_symbol_declarations(
    domain: &AnnotatedSyntaxTree,
    problem: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
    context: Checker,
) -> Result<bool, ParserInternalError> {
    // Step 1: Initialize a flag to track overall success of the check.
    let mut all_ok = true;

    // Step 2: Retrieve symbol tables from domain and problem.
    let domain_symbol_table = domain.symbol_table();
    let problem_symbol_table = problem.symbol_table();

    // Step 3: Iterate over all symbols declared in the problem.
    for symbol in problem_symbol_table.values() {
        // Step 4: Iterate over each declaration of the symbol.
        for declaration in symbol.declarations() {
            // Step 5: Skip declarations exempt from conflict check or not from problem source.
            if !is_declaration_exempt_from_conflict_check(declaration)
                && *declaration.source() == SymbolOrigin::Problem
            {
                // Step 6: Check if there are relevant domain declarations for the symbol.
                if has_relevant_domain_declarations(domain_symbol_table, symbol.name()) {
                    // Step 7: Get the kinds of the relevant domain declarations.
                    let domain_kinds: Vec<SymbolKind> = get_relevant_domain_kinds(domain_symbol_table, symbol.name());

                    // Step 8: Check if the kind of the problem declaration exists in the domain kinds.
                    let same_kind_exists = domain_kinds.iter().any(|k| k == declaration.kind());

                    // Step 9: If no matching kind found, report a conflict error.
                    if !same_kind_exists {
                        report_cross_conflict_symbol_error(
                            diagnostic_manager,
                            declaration,
                            domain_kinds,
                            problem,
                            context
                        )?;

                        // Step 10: Mark the overall check as failed.
                        all_ok = false;
                    }
                }
            }
        }
    }

    // Step 11: Return whether all checks passed (true) or conflicts were found (false).
    Ok(all_ok)
}


/// Checks if there are any relevant domain declarations for the given symbol name
/// in the domain's symbol table, excluding declarations exempt from conflict checks.
///
/// This function queries the domain symbol table for declarations matching the
/// specified symbol name within the root scope. It returns `true` if there exists
/// at least one declaration that is not exempt from conflict checking (e.g., not
/// special symbol kinds like DomainName or ProblemName).
///
/// # Arguments
///
/// * `domain_symbol_table` - Reference to the domain's `SymbolTable` containing all symbols.
/// * `symbol_name` - The name of the symbol to check for relevant declarations.
///
/// # Returns
///
/// Returns `true` if there is at least one relevant declaration for the symbol in the domain,
/// otherwise returns `false`.
///
/// # Example
///
/// ```rust
/// if has_relevant_domain_declarations(&domain_symbol_table, "my_symbol") {
///     println!("Relevant declarations found for 'my_symbol'");
/// } else {
///     println!("No relevant declarations for 'my_symbol'");
/// }
/// ```
fn has_relevant_domain_declarations(
    domain_symbol_table: &SymbolTable,
    symbol_name: &str
) -> bool {
    domain_symbol_table
        .collect_declarations(Some(symbol_name), None, Some(&Scope::root()))
        .into_iter()
        .any(|d| !is_declaration_exempt_from_conflict_check(&d))
}

/// Retrieves the kinds of all relevant declarations for a given symbol name
/// from the domain's symbol table, excluding those exempt from conflict checks.
///
/// This function queries the domain symbol table for declarations matching the
/// provided symbol name within the root scope. It filters out declarations
/// that are exempt from conflict checking (e.g., certain special symbol kinds)
/// and collects the kinds (`SymbolKind`) of the remaining declarations into a vector.
///
/// # Arguments
///
/// * `domain_symbol_table` - Reference to the domain's `SymbolTable` containing all symbols.
/// * `symbol_name` - Reference to the `String` name of the symbol to query.
///
/// # Returns
///
/// Returns a vector of `SymbolKind` representing the kinds of relevant domain
/// declarations for the specified symbol name.
///
/// # Example
///
/// ```rust
/// let domain_kinds = get_relevant_domain_kinds(&domain_symbol_table, &"my_symbol".to_string());
/// for kind in domain_kinds {
///     println!("Relevant domain kind: {:?}", kind);
/// }
/// ```
fn get_relevant_domain_kinds(
    domain_symbol_table: &SymbolTable,
    symbol_name: &String,
) -> Vec<SymbolKind> {
    domain_symbol_table
        .collect_declarations(Some(symbol_name), None, Some(&Scope::root()))
        .into_iter()
        .filter(|d| !is_declaration_exempt_from_conflict_check(d))
        .map(|d| d.kind().clone())
        .collect()
}

/// Reports a conflict error when a symbol declared in the problem syntax tree
/// conflicts with existing declarations in the domain syntax tree.
///
/// This function creates and records a diagnostic error indicating that the
/// symbol from the problem declaration conflicts with one or more declarations
/// in the domain, specifying the conflicting symbol name, the kind of the
/// problem declaration, and the kinds of the domain declarations.
///
/// The diagnostic includes location and context information extracted from the
/// problem declaration and syntax tree.
///
/// # Parameters
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where the
///   error will be recorded.
/// - `problem_declaration`: The `Declaration` in the problem syntax tree that
///   conflicts with existing domain declarations.
/// - `domain_kinds`: A vector of `SymbolKind` representing the conflicting
///   declaration kinds found in the domain syntax tree.
/// - `problem_syntax_tree`: Reference to the problem's annotated syntax tree,
///   used to retrieve filename and span information.
/// - `context`: The `CheckerContext` indicating where in the compilation/analysis
///   pipeline this diagnostic arises.
///
/// # Returns
/// - `Ok(())` if the diagnostic was successfully created and added.
/// - `Err(ParserInternalError)` if an unexpected error occurs (currently none expected).
///
/// # Example
/// ```rust
/// report_cross_conflict_symbol_error(
///     &mut diagnostic_manager,
///     &problem_declaration,
///     domain_kinds,
///     &problem_syntax_tree,
///     CheckerContext::Linker,
/// )?;
/// ```
pub fn report_cross_conflict_symbol_error(
    diagnostic_manager: &mut DiagnosticManager,
    problem_declaration: &Declaration,
    domain_kinds: Vec<SymbolKind>,
    problem_syntax_tree: &AnnotatedSyntaxTree,
    context: Checker,
) -> Result<(), ParserInternalError> {
    let error = Diagnostic::new(
        DiagnosticKind::CrossConflictSymbolDeclarationError {
            symbol: problem_declaration.symbol().clone(),
            problem_kind: problem_declaration.kind().clone(),
            domain_kinds,
        },
        context.into(),
        problem_syntax_tree.filename().clone(),
        problem_declaration.span().clone(),
    );

    diagnostic_manager.add_diagnostic(error);
    Ok(())
}


/// Checks whether a given symbol declaration should be exempt from conflict checks.
///
/// This function returns `true` if the declaration's kind is among those
/// that are considered exempt from conflict checking, specifically
/// `DomainName` and `ProblemName`. Such declarations typically represent
/// special symbols that do not participate in regular symbol conflict rules.
///
/// # Arguments
///
/// * `declaration` - A reference to the `Declaration` to check.
///
/// # Returns
///
/// * `true` if the declaration kind is `DomainName` or `ProblemName`, meaning it
///   should be skipped during conflict checks.
/// * `false` otherwise.
///
/// # Examples
///
/// ```rust
/// let decl = get_some_declaration();
/// if is_declaration_exempt_from_conflict_check(&decl) {
///     println!("Declaration is exempt from conflict checks.");
/// } else {
///     println!("Declaration must be checked for conflicts.");
/// }
/// ```
fn is_declaration_exempt_from_conflict_check(declaration: &Declaration) -> bool {
    matches!(
        declaration.kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    )
}
