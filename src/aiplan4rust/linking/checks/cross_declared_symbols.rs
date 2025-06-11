use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::hir::HirTree;
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::semantic::symbol::SymbolSource;

/// Checks for conflicting symbol declarations between the problem and domain syntax trees.
///
/// This function verifies that no symbol declared in the problem conflicts with
/// existing declarations in the domain. Specifically, it detects symbols that share
/// the same name but differ in kind between the problem and domain declarations.
/// When such conflicts are found, diagnostic errors are emitted.
///
/// # Parameters
///
/// - `domain`: Reference to the domain's annotated syntax tree, containing its symbol table.
/// - `problem`: Reference to the problem's annotated syntax tree, containing its symbol table.
/// - `source`: The `DiagnosticSource` identifying the analysis phase producing diagnostics.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where conflict diagnostics
///   are recorded.
///
/// # Returns
///
/// - `Ok(true)` if no conflicting declarations were detected.
/// - `Ok(false)` if conflicts were found and reported.
/// - `Err(ParserInternalError)` if an internal error occurs during checking.
///
/// # Behavior
///
/// The function iterates over all symbols declared in the problem's symbol table. For each
/// declaration originating from the problem (and not exempt from conflict checks), it
/// checks whether the domain declares symbols with the same name. If the domain declares
/// symbols of a different kind, a conflict diagnostic is generated and recorded.
///
/// # Steps
///
/// 1. Initialize a success flag.
/// 2. Retrieve symbol tables from domain and problem.
/// 3. Iterate over problem symbols and their declarations.
/// 4. Skip declarations exempt from conflict checks or not from the problem source.
/// 5. Check if the domain declares the same symbol name.
/// 6. Gather kinds of the domain's relevant declarations.
/// 7. Check if any domain kind matches the problem declaration kind.
/// 8. If no match, report a conflict error and update the success flag.
/// 9. Return the overall success status.
///
/// # Example
///
/// ```rust
/// let result = check_cross_declared_symbols(
///     &domain_ast,
///     &problem_ast,
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
///
/// match result {
///     Ok(true) => println!("No conflicts found."),
///     Ok(false) => println!("Conflicting declarations detected."),
///     Err(e) => eprintln!("Internal error: {}", e),
/// }
/// ```
pub fn check_cross_declared_symbols(
    domain: &HirTree,
    problem: &HirTree,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
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
                && *declaration.source() == SymbolSource::Problem
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
                            declaration,
                            domain_kinds,
                            problem.filename(),
                            source,
                            diagnostic_manager,

                        );

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
/// problem declaration and provided filename.
///
/// # Parameters
/// - `problem_declaration`: The `Declaration` in the problem syntax tree that
///   conflicts with existing domain declarations.
/// - `domain_kinds`: A vector of `SymbolKind` representing the conflicting
///   declaration kinds found in the domain syntax tree.
/// - `filename`: The name of the source file containing the problem declaration.
/// - `source`: The `DiagnosticSource` identifying the analysis phase producing this diagnostic.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where the
///   error will be recorded.
///
/// # Returns
/// This function does not return a `Result` because it does not fail under normal conditions.
///
/// # Example
/// ```rust
/// report_cross_conflict_symbol_error(
///     &problem_declaration,
///     domain_kinds,
///     "problem_file.pddl",
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
/// ```
fn report_cross_conflict_symbol_error(
    problem_declaration: &Declaration,
    domain_kinds: Vec<SymbolKind>,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
)  {
    let error = Diagnostic::new(
        DiagnosticKind::CrossConflictSymbolDeclarationError {
            symbol: problem_declaration.symbol().clone(),
            problem_kind: problem_declaration.kind().clone(),
            domain_kinds,
        },
        source,
        filename.to_string(),
        problem_declaration.span().clone(),
    );

    diagnostic_manager.add_diagnostic(error);
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
